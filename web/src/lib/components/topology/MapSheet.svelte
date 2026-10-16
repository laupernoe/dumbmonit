<script lang="ts">
	/**
	 * The map as a sheet of paper: title, scope and date on top, the whole
	 * map at scale 1, the legend underneath, on a white ground. It is what
	 * the browser prints (scaled to the page by CSS) and what the SVG and PNG
	 * exports serialise, so all three always match.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import MapContent from './MapContent.svelte';
	import type { MapLayout } from './layout';
	import { EDGE_DASH, SOURCES, sourceLabel } from './legend';

	interface Props {
		layout: MapLayout;
		title: string;
		subtitle: string;
		element?: SVGSVGElement | null;
		class?: string;
	}

	let { layout, title, subtitle, element = $bindable(null), class: className = '' }: Props = $props();

	const PAD = 32;
	const HEADER = 76;
	const LEGEND = 64;
	const width = $derived(Math.max(layout.width, 960) + PAD * 2);
	const height = $derived(HEADER + layout.height + LEGEND + PAD * 2);
	const legendY = $derived(PAD + HEADER + layout.height + 30);

	/** Legend entries laid out on one or two lines, each sized by its label. */
	const entries = $derived.by(() => {
		const items = [
			...SOURCES.map((source) => ({ kind: 'line' as const, dash: EDGE_DASH[source], label: sourceLabel(source) })),
			{ kind: 'down' as const, dash: '', label: m.topology_legend_down() },
			{ kind: 'impacted' as const, dash: '', label: m.topology_legend_impacted() }
		];
		let x = PAD;
		let y = 0;
		return items.map((item) => {
			const w = 44 + item.label.length * 6.4 + 28;
			if (x + w > width - PAD) {
				x = PAD;
				y += 22;
			}
			const placed = { ...item, x, y };
			x += w;
			return placed;
		});
	});
</script>

<svg
	bind:this={element}
	viewBox={`0 0 ${width} ${height}`}
	class={className}
	role="img"
	aria-label={`${title} — ${subtitle}`}
	data-topology-sheet
>
	<rect x="0" y="0" {width} {height} fill="#ffffff" />
	<text x={PAD} y={PAD + 26} class="fill-ink" font-size="26" font-weight="700">{title}</text>
	<text x={PAD} y={PAD + 52} class="fill-ink-2" font-size="13">{subtitle}</text>
	<line x1={PAD} y1={PAD + HEADER - 8} x2={width - PAD} y2={PAD + HEADER - 8} class="stroke-line-strong" stroke-width="1" />
	<g transform={`translate(${PAD},${PAD + HEADER})`}>
		<MapContent {layout} interactive={false} />
	</g>
	<g transform={`translate(0,${legendY})`}>
		<text x={PAD} y="-8" class="fill-ink" font-size="12" font-weight="700">{m.topology_legend_title()}</text>
		{#each entries as entry (entry.label)}
			<g transform={`translate(${entry.x},${entry.y + 10})`}>
				{#if entry.kind === 'line'}
					<line x1="0" y1="0" x2="36" y2="0" class="stroke-ink-3" stroke-width="1.8" stroke-linecap="round" stroke-dasharray={entry.dash || undefined} />
				{:else if entry.kind === 'down'}
					<rect x="6" y="-7" width="26" height="14" rx="3" class="fill-warning-soft stroke-warning" stroke-width="2.5" />
				{:else}
					<rect x="6" y="-7" width="26" height="14" rx="3" fill="#ffffff" class="stroke-advisory" stroke-width="1.5" stroke-dasharray="5 3" />
				{/if}
				<text x="44" y="4" class="fill-ink-2" font-size="11">{entry.label}</text>
			</g>
		{/each}
	</g>
</svg>
