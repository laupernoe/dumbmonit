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
	import { createAgentToken, listAgentTokens, revokeAgentToken, type AgentToken, type CreatedAgentToken, type RelayAgent } from '$lib/api';
	import { listRelays } from '$lib/api/relay';
	import { displayState, formatDateTime, formatRelative, parseServerDate, STATE_LABEL, STATE_TONE, type TargetState } from '$lib/format';
	import { alertsStore } from '$lib/stores/alerts.svelte';
	import { auth } from '$lib/stores/auth.svelte';
	import { Button, Confirm, CopyBlock, EmptyState, ErrorNotice, Field, Led, Panel, Plate, Skeleton, Toggle } from '$lib/ui';
	import AgentChecksums from '$lib/components/device-form/AgentChecksums.svelte';

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

	function devices(n: number): string {
		return `${n} device${n === 1 ? '' : 's'}`;
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
	function count(raw: string, what: string): number | null | 'error' {
		const text = raw.trim();
		if (!text) return null;
		const value = Number(text);
		if (!Number.isInteger(value) || value < 1) {
			scopeError = `${what} must be a whole number, one or more.`;
			return 'error';
		}
		return value;
	}

	async function create(event: SubmitEvent) {
		event.preventDefault();
		createError = null;
		scopeError = null;
		if (!name.trim()) {
			nameError = 'Name the token, for example after the machine it will enrol.';
			return;
		}
		nameError = null;
		const uses = reusable ? count(maxUses, 'The number of machines') : null;
		const days = count(expiresInDays, 'The number of days');
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
		if (token.max_uses === null) return `Fleet · ${token.uses} enrolled`;
		if (token.max_uses === 1) return token.uses > 0 ? 'Single use · used' : 'Single use';
		return `Fleet · ${token.uses}/${token.max_uses} enrolled`;
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
	title="Agents"
	description="An agent is one small program on a Linux, Windows, macOS or FreeBSD machine: it reports the system, disks, services, Docker containers and backups, and lets you restart or update a container from here. In relay mode it also probes, for this server, the devices of its own network — a second site, a client, anything behind a NAT — and only ever connects out."
	padded={false}
>
	{#snippet aside()}
		{#if !auth.isAdmin}
			<Plate tone="ghost" label="Viewer — read only" />
		{/if}
	{/snippet}

	<!-- The agents reporting here, each with its state and what it relays. -->
	{#if agents !== null}
		<div class="border-b border-line px-5 py-4">
			{#if agents.length === 0}
				<p class="text-sm text-ink-2">No agent has reported to this server yet.</p>
			{:else}
				<ul class="grid gap-1.5" role="list" aria-label="Agents reporting to this server">
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
									<Plate tone="info" title="Relay mode: it probes devices of its own network for this server">
										<RadioTower class="size-3.5" aria-hidden="true" />
										Relay{agent.site ? ` · ${agent.site}` : ''}
									</Plate>
									<span class="text-[0.8125rem] text-ink-2">probes {devices(agent.relayed)}</span>
								{:else if agent.relayed > 0}
									<Plate tone="advisory" label={`Relay off — ${devices(agent.relayed)} waiting`} title="Set relay: true on this agent so it picks up their probes" />
								{:else}
									<span class="text-[0.8125rem] text-ink-2">Watches its own machine</span>
								{/if}
								<span class="tnum ml-auto text-[0.8125rem] text-ink-2">
									Last report <time title={formatDateTime(agent.last_seen_at)}>{formatRelative(agent.last_seen_at)}</time>
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
						Install an agent
					</Button>
					<Button variant="ghost" size="sm" href="/targets/new?kind=agent&via=relay">
						<RadioTower class="size-4" aria-hidden="true" />
						Watch a remote site
						<ArrowRight class="size-3.5" aria-hidden="true" />
					</Button>
				</div>
			{/if}
		</div>
	{/if}

	<div class="px-5 py-4">
		<h3 class="text-sm font-semibold text-ink">Enrolment tokens</h3>
		<p class="mt-0.5 mb-3 text-sm text-ink-2">
			Adding a device of type agent makes a single-use token for you. Create one here for a fleet, a playbook or a machine image.
		</p>
		{#if auth.isAdmin}
		<form class="flex flex-col gap-3 sm:flex-row sm:items-start" onsubmit={create} novalidate>
			<Field label="New token" for="token-name" error={nameError} class="flex-1" help="Only used to recognise the token in this list.">
				<input
					id="token-name"
					type="text"
					class="input"
					bind:value={name}
					placeholder="File server"
					autocomplete="off"
					disabled={creating}
					aria-invalid={nameError ? 'true' : undefined}
					oninput={() => (nameError = null)}
				/>
			</Field>
			<!-- Offset by the label height so the button sits level with the input. -->
			<Button type="submit" variant="secondary" class="sm:mt-[1.625rem]" loading={creating}>
				<KeyRound class="size-4" aria-hidden="true" />
				Create token
			</Button>
		</form>

		<div class="mt-3 flex flex-col gap-3 rounded-[var(--radius-card)] border border-line bg-canvas-deep px-4 py-3">
			<div class="flex items-start gap-3">
				<Toggle id="token-reusable" bind:checked={reusable} label="Reusable for a fleet" />
				<div class="min-w-0">
					<label for="token-reusable" class="text-sm font-semibold text-ink">Reusable for a fleet</label>
					<p class="text-sm text-ink-2">
						Off, the token enrols one machine and then enrols nothing more — the right default for a single
						install. On, it can go into a playbook or an image.
					</p>
				</div>
			</div>
			<div class="flex flex-col gap-3 sm:flex-row">
				{#if reusable}
					<Field label="Machines it may enrol" for="token-max-uses" class="flex-1" help="Leave empty for no limit.">
						<input id="token-max-uses" type="number" min="1" step="1" class="input" bind:value={maxUses} placeholder="No limit" disabled={creating} />
					</Field>
				{/if}
				<Field label="Stops enrolling after" for="token-expires" class="flex-1" help="Days. Machines already enrolled keep reporting; leave empty for no deadline.">
					<input id="token-expires" type="number" min="1" step="1" class="input" bind:value={expiresInDays} placeholder="No deadline" disabled={creating} />
				</Field>
			</div>
			{#if scopeError}
				<p class="text-sm text-warning">{scopeError}</p>
			{/if}
		</div>

		{#if createError}
			<ErrorNotice error={createError} title="Could not create the token" class="mt-3" />
		{/if}
		{/if}

		<div aria-live="polite">
			{#if created}
				<div class="rise-in mt-4 rounded-[var(--radius-card)] border border-advisory/40 bg-surface p-4">
					<div class="flex flex-wrap items-center justify-between gap-2">
						<div class="flex flex-wrap items-center gap-2">
							<p class="font-semibold text-ink">Token “{created.name}” created</p>
							<Plate tone="advisory" label="Shown once — copy it now" />
						</div>
						<Button variant="ghost" size="sm" onclick={() => (created = null)}>I've copied it</Button>
					</div>
					<div class="mt-4 grid gap-4">
						<div>
							<p class="mb-1.5 text-sm font-semibold text-ink">Token</p>
							<CopyBlock value={created.secret} label="Copy token" secret />
						</div>
						{#if created.install_linux}
							<div>
								<p class="mb-1.5 text-sm font-semibold text-ink">Install on Linux</p>
								<CopyBlock value={created.install_linux} label="Copy command" />
							</div>
						{/if}
						{#if created.install_windows}
							<div>
								<p class="mb-1.5 text-sm font-semibold text-ink">Install on Windows (PowerShell)</p>
								<CopyBlock value={created.install_windows} label="Copy command" />
							</div>
						{/if}
						<AgentChecksums />
					</div>
				</div>
			{/if}
		</div>

		<div class={auth.isAdmin ? 'mt-4' : ''}>
			{#if error}
				<ErrorNotice {error} title="Could not load the tokens" onretry={() => void load()} />
			{:else if loading}
				<Skeleton class="h-14 w-full" />
			{:else if tokens.length === 0}
				<EmptyState icon={Cpu} title="No token yet." description="Create one, then run the install command on the machine." />
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
										{#if revoked}<Plate tone="ghost" label="Revoked" />{/if}
										{#if !revoked}
											<Plate tone={token.max_uses === null ? 'info' : 'ghost'} label={scopeLabel(token)} />
											{#if spent(token)}<Plate tone="advisory" label="Enrols no more" />{/if}
										{/if}
									</div>
									<p class="mt-1 text-sm text-ink-2">
										Created <time class="tnum" title={formatDateTime(token.created_at)}>{formatRelative(token.created_at)}</time>
										· Last used <time class="tnum" title={formatDateTime(token.last_used_at)}>{formatRelative(token.last_used_at)}</time>
										{#if token.expires_at}
											· Stops enrolling <time class="tnum" title={formatDateTime(token.expires_at)}>{formatRelative(token.expires_at)}</time>
										{/if}
										{#if revoked}
											· Revoked <time class="tnum" title={formatDateTime(token.revoked_at)}>{formatRelative(token.revoked_at)}</time>
										{/if}
									</p>
									{#if !revoked && spent(token)}
										<p class="mt-1 text-sm text-ink-2">
											Machines enrolled with it keep reporting; it just cannot let a new one in.
										</p>
									{/if}
								</div>
								{#if !revoked && auth.isAdmin}
									<Confirm confirmLabel="Revoke for good?" loading={revoking === token.id} onconfirm={() => revoke(token)}>Revoke</Confirm>
								{/if}
							</div>
							{#if revokeError?.id === token.id}
								<ErrorNotice error={revokeError.cause} title="Could not revoke the token" class="mt-3" />
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>
</Panel>
