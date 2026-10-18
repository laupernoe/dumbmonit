<script lang="ts">
	/**
	 * A rise or a fall, never by colour alone: an arrow, the signed figure, and
	 * the words for a screen reader. Teal up, coral down, ink when flat.
	 */
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	import { TREND_ARROW, formatPercent, trend } from '#lib/wall/widgets.js';

	interface Props {
		/** Percent. */
		percent: number | null;
	}

	let { percent }: Props = $props();

	const direction = $derived(trend(percent));
	const text = $derived(percent === null ? '' : formatPercent(percent, getLocale()));
	const spoken = $derived(
		direction === 'up'
			? m.wall_widgets_trend_up({ change: text })
			: direction === 'down'
				? m.wall_widgets_trend_down({ change: text })
				: m.wall_widgets_trend_flat()
	);
</script>

{#if direction}
	<span class="trend trend--{direction} tnum">
		<span aria-hidden="true" class="arrow">{TREND_ARROW[direction]}</span>
		<span aria-hidden="true">{text}</span>
		<span class="sr-only">{spoken}</span>
	</span>
{:else}
	<span class="trend tnum" aria-hidden="true">—</span>
{/if}

<style>
	.trend {
		display: inline-flex;
		align-items: baseline;
		justify-content: flex-end;
		gap: 0.3em;
		white-space: nowrap;
		font-weight: 650;
		color: var(--c-ink-2);
	}
	.arrow {
		font-size: 0.7em;
	}
	.trend--up {
		color: var(--c-signal-ink);
	}
	.trend--down {
		color: var(--c-warning-ink);
	}
</style>
