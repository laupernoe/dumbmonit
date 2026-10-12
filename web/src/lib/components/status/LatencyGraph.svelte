<script lang="ts">
	/**
	 * Optional response-time graph of one public service: a small line over the
	 * last 24 hours or 7 days (switch), with the average and the peak spelled out
	 * so the figures never depend on the drawing. Renders nothing without data.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import type { PublicLatencySeries } from '#lib/api/index.js';

	interface Props {
		series: PublicLatencySeries | undefined;
		label: string;
	}

	let { series, label }: Props = $props();

	let range = $state<'h24' | 'd7'>('h24');
	const points = $derived(series ? series[range] : []);
	const hasAny = $derived(!!series && (series.h24.length > 0 || series.d7.length > 0));

	const W = 300;
	const H = 56;
	const stats = $derived.by(() => {
		if (points.length === 0) return null;
		const values = points.map((p) => p[1]);
		const max = Math.max(...values);
		const avg = values.reduce((a, b) => a + b, 0) / values.length;
		const t0 = points[0][0];
		const span = Math.max(1, points[points.length - 1][0] - t0);
		const top = Math.max(max, 1);
		const path = points
			.map(([t, v], i) => `${i === 0 ? 'M' : 'L'}${(((t - t0) / span) * W).toFixed(1)},${(H - 3 - (v / top) * (H - 6)).toFixed(1)}`)
			.join(' ');
		return { max, avg, path };
	});

	function fmt(value: number): string {
		if (value < 1000) return m.status_service_row_ms({ value: Math.round(value) });
		return m.status_service_row_seconds({ value: (Math.round(value / 100) / 10).toString() });
	}
</script>

{#if hasAny}
	<div class="mt-3">
		<div class="flex flex-wrap items-center justify-between gap-2">
			<span class="label-tape text-ink-3">{m.status_latency_title()}</span>
			<div class="inline-flex overflow-hidden rounded-[var(--radius-plate)] border border-line" role="group" aria-label={m.status_latency_range_label()}>
				{#each [['h24', m.status_latency_range_24h()], ['d7', m.status_latency_range_7d()]] as [value, text] (value)}
					<button
						type="button"
						class={`min-h-8 px-3 text-xs font-semibold focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-ink ${range === value ? 'bg-ink text-surface' : 'bg-surface text-ink-2'}`}
						aria-pressed={range === value}
						onclick={() => (range = value as 'h24' | 'd7')}
					>
						{text}
					</button>
				{/each}
			</div>
		</div>
		{#if stats}
			<svg class="mt-2 h-14 w-full text-info" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" role="img" aria-label={m.status_latency_chart_label({ label })}>
				<line x1="0" y1={H - 0.5} x2={W} y2={H - 0.5} stroke="currentColor" stroke-opacity="0.25" vector-effect="non-scaling-stroke" />
				<path d={stats.path} fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" vector-effect="non-scaling-stroke" />
			</svg>
			<p class="tnum mt-1 text-[0.8125rem] text-ink-2">{m.status_latency_summary({ avg: fmt(stats.avg), peak: fmt(stats.max) })}</p>
		{:else}
			<p class="mt-2 text-[0.8125rem] text-ink-3">{m.status_latency_none()}</p>
		{/if}
	</div>
{/if}
