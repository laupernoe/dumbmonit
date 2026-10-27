<script lang="ts">
	/**
	 * Wall mode, Enterprise edition: a room monitor in the console's own voice.
	 * Black ground, Geist, no scenery. From the top: the verdict in a few words
	 * and the clock, the five fleet figures, the health river (the one big
	 * picture of the board), then what is open right now and who is on call.
	 *
	 * Reads the same stores as the console (monitors, probes and alerts every
	 * 30 s; the river and the figures from the fleet store). Escape or Exit
	 * goes back to the console. The screen stays awake while the wall is open.
	 */
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { AlertTriangle, CircleAlert, CircleCheck, CircleDashed, X } from 'lucide-svelte';
	import { getOnCallNow, listAlertRules, listOnCallPeople, type AlertRule, type OnCallEntry, type Person } from '#lib/api/index.js';
	import { countAlertsByTarget, countsAsProblem } from '#lib/alerts-attention.js';
	import { TONE_BAR } from '#lib/components/alerts/helpers.js';
	import { readSky } from '#lib/components/overview/sky.js';
	import HealthRiver from '#lib/components/enterprise/HealthRiver.svelte';
	import Sparkline from '#lib/components/enterprise/Sparkline.svelte';
	import { fleetStore } from '#lib/components/enterprise/fleet.svelte.js';
	import { RIVER_SLOTS, mean, meanTimeToResolve } from '#lib/components/enterprise/fleet.js';
	import { STATUS_RANK, consoleStatus } from '#lib/components/enterprise/status.js';
	import { formatDateTime, formatDuration, formatRelative } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { instanceStore } from '#lib/stores/instance.svelte.js';
	import { consoleState } from '#lib/stores/console.svelte.js';
	import { Button, calmMotion } from '#lib/ui/index.js';

	const ALERT_ROWS = 7;

	let now = $state(new Date());
	let rules = $state<AlertRule[]>([]);
	let onCall = $state<OnCallEntry[]>([]);
	let people = $state<Map<number, Person>>(new Map());

	// The shared stores poll from the layout; this view only asks the fleet
	// store to keep its figures fresh, and reads the rules and the rota once.
	$effect(() => untrack(() => fleetStore.start()));
	$effect(() => {
		const controller = new AbortController();
		listAlertRules(controller.signal)
			.then((list) => (rules = list))
			.catch(() => {});
		if (!auth.scoped) {
			const refreshRota = () =>
				Promise.all([getOnCallNow(controller.signal), listOnCallPeople(controller.signal)])
					.then(([entries, list]) => {
						onCall = entries;
						people = new Map(list.map((person) => [person.id, person]));
					})
					.catch(() => {});
			void refreshRota();
			const timer = setInterval(() => void refreshRota(), 5 * 60_000);
			return () => {
				controller.abort();
				clearInterval(timer);
			};
		}
		return () => controller.abort();
	});

	// One tick per second drives the clock and "updated … ago".
	$effect(() => {
		const timer = setInterval(() => (now = new Date()), 1000);
		return () => clearInterval(timer);
	});

	// Keep the screen on; browsers release the lock when the tab is hidden.
	$effect(() => {
		let lock: WakeLockSentinel | null = null;
		let disposed = false;
		const request = async () => {
			if (disposed || document.visibilityState !== 'visible') return;
			try {
				lock = (await navigator.wakeLock?.request('screen')) ?? null;
			} catch {
				// Not permitted (battery saver, insecure context): the wall still works.
			}
		};
		void request();
		document.addEventListener('visibilitychange', request);
		return () => {
			disposed = true;
			document.removeEventListener('visibilitychange', request);
			void lock?.release().catch(() => {});
		};
	});

	function exit() {
		void goto('/');
	}
	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			exit();
		}
	}

	// --- The verdict -----------------------------------------------------------
	const sky = $derived(readSky({ targets: alertsStore.targets, probes: alertsStore.probes, alerts: alertsStore.alerts, rules }));
	const verdict = $derived.by(() => {
		if (sky.counts.devices === 0) return { tone: 'ghost' as const, text: m.wall_status_empty() };
		if (sky.attention > 0) {
			const severe = sky.counts.warnings > 0 || sky.counts.unreachable > 0;
			return {
				tone: severe ? ('warning' as const) : ('advisory' as const),
				text: sky.attention === 1 ? m.wall_status_problem_one() : m.wall_status_problems({ count: sky.attention })
			};
		}
		if (sky.counts.reporting === 0) return { tone: 'ghost' as const, text: m.wall_status_waiting() };
		return { tone: 'signal' as const, text: m.wall_status_ok() };
	});
	const VERDICT_ICON = { signal: CircleCheck, warning: AlertTriangle, advisory: CircleAlert, ghost: CircleDashed };
	const VERDICT_TEXT = { signal: 'text-signal-ink', warning: 'text-warning-ink', advisory: 'text-advisory-ink', ghost: 'text-ink-2' };
	const Icon = $derived(VERDICT_ICON[verdict.tone]);

	const clock = $derived(now.toLocaleTimeString(getLocale(), { hour: '2-digit', minute: '2-digit', hour12: false }));
	const dateLabel = $derived(now.toLocaleDateString(getLocale(), { weekday: 'long', day: 'numeric', month: 'long' }));
	// The shared store does not stamp its reads: each refresh replaces its lists, so a change of any is "now".
	let checked = $state<Date | null>(null);
	$effect(() => {
		if (alertsStore.loading) return;
		void alertsStore.alerts;
		void alertsStore.targets;
		void alertsStore.probes;
		checked = new Date();
	});
	const updated = $derived.by(() => {
		if (!checked) return m.wall_updated_waiting();
		const seconds = Math.max(0, Math.round((now.getTime() - checked.getTime()) / 1000));
		return seconds < 90 ? m.wall_updated_seconds({ seconds }) : m.wall_updated_relative({ when: formatRelative(checked) });
	});

	// --- The five figures ------------------------------------------------------
	const alertCounts = $derived(countAlertsByTarget(alertsStore.alerts));
	const statuses = $derived(
		alertsStore.targets.map((target) => ({
			target,
			status: consoleStatus(target, alertsStore.probes.get(target.id), alertCounts.get(target.id))
		}))
	);
	const total = $derived(statuses.filter((r) => r.status !== 'paused').length);
	const up = $derived(statuses.filter((r) => r.status === 'up' || r.status === 'degraded').length);
	const down = $derived(statuses.filter((r) => r.status === 'down').length);
	const paused = $derived(statuses.filter((r) => r.status === 'paused').length);
	const firing = $derived(alertsStore.alerts.filter(countsAsProblem));
	const critical = $derived(firing.filter((a) => a.severity === 'critical').length);
	const availability = $derived(mean(fleetStore.figures.uptime30.values()));
	const mttr = $derived(meanTimeToResolve(fleetStore.history));
	const resolved = $derived(fleetStore.history.filter((e) => e.to_phase === 'resolved').length);
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

	type Kpi = { id: string; label: string; value: string; unit: string; note: string; tone: string; spark: number[]; stroke: string };
	const kpis = $derived<Kpi[]>([
		{
			id: 'up',
			label: m.console_kpi_up(),
			value: String(up),
			unit: m.console_kpi_up_unit({ total }),
			note: m.console_kpi_up_note({ down, paused }),
			tone: down > 0 ? 'text-warning-ink' : 'text-ink-2',
			spark: [],
			stroke: 'stroke-signal'
		},
		{
			id: 'availability',
			label: m.console_kpi_availability(),
			value: availability === null ? '—' : availability >= 99.9995 ? '100' : availability.toFixed(3),
			unit: availability === null ? '' : '%',
			note: availability === null ? m.console_kpi_none() : m.console_kpi_availability_note({ count: fleetStore.figures.uptime30.size }),
			tone: 'text-ink-2',
			spark: healthTrend,
			stroke: 'stroke-signal'
		},
		{
			id: 'p95',
			label: m.console_kpi_p95(),
			value: fleetStore.figures.fleetP95 === null ? '—' : String(Math.round(fleetStore.figures.fleetP95)),
			unit: fleetStore.figures.fleetP95 === null ? '' : 'ms',
			note: m.console_kpi_p95_note({ range: consoleState.range }),
			tone: 'text-ink-2',
			spark: latencyTrend,
			stroke: 'stroke-info'
		},
		{
			id: 'alerts',
			label: m.console_kpi_alerts(),
			value: String(firing.length),
			unit: '',
			note: m.console_kpi_alerts_note({ critical, warning: firing.length - critical }),
			tone: critical > 0 ? 'text-warning-ink' : firing.length > 0 ? 'text-advisory-ink' : 'text-ink-2',
			spark: [],
			stroke: 'stroke-warning'
		},
		{
			id: 'mttr',
			label: m.console_kpi_mttr(),
			value: mttr === null ? '—' : formatDuration(mttr),
			unit: '',
			note: mttr === null ? m.console_kpi_none() : m.console_kpi_mttr_note({ count: resolved }),
			tone: 'text-ink-2',
			spark: [],
			stroke: 'stroke-info'
		}
	]);

	const riverRows = $derived(
		[...statuses]
			.filter((row) => fleetStore.river.has(row.target.id))
			.sort((a, b) => STATUS_RANK[a.status] - STATUS_RANK[b.status] || a.target.name.localeCompare(b.target.name))
			.map((row) => ({ id: row.target.id, name: row.target.name, cells: fleetStore.river.get(row.target.id) ?? [] }))
	);

	// --- Open alerts and the rota ----------------------------------------------
	const attention = $derived(sky.needsYou.filter((row) => row.rank < 4));
	function rowTitle(row: (typeof attention)[number]): string {
		if (row.kind === 'device') return row.target.name;
		return row.target ? `${row.target.name}: ${row.alert.rule_name}` : row.alert.rule_name;
	}
	const onCallRows = $derived(
		onCall
			.filter((entry) => entry.user_id !== null)
			.slice(0, 4)
			.map((entry) => {
				const person = people.get(entry.user_id as number);
				const name = person?.display_name || person?.username || m.console_panel_oncall_unknown();
				return { key: `${entry.schedule_id}:${entry.user_id}`, name, initial: name.charAt(0).toUpperCase(), entry };
			})
	);

	const animate = !calmMotion();
</script>

<svelte:head><title>{m.wall_page_title()}</title></svelte:head>
<svelte:window onkeydown={onKeydown} />

<div class="ent-wall fixed inset-0 z-[60] box-border overflow-y-auto bg-canvas pt-[var(--safe-top)] pr-[var(--safe-right)] pb-[var(--safe-bottom)] pl-[var(--safe-left)] text-ink">
	<div class="mx-auto flex min-h-full w-full max-w-[120rem] flex-col gap-5 px-5 py-5 lg:gap-6 lg:px-10 lg:py-8">
		<header class="flex flex-wrap items-start justify-between gap-x-10 gap-y-3">
			<div class="min-w-0">
				<p class="m-0 truncate text-sm text-ink-2 lg:text-base">{instanceStore.displayName}</p>
				<div class={`mt-2 flex items-center gap-3 lg:gap-5 ${VERDICT_TEXT[verdict.tone]}`} aria-live="polite">
					<Icon class="size-9 shrink-0 lg:size-14" strokeWidth={1.6} aria-hidden="true" />
					<h1 class="m-0 text-[2.25rem] leading-none font-semibold tracking-[-0.035em] lg:text-[4rem]">{verdict.text}</h1>
				</div>
			</div>
			<div class="text-right">
				<time class="block font-mono text-[3rem] leading-none font-medium tracking-[-0.04em] text-ink tabular-nums lg:text-[5.5rem]" datetime={now.toISOString()}>{clock}</time>
				<p class="m-0 mt-2 text-sm text-ink-2 capitalize lg:text-lg">{dateLabel}</p>
				<p class="m-0 mt-1 flex items-center justify-end gap-2 font-mono text-xs text-ink-3">
					<span aria-hidden="true" class={`size-1.5 rounded-full ${animate ? 'ob-live' : ''} bg-signal`}></span>
					{updated}
				</p>
			</div>
		</header>

		<section aria-label={m.misc_nav_overview()} class="grid grid-cols-2 gap-px overflow-hidden rounded-xl border border-line bg-line lg:grid-cols-5">
			{#each kpis as kpi, i (kpi.id)}
				<div class={`min-w-0 bg-surface px-4 py-4 lg:px-6 lg:py-6 ${i === 4 ? 'max-lg:col-span-2' : ''}`}>
					<div class="truncate text-sm text-ink-2 lg:text-base">{kpi.label}</div>
					<div class="mt-2 flex items-end gap-3">
						<span class="font-mono text-4xl font-medium tracking-[-0.04em] text-ink tabular-nums lg:text-6xl">{kpi.value}</span>
						{#if kpi.unit}<span class="pb-1.5 text-sm text-ink-2 lg:text-lg">{kpi.unit}</span>{/if}
						{#if kpi.spark.length > 1}
							<Sparkline values={kpi.spark} width={96} height={32} stroke={kpi.stroke} draw={animate} class="ml-auto max-lg:hidden" />
						{/if}
					</div>
					<div class={`mt-1.5 truncate text-sm ${kpi.tone}`}>{kpi.note}</div>
				</div>
			{/each}
		</section>

		<HealthRiver rows={riverRows} limit={16} />

		<div class="grid min-h-0 flex-1 gap-5 lg:grid-cols-[minmax(0,1.7fr)_minmax(0,1fr)] lg:gap-6">
			<section class="min-w-0 rounded-xl border border-line bg-surface p-4 lg:p-6" aria-labelledby="ent-wall-alerts">
				<div class="flex items-center">
					<h2 id="ent-wall-alerts" class="m-0 text-base font-semibold text-ink lg:text-lg">{m.console_panel_attention()}</h2>
					<span class="ml-auto font-mono text-sm text-ink-2">{attention.length}</span>
				</div>
				{#if attention.length === 0}
					<p class="mt-3 text-sm text-ink-2 lg:text-base">{m.console_panel_attention_none()}</p>
				{:else}
					<ul class="mt-3 grid gap-2">
						{#each attention.slice(0, ALERT_ROWS) as row (row.key)}
							<li class="flex gap-3 rounded-[9px] border border-line bg-canvas p-3">
								<span aria-hidden="true" class={`w-[3px] shrink-0 rounded-sm ${TONE_BAR[row.tone]}`}></span>
								<span class="min-w-0 flex-1">
									<span class="flex items-center gap-2">
										<span class="shrink-0 font-mono text-xs font-semibold text-ink-2 uppercase">{row.plate}</span>
										<span class="truncate text-base font-medium text-ink lg:text-lg">{rowTitle(row)}</span>
									</span>
								</span>
								{#if row.since}<span class="shrink-0 font-mono text-sm text-ink-3">{formatRelative(row.since)}</span>{/if}
							</li>
						{/each}
					</ul>
					{#if attention.length > ALERT_ROWS}
						<p class="mt-3 mb-0 text-sm text-ink-3">{m.console_panel_attention_all({ count: attention.length })}</p>
					{/if}
				{/if}
			</section>

			<section class="min-w-0 rounded-xl border border-line bg-surface p-4 lg:p-6" aria-labelledby="ent-wall-oncall">
				<h2 id="ent-wall-oncall" class="m-0 text-base font-semibold text-ink lg:text-lg">{m.console_panel_oncall()}</h2>
				{#if onCallRows.length > 0}
					<ul class="mt-3 grid gap-3">
						{#each onCallRows as row (row.key)}
							<li class="flex items-center gap-3">
								<span aria-hidden="true" class="grid size-10 shrink-0 place-items-center rounded-full bg-info-soft text-base font-semibold text-info-ink">{row.initial}</span>
								<div class="min-w-0">
									<div class="truncate text-base font-medium text-ink lg:text-lg">{row.name}</div>
									<div class="truncate text-sm text-ink-2">
										{row.entry.until
											? m.console_panel_oncall_until({ schedule: row.entry.schedule_name, time: formatDateTime(row.entry.until) })
											: row.entry.schedule_name}
									</div>
								</div>
							</li>
						{/each}
					</ul>
				{:else}
					<p class="mt-3 text-sm text-ink-2 lg:text-base">{m.wall_ent_oncall_none()}</p>
				{/if}
			</section>
		</div>
	</div>

	<!-- Exit: out of sight on a display nobody touches, there on hover, focus or touch. -->
	<div class="ent-wall-exit fixed top-3 right-3 z-10">
		<Button variant="ghost" size="sm" onclick={exit} aria-label={m.wall_exit_aria()} class="min-h-10">
			<X class="size-4" aria-hidden="true" />
			{m.wall_exit()}
			<kbd class="ml-1 rounded-md border border-line-strong bg-surface-2 px-1.5 text-[0.6875rem] text-ink-3 max-sm:hidden">esc</kbd>
		</Button>
	</div>
</div>

<style>
	.ent-wall-exit {
		opacity: 0;
		transition: opacity 200ms ease;
	}
	.ent-wall:hover .ent-wall-exit,
	.ent-wall-exit:focus-within {
		opacity: 1;
	}
	@media (hover: none) {
		.ent-wall-exit {
			opacity: 1;
		}
	}
</style>
