<script lang="ts">
	/**
	 * Overview: the night-sky banner with its one line of status, a band of
	 * five figures, the health river, and the dense monitor table that feeds
	 * the context panel on the right. A fresh instance gets the first-run guide
	 * (administrators) or a way in instead of empty tables.
	 *
	 * Monitors, probes and alerts come from the app-wide store (polled every
	 * 30 s); the river, latencies, p95 and availability from the fleet store
	 * (one set of queries, refreshed every minute while LIVE is on).
	 */
	import { untrack } from 'svelte';
	import { getOnboarding, listCollectors, type OnboardingState } from '#lib/api/index.js';
	import { countAlertsByTarget, countsAsProblem } from '#lib/alerts-attention.js';
	import { formatDuration } from '#lib/format.js';
	import { kindLabel } from '#lib/kind-label.js';
	import { m } from '#lib/paraglide/messages.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { consoleState } from '#lib/stores/console.svelte.js';
	import { Button, EmptyState, ErrorNotice, calmMotion } from '#lib/ui/index.js';
	import FirstRun from '#lib/components/overview/FirstRun.svelte';
	import { bannerLine, type BannerLine } from './banner.js';
	import StatusBanner from './StatusBanner.svelte';
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

	// --- The banner's one line ---------------------------------------------------
	function words(line: BannerLine): { headline: string; sub: string } {
		switch (line.kind) {
			case 'empty':
				return { headline: m.console_banner_empty(), sub: '' };
			case 'down':
				return {
					headline: line.name !== null ? m.console_banner_down_one({ name: line.name }) : m.console_banner_down_many({ count: line.count }),
					sub:
						line.held > 0
							? line.sent
								? m.console_banner_held_sent({ count: line.held })
								: m.console_banner_held({ count: line.held })
							: line.sent
								? m.console_banner_sent()
								: ''
				};
			case 'attention':
				if (line.count === 0) return { headline: m.console_banner_alerts({ count: line.alerts }), sub: '' };
				return {
					headline: line.name !== null ? m.console_banner_attention_one({ name: line.name }) : m.console_banner_attention_many({ count: line.count }),
					sub: line.alerts > 0 ? m.console_banner_alerts({ count: line.alerts }) : ''
				};
			case 'waiting':
				return { headline: m.console_banner_up_of({ up: line.up, total: line.total }), sub: m.console_banner_waiting({ count: line.waiting }) };
			case 'up':
				if (line.count === 0) return { headline: m.console_banner_all_paused(), sub: '' };
				return { headline: m.console_banner_all_up({ count: line.count }), sub: line.paused > 0 ? m.console_banner_paused({ count: line.paused }) : '' };
		}
	}

	/** Nothing read yet: the line waits rather than say "nothing to watch". */
	const reading = $derived(alertsStore.loading && alertsStore.targets.length === 0);
	const banner = $derived(
		reading
			? { headline: '', sub: '' }
			: words(bannerLine(rows.map((row) => ({ id: row.target.id, name: row.target.name, status: row.status })), alertsStore.alerts))
	);
	const noMonitors = $derived(!reading && alertsStore.available && alertsStore.targets.length === 0);

	// --- First run -----------------------------------------------------------------
	// The three-step guide, as the server keeps it. Decided once per visit and
	// then held, so the last step turning green does not make it vanish.
	// Silent on failure: no guide is better than an error over a working console.
	let onboarding = $state<OnboardingState | null>(null);
	let showGuide = $state(false);
	let hasDemo = $state(false);
	$effect(() => {
		// A viewer can neither add a device nor create a channel.
		if (!auth.isAdmin) return;
		const controller = new AbortController();
		getOnboarding(controller.signal)
			.then((next) => {
				onboarding = next;
				showGuide = !next.skipped && !next.complete;
				if (!showGuide) return;
				void listCollectors(controller.signal)
					.then((kinds) => (hasDemo = kinds.some((kind) => kind.kind === 'dummy')))
					.catch(() => {});
			})
			.catch(() => {});
		return () => controller.abort();
	});

	/** "Or [install the agent on a machine], or [watch a remote site] through one." cut around its two links. */
	const orParts = $derived(
		m
			.overview_page_or({
				agent: '\u0000' + m.overview_page_or_agent() + '\u0000',
				relay: '\u0000' + m.overview_page_or_relay() + '\u0000'
			})
			.split('\u0000')
	);

	const animate = !calmMotion();
</script>

<div class="flex min-h-0 flex-1 flex-col gap-3.5">
	<h1 class="sr-only">{m.misc_nav_overview()}</h1>

	<StatusBanner headline={banner.headline} sub={banner.sub} />

	{#if showGuide && onboarding}
		<!-- The guide replaces the console until it is done: one thing to do at a time. -->
		<FirstRun
			guide={onboarding}
			{hasDemo}
			onchange={(next) => {
				onboarding = next;
				if (next.skipped) showGuide = false;
			}}
		/>
	{:else if !alertsStore.available && alertsStore.targets.length === 0 && alertsStore.lastError}
		<ErrorNotice error={alertsStore.lastError} title={m.overview_page_error_load()} onretry={() => void alertsStore.refresh()} />
	{:else if noMonitors}
		<EmptyState title={m.overview_page_empty_title()} description={m.overview_page_empty_text()} mascot="watch">
			{#snippet action()}
				<Button variant="primary" href="/targets/new">{m.overview_first_device_action()}</Button>
				{#if auth.isAdmin}
					<p class="mt-4 text-sm text-ink-2">
						{orParts[0]}<a href="/targets/new?kind=agent" class="inline-block py-1 font-semibold text-ink underline decoration-line-strong underline-offset-2 hover:decoration-ink">{orParts[1]}</a>{orParts[2]}<a href="/targets/new?kind=agent&via=relay" class="inline-block py-1 font-semibold text-ink underline decoration-line-strong underline-offset-2 hover:decoration-ink">{orParts[3]}</a>{orParts[4]}
					</p>
				{/if}
			{/snippet}
		</EmptyState>
	{:else}
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
	{/if}
</div>
