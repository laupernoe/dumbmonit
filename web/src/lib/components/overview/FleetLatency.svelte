<script lang="ts">
	/**
	 * Response time across the fleet over the header's range: the p95 and the
	 * median of every probe, so a slow spell shows as the p95 line leaving the
	 * median behind. Beside it, the slowest monitors by p95; a tap opens one.
	 * Reloaded at once when the range changes, then every minute while LIVE
	 * is on. The chart code (uPlot) loads only with this card.
	 */
	import { goto } from '#lib/navigation.js';
	import type ChartComponent from '#lib/components/Chart.svelte';
	import { formatDuration } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { consoleState } from '#lib/stores/console.svelte.js';
	import { Skeleton } from '#lib/ui/index.js';
	import { TopN, type TopItem } from '#lib/viz/index.js';
	import { fleetStore } from '#lib/components/enterprise/fleet.svelte.js';
	import { loadFleetLatency, type FleetLatencyTrend } from '#lib/components/enterprise/fleet.js';

	const REFRESH_MS = 60_000;

	let Chart = $state<typeof ChartComponent | null>(null);
	$effect(() => {
		void import('#lib/components/Chart.svelte').then((mod) => (Chart = mod.default));
	});

	let trend = $state<FleetLatencyTrend | null>(null);
	let failed = $state(false);

	$effect(() => {
		const seconds = consoleState.rangeSeconds;
		const controller = new AbortController();
		const load = () =>
			loadFleetLatency(seconds, controller.signal)
				.then((next) => {
					trend = next;
					failed = false;
				})
				.catch(() => {
					if (!controller.signal.aborted) failed = true;
				});
		void load();
		const timer = setInterval(() => {
			if (consoleState.live && document.visibilityState !== 'hidden') void load();
		}, REFRESH_MS);
		return () => {
			clearInterval(timer);
			controller.abort();
		};
	});

	const series = $derived(
		trend
			? [
					{ label: m.overview_latency_p95(), points: trend.p95 },
					{ label: m.overview_latency_median(), points: trend.median }
				].filter((serie) => serie.points.length > 0)
			: []
	);

	const names = $derived(new Map(alertsStore.targets.map((target) => [target.id, target.name])));
	const slowest = $derived<TopItem[]>(
		[...fleetStore.figures.p95]
			.filter(([id, value]) => names.has(id) && Number.isFinite(value))
			.map(([id, value]) => ({ key: String(id), label: names.get(id) ?? String(id), value }))
	);
	const range = $derived(formatDuration(consoleState.rangeSeconds));
</script>

<section class="min-w-0 rounded-xl border border-line bg-surface px-4 pt-3.5 pb-4" aria-labelledby="latency-title">
	<div class="flex flex-wrap items-baseline gap-x-2.5 gap-y-1">
		<h2 id="latency-title" class="m-0 text-base font-semibold text-ink">{m.overview_latency_title()}</h2>
		<span class="text-xs text-ink-2">{m.overview_latency_hint({ range })}</span>
	</div>
	<div class="mt-3 grid gap-5 lg:grid-cols-[minmax(0,1fr)_minmax(0,15rem)]">
		<div class="min-w-0">
			{#if failed && !trend}
				<p class="py-6 text-sm text-ink-2">{m.overview_chart_error()}</p>
			{:else if !trend || !Chart}
				<Skeleton class="h-[200px] w-full" />
			{:else if series.length === 0}
				<p class="py-6 text-sm text-ink-2">{m.overview_latency_empty()}</p>
			{:else}
				<Chart {series} height={200} unit="ms" colorTokens={['info', 'ink-3']} />
			{/if}
		</div>
		<div class="min-w-0">
			<h3 class="m-0 text-sm font-semibold text-ink">{m.overview_latency_slowest()}</h3>
			{#if slowest.length === 0}
				<p class="mt-2 text-xs text-ink-2">{fleetStore.loading ? m.console_kpi_none() : m.overview_latency_empty()}</p>
			{:else}
				<TopN
					items={slowest}
					limit={5}
					unit="ms"
					format={(value) => `${Math.round(value)} ms`}
					label={m.overview_latency_slowest()}
					onselect={(key) => void goto(`/targets/${key}`)}
					class="mt-2"
				/>
			{/if}
		</div>
	</div>
</section>
