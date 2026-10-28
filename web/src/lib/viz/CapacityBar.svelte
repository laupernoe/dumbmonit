<script lang="ts">
	/**
	 * Fill bar of a capacity (pool, volume, datastore, licence quota) with the
	 * figures written out and, when the caller has a forecast, the days left
	 * before it is full. The forecast comes from the server (`predict_linear`);
	 * this component only displays it. Presentational: strings are props.
	 */
	import { unit01 } from './geometry.js';
	import { TONE_COLOR, TONE_GLYPH, TONE_INK } from './tone.js';
	import type { Tone } from './types.js';

	interface Props {
		used: number;
		capacity: number;
		format?: (value: number) => string;
		/** Warning and critical fill ratios. */
		warnAt?: number;
		criticalAt?: number;
		/** Days until full at the current trend; null/undefined = no forecast. */
		forecastDays?: number | null;
		/** Sentence for the forecast, translated by the caller ("Full in about 23 days"). */
		forecastLabel?: string;
		/** Forecasts closer than this many days are warned about. */
		forecastWarnDays?: number;
		/** Forecasts closer than this many days are critical. */
		forecastCriticalDays?: number;
		/** What fills up; also the accessible name. */
		label: string;
		/**
		 * Part of `used` that is reserved by design (a thick iSCSI LUN on a NAS
		 * volume): drawn as a hatched first layer, and left out of the fill tone,
		 * which then reads `(used - reserved) / (capacity - reserved)`.
		 */
		reserved?: number;
		/** Legend of the reserved layer, translated by the caller ("Reserved by LUNs"). */
		reservedLabel?: string;
		/** Legend of the rest of `used` when `reserved` is set ("Data"). */
		dataLabel?: string;
		/** Words for the tones, shown with the glyph. */
		toneLabels?: Partial<Record<Tone, string>>;
		class?: string;
	}
	let {
		used,
		capacity,
		format = (v: number) => String(Math.round(v * 100) / 100),
		warnAt = 0.8,
		criticalAt = 0.9,
		forecastDays = null,
		forecastLabel = '',
		forecastWarnDays = 30,
		forecastCriticalDays = 7,
		label,
		reserved = 0,
		reservedLabel = '',
		dataLabel = '',
		toneLabels = {},
		class: cls = ''
	}: Props = $props();

	const held = $derived(Math.max(0, Math.min(reserved, used, capacity)));
	const ratio = $derived.by(() => {
		const room = capacity - held;
		if (held > 0) return room > 0 ? Math.max(0, used - held) / room : 0;
		return capacity > 0 ? used / capacity : 0;
	});
	const heldWidth = $derived(`${unit01(held, 0, capacity) * 100}%`);
	/** Where a threshold ratio sits on the bar: past the reserved layer, if any. */
	function markAt(threshold: number): string {
		if (capacity <= 0) return `${threshold * 100}%`;
		return `${unit01(held + threshold * (capacity - held), 0, capacity) * 100}%`;
	}
	const fillTone = $derived<Tone>(ratio >= criticalAt ? 'warning' : ratio >= warnAt ? 'advisory' : 'signal');
	const forecastTone = $derived<Tone>(
		forecastDays == null
			? 'ghost'
			: forecastDays <= forecastCriticalDays
				? 'warning'
				: forecastDays <= forecastWarnDays
					? 'advisory'
					: 'signal'
	);
	const percent = $derived(Math.round(ratio * 100));
	const width = $derived(
		`${Math.max(used - held > 0 ? 1 : 0, unit01(used - held, 0, capacity) * 100)}%`
	);
	const toneWord = $derived(toneLabels[fillTone] ?? '');
</script>

<div class={`min-w-0 ${cls}`}>
	<div class="flex items-baseline justify-between gap-2 text-xs">
		<span class="truncate text-ink-2">{label}</span>
		<span class="shrink-0 tabular-nums text-ink">{format(used)} / {format(capacity)}{held > 0 ? '' : ` · ${percent}%`}</span>
	</div>
	<div
		class="track mt-1"
		role="meter"
		aria-label={label}
		aria-valuemin={0}
		aria-valuemax={capacity}
		aria-valuenow={used}
		aria-valuetext={`${percent}%${toneWord ? `, ${toneWord}` : ''}`}
	>
		{#if held > 0}
			<div class="held" style:width={heldWidth}></div>
		{/if}
		<div class="fill" class:after={held > 0} style:width style:background={TONE_COLOR[fillTone]}></div>
		{#if warnAt < 1}
			<span class="mark" style:left={markAt(warnAt)} aria-hidden="true"></span>
		{/if}
		{#if criticalAt < 1}
			<span class="mark" style:left={markAt(criticalAt)} aria-hidden="true"></span>
		{/if}
	</div>
	{#if held > 0 && (reservedLabel || dataLabel)}
		<p class="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-ink-2">
			{#if reservedLabel}
				<span class="inline-flex items-center gap-1.5"><span class="swatch held-swatch" aria-hidden="true"></span>{reservedLabel} · {format(held)}</span>
			{/if}
			{#if dataLabel}
				<span class="inline-flex items-center gap-1.5"
					><span class="swatch" style:background={TONE_COLOR[fillTone]} aria-hidden="true"></span>{dataLabel} · {format(Math.max(0, used - held))}</span
				>
			{/if}
		</p>
	{/if}
	{#if toneWord && fillTone !== 'signal'}
		<p class="mt-1 text-xs font-medium" style:color={TONE_INK[fillTone]}>
			<span aria-hidden="true">{TONE_GLYPH[fillTone]}</span>
			{toneWord}
		</p>
	{/if}
	{#if forecastDays != null && forecastLabel}
		<p class="mt-1 text-xs" style:color={TONE_INK[forecastTone === 'signal' ? 'ghost' : forecastTone]}>
			<span aria-hidden="true">{forecastTone === 'signal' ? '↗' : TONE_GLYPH[forecastTone]}</span>
			{forecastLabel}
		</p>
	{/if}
</div>

<style>
	.track {
		position: relative;
		height: 0.625rem;
		border-radius: 9999px;
		background: var(--c-ghost);
		overflow: hidden;
	}
	.track {
		display: flex;
	}
	.fill {
		height: 100%;
		border-radius: 9999px;
		opacity: 0.85;
	}
	.fill.after {
		border-top-left-radius: 0;
		border-bottom-left-radius: 0;
	}
	/* Reserved by design: hatched, so it never reads as a fill level. */
	.held,
	.held-swatch {
		background: repeating-linear-gradient(
			135deg,
			var(--c-ink-3) 0 3px,
			transparent 3px 6px
		);
		opacity: 0.7;
	}
	.held {
		height: 100%;
		flex: none;
	}
	.fill {
		flex: none;
	}
	.swatch {
		display: inline-block;
		width: 0.75rem;
		height: 0.5rem;
		border-radius: 2px;
	}
	.mark {
		position: absolute;
		top: 0;
		bottom: 0;
		width: 1px;
		background: var(--c-ink-3);
		opacity: 0.6;
	}
	@media (prefers-reduced-motion: no-preference) {
		.fill {
			transition: width 200ms ease-out;
		}
	}
</style>
