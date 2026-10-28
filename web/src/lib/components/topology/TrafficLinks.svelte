<script lang="ts">
	/**
	 * The weathermap on a phone: a ranked list of the busiest links instead of
	 * a canvas nobody can read at 390 px. Each row spells the rate and the
	 * share of the port speed, with a bar; tapping it opens the same figures
	 * and curve as the link panel of the map.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { ChevronDown } from 'lucide-svelte';
	import type { PlacedEdge } from './layout';
	import LinkTraffic from './LinkTraffic.svelte';
	import { loadText, mostLoaded, type Band } from './traffic';

	interface Props {
		edges: PlacedEdge[];
		name: (id: string) => string;
		/** Links shown at most. */
		limit?: number;
		class?: string;
	}

	let { edges, name, limit = 12, class: className = '' }: Props = $props();

	const ranked = $derived(mostLoaded(edges, limit));
	let open = $state<string | null>(null);

	/** Whole class names, so Tailwind sees them. */
	const FILL: Record<Band, string> = { idle: 'bg-ink-3', low: 'bg-signal', mid: 'bg-signal', high: 'bg-advisory', full: 'bg-warning', unknown: 'bg-ink-2' };
</script>

<section class={className} aria-label={m.topology_traffic_top_title()}>
	<h2 class="mb-2 text-sm font-bold text-ink">{m.topology_traffic_top_title()}</h2>
	{#if ranked.length === 0}
		<p class="text-sm text-ink-2">{m.topology_traffic_top_empty()}</p>
	{:else}
		<ul class="overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface">
			{#each ranked as { edge, load } (edge.key)}
				<li class="border-b border-line last:border-b-0">
					<button
						type="button"
						class="flex min-h-14 w-full flex-col gap-1.5 px-3 py-2 text-left hover:bg-surface-2 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-signal"
						aria-expanded={open === edge.key}
						onclick={() => (open = open === edge.key ? null : edge.key)}
					>
						<span class="flex items-center gap-2">
							<span class="min-w-0 flex-1 truncate text-sm font-semibold text-ink">{name(edge.from)} <span class="text-ink-3" aria-hidden="true">↔</span> {name(edge.to)}</span>
							<ChevronDown class={`size-4 shrink-0 text-ink-3 transition-transform ${open === edge.key ? 'rotate-180' : ''}`} aria-hidden="true" />
						</span>
						<span class="flex items-center gap-3">
							<span class="h-2 min-w-0 flex-1 overflow-hidden rounded-full bg-surface-2" aria-hidden="true">
								<span class={`block h-full rounded-full ${edge.stale ? 'bg-ink-3' : FILL[load.band]}`} style={`width: ${Math.max(load.pct ?? 0, load.bps > 0 ? 2 : 0)}%`}></span>
							</span>
							<span class="shrink-0 text-xs font-semibold tabular-nums text-ink">{loadText(load, getLocale())}</span>
						</span>
						{#if load.errors}
							<span class="text-xs font-semibold text-advisory-ink">{m.topology_traffic_errors_short()}</span>
						{/if}
					</button>
					{#if open === edge.key && edge.traffic}
						<div class="border-t border-line bg-canvas px-3 py-3">
							<LinkTraffic from={edge.from} to={edge.to} traffic={edge.traffic} {name} stale={edge.stale} fromPort={edge.fromPort} toPort={edge.toPort} />
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</section>
