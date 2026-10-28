<script lang="ts">
	/**
	 * Settings → Agents → Agent channel: the encrypted port agents report to
	 * (TLS 1.3, a client certificate per machine, post-quantum key exchange),
	 * the switch that closes the plain channel of the web port, the
	 * fingerprint the install commands pin, and the certificate authority
	 * rotation, folded away. Administrators only; the parent loads `overview`.
	 */
	import { ShieldCheck } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { cancelAgentCaRotation, completeAgentCaRotation, getAgentChannel, setAgentChannelRequired, startAgentCaRotation } from '#lib/api/agent_tls.js';
	import type { AgentChannelOverview } from '#lib/api/types.js';
	import { formatDateTime } from '#lib/format.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { Button, Confirm, CopyBlock, Disclosure, ErrorNotice, Toggle } from '#lib/ui/index.js';

	interface Props {
		overview: AgentChannelOverview;
	}
	let { overview = $bindable() }: Props = $props();

	const encrypted = $derived(overview.agents.filter((agent) => agent.channel === 'mtls').length);
	const plain = $derived(overview.agents.filter((agent) => agent.channel === 'legacy').length);
	/** Agents still on the plain channel, named from the shared store: the ones to update. */
	const plainAgents = $derived(
		overview.agents
			.filter((agent) => agent.channel === 'legacy')
			.map((agent) => ({ id: agent.id, name: alertsStore.targets.find((t) => t.id === agent.id)?.name ?? `#${agent.id}` }))
	);

	let busy = $state(false);
	let error = $state<unknown>(null);

	async function act(change: () => Promise<AgentChannelOverview | void>) {
		busy = true;
		error = null;
		try {
			const next = await change();
			overview = next ?? (await getAgentChannel());
		} catch (cause) {
			error = cause;
		} finally {
			busy = false;
		}
	}
</script>

<div class="border-b border-line px-5 py-4">
	<div class="flex items-start gap-3">
		<ShieldCheck class="mt-0.5 size-5 shrink-0 text-signal" aria-hidden="true" />
		<div class="min-w-0 flex-1">
			<h3 class="font-semibold text-ink">{m.settings_agents_tls_title()}</h3>
			{#if overview.enabled && overview.port !== null}
				<p class="mt-1 max-w-prose text-sm text-ink-2">{m.settings_agents_tls_summary({ port: overview.port, group: overview.key_exchange })}</p>
				<p class="mt-1 max-w-prose text-sm text-ink-2">{m.settings_agents_tls_port_hint({ port: overview.port })}</p>
				<p class="tnum mt-1 text-sm text-ink-2">{m.settings_agents_tls_counts({ encrypted, plain })}</p>
				{#if plainAgents.length > 0}
					<p class="mt-1 flex flex-wrap items-center gap-x-3 text-sm text-ink-2">
						<span class="font-semibold text-ink">{m.settings_agents_tls_legacy()}</span>
						{#each plainAgents as agent (agent.id)}
							<a class="inline-flex min-h-10 items-center underline decoration-line underline-offset-2 hover:text-ink" href={`/targets/${agent.id}`}>{agent.name}</a>
						{/each}
					</p>
				{/if}
			{:else}
				<p class="mt-1 max-w-prose text-sm text-advisory-ink">{m.settings_agents_tls_disabled()}</p>
			{/if}
		</div>
	</div>

	{#if overview.enabled}
		<div class="mt-4 flex items-start gap-3">
			<Toggle
				id="agent-tls-required"
				checked={overview.required_setting}
				disabled={busy}
				label={m.settings_agents_tls_require()}
				onchange={(value) => act(() => setAgentChannelRequired(value))}
			/>
			<div class="min-w-0">
				<label for="agent-tls-required" class="text-sm font-semibold text-ink">{m.settings_agents_tls_require()}</label>
				<p class="max-w-prose text-sm text-ink-2">{m.settings_agents_tls_require_help()}</p>
				{#if !overview.required_setting && overview.legacy_until}
					<p class="mt-1 text-sm text-ink-2">
						{m.settings_agents_tls_legacy_until({ when: formatDateTime(overview.legacy_until) })}
					</p>
				{/if}
			</div>
		</div>

		{#if overview.ca_pin}
			<div class="mt-4">
				<p class="mb-1.5 text-sm font-semibold text-ink">{m.settings_agents_tls_fingerprint()}</p>
				<CopyBlock value={overview.ca_pin} label={m.settings_agents_tls_copy_fingerprint()} />
				<p class="mt-1.5 max-w-prose text-sm text-ink-2">{m.settings_agents_tls_fingerprint_help()}</p>
			</div>
		{/if}

		<Disclosure title={m.settings_agents_tls_rotation_title()} class="mt-4">
			<p class="mt-0.5 max-w-prose text-sm text-ink-2">{m.settings_agents_tls_rotation_intro()}</p>
			{#if overview.rotation}
				<div class="mt-3 grid gap-3">
					<p class="tnum text-sm text-ink">
						{m.settings_agents_tls_rotation_progress({ ready: overview.rotation.ready, total: overview.rotation.total })}
					</p>
					<div>
						<p class="mb-1.5 text-sm font-semibold text-ink">{m.settings_agents_tls_rotation_next()}</p>
						<CopyBlock value={overview.rotation.next_pin} label={m.settings_agents_tls_copy_fingerprint()} />
					</div>
					<p class="max-w-prose text-sm text-advisory-ink">{m.settings_agents_tls_rotation_warning()}</p>
					<div class="flex flex-wrap gap-2">
						<Confirm
							variant="secondary"
							confirmLabel={m.settings_agents_tls_rotation_complete_confirm()}
							loading={busy}
							onconfirm={() => act(completeAgentCaRotation)}
						>
							{m.settings_agents_tls_rotation_complete()}
						</Confirm>
						<Button variant="ghost" size="sm" disabled={busy} onclick={() => act(cancelAgentCaRotation)}>
							{m.settings_agents_tls_rotation_cancel()}
						</Button>
					</div>
				</div>
			{:else}
				<Button variant="secondary" size="sm" class="mt-3" loading={busy} onclick={() => act(startAgentCaRotation)}>
					{m.settings_agents_tls_rotation_start()}
				</Button>
			{/if}
		</Disclosure>
	{/if}

	{#if error}
		<ErrorNotice {error} title={m.settings_agents_tls_save_error()} class="mt-3" />
	{/if}
</div>
