<script lang="ts">
	import { withBase } from '#lib/paths.js';
	/**
	 * "Deploy with Active Directory": a trigger button and its modal wizard.
	 *
	 * Four steps: make a fleet token and its deployment script, download the
	 * script, run it on a domain controller, then watch the servers enrol. The
	 * token and the script exist in clear only in this session (the server never
	 * shows them again), so closing the dialog forgets them. Admin only: the
	 * caller decides whether to render it.
	 *
	 * A domain at a site behind a firewall installs "through a relay": the
	 * servers download the agent from the relay's gateway and report through
	 * it, end-to-end encrypted to this server.
	 */
	import { tick } from 'svelte';
	import { ArrowLeft, ArrowRight, Check, Download, KeyRound, Network, OctagonX, X } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { closeAgentToken, createAdKit, listAgentTokens, listTargets, type CreatedAdKit, type RelayAgent, type Target } from '#lib/api/index.js';
	import { listRelays } from '#lib/api/relay.js';
	import { Button, Confirm, CopyBlock, DocLink, ErrorNotice, Field, Led, Plate } from '#lib/ui/index.js';

	interface Props {
		/** Called whenever the token list changed (created, closed), so the page can reload it. */
		onchange?: () => void;
		class?: string;
	}
	let { onchange, class: className = '' }: Props = $props();

	const POLL_MS = 5000;
	const RECENT = 8;

	let open = $state(false);
	let step = $state(1);
	let dialog = $state<HTMLElement | null>(null);
	let returnFocus: HTMLElement | null = null;

	// Step 1
	let domain = $state('');
	let maxUses = $state('');
	/** Which servers get the agent: the whole domain (future servers included), one OU or one group. */
	let scope = $state<'all' | 'ou' | 'group'>('all');
	let targetOu = $state('');
	let securityGroup = $state('');
	let includeDcs = $state(false);
	let sysvol = $state(false);
	/** `null`: not chosen yet, so the default follows the scope (never expires for the whole domain). */
	let validityChoice = $state<'30' | '365' | 'never' | null>(null);
	const validity = $derived(validityChoice ?? (scope === 'all' ? 'never' : '30'));
	let scopeError = $state<string | null>(null);
	let serverHost = $state('');
	/** Relay agents of a remote site the domain may install through. */
	let relays = $state<RelayAgent[]>([]);
	/** `''`: direct; otherwise the id of the relay, as the select gives it. */
	let relayChoice = $state('');
	let relayHost = $state('');
	const relay = $derived(relays.find((r) => String(r.id) === relayChoice) ?? null);
	$effect(() => {
		if (!open) return;
		const controller = new AbortController();
		listRelays(controller.signal)
			.then((list) => (relays = list.filter((r) => r.relay).sort((a, b) => a.name.localeCompare(b.name))))
			.catch(() => (relays = []));
		return () => controller.abort();
	});
	let dnsServer = $state('');
	let serverHostError = $state<string | null>(null);
	let dnsServerError = $state<string | null>(null);
	let domainError = $state<string | null>(null);
	let numberError = $state<string | null>(null);
	let creating = $state(false);
	let createError = $state<unknown>(null);
	let kit = $state<CreatedAdKit | null>(null);

	// Step 2
	let downloaded = $state(false);

	// Step 4
	let uses = $state(0);
	let max = $state<number | null>(null);
	let servers = $state<Target[]>([]);
	let pollFailed = $state(false);
	let closed = $state(false);
	let closing = $state(false);
	let closeError = $state<unknown>(null);

	const STEPS = [m.settings_agents_ad_step1, m.settings_agents_ad_step2, m.settings_agents_ad_step3, m.settings_agents_ad_step4];
	const RUN = String.raw`Unblock-File .\Deploy-DumbMonit.ps1; powershell -NoProfile -ExecutionPolicy Bypass -File .\Deploy-DumbMonit.ps1`;

	/** The exact command for the options chosen: it changes live with the form. */
	const command = $derived.by(() => {
		let line = RUN;
		if (scope === 'all') line += ' -AllServers';
		else if (scope === 'ou' && targetOu.trim()) line += ` -TargetOU "${targetOu.trim()}"`;
		else if (scope === 'group' && securityGroup.trim()) line += ` -SecurityGroup "${securityGroup.trim()}"`;
		if (scope !== 'all' && includeDcs) line += ' -IncludeDomainControllers';
		if (sysvol) line += ' -UseSysvol';
		return line;
	});
	const previewCommand = $derived(`${command} -WhatIf`);

	/** Same checks as the server's `optional_field`, plus what could break out of a double-quoted PowerShell string. */
	function safeText(text: string): boolean {
		return text.length <= 512 && !/[\p{Cc}"'`$;\u2018\u2019\u201C\u201D]/u.test(text);
	}

	function show(event?: Event) {
		returnFocus = (event?.currentTarget as HTMLElement | null) ?? (document.activeElement as HTMLElement | null);
		open = true;
		void tick().then(() => dialog?.focus());
	}

	function close() {
		open = false;
		returnFocus?.focus();
		// The secret is gone for good once the dialog is closed.
		kit = null;
		serverHostError = null;
		dnsServerError = null;
		step = 1;
		scope = 'all';
		targetOu = '';
		securityGroup = '';
		includeDcs = false;
		sysvol = false;
		validityChoice = null;
		scopeError = null;
		downloaded = false;
		closed = false;
		closeError = null;
		createError = null;
		servers = [];
		uses = 0;
		max = null;
		pollFailed = false;
	}

	/** DNS name (labels of letters, digits, `-`, `_`) or IPv4 address, optional `:port`. */
	const HOST_RE = /^(?=.{1,253}$)([A-Za-z0-9_]([A-Za-z0-9_-]{0,61}[A-Za-z0-9_])?)(\.[A-Za-z0-9_]([A-Za-z0-9_-]{0,61}[A-Za-z0-9_])?)*\.?(:\d{1,5})?$/;

	function validHost(raw: string): boolean {
		const text = raw.trim();
		if (!HOST_RE.test(text)) return false;
		const port = text.includes(':') ? Number(text.split(':')[1]) : 1;
		return port >= 1 && port <= 65535;
	}

	/** IPv4 (four 0-255 groups) or a plain IPv6 literal. */
	function validIp(raw: string): boolean {
		const text = raw.trim();
		const v4 = text.match(/^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/);
		if (v4) return v4.slice(1).every((n) => Number(n) <= 255);
		return text.includes(':') && /^[0-9A-Fa-f:.]+$/.test(text);
	}

	/** A whole number >= 1, `null` when blank, `'error'` otherwise. */
	function whole(raw: string): number | null | 'error' {
		const text = raw.trim();
		if (!text) return null;
		const value = Number(text);
		return Number.isInteger(value) && value >= 1 ? value : 'error';
	}

	async function create(event: SubmitEvent) {
		event.preventDefault();
		createError = null;
		domainError = null;
		numberError = null;
		scopeError = null;
		serverHostError = null;
		dnsServerError = null;
		if (!domain.trim()) {
			domainError = m.settings_agents_ad_domain_required();
			return;
		}
		if (!relay && serverHost.trim() && !validHost(serverHost)) {
			serverHostError = m.settings_agents_ad_server_host_invalid();
			return;
		}
		if (dnsServer.trim() && !validIp(dnsServer)) {
			dnsServerError = m.settings_agents_ad_dns_server_invalid();
			return;
		}
		if (scope === 'ou') {
			const dn = targetOu.trim();
			if (!safeText(dn) || !/(^|,)\s*DC=[^,]+/i.test(dn)) {
				scopeError = m.settings_agents_ad_ou_invalid();
				return;
			}
		} else if (scope === 'group') {
			const name = securityGroup.trim();
			if (!name || !safeText(name)) {
				scopeError = m.settings_agents_ad_group_invalid();
				return;
			}
		}
		const uses_ = whole(maxUses);
		if (uses_ === 'error') {
			numberError = m.settings_agents_ad_number_invalid();
			return;
		}
		creating = true;
		try {
			kit = await createAdKit({
				domain: domain.trim(),
				// The address the servers will reach this one at.
				base_url: location.origin,
				server_host: relay ? null : serverHost.trim() || null,
				...(relay ? { relay_id: relay.id, relay_host: relayHost.trim() || null } : {}),
				dns_server: dnsServer.trim() || null,
				max_uses: uses_,
				// `null`: the token never expires (still limited to this domain, revocable).
				expires_in_days: validity === 'never' ? null : Number(validity),
				all_servers: scope === 'all',
				target_ou: scope === 'ou' ? targetOu.trim() : null,
				include_domain_controllers: scope !== 'all' && includeDcs,
				security_group: scope === 'group' ? securityGroup.trim() : null,
				sysvol_binary: sysvol
			});
			uses = kit.token.uses;
			max = kit.token.max_uses;
			step = 2;
			onchange?.();
		} catch (cause) {
			createError = cause;
		} finally {
			creating = false;
		}
	}

	function download() {
		if (!kit) return;
		const url = URL.createObjectURL(new Blob([kit.script], { type: 'text/plain;charset=utf-8' }));
		const link = document.createElement('a');
		link.href = url;
		link.download = kit.filename;
		document.body.append(link);
		link.click();
		link.remove();
		setTimeout(() => URL.revokeObjectURL(url), 1000);
		downloaded = true;
	}

	async function stop() {
		if (!kit) return;
		closing = true;
		closeError = null;
		try {
			await closeAgentToken(kit.token.id);
			closed = true;
			onchange?.();
		} catch (cause) {
			closeError = cause;
		} finally {
			closing = false;
		}
	}

	// Live count and latest servers, only while step 4 is on screen.
	$effect(() => {
		if (!open || step !== 4 || !kit) return;
		const controller = new AbortController();
		const { signal } = controller;
		const id = kit.token.id;
		const folder = kit.folder;
		async function poll() {
			try {
				const [tokens, targets] = await Promise.all([
					listAgentTokens(signal),
					listTargets(signal).catch((cause) => {
						if (signal.aborted) throw cause;
						return null;
					})
				]);
				if (signal.aborted) return;
				const mine = tokens.find((t) => t.id === id);
				if (mine) {
					uses = mine.uses;
					max = mine.max_uses;
				}
				if (targets) servers = targets.filter((t) => t.group_name === folder).slice(-RECENT).reverse();
				pollFailed = false;
			} catch {
				if (!signal.aborted) pollFailed = true;
			}
		}
		void poll();
		const timer = setInterval(poll, POLL_MS);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			close();
		} else if (event.key === 'Tab' && dialog) {
			const items = [...dialog.querySelectorAll<HTMLElement>('a[href], button:not([disabled]), input:not([disabled])')].filter((el) => el.offsetParent !== null);
			if (items.length === 0) return;
			const first = items[0];
			const final = items[items.length - 1];
			const active = document.activeElement;
			if (event.shiftKey && (active === first || active === dialog)) {
				event.preventDefault();
				final.focus();
			} else if (!event.shiftKey && active === final) {
				event.preventDefault();
				first.focus();
			}
		}
	}
</script>

<Button variant="secondary" size="sm" class={className} onclick={show} aria-haspopup="dialog">
	<Network class="size-4" aria-hidden="true" />
	{m.settings_agents_ad_button()}
</Button>

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div class="fixed inset-0 z-[60] bg-black/50" aria-hidden="true" onclick={close}></div>
	<div
		bind:this={dialog}
		role="dialog"
		aria-modal="true"
		aria-labelledby="ad-title"
		tabindex="-1"
		{onkeydown}
		class="fixed inset-x-0 bottom-0 z-[62] max-h-[92dvh] overflow-x-hidden overflow-y-auto rounded-t-[var(--radius-card)] border border-line bg-surface p-5 pb-[calc(1.25rem+var(--safe-bottom))] text-ink shadow-float outline-none sm:inset-x-auto sm:top-1/2 sm:bottom-auto sm:left-1/2 sm:w-[44rem] sm:max-w-[calc(100vw-2rem)] sm:-translate-x-1/2 sm:-translate-y-1/2 sm:rounded-[var(--radius-card)] sm:pb-5"
	>
		<div class="flex items-start justify-between gap-3">
			<div class="min-w-0">
				<h2 id="ad-title" class="text-lg font-semibold">{m.settings_agents_ad_title()}</h2>
				<p class="mt-1 text-sm text-ink-2">{m.settings_agents_ad_description()}</p>
			</div>
			<button
				type="button"
				class="-mt-1.5 -mr-2 inline-flex size-10 shrink-0 items-center justify-center rounded-md text-ink-3 hover:bg-surface-2 hover:text-ink"
				aria-label={m.settings_agents_ad_close()}
				onclick={close}
			>
				<X class="size-4" aria-hidden="true" />
			</button>
		</div>

		<!-- Progress: a segment per step, with the step named in words. -->
		<div class="mt-4" aria-live="polite">
			<ol class="flex gap-1.5" role="list" aria-hidden="true">
				{#each STEPS as _, i (i)}
					<li class={`h-1 flex-1 rounded-full ${i + 1 <= step ? 'bg-signal' : 'bg-line'}`}></li>
				{/each}
			</ol>
			<p class="mt-2 text-sm text-ink-2">
				{m.settings_agents_ad_step({ n: step })} · <span class="font-semibold text-ink">{STEPS[step - 1]()}</span>
			</p>
		</div>

		<div class="mt-4">
			{#if step === 1}
				<form class="flex flex-col gap-3" onsubmit={create} novalidate>
					<Field label={m.settings_agents_ad_domain()} for="ad-domain" error={domainError} help={m.settings_agents_ad_domain_help()}>
						<input
							id="ad-domain"
							type="text"
							class="input"
							bind:value={domain}
							placeholder={m.settings_agents_ad_domain_placeholder()}
							autocomplete="off"
							autocapitalize="none"
							spellcheck="false"
							disabled={creating}
							aria-invalid={domainError ? 'true' : undefined}
							oninput={() => (domainError = null)}
						/>
					</Field>
					{#if relays.length > 0}
						<div class="flex flex-col gap-3 sm:flex-row">
							<Field label={m.settings_agents_ad_route()} for="ad-route" class="flex-1" help={m.settings_agents_ad_route_help()}>
								<select id="ad-route" class="input" bind:value={relayChoice} disabled={creating}>
									<option value="">{m.deviceform_agentflow_route_direct()}</option>
									{#each relays as option (option.id)}
										<option value={String(option.id)}>
											{option.site ? m.deviceform_agentflow_route_relay_site({ relay: option.name, site: option.site }) : m.deviceform_agentflow_route_relay({ relay: option.name })}
										</option>
									{/each}
								</select>
							</Field>
							{#if relay}
								<Field label={m.deviceform_agentflow_relay_host()} for="ad-relay-host" class="flex-1" help={m.deviceform_agentflow_relay_host_help({ port: String(relay.gateway_wanted ?? relay.gateway_port ?? 8443) })}>
									<input id="ad-relay-host" type="text" class="input" bind:value={relayHost} placeholder={relay.gateway_host ?? '192.168.1.10'} autocomplete="off" spellcheck="false" disabled={creating} />
								</Field>
							{/if}
						</div>
					{/if}
					<div class="flex flex-col gap-3 sm:flex-row">
						{#if !relay}
						<Field label={m.settings_agents_ad_server_host()} for="ad-server-host" class="flex-1" error={serverHostError} help={m.settings_agents_ad_server_host_help({ origin: location.host })}>
							<input
								id="ad-server-host"
								type="text"
								class="input"
								bind:value={serverHost}
								placeholder={location.hostname}
								autocomplete="off"
								autocapitalize="none"
								spellcheck="false"
								disabled={creating}
								aria-invalid={serverHostError ? 'true' : undefined}
								oninput={() => (serverHostError = null)}
							/>
						</Field>
						{/if}
						<Field label={m.settings_agents_ad_dns_server()} for="ad-dns-server" class="flex-1" error={dnsServerError} help={m.settings_agents_ad_dns_server_help()}>
							<input
								id="ad-dns-server"
								type="text"
								class="input"
								bind:value={dnsServer}
								placeholder="10.0.0.2"
								autocomplete="off"
								spellcheck="false"
								disabled={creating}
								aria-invalid={dnsServerError ? 'true' : undefined}
								oninput={() => (dnsServerError = null)}
							/>
						</Field>
					</div>
					<div class="flex flex-col gap-3 sm:flex-row">
						<Field label={m.settings_agents_ad_max()} for="ad-max" class="flex-1" help={m.settings_agents_ad_max_help()}>
							<input id="ad-max" type="number" min="1" step="1" inputmode="numeric" class="input" bind:value={maxUses} placeholder={m.settings_agents_ad_unlimited()} disabled={creating} />
						</Field>
						<Field label={m.settings_agents_ad_validity()} for="ad-validity" class="flex-1" help={m.settings_agents_ad_days_help()}>
							<select id="ad-validity" class="input" value={validity} onchange={(e) => (validityChoice = e.currentTarget.value as '30' | '365' | 'never')} disabled={creating}>
								<option value="30">{m.settings_agents_ad_validity_30()}</option>
								<option value="365">{m.settings_agents_ad_validity_365()}</option>
								<option value="never">{m.settings_agents_ad_validity_never()}</option>
							</select>
						</Field>
					</div>
					{#if numberError}<p class="text-sm text-warning" role="alert">{numberError}</p>{/if}
					{#if validity === 'never'}
						<p class="rounded-lg border border-advisory/40 bg-surface-2 px-3.5 py-2.5 text-sm text-ink">{m.settings_agents_ad_validity_never_warning()}</p>
					{/if}

					<fieldset class="flex flex-col gap-2 rounded-lg border border-line px-3.5 py-3" disabled={creating}>
						<legend class="px-1 text-sm font-semibold text-ink">{m.settings_agents_ad_scope()}</legend>
						<div>
							<label class="flex min-h-10 items-start gap-2 py-2 text-sm text-ink">
								<input type="radio" name="ad-scope" value="all" class="mt-0.5 size-4 shrink-0 accent-[var(--color-signal)]" bind:group={scope} onchange={() => (scopeError = null)} />
								<span>{m.settings_agents_ad_scope_all()}</span>
							</label>
						</div>
						<div>
							<label class="flex min-h-10 items-start gap-2 py-2 text-sm text-ink">
								<input type="radio" name="ad-scope" value="ou" class="mt-0.5 size-4 shrink-0 accent-[var(--color-signal)]" bind:group={scope} onchange={() => (scopeError = null)} />
								<span>{m.settings_agents_ad_scope_ou()}</span>
							</label>
							{#if scope === 'ou'}
								<Field label={m.settings_agents_ad_ou_field()} for="ad-ou" class="mt-2 pl-6">
									<input id="ad-ou" type="text" class="input" bind:value={targetOu} placeholder="OU=Servers,DC=corp,DC=example,DC=com" autocomplete="off" autocapitalize="none" spellcheck="false" oninput={() => (scopeError = null)} aria-invalid={scopeError ? 'true' : undefined} />
								</Field>
							{/if}
						</div>
						<div>
							<label class="flex min-h-10 items-start gap-2 py-2 text-sm text-ink">
								<input type="radio" name="ad-scope" value="group" class="mt-0.5 size-4 shrink-0 accent-[var(--color-signal)]" bind:group={scope} onchange={() => (scopeError = null)} />
								<span>{m.settings_agents_ad_scope_group()}</span>
							</label>
							{#if scope === 'group'}
								<Field label={m.settings_agents_ad_group_field()} for="ad-group" class="mt-2 pl-6">
									<input id="ad-group" type="text" class="input" bind:value={securityGroup} placeholder="DumbMonit Servers" autocomplete="off" autocapitalize="none" spellcheck="false" oninput={() => (scopeError = null)} aria-invalid={scopeError ? 'true' : undefined} />
								</Field>
							{/if}
						</div>
						{#if scopeError}<p class="text-sm text-warning" role="alert">{scopeError}</p>{/if}
					</fieldset>

					<div>
						<label class="flex min-h-10 items-center gap-2 text-sm text-ink">
							<input type="checkbox" class="size-4 shrink-0 accent-[var(--color-signal)]" checked={scope === 'all' || includeDcs} onchange={(e) => (includeDcs = e.currentTarget.checked)} disabled={creating || scope === 'all'} />
							{m.settings_agents_ad_dcs()}
						</label>
					</div>
					<div>
						<label class="flex min-h-10 items-center gap-2 text-sm text-ink">
							<input type="checkbox" class="size-4 shrink-0 accent-[var(--color-signal)]" bind:checked={sysvol} disabled={creating} />
							{m.settings_agents_ad_sysvol()}
						</label>
					</div>

					<div class="min-w-0">
						<p class="mb-1 text-sm font-semibold text-ink">{m.settings_agents_ad_command_title()}</p>
							<CopyBlock value={command} label={m.settings_agents_copy_command()} />
					</div>

					{#if createError}
						<ErrorNotice error={createError} title={m.settings_agents_create_error()} />
					{/if}
					<div class="flex justify-end">
						<Button type="submit" variant="primary" loading={creating}>
							<KeyRound class="size-4" aria-hidden="true" />
							{m.settings_agents_ad_create()}
						</Button>
					</div>
				</form>
			{:else if step === 2}
				<div class="mt-3 flex flex-wrap items-center gap-3">
					<Button variant="primary" onclick={download} feedback>
						<Download class="size-4" aria-hidden="true" />
						{m.settings_agents_ad_download()}
					</Button>
					{#if downloaded}
						<Plate tone="signal"><Check class="size-3.5" aria-hidden="true" />{m.settings_agents_ad_downloaded()}</Plate>
					{/if}
				</div>
				<div class="mt-3 min-w-0">
					<CopyBlock value={command} label={m.settings_agents_copy_command()} />
				</div>
				<p class="mt-3 rounded-lg border border-advisory/40 bg-surface-2 px-3.5 py-3 text-sm text-ink">{m.settings_agents_ad_secret_warning()}</p>
				{#if kit}
					<dl class="mt-3 grid gap-1 rounded-lg border border-line px-3.5 py-3 text-sm">
						<div class="flex flex-wrap gap-x-2">
							<dt class="text-ink-2">{m.settings_agents_ad_summary_server()}</dt>
							<dd class="min-w-0 break-all font-mono text-ink">{kit.server_url}</dd>
						</div>
						<div class="flex flex-wrap gap-x-2">
							<dt class="text-ink-2">{m.settings_agents_ad_summary_dns()}</dt>
							<dd class="min-w-0 break-all font-mono text-ink">{kit.dns_server ?? m.settings_agents_ad_summary_dns_none()}</dd>
						</div>
					</dl>
				{/if}
			{:else if step === 3}
				<div class="mt-3 grid gap-3">
					<div class="min-w-0">
						<p class="mb-1.5 text-sm font-semibold text-ink">{m.settings_agents_ad_run_preview()}</p>
						<CopyBlock value={previewCommand} label={m.settings_agents_copy_command()} />
					</div>
					<div class="min-w-0">
						<p class="mb-1.5 text-sm font-semibold text-ink">{m.settings_agents_ad_run_apply()}</p>
						<CopyBlock value={command} label={m.settings_agents_copy_command()} />
					</div>
				</div>
				<DocLink page="devices/agent-active-directory/" class="mt-2" />
			{:else}
				<div class="mt-3 flex flex-wrap items-center gap-3 rounded-lg border border-line bg-canvas-deep/50 px-4 py-3" aria-live="polite">
					<Led tone={uses > 0 ? 'signal' : 'ghost'} blink={uses === 0} />
					<p class="tnum text-lg font-semibold text-ink">
						{max === null ? m.settings_agents_ad_count({ count: uses }) : m.settings_agents_ad_count_of({ count: uses, max })}
					</p>
					{#if uses === 0}<span class="text-sm text-ink-2">{m.settings_agents_ad_waiting()}</span>{/if}
				</div>
				{#if pollFailed}<p class="mt-2 text-sm text-warning" role="status">{m.settings_agents_ad_polling_error()}</p>{/if}

				{#if servers.length > 0}
					<p class="mt-4 mb-1.5 text-sm font-semibold text-ink">{m.settings_agents_ad_recent()}</p>
					<ul class="grid gap-1.5" role="list">
						{#each servers as server (server.id)}
							<li class="min-w-0 truncate rounded-lg border border-line px-3 py-2 text-sm">
								<a href={withBase(`/targets/${server.id}`)} class="font-semibold text-ink hover:underline">{server.name}</a>
							</li>
						{/each}
					</ul>
				{/if}

				<div class="mt-4 rounded-lg border border-line px-3.5 py-3">
					<div class="mt-2 flex flex-wrap items-center gap-3">
						{#if closed}
							<Plate tone="advisory"><OctagonX class="size-3.5" aria-hidden="true" />{m.settings_agents_ad_stopped()}</Plate>
						{:else}
							<Confirm size="md" variant="secondary" confirmLabel={m.settings_agents_ad_token_stop_confirm()} loading={closing} onconfirm={stop}>
								{m.settings_agents_ad_stop()}
							</Confirm>
						{/if}
					</div>
					{#if closeError}<ErrorNotice error={closeError} title={m.settings_agents_ad_stop_error()} class="mt-3" />{/if}
				</div>
			{/if}
		</div>

		{#if step > 1}
			<div class="mt-5 flex flex-wrap items-center justify-between gap-2">
				{#if step < 4}
					<Button variant="ghost" onclick={() => (step -= 1)} disabled={step === 2}>
						<ArrowLeft class="size-4" aria-hidden="true" />
						{m.settings_agents_ad_back()}
					</Button>
					<Button variant="primary" onclick={() => (step += 1)}>
						{m.settings_agents_ad_next()}
						<ArrowRight class="size-4" aria-hidden="true" />
					</Button>
				{:else}
					<Button variant="ghost" onclick={() => (step -= 1)}>
						<ArrowLeft class="size-4" aria-hidden="true" />
						{m.settings_agents_ad_back()}
					</Button>
					<Button variant="primary" onclick={close}>{m.settings_agents_ad_done()}</Button>
				{/if}
			</div>
		{/if}
	</div>
{/if}
