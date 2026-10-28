<script lang="ts">
	/**
	 * Sorted horizontal bars with the value written at the end and, when a
	 * previous value is given, a delta (▲ ▼ and a signed figure: the direction
	 * is a shape and a number, the colour only says good or bad).
	 * Presentational: strings come in as props.
	 */
	import { delta } from './geometry.js';
	import { TONE_COLOR, TONE_INK } from './tone.js';

	export interface TopItem {
		key: string;
		label: string;
		value: number;
		/** Same quantity over the previous period. */
		previous?: number | null;
	}
	interface Props {
		items: TopItem[];
		/** Bars shown; the rest is summed up in `moreLabel`. */
		limit?: number;
		format?: (value: number) => string;
		unit?: string;
		/** Which way is good, to colour the deltas. Neutral when empty. */
		better?: 'higher' | 'lower' | '';
		/** Shared maximum, for example 100 for percentages; the largest value when omitted. */
		max?: number;
		/** Accessible name of the list. */
		label: string;
		/** "+3 more", already translated and counted by the caller. */
		moreLabel?: string;
		/** Spoken after a delta, for example "vs previous period". */
		deltaLabel?: string;
		/** Rows become buttons when set. */
		onselect?: (key: string) => void;
		class?: string;
	}
	let {
		items,
		limit = 8,
		format,
		unit = '',
		better = '',
		max,
		label,
		moreLabel,
		deltaLabel = '',
		onselect,
		class: cls = ''
	}: Props = $props();

	const fmt = (v: number) =>
		format ? format(v) : unit ? `${Math.round(v * 100) / 100} ${unit}` : `${Math.round(v * 100) / 100}`;
	const sorted = $derived([...items].sort((a, b) => b.value - a.value).slice(0, limit));
	const top = $derived(max ?? Math.max(0, ...sorted.map((i) => i.value)));

	function change(item: TopItem) {
		const d = delta(item.value, item.previous);
		if (item.previous == null || d.direction === 'flat') {
			return item.previous == null ? null : { glyph: '=', text: '0%', tone: 'ghost' as const };
		}
		const pct = d.ratio == null ? '' : `${d.ratio > 0 ? '+' : '−'}${Math.round(Math.abs(d.ratio) * 100)}%`;
		const good = better === '' ? null : (d.direction === 'up') === (better === 'higher');
		return {
			glyph: d.direction === 'up' ? '▲' : '▼',
			text: pct,
			tone: good === null ? ('ghost' as const) : good ? ('signal' as const) : ('warning' as const)
		};
	}
</script>

<ol class={`m-0 grid list-none gap-1 p-0 ${cls}`} aria-label={label}>
	{#each sorted as item (item.key)}
		{@const d = change(item)}
		<li class="min-w-0">
			<svelte:element
				this={onselect ? 'button' : 'div'}
				type={onselect ? 'button' : undefined}
				class="row grid w-full min-w-0 items-center gap-x-2 text-left"
				onclick={onselect ? () => onselect(item.key) : undefined}
			>
				<span class="truncate text-xs text-ink" title={item.label}>{item.label}</span>
				<span class="text-xs tabular-nums text-ink">{fmt(item.value)}</span>
				<span
					class="delta text-[0.6875rem] tabular-nums"
					style:color={d ? TONE_INK[d.tone] : undefined}
					aria-label={d ? `${d.glyph} ${d.text} ${deltaLabel}`.trim() : undefined}
				>
					{#if d}<span aria-hidden="true">{d.glyph}</span> {d.text}{/if}
				</span>
				<span class="track col-span-3" aria-hidden="true">
					<span
						class="bar"
						style:width={`${top > 0 ? Math.max(1, (Math.min(item.value, top) / top) * 100) : 0}%`}
						style:background={TONE_COLOR.signal}
					></span>
				</span>
			</svelte:element>
		</li>
	{/each}
	{#if moreLabel}
		<li class="text-[0.6875rem] text-ink-3">{moreLabel}</li>
	{/if}
</ol>

<style>
	.row {
		grid-template-columns: minmax(0, 1fr) auto 4.25rem;
		min-height: 2.5rem;
		padding: 0.25rem 0;
		background: none;
		border: 0;
		color: inherit;
		font: inherit;
		align-content: center;
	}
	button.row {
		cursor: pointer;
		border-radius: 0.375rem;
	}
	button.row:hover {
		background: var(--c-surface-2);
	}
	button.row:focus-visible {
		outline: 2px solid var(--c-focus);
		outline-offset: 1px;
	}
	.delta {
		text-align: right;
		white-space: nowrap;
	}
	.track {
		display: block;
		height: 0.375rem;
		border-radius: 9999px;
		background: var(--c-ghost);
		overflow: hidden;
		margin-top: 0.125rem;
	}
	.bar {
		display: block;
		height: 100%;
		border-radius: 9999px;
		opacity: 0.8;
	}
	@media (prefers-reduced-motion: no-preference) {
		.bar {
			transition: width 200ms ease-out;
		}
	}
</style>
