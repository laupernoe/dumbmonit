<script lang="ts">
	/**
	 * Settings → Agents: only what is an instance setting. The encrypted agent
	 * channel (mTLS, CA rotation), automatic agent updates, and the enrolment
	 * tokens already issued, with revocation.
	 *
	 * Adding a machine — token, install command, first connection, naming,
	 * route through a relay, the Active Directory kit — happens in the add flow
	 * (`/targets/new?kind=agent`), never here; the agents themselves are on the
	 * rack and their own pages.
	 */
	import { Cpu, OctagonX, Plus } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { closeAgentToken, listAgentTokens, revokeAgentToken, type AgentToken } from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, Disclosure, EmptyState, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import AgentChannel from '#lib/components/settings/AgentChannel.svelte';
	import AgentUpdates from '#lib/components/settings/AgentUpdates.svelte';
	import { getAgentChannel } from '#lib/api/agent_tls.js';
	import type { AgentChannelOverview } from '#lib/api/types.js';

	// --- The encrypted agent channel (administrators) --------------------------

	/** `null` while unread, for non-administrators, or on an older server. */
	let channel = $state<AgentChannelOverview | null>(null);
	let channelError = $state<unknown>(null);

	$effect(() => {
		if (!auth.isAdmin) return;
		const controller = new AbortController();
		getAgentChannel(controller.signal)
			.then((overview) => (channel = overview))
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				channelError = cause;
			});
		return () => controller.abort();
	});

	let tokens = $state<AgentToken[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		// The enrolment token list is admin-only on the server.
		if (!auth.isAdmin) {
			tokens = [];
			loading = false;
			return;
		}
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

	// --- Stop enrolments (Active Directory tokens) ---------------------------

	let stopping = $state<number | null>(null);

	async function stopEnrolments(token: AgentToken) {
		stopping = token.id;
		revokeError = null;
		try {
			await closeAgentToken(token.id);
			tokens = await listAgentTokens();
		} catch (cause) {
			revokeError = { id: token.id, cause };
		} finally {
			stopping = null;
		}
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
	description={m.settings_agents_settings_description()}
	padded={false}
>
	{#snippet aside()}
		{#if !auth.isAdmin}
			<Plate tone="ghost" label={auth.readOnlyLabel} />
		{/if}
	{/snippet}

	{#if auth.isAdmin}
		<div class="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-line px-5 py-3 text-sm text-ink-2">
			<span>{m.settings_agents_add_elsewhere()}</span>
			<Button variant="secondary" size="sm" href="/targets/new?kind=agent">
				<Plus class="size-4" aria-hidden="true" />
				{m.settings_agents_install()}
			</Button>
		</div>
	{/if}

	{#if channel}
		<AgentChannel bind:overview={channel} />
	{:else if channelError}
		<div class="border-b border-line px-5 py-4">
			<ErrorNotice error={channelError} title={m.settings_agents_tls_load_error()} />
		</div>
	{/if}

	{#if auth.isAdmin}
		<AgentUpdates />
	{/if}

	<div class="px-5 py-4">
		<Disclosure title={m.settings_agents_tokens_title()}>
		<p class="mt-0.5 mb-3 text-sm text-ink-2">
			{m.settings_agents_tokens_issued_intro()}
		</p>
		<div>
			{#if error}
				<ErrorNotice {error} title={m.settings_agents_load_error()} onretry={() => void load()} />
			{:else if loading}
				<Skeleton class="h-14 w-full" />
			{:else if tokens.length === 0}
				<EmptyState icon={Cpu} title={m.settings_agents_empty_title()} description={m.settings_agents_tokens_none()} />
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
										{#if token.ad_domain}<Plate tone="info" label={m.settings_agents_ad_badge({ domain: token.ad_domain })} />{/if}
										{#if auth.mspOn && token.client_id !== undefined}
											{@const owner = auth.msp?.clients.find((c) => c.id === token.client_id)}
											{#if owner}<Plate tone="muted" bare label={m.settings_agents_client_badge({ name: owner.name })} />{/if}
										{/if}
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
								{#if !revoked && auth.isAdmin && token.ad_domain && !spent(token)}
									<Confirm variant="secondary" confirmLabel={m.settings_agents_ad_token_stop_confirm()} loading={stopping === token.id} onconfirm={() => stopEnrolments(token)}>
										<OctagonX class="size-4" aria-hidden="true" />{m.settings_agents_ad_token_stop()}
									</Confirm>
								{/if}
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
		</Disclosure>
	</div>
</Panel>
