<script lang="ts">
	/**
	 * Sparkline for tables and cards: a line (or bars for counters), the last
	 * point marked, an optional threshold line, and min / max in the tooltip.
	 * Decorative unless `label` is given: the number next to it carries the
	 * meaning. Presentational: strings come in as props.
	 */
	import { sparkGeometry } from './geometry.js';
	import { TONE_COLOR } from './tone.js';
	import type { Tone } from './types.js';

	interface Props {
		values: number[];
		width?: number;
		height?: number;
		variant?: 'line' | 'bars';
		tone?: Tone;
		/** A level to draw as a dashed line (an alert threshold); out of range = not drawn. */
		threshold?: number | null;
		/** Mark the last point. */
		showLast?: boolean;
		/** Fill under the line. */
		fill?: boolean;
		/** Fixed scale, for example 0..100 for percentages. */
		floor?: number;
		ceil?: number;
		format?: (value: number) => string;
		/** Accessible name; the sparkline is hidden from assistive technology without it. */
		label?: string;
		/** Draw the line in once, on first show (ignored under reduced motion). */
		draw?: boolean;
		class?: string;
	}
	let {
		values,
		width = 96,
		height = 24,
		variant = 'line',
		tone = 'ghost',
		threshold = null,
		showLast = true,
		fill = false,
		floor,
		ceil,
		format = (v: number) => String(Math.round(v * 100) / 100),
		label,
		draw = false,
		class: cls = ''
	}: Props = $props();

	const geo = $derived(sparkGeometry(values, width, height, 3, floor, ceil));
	const color = $derived(TONE_COLOR[tone === 'ghost' ? 'info' : tone]);
	const barW = $derived(geo.points.length > 0 ? Math.max(1, width / geo.points.length - 1) : 1);
	const thresholdY = $derived.by(() => {
		if (threshold == null || geo.points.length === 0) return null;
		const lo = floor ?? geo.min;
		const hi = ceil ?? geo.max;
		if (threshold < lo || threshold > hi) return null;
		const inner = height - 6;
		return 3 + inner - ((threshold - lo) / (hi - lo || 1)) * inner;
	});
	const summary = $derived(geo.points.length ? `${label ?? ''} ${format(geo.min)} – ${format(geo.max)}`.trim() : '');
</script>

<svg
	{width}
	{height}
	viewBox={`0 0 ${width} ${height}`}
	class={`block max-w-full ${cls}`}
	role={label ? 'img' : undefined}
	aria-label={label ? summary : undefined}
	aria-hidden={label ? undefined : 'true'}
>
	{#if summary}<title>{summary}</title>{/if}
	{#if geo.points.length === 0}
		<line x1="0" x2={width} y1={height / 2} y2={height / 2} stroke="var(--c-line-strong)" stroke-width="1" stroke-dasharray="2 4" />
	{:else}
		{#if thresholdY != null}
			<line x1="0" x2={width} y1={thresholdY} y2={thresholdY} stroke="var(--c-warning)" stroke-width="1" stroke-dasharray="3 3" />
		{/if}
		{#if variant === 'bars'}
			{#each geo.points as [x, y], i (i)}
				<rect x={Math.min(width - barW, Math.max(0, x - barW / 2))} {y} width={barW} height={Math.max(1, height - y)} fill={color} opacity="0.75" />
			{/each}
		{:else}
			{#if fill && geo.area}
				<path d={geo.area} fill={color} opacity="0.12" />
			{/if}
			{#if geo.path}
				<path
					class:draw
					d={geo.path}
					fill="none"
					stroke={color}
					stroke-width="1.5"
					stroke-linejoin="round"
					stroke-linecap="round"
					pathLength="1"
				/>
			{/if}
			{#if showLast && geo.last}
				<circle cx={geo.last[0]} cy={geo.last[1]} r="2.25" fill={color} stroke="var(--c-surface)" stroke-width="1" />
			{/if}
		{/if}
	{/if}
</svg>

<style>
	@media (prefers-reduced-motion: no-preference) {
		.draw {
			stroke-dasharray: 1;
			stroke-dashoffset: 1;
			animation: spark-draw 600ms ease-out forwards;
		}
	}
	@keyframes spark-draw {
		to {
			stroke-dashoffset: 0;
		}
	}
</style>
