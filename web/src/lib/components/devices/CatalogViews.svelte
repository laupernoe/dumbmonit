<script lang="ts">
	/**
	 * The views a device kind declares in the catalogue: a row of figures, then
	 * the few charts worth reading (`catalog.ts` fills them). Rankings (`topn`,
	 * `heatmap`) use a plain bar list for now.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import Chart from '#lib/components/Chart.svelte';
	import { formatDuration, formatLatency, formatRelative } from '#lib/format.js';
	import { ChevronDown } from 'lucide-svelte';
	import { formatRate } from './metrics.js';
	import type { CatalogChart, CatalogLayout, StatFigure } from './catalog.js';

	interface Props {
		layout: CatalogLayout;
	}

	let { layout }: Props = $props();

	/** Figures shown before "Show all": two rows on a wide screen. */
	const FIRST = 8;
	let showAll = $state(false);
	const shownStats = $derived(showAll ? layout.stats : layout.stats.slice(0, FIRST));

	function round(value: number): number {
		return Math.abs(value) >= 100 ? Math.round(value) : Math.round(value * 10) / 10;
	}

	/** "12.3 GB", "850 kbit/s": decimal steps, as storage and network people read them. */
	function scaled(value: number, units: string[]): string {
		let v = value;
		let i = 0;
		while (Math.abs(v) >= 1000 && i < units.length - 1) {
			v /= 1000;
			i += 1;
		}
		return `${round(v)} ${units[i]}`;
	}

	/** A value in its unit, compact enough for a tile or a ranking row. */
	function formatValue(value: number | null, unit: string): string {
		if (value === null || !Number.isFinite(value)) return '—';
		switch (unit) {
			case '%':
				return `${round(value)}%`;
			case 'ratio':
				return `${round(value * 100)}%`;
			case 's':
				return value < 1 ? formatLatency(value) : formatDuration(value);
			case 'days':
				return formatDuration(value * 86_400);
			case 'timestamp':
				return value > 0 ? formatRelative(new Date(value * 1000)) : '—';
			case 'B':
				return scaled(value, ['B', 'kB', 'MB', 'GB', 'TB', 'PB']);
			case 'B/s':
				return formatRate(value, 'B/s');
			case 'bit/s':
				return scaled(value, ['bit/s', 'kbit/s', 'Mbit/s', 'Gbit/s']);
			case 'bool':
				return value >= 1 ? m.devices_target_stat_yes() : m.devices_target_stat_no();
			case '':
				return `${round(value)}`;
			default:
				// "12/s" reads better than "12 /s".
				return unit.startsWith('/') ? `${round(value)}${unit}` : `${round(value)} ${unit}`;
		}
	}

	function figureText(figure: StatFigure): string {
		switch (figure.mode) {
			case 'info':
			case 'state':
				return figure.text;
			case 'ratio':
				return `${figure.value ?? 0}/${figure.count}`;
			case 'bool':
				return formatValue(figure.value, 'bool');
			default:
				return formatValue(figure.value, figure.unit);
		}
	}

	function figureHint(figure: StatFigure): string {
		if (figure.mode === 'max') return m.devices_target_stat_max_of({ count: figure.count });
		if (figure.mode === 'min') return m.devices_target_stat_min_of({ count: figure.count });
		return '';
	}

	/** A declared state tone, as a text colour; the word itself carries the meaning. */
	const TONE: Record<string, string> = {
		ok: 'text-signal-ink',
		warning: 'text-advisory-ink',
		critical: 'text-warning-ink'
	};

	/** Units shown on a chart's tape; `bool` and plain counts need none. */
	function tape(chart: CatalogChart): string {
		return chart.unit === 'bool' ? '' : chart.unit;
	}

	function share(chart: CatalogChart, value: number): number {
		const top = chart.rows[0]?.value ?? 0;
		return top > 0 ? Math.max(2, (value / top) * 100) : 0;
	}
</script>

{#if layout.stats.length > 0}
	<div class={`px-4 pt-4 sm:px-5 ${layout.charts.length === 0 ? 'pb-4' : ''}`}>
		<ul class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4" id="catalog-figures">
			{#each shownStats as figure (figure.name)}
				<li class="flex min-w-0 flex-col gap-1 rounded-lg border border-line px-3 py-2" title={figure.name}>
					<span class={`display tnum truncate text-xl ${TONE[figure.tone] ?? 'text-ink'}`}>{figureText(figure)}</span>
					<span class="label-tape truncate">{figure.label}</span>
					{#if figureHint(figure)}<span class="text-[0.8125rem] text-ink-2">{figureHint(figure)}</span>{/if}
				</li>
			{/each}
		</ul>
		{#if layout.stats.length > FIRST}
			<button
				type="button"
				class="mt-2 inline-flex min-h-10 items-center gap-2 text-sm font-medium text-ink-2 hover:text-ink"
				aria-expanded={showAll}
				aria-controls="catalog-figures"
				onclick={() => (showAll = !showAll)}
			>
				<ChevronDown class={`size-4 transition-transform motion-reduce:transition-none ${showAll ? 'rotate-180' : ''}`} aria-hidden="true" />
				{showAll ? m.devices_target_stat_less() : m.devices_target_stat_more({ count: layout.stats.length })}
			</button>
		{/if}
	</div>
{/if}

{#if layout.charts.length > 0}
	<div class="grid gap-4 px-4 py-4 sm:px-5 lg:grid-cols-2">
		{#each layout.charts as chart, i (chart.key)}
			<div class="rise-in min-w-0" style="--rise-delay: {Math.min(i, 8) * 40}ms">
				<div class="flex items-center justify-between gap-2 pb-1">
					<h3 class="truncate text-sm font-semibold text-ink">{chart.title}</h3>
					{#if tape(chart)}<span class="label-tape label-tape-unit">{tape(chart)}</span>{/if}
				</div>
				{#if chart.view === 'topn' || chart.view === 'heatmap'}
					<!-- TODO: swap for lib/viz TopN.svelte / Heatmap.svelte once feat/viz-kit lands. -->
					{#if chart.rows.length === 0}
						<p class="py-6 text-sm text-ink-2">{chart.view === 'heatmap' ? m.devices_target_rank_quiet() : m.devices_target_rank_empty()}</p>
					{:else}
						<p class="text-[0.8125rem] text-ink-2">{chart.view === 'heatmap' ? m.devices_target_rank_peak() : m.devices_target_rank_latest()}</p>
						<ol class="mt-1 flex flex-col gap-1.5">
							{#each chart.rows as row (row.key)}
								<li class="grid grid-cols-[minmax(0,8rem)_1fr_auto] items-center gap-3 text-[0.8125rem]">
									<span class="truncate text-ink-2" title={row.key}>{row.key}</span>
									<span class="h-2 overflow-hidden rounded-full bg-surface-2" aria-hidden="true">
										<span class="block h-full rounded-full bg-signal" style="width: {share(chart, row.value)}%"></span>
									</span>
									<span class="tnum text-right font-medium text-ink">{formatValue(row.value, chart.unit)}</span>
								</li>
							{/each}
						</ol>
					{/if}
				{:else}
					<Chart series={chart.series} unit={tape(chart)} height={200} />
				{/if}
			</div>
		{/each}
	</div>
{/if}
