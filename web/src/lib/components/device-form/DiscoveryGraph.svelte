<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The scan as a graph: the network at the centre, one card per address
	 * around it, the services listed inside their address's card. Cards are
	 * plain HTML (real links and buttons, selectable text) laid over an SVG
	 * that only draws the links; positions come from `discovery-graph.ts`.
	 * Scaled down to fit the width, never below a readable size (then it scrolls).
	 */
	import { Network } from 'lucide-svelte';
	import type { DiscoveredHost } from '#lib/api/types.js';
	import DiscoveryService from './DiscoveryService.svelte';
	import { CARD_W, HEADER_H, compareIp, layoutGraph, sortServices } from './discovery-graph';

	interface Props {
		hosts: DiscoveredHost[];
		cidr: string;
		/** Relay agent that ran the scan, `null` for the server. */
		viaAgent?: number | null;
	}

	let { hosts, cidr, viaAgent = null }: Props = $props();

	const sorted = $derived([...hosts].sort((a, b) => compareIp(a.ip, b.ip)));
	const layout = $derived(layoutGraph(sorted.map((h) => ({ ip: h.ip, rows: h.services.length }))));
	let box = $state(0);
	const scale = $derived(box > 0 ? Math.max(0.55, Math.min(1, box / layout.width)) : 1);
</script>

<div
	bind:clientWidth={box}
	class="overflow-auto rounded-[var(--radius-card)] border border-line bg-surface-2"
	role="group"
	aria-label={m.deviceform_scan_graph_label({ cidr })}
>
	<div style={`width:${layout.width * scale}px;height:${layout.height * scale}px;margin-inline:auto`}>
		<div class="relative origin-top-left" style={`width:${layout.width}px;height:${layout.height}px;transform:scale(${scale})`}>
			<svg class="absolute inset-0 text-line-strong" width={layout.width} height={layout.height} aria-hidden="true">
				{#each layout.nodes as node (node.ip)}
					<line x1={layout.hub.x} y1={layout.hub.y} x2={node.cx} y2={node.cy} stroke="currentColor" stroke-width="1.5" stroke-dasharray="2 5" stroke-linecap="round" />
				{/each}
			</svg>

			<!-- The network -->
			<div
				class="absolute flex flex-col items-center justify-center rounded-full border border-signal/40 bg-signal-soft text-signal-ink"
				style={`left:${layout.hub.x - layout.hub.r}px;top:${layout.hub.y - layout.hub.r}px;width:${layout.hub.r * 2}px;height:${layout.hub.r * 2}px`}
			>
				<Network class="size-6" aria-hidden="true" />
				<span class="absolute left-1/2 top-full mt-1 -translate-x-1/2 whitespace-nowrap font-mono text-xs font-semibold text-signal-ink">{cidr}</span>
			</div>

			{#each layout.nodes as node, i (node.ip)}
				{@const host = sorted[i]!}
				<article
					class="absolute overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface shadow-lift"
					style={`left:${node.x}px;top:${node.y}px;width:${CARD_W}px;height:${node.h}px`}
					aria-label={host.ip}
				>
					<header class="flex flex-col justify-center border-b border-line px-3" style={`height:${HEADER_H}px`}>
						<p class="tnum truncate font-mono text-sm font-semibold text-ink">{host.ip}</p>
						<p class="truncate text-[0.75rem] text-ink-2">{host.hostname ?? host.vendor ?? m.deviceform_scan_no_name()}</p>
					</header>
					<div class="divide-y divide-line">
						{#each sortServices(host.services) as service (`${service.protocol}${service.port}`)}
							<DiscoveryService {service} ip={host.ip} hostname={host.hostname} compact {viaAgent} />
						{:else}
							<p class="px-3 py-1.5 text-[0.75rem] text-ink-2">{m.deviceform_scan_no_service()}</p>
						{/each}
					</div>
				</article>
			{/each}
		</div>
	</div>
</div>
