<script lang="ts">
	/**
	 * One public service: state plate + name, the daily history bar, then the
	 * uptime readouts (24 h / 7 d / 30 d / 90 d) and the latency. Stacks on phones.
	 */
	import type { PublicStatusItem } from '#lib/api/index.js';
	import { formatPercent } from '#lib/format.js';
	import { Plate } from '#lib/ui/index.js';
	import UptimeBar from './UptimeBar.svelte';
	import { ITEM_STATE } from './words';

	interface Props {
		item: PublicStatusItem;
		days: number;
		/** Tighter row (name, one uptime figure, short bar) for pages with a banner scene. */
		compact?: boolean;
	}

	let { item, days, compact = false }: Props = $props();

	const state = $derived(ITEM_STATE[item.state] ?? ITEM_STATE.unknown);

	function formatLatencyMs(value: number | null): string {
		if (value === null || !Number.isFinite(value)) return '—';
		if (value < 1000) return `${Math.round(value)} ms`;
		return `${(Math.round(value / 100) / 10).toString()} s`;
	}

	// The window adapts to the data's age: 30 and 90 days are listed only once the page covers them.
	const headline = $derived(days >= 90 ? 90 : days >= 30 ? 30 : 7);
	const readouts = $derived([
		{ label: '24 h', value: item.uptime_24h },
		{ label: '7 d', value: item.uptime_7d },
		...(days >= 30 ? [{ label: '30 d', value: item.uptime_30d }] : []),
		...(days >= 90 ? [{ label: '90 d', value: item.uptime_90d }] : [])
	]);
</script>

{#if compact}
	<li class="px-4 py-2.5 sm:px-5">
		<div class="flex items-center gap-3">
			<Plate tone={state.tone} label={state.label} pulse={item.state === 'down'} />
			<span class="min-w-0 flex-1 truncate font-semibold text-ink">{item.label}</span>
			<span class="tnum shrink-0 text-sm text-ink-2" title={`Uptime over ${headline} days`}>
				{formatPercent(headline === 90 ? item.uptime_90d : headline === 30 ? item.uptime_30d : item.uptime_7d)}
			</span>
		</div>
		<div class="mt-2">
			<UptimeBar history={item.history} label={item.label} compact />
		</div>
	</li>
{:else}
<li class="px-4 py-4 sm:px-5">
	<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
		<div class="flex min-w-0 items-center gap-3">
			<Plate tone={state.tone} label={state.label} pulse={item.state === 'down'} />
			<span class="truncate font-semibold text-ink">{item.label}</span>
		</div>
		{#if item.latency_ms !== null}
			<span class="text-sm text-ink-2"><span class="tnum">{formatLatencyMs(item.latency_ms)}</span> response</span>
		{/if}
	</div>

	<div class="mt-3">
		<UptimeBar history={item.history} label={item.label} />
	</div>

	<dl class="graticule mt-2 flex flex-wrap gap-x-6 gap-y-1 pb-1 text-sm">
		{#each readouts as readout (readout.label)}
			<div class="flex items-baseline gap-1.5">
				<dt class="label-tape text-ink-3">{readout.label}</dt>
				<dd class="tnum font-semibold text-ink">{formatPercent(readout.value)}</dd>
			</div>
		{/each}
	</dl>
</li>
{/if}
