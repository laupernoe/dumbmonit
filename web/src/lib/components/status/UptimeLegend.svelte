<script lang="ts">
	/**
	 * Colour key of the daily history bars: each swatch is named by its uptime
	 * band, so a poor day is never read as "down all day" and the colours are
	 * never the only carrier of the meaning.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { DAY_GOOD, DAY_POOR } from './day-tone';

	const pct = (value: number) =>
		new Intl.NumberFormat(getLocale(), { style: 'percent', maximumFractionDigits: 1 }).format(value / 100);

	const keys = $derived([
		{ fill: 'bg-signal', label: m.status_uptime_legend_good({ min: pct(DAY_GOOD) }) },
		{ fill: 'bg-advisory', label: m.status_uptime_legend_partial({ low: pct(DAY_POOR), high: pct(DAY_GOOD) }) },
		{ fill: 'bg-warning', label: m.status_uptime_legend_poor({ max: pct(DAY_POOR) }) },
		{ fill: 'bg-line-strong', label: m.status_uptime_legend_no_data() }
	]);
</script>

<ul class="flex flex-wrap gap-x-4 gap-y-1 text-[0.75rem] text-ink-3" aria-label={m.status_uptime_legend_label()}>
	{#each keys as key (key.fill)}
		<li class="flex items-center gap-1.5">
			<span class={`inline-block size-2.5 rounded-[2px] ${key.fill}`} aria-hidden="true"></span>
			<span class="tnum">{key.label}</span>
		</li>
	{/each}
</ul>
