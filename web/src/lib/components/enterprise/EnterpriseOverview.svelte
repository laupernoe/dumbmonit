<script lang="ts">
	/**
	 * Enterprise overview: a band of five figures, the health river, and the
	 * dense monitor table that feeds the context panel on the right.
	 *
	 * Monitors, probes and alerts come from the app-wide store (polled every
	 * 30 s); the river, latencies, p95 and availability from the fleet store
	 * (one set of queries, refreshed every minute while LIVE is on).
	 */
	import { untrack } from 'svelte';
	import { countAlertsByTarget, countsAsProblem } from '#lib/alerts-attention.js';
	import { formatDuration } from '#lib/format.js';
	import { kindLabel } from '#lib/kind-label.js';
	import { m } from '#lib/paraglide/messages.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { consoleState } from '#lib/stores/console.svelte.js';
	import { ErrorNotice } from '#lib/ui/index.js';
	import { calmMotion } from '#lib/ui/index.js';
	import { fleetStore } from './fleet.svelte.js';
	import { RIVER_SLOTS, mean, meanTimeToResolve } from './fleet.js';
	import HealthRiver from './HealthRiver.svelte';
	import MonitorTable, { type MonitorRow } from './MonitorTable.svelte';
	import Sparkline from './Sparkline.svelte';
	import { STATUS_RANK, consoleStatus } from './status.js';

	// One subscription for the life of the view; the range is followed separately.
	$effect(() => untrack(() => fleetStore.start()));
	// A new range reloads the figures at once.
	$effect(() => {
		consoleState.range;
		fleetStore.rangeChanged();
	});

	const alertCounts = $derived(countAlertsByTarget(alertsStore.alerts));

	const rows = $derived<MonitorRow[]>(
		alertsStore.targets.map((target) => ({
			target,
			status: consoleStatus(target, alertsStore.probes.get(target.id), alertCounts.get(target.id)),
			kind: kindLabel(target.kind),
			latency: fleetStore.latency.get(target.id) ?? [],
			p95: fleetStore.figures.p95.get(target.id) ?? null,
			uptime: fleetStore.figures.uptime30.get(target.id) ?? null
		}))
	);

	const riverRows = $derived(
		[...rows]
			.filter((row) => fleetStore.river.has(row.target.id))
			.sort((a, b) => STATUS_RANK[a.status] - STATUS_RANK[b.status] || a.target.name.localeCompare(b.target.name))
			.map((row) => ({ id: row.target.id, name: row.target.name, cells: fleetStore.river.get(row.target.id) ?? [] }))
	);

	// --- The five figures ------------------------------------------------------
	const total = $derived(rows.filter((r) => r.status !== 'paused').length);
	const up = $derived(rows.filter((r) => r.status === 'up' || r.status === 'degraded').length);
	const down = $derived(rows.filter((r) => r.status === 'down').length);
	const paused = $derived(rows.filter((r) => r.status === 'paused').length);

	/** Per slot of the river: how many monitors were up, and the share of healthy cells. */
	const upTrend = $derived.by(() => {
		const counts: number[] = [];
		for (let i = 0; i < RIVER_SLOTS; i += 1) {
			let n = 0;
			for (const cells of fleetStore.river.values()) if (cells[i] === 'up') n += 1;
			counts.push(n);
		}
		return fleetStore.river.size > 0 ? counts : [];
	});
	const healthTrend = $derived.by(() => {
		const shares: number[] = [];
		for (let i = 0; i < RIVER_SLOTS; i += 1) {
			let measured = 0;
			let healthy = 0;
			for (const cells of fleetStore.river.values()) {
				if (cells[i] === 'none') continue;
				measured += 1;
				if (cells[i] === 'up') healthy += 1;
			}
			if (measured > 0) shares.push(healthy / measured);
		}
		return shares;
	});
	const latencyTrend = $derived.by(() => {
		const series = [...fleetStore.latency.values()];
		const length = Math.max(0, ...series.map((s) => s.length));
		const out: number[] = [];
		for (let i = 0; i < length; i += 1) {
			const value = mean(series.map((s) => s[i]).filter((v) => v !== undefined));
			if (value !== null) out.push(value);
		}
		return out;
	});

	const availability = $derived(mean(fleetStore.figures.uptime30.values()));
	const firing = $derived(alertsStore.alerts.filter(countsAsProblem));
	const critical = $derived(firing.filter((a) => a.severity === 'critical').length);
	const mttr = $derived(meanTimeToResolve(fleetStore.history));
	const resolved = $derived(fleetStore.history.filter((e) => e.to_phase === 'resolved').length);

	type Kpi = { id: string; label: string; value: string; unit: string; note: string; noteTone: string; spark: number[]; stroke: string };
	const kpis = $derived<Kpi[]>([
		{
			id: 'up',
			label: m.console_kpi_up(),
			value: String(up),
			unit: m.console_kpi_up_unit({ total }),
			note: m.console_kpi_up_note({ down, paused }),
			noteTone: down > 0 ? 'text-warning-ink' : 'text-ink-2',
			spark: upTrend,
			stroke: 'stroke-signal'
		},
		{
			id: 'availability',
			label: m.console_kpi_availability(),
			value: availability === null ? '—' : availability >= 99.9995 ? '100' : availability.toFixed(3),
			unit: availability === null ? '' : '%',
			note: availability === null ? m.console_kpi_none() : m.console_kpi_availability_note({ count: fleetStore.figures.uptime30.size }),
			noteTone: 'text-ink-2',
			spark: healthTrend,
			stroke: 'stroke-signal'
		},
		{
			id: 'p95',
			label: m.console_kpi_p95(),
			value: fleetStore.figures.fleetP95 === null ? '—' : String(Math.round(fleetStore.figures.fleetP95)),
			unit: fleetStore.figures.fleetP95 === null ? '' : 'ms',
			note: m.console_kpi_p95_note({ range: consoleState.range }),
			noteTone: 'text-ink-2',
			spark: latencyTrend,
			stroke: 'stroke-info'
		},
		{
			id: 'alerts',
			label: m.console_kpi_alerts(),
			value: String(firing.length),
			unit: '',
			note: m.console_kpi_alerts_note({ critical, warning: firing.length - critical }),
			noteTone: critical > 0 ? 'text-warning-ink' : firing.length > 0 ? 'text-advisory-ink' : 'text-ink-2',
			spark: [],
			stroke: 'stroke-warning'
		},
		{
			id: 'mttr',
			label: m.console_kpi_mttr(),
			value: mttr === null ? '—' : formatDuration(mttr),
			unit: '',
			note: mttr === null ? m.console_kpi_none() : m.console_kpi_mttr_note({ count: resolved }),
			noteTone: 'text-ink-2',
			spark: [],
			stroke: 'stroke-info'
		}
	]);

	const animate = !calmMotion();
</script>

<div class="flex min-h-0 flex-1 flex-col gap-3.5">
	<h1 class="sr-only">{m.misc_nav_overview()}</h1>

	<div class="grid grid-cols-2 gap-px overflow-hidden rounded-xl border border-line bg-line sm:grid-cols-3 xl:grid-cols-5">
		{#each kpis as kpi, i (kpi.id)}
			<div class={`min-w-0 bg-surface px-4 py-3.5 ${animate ? 'rise-in' : ''} ${i === 4 ? 'max-sm:col-span-2' : ''}`} style={`--rise-delay: ${i * 50}ms`}>
				<div class="truncate text-xs text-ink-2">{kpi.label}</div>
				<div class="mt-1.5 flex items-end gap-2.5">
					<span class="font-mono text-2xl font-medium tracking-[-0.03em] text-ink tabular-nums">{kpi.value}</span>
					{#if kpi.unit}<span class="pb-1 text-xs text-ink-2">{kpi.unit}</span>{/if}
					{#if kpi.spark.length > 1}
						<Sparkline values={kpi.spark} width={84} height={26} stroke={kpi.stroke} draw={animate} class="ml-auto max-sm:hidden" />
					{/if}
				</div>
				<div class={`mt-1 truncate text-xs ${kpi.noteTone}`}>{kpi.note}</div>
			</div>
		{/each}
	</div>

	{#if fleetStore.error}
		<ErrorNotice error={fleetStore.error} />
	{/if}

	<HealthRiver rows={riverRows} />

	<MonitorTable {rows} />
</div>
