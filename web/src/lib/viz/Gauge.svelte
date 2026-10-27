<script lang="ts">
	/**
	 * Bounded gauge (percentage, battery, temperature against its limit): an arc,
	 * the value written in the middle, and the state as a glyph plus a word when
	 * a threshold is crossed. Never for a throughput (no meaningful maximum).
	 * Presentational: strings come in as props.
	 */
	import { arcPath, semiPoint, unit01 } from './geometry.js';
	import { TONE_COLOR, TONE_GLYPH, TONE_INK } from './tone.js';
	import type { Tone } from './types.js';

	export interface Threshold {
		/** The tone applies from this value up. */
		at: number;
		tone: Tone;
	}
	interface Props {
		value: number | null;
		min?: number;
		max?: number;
		/** Ascending. Below the first one the tone is `signal`. */
		thresholds?: Threshold[];
		/** Words for the tones ("OK", "High", "Critical"), spoken and shown with the glyph. */
		toneLabels?: Partial<Record<Tone, string>>;
		format?: (value: number) => string;
		unit?: string;
		/** What the gauge measures; also its accessible name. */
		label: string;
		/** Shown instead of the value when it is null. */
		noDataLabel?: string;
		class?: string;
	}
	let {
		value,
		min = 0,
		max = 100,
		thresholds = [],
		toneLabels = {},
		format,
		unit = '',
		label,
		noDataLabel = '–',
		class: cls = ''
	}: Props = $props();

	const W = 120;
	const H = 72;
	const CX = 60;
	const CY = 62;
	const R = 50;

	const t = $derived(value == null ? 0 : unit01(value, min, max));
	const tone = $derived.by((): Tone => {
		if (value == null) return 'ghost';
		let current: Tone = 'signal';
		for (const th of thresholds) if (value >= th.at) current = th.tone;
		return current;
	});
	const text = $derived(
		value == null ? noDataLabel : format ? format(value) : `${Math.round(value * 10) / 10}${unit ? ` ${unit}` : ''}`
	);
	const toneWord = $derived(toneLabels[tone] ?? '');
	const ticks = $derived(
		thresholds.map((th) => {
			const at = unit01(th.at, min, max);
			return { a: semiPoint(CX, CY, R - 7, at), b: semiPoint(CX, CY, R + 7, at), tone: th.tone };
		})
	);
</script>

<figure class={`m-0 inline-flex max-w-full flex-col items-center ${cls}`}>
	<svg
		viewBox={`0 0 ${W} ${H}`}
		class="block w-full max-w-[12rem]"
		role="meter"
		aria-label={label}
		aria-valuemin={min}
		aria-valuemax={max}
		aria-valuenow={value ?? undefined}
		aria-valuetext={`${text}${toneWord ? `, ${toneWord}` : ''}`}
	>
		<path d={arcPath(CX, CY, R, 0, 1)} fill="none" stroke="var(--c-ghost)" stroke-width="9" stroke-linecap="round" />
		{#if value != null && t > 0}
			<path
				class="fill"
				d={arcPath(CX, CY, R, 0, t)}
				fill="none"
				stroke={TONE_COLOR[tone]}
				stroke-width="9"
				stroke-linecap="round"
			/>
		{/if}
		{#each ticks as tick, i (i)}
			<line x1={tick.a[0]} y1={tick.a[1]} x2={tick.b[0]} y2={tick.b[1]} stroke="var(--c-ink-2)" stroke-width="1.5" />
		{/each}
		<text x={CX} y={CY - 8} text-anchor="middle" class="num" fill="var(--c-ink)">{text}</text>
	</svg>
	<figcaption class="mt-0.5 flex flex-col items-center gap-0.5 text-center">
		<span class="text-xs text-ink-2">{label}</span>
		{#if toneWord}
			<span class="text-xs font-medium" style:color={TONE_INK[tone]}>
				<span aria-hidden="true">{TONE_GLYPH[tone]}</span>
				{toneWord}
			</span>
		{/if}
	</figcaption>
</figure>

<style>
	.num {
		font-size: 15px;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}
	@media (prefers-reduced-motion: no-preference) {
		.fill {
			transition: stroke 200ms ease-out;
		}
	}
</style>
