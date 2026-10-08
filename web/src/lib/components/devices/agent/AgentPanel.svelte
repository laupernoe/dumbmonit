<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Agent panel of a device: which machine the agent describes, and — the part
	 * that matters for security — whether this registration is bound to that one
	 * agent installation.
	 *
	 * Unbound means refused: a host enrolled before binding existed, and never
	 * bound since, no longer gets its measurements, container commands or relayed
	 * probes accepted. The way back is the re-enrolment window below.
	 */
	import { ShieldCheck, ShieldAlert } from 'lucide-svelte';
	import { allowAgentRebind, getAgentHost, type AgentHost, type Target } from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, ErrorNotice, Panel, Plate } from '#lib/ui/index.js';
	import RelayPanel from '#lib/components/devices/relay/RelayPanel.svelte';
	import MdaemonQueues from '#lib/components/devices/mdaemon/MdaemonQueues.svelte';
	import BackupReposPanel from './BackupReposPanel.svelte';
	import WireguardPanel from './WireguardPanel.svelte';
	import AppliancePanel from '#lib/components/devices/appliance/AppliancePanel.svelte';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let host = $state<AgentHost | null>(null);
	let rebinding = $state(false);
	let rebindError = $state<unknown>(null);
	let reloads = $state(0);

	$effect(() => {
		const id = target.id;
		// Read so the effect re-runs after a re-enrolment window is opened.
		reloads;
		const controller = new AbortController();
		getAgentHost(id, controller.signal)
			.then((agent) => (host = agent))
			.catch(() => (host = null));
		return () => controller.abort();
	});

	const BINDING = $derived({
		bound: {
			tone: 'signal' as const,
			icon: ShieldCheck,
			label: m.devices_agent_bound(),
			detail: m.devices_agent_bound_detail()
		},
		unbound: {
			tone: 'warning' as const,
			icon: ShieldAlert,
			label: m.devices_agent_unbound(),
			detail: m.devices_agent_unbound_detail()
		}
	});

	/** An unknown state from a newer server reads as "not bound", never as safe. */
	const binding = $derived(host ? (BINDING[host.binding as keyof typeof BINDING] ?? BINDING.unbound) : null);
	const unbound = $derived(!!host && host.binding !== 'bound');
	/** A re-enrolment window that has not run out yet. */
	const window_ = $derived.by(() => {
		if (!host?.rebind_until) return null;
		return new Date(host.rebind_until + 'Z') > new Date() ? host.rebind_until : null;
	});

	async function rebind() {
		rebinding = true;
		rebindError = null;
		try {
			await allowAgentRebind(target.id);
			reloads += 1;
		} catch (cause) {
			rebindError = cause;
		} finally {
			rebinding = false;
		}
	}
</script>

{#if host && binding}
	{@const Icon = binding.icon}
	<Panel title={m.devices_agent_title()} description={m.devices_agent_description()}>
		{#snippet aside()}
			<Plate tone={binding.tone} label={binding.label} />
		{/snippet}

		<dl class="grid gap-x-6 gap-y-2 text-sm sm:grid-cols-[auto_1fr]">
			<dt class="text-ink-2">{m.devices_agent_machine()}</dt>
			<dd class="text-ink">{host.hostname}</dd>
			<dt class="text-ink-2">{m.devices_agent_system()}</dt>
			<dd class="text-ink">{host.os_version ?? host.os}{host.arch ? ` · ${host.arch}` : ''}</dd>
			<dt class="text-ink-2">{m.devices_agent_agent()}</dt>
			<dd class="text-ink">{host.agent_version}</dd>
			<dt class="text-ink-2">{m.devices_agent_last_batch()}</dt>
			<dd class="text-ink">
				<time class="tnum" title={formatDateTime(host.last_seen_at)}>{formatRelative(host.last_seen_at)}</time>
			</dd>
			<dt class="text-ink-2">{m.devices_agent_binding()}</dt>
			<dd class="text-ink">
				{#if host.bound_at}
					<time class="tnum" title={formatDateTime(host.bound_at)}>{m.devices_agent_bound_when({ when: formatRelative(host.bound_at) })}</time>
				{:else}
					{m.devices_agent_not_bound()}
				{/if}
			</dd>
		</dl>

		<p class="mt-3 flex items-start gap-2 text-sm text-ink-2">
			<Icon class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
			<span>{binding.detail}</span>
		</p>
		{#if unbound && !host.binding_supported}
			<p class="mt-2 text-sm text-ink-2">
				{m.devices_agent_too_old({ version: host.agent_version })}
			</p>
		{/if}

		{#if window_}
			<p class="mt-3 rounded-[var(--radius-card)] border border-advisory/40 bg-surface px-3 py-2 text-sm text-ink">
				{m.devices_agent_window_open({ until: formatDateTime(window_) })}
			</p>
		{:else if auth.isAdmin}
			<div class="mt-3 flex flex-wrap items-center gap-3">
				<Confirm variant={unbound ? 'secondary' : 'danger'} confirmLabel={m.devices_agent_open_confirm()} loading={rebinding} onconfirm={rebind}>{m.devices_agent_allow_rebind()}</Confirm>
				<p class="text-sm text-ink-2">
					{#if unbound}
						{m.devices_agent_rebind_hint_unbound()}
					{:else}
						{m.devices_agent_rebind_hint_bound()}
					{/if}
				</p>
			</div>
		{/if}

		{#if rebindError}
			<ErrorNotice error={rebindError} title={m.devices_agent_err_rebind()} class="mt-3" />
		{/if}
	</Panel>
{/if}

<RelayPanel {target} />

<!-- Shown only when this machine runs MDaemon and the agent reads its counters. -->
<MdaemonQueues agent={target.id} />

<!-- Each renders nothing unless the agent reports WireGuard tunnels or declared restic/Borg repositories. -->
<BackupReposPanel {target} />

<WireguardPanel {target} />

<!-- Shown only on a Hyper-V host, where the agent reads Hyper-V's performance counters. -->
<AppliancePanel {target} variant="hyperv" />
