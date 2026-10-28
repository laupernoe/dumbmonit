<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/** The scan as a list: one block per address, its services underneath. Works at 390px. */
	import type { DiscoveredHost } from '#lib/api/types.js';
	import DiscoveryService from './DiscoveryService.svelte';
	import { compareIp, sortServices } from './discovery-graph';

	interface Props {
		hosts: DiscoveredHost[];
		/** Relay agent that ran the scan, `null` for the server. */
		viaAgent?: number | null;
	}

	let { hosts, viaAgent = null }: Props = $props();
	const sorted = $derived([...hosts].sort((a, b) => compareIp(a.ip, b.ip)));
</script>

<ul class="grid gap-3">
	{#each sorted as host (host.ip)}
		<li class="overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface">
			<div class="flex flex-wrap items-baseline gap-x-3 gap-y-0.5 border-b border-line px-4 py-2.5">
				<p class="tnum font-mono text-sm font-semibold text-ink">{host.ip}</p>
				{#if host.hostname}<p class="min-w-0 truncate text-[0.8125rem] text-ink-2">{host.hostname}</p>{/if}
				{#if host.vendor || host.mac}
					<p class="min-w-0 truncate font-mono text-[0.75rem] text-ink-3">{[host.vendor, host.mac].filter(Boolean).join(' · ')}</p>
				{/if}
			</div>
			<div class="divide-y divide-line">
				{#each sortServices(host.services) as service (`${service.protocol}${service.port}`)}
					<DiscoveryService {service} ip={host.ip} hostname={host.hostname} {viaAgent} />
				{:else}
					<p class="px-4 py-3 text-[0.8125rem] text-ink-2">{m.deviceform_scan_no_service()}</p>
				{/each}
			</div>
		</li>
	{/each}
</ul>
