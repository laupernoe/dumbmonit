<script lang="ts">
	import { withBase } from '#lib/paths.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Agent panel of a device, on the data page: a compact read-only summary of
	 * the agent (version, last report, binding) and the panels that show what it
	 * reports. Everything that changes the agent (relay, logs, services to
	 * watch, re-enrolment) lives in the monitor's settings (`AgentSettings`).
	 *
	 * Unbound means refused: a host enrolled before binding existed, and never
	 * bound since, no longer gets its measurements, container commands or relayed
	 * probes accepted. The summary says so and points to the settings.
	 */
	import { ShieldCheck, ShieldAlert } from 'lucide-svelte';
	import { getAgentHost, type AgentHost, type Target } from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Plate } from '#lib/ui/index.js';
	import MailAgentPanel from '#lib/components/devices/mdaemon/MailAgentPanel.svelte';
	import DsmPanel from './DsmPanel.svelte';
	import AgentEventsPanel from './AgentEventsPanel.svelte';
	import AppliancePanel from '#lib/components/devices/appliance/AppliancePanel.svelte';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let host = $state<AgentHost | null>(null);

	$effect(() => {
		const id = target.id;
		const controller = new AbortController();
		getAgentHost(id, controller.signal)
			.then((agent) => (host = agent))
			.catch(() => (host = null));
		return () => controller.abort();
	});

	/** An unknown state from a newer server reads as "not bound", never as safe. */
	const unbound = $derived(!!host && host.binding !== 'bound');
	const system = $derived(host ? `${host.os_version ?? host.os}${host.arch ? ` · ${host.arch}` : ''}` : '');
</script>

<!-- Shown only on a Synology NAS running the agent package: volumes, RAID, disks, packages. -->
<DsmPanel {target} />

{#if host}
	<section class="rounded-[var(--radius-card)] border border-line bg-surface px-4 py-3 shadow-lift sm:px-5" aria-label={m.devices_agent_title()}>
		<div class="flex flex-wrap items-center gap-x-5 gap-y-2 text-sm">
			<h2 class="text-base font-semibold tracking-tight text-ink">{m.devices_agent_title()}</h2>
			<span class="text-ink-2">
				{m.devices_agent_agent()} <span class="tnum text-ink">{host.agent_version}</span>
			</span>
			<span class="min-w-0 break-words text-ink-2" title={system}>{system}</span>
			<span class="text-ink-2">
				{m.devices_agent_last_batch()}
				<time class="tnum text-ink" title={formatDateTime(host.last_seen_at)}>{formatRelative(host.last_seen_at)}</time>
			</span>
			<span class="inline-flex items-center gap-1.5 sm:ml-auto">
				{#if unbound}
					<ShieldAlert class="size-4 text-warning-ink" aria-hidden="true" />
					<Plate tone="warning" label={m.devices_agent_unbound()} />
				{:else}
					<ShieldCheck class="size-4 text-signal-ink" aria-hidden="true" />
					<Plate tone="signal" label={m.devices_agent_bound()} />
				{/if}
			</span>
		</div>
		{#if unbound}
			<p class="mt-2 text-sm text-ink-2">
				{m.devices_agent_unbound_detail()}
				{#if auth.isAdmin}
					<a href={withBase(`/targets/${target.id}/edit`)} class="inline-flex min-h-10 items-center text-ink underline decoration-line-strong underline-offset-2 hover:text-signal-ink">{m.devices_agent_open_settings()}</a>
				{/if}
			</p>
		{/if}
	</section>
{/if}

<!-- Shown only when this machine runs MDaemon or SecurityGateway and the agent found it. -->
<MailAgentPanel agent={target.id} product="mdaemon" />
<MailAgentPanel agent={target.id} product="securitygateway" />

<!-- Windows only: event log rules that raise alerts, and the latest matched events. -->
<AgentEventsPanel {target} />

<!-- Shown only on a Hyper-V host, where the agent reads Hyper-V's performance counters. -->
<AppliancePanel {target} variant="hyperv" />
