<script lang="ts">
	/**
	 * Overview: the night-sky banner with its pigeon and one line of status,
	 * four figures, then what needs attention (only what is broken, with its
	 * quick action, and the week ahead) beside the health river, then two
	 * charts worth reading — the fleet's response time over the range, the
	 * alerts of the last seven days. The whole fleet lives on Monitors. A fresh
	 * instance gets the first-run guide (administrators) or a way in instead.
	 *
	 * Monitors, probes and alerts come from the app-wide store (polled every
	 * 30 s); the river, p95, availability and the alert history from the fleet
	 * store (one set of queries, refreshed every minute while LIVE is on).
	 */
	import { untrack } from 'svelte';
	import { getOnboarding, listCollectors, type OnboardingState } from '#lib/api/index.js';
	import { countAlertsByTarget, countsAsProblem } from '#lib/alerts-attention.js';
	import { m } from '#lib/paraglide/messages.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { consoleState } from '#lib/stores/console.svelte.js';
	import { Button, EmptyState, ErrorNotice, PHONE_QUERY, calmMotion } from '#lib/ui/index.js';
	import FirstRun from '#lib/components/overview/FirstRun.svelte';
	import NeedsAttention from '#lib/components/overview/NeedsAttention.svelte';
	import FleetLatency from '#lib/components/overview/FleetLatency.svelte';
	import AlertTrend from '#lib/components/overview/AlertTrend.svelte';
	import { bannerLine, type BannerLine } from './banner.js';
	import StatusBanner from './StatusBanner.svelte';
	import { fleetStore } from './fleet.svelte.js';
	import { mean } from './fleet.js';
	import HealthRiver from './HealthRiver.svelte';
	import { STATUS_RANK, consoleStatus, type ConsoleStatus } from './status.js';

	// One subscription for the life of the view; the range is followed separately.
	$effect(() => untrack(() => fleetStore.start()));
	// A new range reloads the figures at once.
	$effect(() => {
		consoleState.range;
		fleetStore.rangeChanged();
	});

	const alertCounts = $derived(countAlertsByTarget(alertsStore.alerts));

	const monitors = $derived<{ id: number; name: string; status: ConsoleStatus }[]>(
		alertsStore.targets.map((target) => ({
			id: target.id,
			name: target.name,
			status: consoleStatus(target, alertsStore.probes.get(target.id), alertCounts.get(target.id))
		}))
	);

	const riverRows = $derived(
		monitors
			.filter((row) => fleetStore.river.has(row.id))
			.sort((a, b) => STATUS_RANK[a.status] - STATUS_RANK[b.status] || a.name.localeCompare(b.name))
			.map((row) => ({ id: row.id, name: row.name, cells: fleetStore.river.get(row.id) ?? [] }))
	);

	// --- The four figures (MTTR lives in Status › Reports) -----------------------
	const total = $derived(monitors.filter((r) => r.status !== 'paused').length);
	const up = $derived(monitors.filter((r) => r.status === 'up' || r.status === 'degraded').length);
	const down = $derived(monitors.filter((r) => r.status === 'down').length);
	const paused = $derived(monitors.filter((r) => r.status === 'paused').length);
	const availability = $derived(mean(fleetStore.figures.uptime30.values()));
	const firing = $derived(alertsStore.alerts.filter(countsAsProblem));
	const critical = $derived(firing.filter((a) => a.severity === 'critical').length);

	type Kpi = { id: string; label: string; value: string; unit: string; note: string; noteTone: string };
	const kpis = $derived<Kpi[]>([
		{
			id: 'up',
			label: m.console_kpi_up(),
			value: String(up),
			unit: m.console_kpi_up_unit({ total }),
			note: m.console_kpi_up_note({ down, paused }),
			noteTone: down > 0 ? 'text-warning-ink' : 'text-ink-2'
		},
		{
			id: 'availability',
			label: m.console_kpi_availability(),
			value: availability === null ? '—' : availability >= 99.9995 ? '100' : availability.toFixed(2),
			unit: availability === null ? '' : '%',
			note: availability === null ? m.console_kpi_none() : m.console_kpi_availability_note({ count: fleetStore.figures.uptime30.size }),
			noteTone: 'text-ink-2'
		},
		{
			id: 'p95',
			label: m.console_kpi_p95(),
			value: fleetStore.figures.fleetP95 === null ? '—' : String(Math.round(fleetStore.figures.fleetP95)),
			unit: fleetStore.figures.fleetP95 === null ? '' : 'ms',
			note: m.console_kpi_p95_note({ range: consoleState.range }),
			noteTone: 'text-ink-2'
		},
		{
			id: 'alerts',
			label: m.console_kpi_alerts(),
			value: String(firing.length),
			unit: '',
			note: m.console_kpi_alerts_note({ critical, warning: firing.length - critical }),
			noteTone: critical > 0 ? 'text-warning-ink' : firing.length > 0 ? 'text-advisory-ink' : 'text-ink-2'
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
	const line = $derived(bannerLine(monitors, alertsStore.alerts));
	const banner = $derived(reading ? { headline: '', sub: '' } : words(line));
	const mood = $derived<'sleep' | 'watch' | 'alert'>(
		line.kind === 'down' ? 'alert' : line.kind === 'attention' || line.kind === 'waiting' ? 'watch' : 'sleep'
	);
	const noMonitors = $derived(!reading && alertsStore.available && alertsStore.targets.length === 0);

	// --- Phones: the river behind a tap ------------------------------------------
	let phone = $state(false);
	let riverOpen = $state(false);
	$effect(() => {
		const query = window.matchMedia(PHONE_QUERY);
		phone = query.matches;
		const onChange = (event: MediaQueryListEvent) => (phone = event.matches);
		query.addEventListener('change', onChange);
		return () => query.removeEventListener('change', onChange);
	});

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

<div class="flex min-h-0 flex-1 flex-col gap-4">
	<h1 class="sr-only">{m.misc_nav_overview()}</h1>

	<StatusBanner headline={banner.headline} sub={banner.sub} {mood} />

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
		<div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
			{#each kpis as kpi, i (kpi.id)}
				<div class={`min-w-0 rounded-xl border border-line bg-surface px-4 py-3.5 sm:px-[18px] sm:py-4 ${animate ? 'rise-in' : ''}`} style={`--rise-delay: ${i * 40}ms`}>
					<p class="m-0 truncate text-sm font-medium text-ink-2">{kpi.label}</p>
					<p class="m-0 mt-2 flex items-baseline gap-1.5">
						{#key kpi.value}
							<span class="state-in font-mono text-2xl leading-none font-medium tracking-[-0.03em] text-ink tabular-nums sm:text-[2rem]">{kpi.value}</span>
						{/key}
						{#if kpi.unit}<span class="truncate text-sm text-ink-3 sm:text-base">{kpi.unit}</span>{/if}
					</p>
					<p class={`m-0 mt-2 truncate text-xs font-medium ${kpi.noteTone}`}>{kpi.note}</p>
				</div>
			{/each}
		</div>

		{#if fleetStore.error}
			<ErrorNotice error={fleetStore.error} />
		{/if}

		<div class="grid gap-4 xl:grid-cols-[minmax(0,1.5fr)_minmax(0,1fr)]">
			<NeedsAttention />
			<div class="flex min-w-0 flex-col gap-2">
				{#if phone}
					<button
						type="button"
						class="flex min-h-11 items-center justify-between rounded-xl border border-line bg-surface px-4 text-sm font-semibold text-ink"
						aria-expanded={riverOpen}
						aria-controls="overview-river"
						onclick={() => (riverOpen = !riverOpen)}
					>
						{m.console_river_title()}
						<span class="text-xs font-medium text-ink-2">{riverOpen ? m.overview_river_hide() : m.overview_river_show()}</span>
					</button>
				{/if}
				{#if !phone || riverOpen}
					<div id="overview-river" class="min-w-0 flex-1 [&>section]:h-full">
						<HealthRiver rows={riverRows} limit={8} moreHref="/targets" scan={false} />
					</div>
				{/if}
			</div>
		</div>

		<div class="grid gap-4 xl:grid-cols-[minmax(0,1.5fr)_minmax(0,1fr)]">
			<FleetLatency />
			<AlertTrend />
		</div>
	{/if}
</div>
