<script lang="ts">
	/** Time-series chart of a MetricsQL query over the dashboard's range. */
	import { queryRange } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import Chart, { type Serie } from '#lib/components/Chart.svelte';
	import { ErrorNotice, Skeleton } from '#lib/ui/index.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { stepFor } from '#lib/dashboards/widgets.js';

	interface Props {
		query: string;
		unit: string;
		/** Seconds back from now. */
		range: number;
		/** Changes on every automatic refresh. */
		tick: number;
	}

	let { query, unit, range, tick }: Props = $props();

	const MAX_SERIES = 12;
	let series = $state<Serie[] | null>(null);
	let error = $state<unknown>(null);
	let height = $state(160);
	let body = $state<HTMLDivElement | null>(null);

	const names = $derived(new Map(alertsStore.targets.map((t) => [String(t.id), t.name])));

	function labelOf(metric: Record<string, string>): string {
		const parts = Object.entries(metric)
			.filter(([key]) => key !== '__name__')
			.map(([key, value]) => (key === 'target' ? (names.get(value) ?? value) : value));
		return parts.length > 0 ? parts.join(' · ') : (metric.__name__ ?? query);
	}

	$effect(() => {
		void tick;
		const text = query;
		const seconds = range;
		const controller = new AbortController();
		const end = Math.floor(Date.now() / 1000);
		queryRange({ query: text, start: (end - seconds) * 1000, end: end * 1000, step: stepFor(seconds) }, controller.signal)
			.then((rows) => {
				series = rows.slice(0, MAX_SERIES).map((row) => ({
					label: labelOf(row.metric),
					points: row.values
						.map(([ts, value]) => [ts, Number.parseFloat(value)] as [number, number])
						.filter(([, value]) => Number.isFinite(value))
				}));
				error = null;
			})
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				error = cause;
			});
		return () => controller.abort();
	});

	$effect(() => {
		if (!body) return;
		const observer = new ResizeObserver((entries) => {
			height = Math.max(100, Math.floor(entries[0].contentRect.height) - 4);
		});
		observer.observe(body);
		return () => observer.disconnect();
	});
</script>

<div bind:this={body} class="h-full min-h-24">
	{#if error}
		<ErrorNotice {error} local={false} />
	{:else if series === null}
		<Skeleton class="h-full min-h-24 w-full" />
	{:else if series.length === 0 || series.every((s) => s.points.length === 0)}
		<p class="text-sm text-ink-2">{m.dash_widget_no_data()}</p>
	{:else}
		<Chart {series} {height} {unit} />
	{/if}
</div>
