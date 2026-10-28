<script lang="ts">
	/**
	 * What a link carries: the rate each way with its share of the port speed,
	 * the errors, and the last hour as a small curve. Shared by the link panel
	 * on the map and by the rows of the phone fallback. Everything is written
	 * (rates, percentages, peak): the curve is a complement, its two lines are
	 * told apart by their stroke, not only their colour.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { getPortTraffic } from '#lib/api/topology.js';
	import type { PortTraffic, TopologyEdgeTraffic } from '#lib/api/index.js';
	import { formatPct, formatRate, sparkPaths } from './traffic';
	import { portsText } from './legend';

	interface Props {
		from: string;
		to: string;
		traffic: TopologyEdgeTraffic;
		/** Name of a node id. */
		name: (id: string) => string;
		/** An end of the link is down or unreachable: the figures are not current. */
		stale?: boolean;
		fromPort?: string | null;
		toPort?: string | null;
	}

	let { from, to, traffic, name, stale = false, fromPort = null, toPort = null }: Props = $props();

	const WIDTH = 300;
	const HEIGHT = 64;

	const targetId = $derived.by(() => {
		const id = Number(traffic.measured_on.replace(/^t/, ''));
		return Number.isInteger(id) && id > 0 ? id : null;
	});
	const portsKey = $derived(traffic.ports.join(','));
	const locale = $derived(getLocale());

	let curve = $state<PortTraffic | null>(null);
	let failed = $state(false);

	$effect(() => {
		const id = targetId;
		const ports = portsKey ? portsKey.split(',') : [];
		curve = null;
		failed = false;
		if (id === null || ports.length === 0) {
			failed = true;
			return;
		}
		const controller = new AbortController();
		getPortTraffic(id, ports, controller.signal)
			.then((result) => (curve = result))
			.catch(() => {
				if (!controller.signal.aborted) failed = true;
			});
		return () => controller.abort();
	});

	const paths = $derived(curve ? sparkPaths(curve.points, WIDTH, HEIGHT) : null);
	const measured = $derived(name(traffic.measured_on));
	const perMinute = $derived(new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(traffic.errors_per_sec * 60));

	function share(value: number | null): string {
		return value === null ? '' : ` · ${formatPct(value * 100, locale)}`;
	}
</script>

<div class="flex flex-col gap-3 text-sm text-ink">
	<p class="font-semibold">{name(from)} <span class="text-ink-3" aria-hidden="true">↔</span> {name(to)}</p>
	<p class="-mt-2 text-xs text-ink-2">
		{m.topology_traffic_ports({ ports: portsText(fromPort, toPort) ?? traffic.ports.join(', '), name: measured })}
	</p>

	{#if stale}
		<p class="rounded-lg border border-dashed border-advisory bg-advisory-soft px-3 py-2 text-xs text-advisory-ink">{m.topology_traffic_stale()}</p>
	{/if}

	<dl class="grid grid-cols-[1fr_auto] gap-x-4 gap-y-1.5">
		<dt class="text-ink-2">{m.topology_traffic_in({ name: measured })}</dt>
		<dd class="text-right font-semibold tabular-nums">{formatRate(traffic.bps_in, locale)}{share(traffic.utilization_in)}</dd>
		<dt class="text-ink-2">{m.topology_traffic_out({ name: measured })}</dt>
		<dd class="text-right font-semibold tabular-nums">{formatRate(traffic.bps_out, locale)}{share(traffic.utilization_out)}</dd>
		<dt class="text-ink-2">{m.topology_traffic_speed_label()}</dt>
		<dd class="text-right tabular-nums">{traffic.speed_bps === null ? m.topology_traffic_speed_unknown() : formatRate(traffic.speed_bps, locale)}</dd>
		<dt class="text-ink-2">{m.topology_traffic_errors_label()}</dt>
		<dd class={`text-right tabular-nums ${traffic.errors_per_sec >= 0.05 ? 'font-semibold text-advisory-ink' : ''}`}>
			{traffic.errors_per_sec > 0 ? m.topology_traffic_errors_rate({ count: perMinute }) : m.topology_traffic_errors_none()}
		</dd>
	</dl>

	<div>
		<p class="mb-1 flex items-baseline justify-between text-xs text-ink-2">
			<span>{m.topology_traffic_hour()}</span>
			{#if paths && paths.max > 1}<span class="tabular-nums">{m.topology_traffic_hour_peak({ rate: formatRate(paths.max, locale) })}</span>{/if}
		</p>
		{#if failed}
			<p class="text-xs text-ink-2">{m.topology_traffic_hour_error()}</p>
		{:else if !paths}
			<div class="h-16 animate-pulse rounded-lg bg-surface-2" aria-hidden="true"></div>
			<span class="sr-only">{m.topology_traffic_hour_loading()}</span>
		{:else if paths.inbound}
			<svg viewBox={`0 0 ${WIDTH} ${HEIGHT}`} class="block h-16 w-full overflow-visible rounded-lg bg-surface-2" role="img" aria-label={m.topology_traffic_chart_label()} preserveAspectRatio="none">
				<path d={paths.outbound} fill="none" class="stroke-ink-2" stroke-width="1.5" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" />
				<path d={paths.inbound} fill="none" class="stroke-signal" stroke-width="1.8" vector-effect="non-scaling-stroke" />
			</svg>
			<p class="mt-1 flex flex-wrap gap-x-4 text-xs text-ink-2">
				<span class="inline-flex items-center gap-1.5"><svg width="18" height="6" aria-hidden="true"><line x1="0" y1="3" x2="18" y2="3" class="stroke-signal" stroke-width="2" /></svg>{m.topology_traffic_legend_in()}</span>
				<span class="inline-flex items-center gap-1.5"><svg width="18" height="6" aria-hidden="true"><line x1="0" y1="3" x2="18" y2="3" class="stroke-ink-2" stroke-width="2" stroke-dasharray="4 3" /></svg>{m.topology_traffic_legend_out()}</span>
			</p>
		{:else}
			<p class="text-xs text-ink-2">{m.topology_traffic_hour_error()}</p>
		{/if}
	</div>
</div>
