<script lang="ts">
	/**
	 * Settings → Agents: what an agent is, the agents that report to this
	 * server and how each one is doing, then the enrolment tokens.
	 *
	 * The agents are read from `GET /api/relays` (every agent device, with its
	 * relay mode, site and the devices it probes) and their state from the
	 * app-wide store, the same truth as the rack. A token is shown in clear
	 * once, right after creation, together with the install commands. After
	 * that only its prefix is ever displayed.
	 */
	import { ArrowRight, Cpu, KeyRound, RadioTower } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { createAgentToken, listAgentTokens, revokeAgentToken, type AgentToken, type CreatedAgentToken, type RelayAgent } from '#lib/api/index.js';
	import { listRelays } from '#lib/api/relay.js';
	import { displayState, formatDateTime, formatRelative, parseServerDate, STATE_LABEL, STATE_TONE, type TargetState } from '#lib/format.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, CopyBlock, EmptyState, ErrorNotice, Field, Led, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import AgentChecksums from '#lib/components/device-form/AgentChecksums.svelte';

	// --- The agents themselves -----------------------------------------------

	/** `null` while unread, or when this server does not list them. */
	let agents = $state<RelayAgent[] | null>(null);

	$effect(() => {
		const controller = new AbortController();
		listRelays(controller.signal)
			.then((list) => (agents = [...list].sort((a, b) => Number(b.relay) - Number(a.relay) || a.name.localeCompare(b.name))))
			.catch(() => (agents = null));
		return () => controller.abort();
	});

	/**
	 * The agent's state as the rack shows it. Before the shared store has the
	 * device, its last report decides: within three minutes is reporting.
	 */
	function agentState(agent: RelayAgent): TargetState {
		const target = alertsStore.targets.find((t) => t.id === agent.id);
		if (target) return displayState(target, alertsStore.probes.get(agent.id));
		const seen = agent.last_seen_at ? parseServerDate(agent.last_seen_at) : null;
		if (!seen) return 'pending';
		return Date.now() - seen.getTime() < 3 * 60_000 ? 'online' : 'offline';
	}

	let tokens = $state<AgentToken[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const list = await listAgentTokens(signal);
			tokens = Array.isArray(list) ? list : [];
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	// --- Create ---------------------------------------------------------------

	let name = $state('');
	let nameError = $state<string | null>(null);
	let creating = $state(false);
	let createError = $state<unknown>(null);
	let created = $state<CreatedAgentToken | null>(null);

	/**
	 * Scope of the token being created. Single use by default: an install
	 * command lives on in shell history and in chat logs, and one that enrols a
	 * whole fleet forever should be a decision, not what happens by accident.
	 */
	let reusable = $state(false);
	let maxUses = $state('');
	let expiresInDays = $state('');
	let scopeError = $state<string | null>(null);

	/** A number the user typed, or `null` for "left blank". Rejects nonsense. */
	function count(raw: string, message: string): number | null | 'error' {
		const text = raw.trim();
		if (!text) return null;
		const value = Number(text);
		if (!Number.isInteger(value) || value < 1) {
			scopeError = message;
			return 'error';
		}
		return value;
	}

	async function create(event: SubmitEvent) {
		event.preventDefault();
		createError = null;
		scopeError = null;
		if (!name.trim()) {
			nameError = m.settings_agents_name_required();
			return;
		}
		nameError = null;
		const uses = reusable ? count(maxUses, m.settings_agents_machines_invalid()) : null;
		const days = count(expiresInDays, m.settings_agents_days_invalid());
		if (uses === 'error' || days === 'error') return;
		creating = true;
		try {
			// `location.origin` is the address machines will reach this server at.
			created = await createAgentToken({
				name: name.trim(),
				base_url: location.origin,
				reusable,
				max_uses: uses,
				expires_in_days: days
			});
			tokens = [...tokens, created];
			name = '';
			maxUses = '';
			expiresInDays = '';
			reusable = false;
		} catch (cause) {
			createError = cause;
		} finally {
			creating = false;
		}
	}

	/** What a token can still do, in the fewest words that stay true. */
	function scopeLabel(token: AgentToken): string {
		if (token.max_uses === null) return m.settings_agents_scope_fleet({ uses: token.uses });
		if (token.max_uses === 1) return token.uses > 0 ? m.settings_agents_scope_single_used() : m.settings_agents_scope_single();
		return m.settings_agents_scope_fleet_ratio({ uses: token.uses, max: token.max_uses });
	}

	/** True once the token can no longer enrol, for any reason short of revocation. */
	function spent(token: AgentToken): boolean {
		if (token.max_uses !== null && token.uses >= token.max_uses) return true;
		return token.expires_at !== null && new Date(token.expires_at + 'Z') <= new Date();
	}

	// --- Revoke ---------------------------------------------------------------

	let revoking = $state<number | null>(null);
	let revokeError = $state<{ id: number; cause: unknown } | null>(null);

	async function revoke(token: AgentToken) {
		revoking = token.id;
		revokeError = null;
		try {
			await revokeAgentToken(token.id);
			tokens = await listAgentTokens();
			if (created?.id === token.id) created = null;
		} catch (cause) {
			revokeError = { id: token.id, cause };
		} finally {
			revoking = null;
		}
	}
</script>

<Panel
	id="agents"
	title={m.settings_agents_title()}
	description={m.settings_agents_description()}
	padded={false}
>
	{#snippet aside()}
		{#if !auth.isAdmin}
			<Plate tone="ghost" label={auth.readOnlyLabel} />
		{/if}
	{/snippet}

	<!-- The agents reporting here, each with its state and what it relays. -->
	{#if agents !== null}
		<div class="border-b border-line px-5 py-4">
			{#if agents.length === 0}
				<p class="text-sm text-ink-2">{m.settings_agents_none()}</p>
			{:else}
				<ul class="grid gap-1.5" role="list" aria-label={m.settings_agents_list_aria()}>
					{#each agents as agent (agent.id)}
						{@const state = agentState(agent)}
						<li>
							<a
								href={`/targets/${agent.id}`}
								class="group flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg border border-line bg-canvas-deep/50 px-3 py-2 transition-colors hover:border-line-strong hover:bg-surface-2"
							>
								<Led tone={STATE_TONE[state]} blink={state === 'offline' || state === 'down'} size="sm" />
								<span class="min-w-0 truncate font-semibold text-ink">{agent.name}</span>
								<Plate tone={STATE_TONE[state] === 'ghost' ? 'ghost' : STATE_TONE[state]} label={STATE_LABEL[state]} bare />
								{#if agent.relay}
									<Plate tone="info" title={m.settings_agents_relay_title()}>
										<RadioTower class="size-3.5" aria-hidden="true" />
										{agent.site ? m.settings_agents_relay_label_site({ site: agent.site }) : m.settings_agents_relay_label()}
									</Plate>
									<span class="text-[0.8125rem] text-ink-2">{m.settings_agents_probes({ count: agent.relayed })}</span>
								{:else if agent.relayed > 0}
									<Plate tone="advisory" label={m.settings_agents_relay_off({ count: agent.relayed })} title={m.settings_agents_relay_off_title()} />
								{:else}
									<span class="text-[0.8125rem] text-ink-2">{m.settings_agents_watches_own()}</span>
								{/if}
								<span class="tnum ml-auto text-[0.8125rem] text-ink-2">
									<time title={formatDateTime(agent.last_seen_at)}>{m.settings_agents_last_report({ when: formatRelative(agent.last_seen_at) })}</time>
								</span>
							</a>
						</li>
					{/each}
				</ul>
			{/if}
			{#if auth.isAdmin}
				<div class="mt-3 flex flex-wrap gap-2">
					<Button variant="secondary" size="sm" href="/targets/new?kind=agent">
						<Cpu class="size-4" aria-hidden="true" />
						{m.settings_agents_install()}
					</Button>
					<Button variant="ghost" size="sm" href="/targets/new?kind=agent&via=relay">
						<RadioTower class="size-4" aria-hidden="true" />
						{m.settings_agents_watch_remote()}
						<ArrowRight class="size-3.5" aria-hidden="true" />
					</Button>
				</div>
			{/if}
		</div>
	{/if}

	<div class="px-5 py-4">
		<h3 class="text-sm font-semibold text-ink">{m.settings_agents_tokens_title()}</h3>
		<p class="mt-0.5 mb-3 text-sm text-ink-2">
			{m.settings_agents_tokens_intro()}
		</p>
		{#if auth.isAdmin}
		<form class="flex flex-col gap-3 sm:flex-row sm:items-start" onsubmit={create} novalidate>
			<Field label={m.settings_agents_new_token()} for="token-name" error={nameError} class="flex-1" help={m.settings_agents_new_token_help()}>
				<input
					id="token-name"
					type="text"
					class="input"
					bind:value={name}
					placeholder={m.settings_agents_name_placeholder()}
					autocomplete="off"
					disabled={creating}
					aria-invalid={nameError ? 'true' : undefined}
					oninput={() => (nameError = null)}
				/>
			</Field>
			<!-- Offset by the label height so the button sits level with the input. -->
			<Button type="submit" variant="secondary" class="sm:mt-[1.625rem]" loading={creating}>
				<KeyRound class="size-4" aria-hidden="true" />
				{m.settings_agents_create()}
			</Button>
		</form>

		<div class="mt-3 flex flex-col gap-3 rounded-[var(--radius-card)] border border-line bg-canvas-deep px-4 py-3">
			<div class="flex items-start gap-3">
				<Toggle id="token-reusable" bind:checked={reusable} label={m.settings_agents_reusable()} />
				<div class="min-w-0">
					<label for="token-reusable" class="text-sm font-semibold text-ink">{m.settings_agents_reusable()}</label>
					<p class="text-sm text-ink-2">
						{m.settings_agents_reusable_help()}
					</p>
				</div>
			</div>
			<div class="flex flex-col gap-3 sm:flex-row">
				{#if reusable}
					<Field label={m.settings_agents_machines()} for="token-max-uses" class="flex-1" help={m.settings_agents_machines_help()}>
						<input id="token-max-uses" type="number" min="1" step="1" class="input" bind:value={maxUses} placeholder={m.settings_agents_no_limit()} disabled={creating} />
					</Field>
				{/if}
				<Field label={m.settings_agents_expires()} for="token-expires" class="flex-1" help={m.settings_agents_expires_help()}>
					<input id="token-expires" type="number" min="1" step="1" class="input" bind:value={expiresInDays} placeholder={m.settings_agents_no_deadline()} disabled={creating} />
				</Field>
			</div>
			{#if scopeError}
				<p class="text-sm text-warning">{scopeError}</p>
			{/if}
		</div>

		{#if createError}
			<ErrorNotice error={createError} title={m.settings_agents_create_error()} class="mt-3" />
		{/if}
		{/if}

		<div aria-live="polite">
			{#if created}
				<div class="rise-in mt-4 rounded-[var(--radius-card)] border border-advisory/40 bg-surface p-4">
					<div class="flex flex-wrap items-center justify-between gap-2">
						<div class="flex flex-wrap items-center gap-2">
							<p class="font-semibold text-ink">{m.settings_agents_created_title({ name: created.name })}</p>
							<Plate tone="advisory" label={m.settings_agents_shown_once()} />
						</div>
						<Button variant="ghost" size="sm" onclick={() => (created = null)}>{m.settings_agents_copied()}</Button>
					</div>
					<div class="mt-4 grid gap-4">
						<div>
							<p class="mb-1.5 text-sm font-semibold text-ink">{m.settings_agents_token()}</p>
							<CopyBlock value={created.secret} label={m.settings_agents_copy_token()} secret />
						</div>
						{#if created.install_linux}
							<div>
								<p class="mb-1.5 text-sm font-semibold text-ink">{m.settings_agents_install_linux()}</p>
								<CopyBlock value={created.install_linux} label={m.settings_agents_copy_command()} />
							</div>
						{/if}
						{#if created.install_windows}
							<div>
								<p class="mb-1.5 text-sm font-semibold text-ink">{m.settings_agents_install_windows()}</p>
								<CopyBlock value={created.install_windows} label={m.settings_agents_copy_command()} />
							</div>
						{/if}
						<AgentChecksums />
					</div>
				</div>
			{/if}
		</div>

		<div class={auth.isAdmin ? 'mt-4' : ''}>
			{#if error}
				<ErrorNotice {error} title={m.settings_agents_load_error()} onretry={() => void load()} />
			{:else if loading}
				<Skeleton class="h-14 w-full" />
			{:else if tokens.length === 0}
				<EmptyState icon={Cpu} title={m.settings_agents_empty_title()} description={m.settings_agents_empty_description()} />
			{:else}
				<ul class="divide-y divide-line rounded-[var(--radius-card)] border border-line" role="list">
					{#each tokens as token (token.id)}
						{@const revoked = token.revoked_at !== null}
						<li class={`px-4 py-3 ${revoked ? 'ghost-cell' : ''}`}>
							<div class="flex flex-wrap items-center gap-x-3 gap-y-2">
								<div class="min-w-0 flex-1">
									<div class="flex flex-wrap items-center gap-2">
										<span class={`font-semibold ${revoked ? 'text-ink-2' : 'text-ink'}`}>{token.name}</span>
										<code class="rounded-md border border-line bg-canvas-deep px-1.5 py-0.5 font-mono text-[0.75rem] text-ink-2">{token.prefix}…</code>
										{#if revoked}<Plate tone="ghost" label={m.settings_agents_revoked()} />{/if}
										{#if !revoked}
											<Plate tone={token.max_uses === null ? 'info' : 'ghost'} label={scopeLabel(token)} />
											{#if spent(token)}<Plate tone="advisory" label={m.settings_agents_enrols_no_more()} />{/if}
										{/if}
									</div>
									<p class="mt-1 text-sm text-ink-2">
										<time class="tnum" title={formatDateTime(token.created_at)}>{m.settings_agents_created_at({ when: formatRelative(token.created_at) })}</time>
										· <time class="tnum" title={formatDateTime(token.last_used_at)}>{m.settings_agents_last_used({ when: formatRelative(token.last_used_at) })}</time>
										{#if token.expires_at}
											· <time class="tnum" title={formatDateTime(token.expires_at)}>{m.settings_agents_stops_at({ when: formatRelative(token.expires_at) })}</time>
										{/if}
										{#if revoked}
											· <time class="tnum" title={formatDateTime(token.revoked_at)}>{m.settings_agents_revoked_at({ when: formatRelative(token.revoked_at) })}</time>
										{/if}
									</p>
									{#if !revoked && spent(token)}
										<p class="mt-1 text-sm text-ink-2">
											{m.settings_agents_spent_note()}
										</p>
									{/if}
								</div>
								{#if !revoked && auth.isAdmin}
									<Confirm confirmLabel={m.settings_agents_revoke_confirm()} loading={revoking === token.id} onconfirm={() => revoke(token)}>{m.settings_agents_revoke()}</Confirm>
								{/if}
							</div>
							{#if revokeError?.id === token.id}
								<ErrorNotice error={revokeError.cause} title={m.settings_agents_revoke_error()} class="mt-3" />
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>
</Panel>
