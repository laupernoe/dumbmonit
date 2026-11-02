<script lang="ts">
	/**
	 * Wall mode: a room monitor that is also a picture. The whole screen is a
	 * world (Paris by default: the Eiffel Tower, the rooftops, the pigeons)
	 * under a sky that follows the real hour, with the weather of the network:
	 * clear when all is well, clouds for advisories, rain when something is
	 * down, a thunderstorm in a real outage. On top of it, readable across the
	 * room: the verdict in a few huge words ("All good" / "N problems"), the
	 * clock and the date, the problems as tiles, then every monitor as a small
	 * tile. The five fleet figures and the health river lie in a quiet strip
	 * along the foot, on the stone of the rooftop.
	 *
	 * Reads the same stores as the console (monitors, probes and alerts every
	 * 30 s; the river and the figures from the fleet store). Escape or Exit
	 * goes back to the console. The screen stays awake while the wall is open.
	 *
	 * The world (Settings sets the default, `?city=` / `?theme=` override it,
	 * the Backdrop and Theme buttons change it for this browser) is loaded on
	 * demand: a plain board (`none`) loads nothing. Night is the default
	 * theme, Day lights Paris; on a phone or a hidden tab the world holds
	 * still, and under `prefers-reduced-motion` it is a still picture.
	 */
	import { untrack, type Component } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { AlertTriangle, CircleAlert, CircleCheck, CircleDashed, X } from 'lucide-svelte';
	import { getOnCallNow, listAlertRules, listOnCallPeople, type AlertRule, type OnCallEntry, type Person } from '#lib/api/index.js';
	import { countAlertsByTarget, countsAsProblem } from '#lib/alerts-attention.js';
	import { readSky } from '#lib/components/overview/sky.js';
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
	import { viewport } from '#lib/stores/viewport.svelte.js';
	import { useSceneLive } from '#lib/scene/live.svelte.js';
	import { Button, Plate, Skeleton, calmMotion } from '#lib/ui/index.js';
	import WallBand, { type BandFigure } from '#lib/components/wall/WallBand.svelte';
	import WallDevices from '#lib/components/wall/WallDevices.svelte';
	import WallProblems from '#lib/components/wall/WallProblems.svelte';
	import WallSceneControl from '#lib/components/wall/WallSceneControl.svelte';
	import {
		DEFAULT_WALL_SCENE,
		cityHasDay,
		parseForcedWallScene,
		readWallScene,
		resolveWallScene,
		writeWallScene,
		type WallSceneChoice
	} from '#lib/components/wall/wallScene.js';
	import { parseForcedWallTheme, readWallTheme, writeWallTheme, type WallTheme } from '#lib/components/wall/wallTheme.js';
	import { parseForcedWeather, weatherFor, weatherForTone, type WallWeather } from '#lib/components/wall/wallWeather.js';

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
	const targets = $derived(alertsStore.targets);
	const probes = $derived(alertsStore.probes);
	const firstLoad = $derived(alertsStore.loading && targets.length === 0);
	const sky = $derived(readSky({ targets, probes, alerts: alertsStore.alerts, rules }));
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
		targets.map((target) => ({
			target,
			status: consoleStatus(target, probes.get(target.id), alertCounts.get(target.id))
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

	const figures = $derived<BandFigure[]>([
		{
			id: 'up',
			label: m.console_kpi_up(),
			value: String(up),
			unit: m.console_kpi_up_unit({ total }),
			note: m.console_kpi_up_note({ down, paused }),
			tone: down > 0 ? 'text-warning-ink' : 'text-ink-2'
		},
		{
			id: 'availability',
			label: m.console_kpi_availability(),
			value: availability === null ? '—' : availability >= 99.9995 ? '100' : availability.toFixed(3),
			unit: availability === null ? '' : '%',
			note: availability === null ? m.console_kpi_none() : m.console_kpi_availability_note({ count: fleetStore.figures.uptime30.size }),
			tone: 'text-ink-2'
		},
		{
			id: 'p95',
			label: m.console_kpi_p95(),
			value: fleetStore.figures.fleetP95 === null ? '—' : String(Math.round(fleetStore.figures.fleetP95)),
			unit: fleetStore.figures.fleetP95 === null ? '' : 'ms',
			note: m.console_kpi_p95_note({ range: consoleState.range }),
			tone: 'text-ink-2'
		},
		{
			id: 'alerts',
			label: m.console_kpi_alerts(),
			value: String(firing.length),
			unit: '',
			note: m.console_kpi_alerts_note({ critical, warning: firing.length - critical }),
			tone: critical > 0 ? 'text-warning-ink' : firing.length > 0 ? 'text-advisory-ink' : 'text-ink-2'
		},
		{
			id: 'mttr',
			label: m.console_kpi_mttr(),
			value: mttr === null ? '—' : formatDuration(mttr),
			unit: '',
			note: mttr === null ? m.console_kpi_none() : m.console_kpi_mttr_note({ count: resolved }),
			tone: 'text-ink-2'
		}
	]);

	// The monitors that need a look first lead the river.
	const riverRows = $derived(
		[...statuses]
			.filter((row) => fleetStore.river.has(row.target.id))
			.sort((a, b) => STATUS_RANK[a.status] - STATUS_RANK[b.status] || a.target.name.localeCompare(b.target.name))
			.map((row) => ({ id: row.target.id, name: row.target.name, cells: fleetStore.river.get(row.target.id) ?? [] }))
	);
	const riverHint = $derived(
		m.console_river_hint({ slot: formatDuration(consoleState.rangeSeconds / RIVER_SLOTS), range: formatDuration(consoleState.rangeSeconds) })
	);

	// --- Who is on call --------------------------------------------------------
	const onCallRows = $derived(
		onCall
			.filter((entry) => entry.user_id !== null)
			.slice(0, 3)
			.map((entry) => {
				const person = people.get(entry.user_id as number);
				const name = person?.display_name || person?.username || m.console_panel_oncall_unknown();
				return { key: `${entry.schedule_id}:${entry.user_id}`, name, initial: name.charAt(0).toUpperCase(), entry };
			})
	);

	const animate = !calmMotion();

	// --- The world -------------------------------------------------------------
	// Settings sets the default, the Backdrop and Theme buttons change it for this
	// browser, and a link (`?city=`, `?theme=`) decides for one display only.
	// The public demo shows the city closest to the viewer; everywhere else it is Paris.
	let sceneChoice = $state<WallSceneChoice>(readWallScene(auth.demo ? 'auto' : DEFAULT_WALL_SCENE));
	const forcedScene = $derived(parseForcedWallScene(page.url.searchParams.get('city')));
	const sceneCity = $derived(resolveWallScene(forcedScene ?? sceneChoice));
	function setScene(choice: WallSceneChoice) {
		sceneChoice = choice;
		writeWallScene(choice);
	}
	let themeChoice = $state<WallTheme>(readWallTheme());
	const forcedTheme = $derived(parseForcedWallTheme(page.url.searchParams.get('theme')));
	const dayAvailable = $derived(cityHasDay(sceneCity));
	// Day lights Paris; the other cities are night scenes.
	const worldTheme = $derived<WallTheme>(dayAvailable ? (forcedTheme ?? themeChoice) : 'dark');
	function setTheme(choice: WallTheme) {
		themeChoice = choice;
		writeWallTheme(choice);
	}
	// Night uses the status pages' dark tokens, Day the light ones; a plain board keeps the console's black.
	const daySkin = $derived(sceneCity !== null && worldTheme === 'light');
	const nightSkin = $derived(sceneCity !== null && worldTheme === 'dark');

	// The scene's code comes only when a city is drawn.
	let World = $state.raw<Component<any> | null>(null);
	$effect(() => {
		if (!sceneCity || World) return;
		let alive = true;
		void import('#lib/components/wall/WallWorld.svelte')
			.then((mod) => {
				if (alive) World = mod.default;
			})
			.catch(() => {
				// The chunk could not be fetched: the board stands on its own.
			});
		return () => {
			alive = false;
		};
	});

	// The weather follows the network (`?weather=` or `?mood=` show a state without one).
	const live = useSceneLive();
	const forcedWeather = $derived(parseForcedWeather(page.url.searchParams.get('weather')));
	const weather = $derived<WallWeather>(
		forcedWeather ??
			(live.tone
				? weatherForTone(live.tone)
				: firstLoad
					? 'clear'
					: weatherFor({ attention: sky.attention, warnings: sky.counts.warnings, unreachable: sky.counts.unreachable }))
	);

	// The landmark's gestures (the tower goes dark, comes back, sparkles) follow changes.
	$effect(() => {
		if (alertsStore.loading) {
			live.reset();
			return;
		}
		const downKeys = new Set<string>();
		const issues = new Set<string>();
		for (const row of sky.needsYou) {
			if (row.kind === 'device' && row.tone === 'warning') downKeys.add(row.key);
			else if (row.kind === 'alert' && row.alert.effective_phase === 'firing' && row.alert.rule_uid.startsWith('host_down')) downKeys.add(row.key);
			if (row.tone === 'warning' || row.tone === 'advisory') issues.add(row.key);
		}
		live.feed({ down: downKeys, issues });
	});

	// A new problem startles the pigeons once; nothing startles on the first load.
	let startle = $state(0);
	let attentionBaseline: number | null = null;
	$effect(() => {
		const attention = sky.attention;
		if (alertsStore.loading) return;
		if (attentionBaseline !== null && attention > attentionBaseline) untrack(() => (startle += 1));
		attentionBaseline = attention;
	});

	// The Eiffel Tower sparkles for the first five minutes of every hour.
	const onTheHour = $derived(now.getMinutes() < 5);
	let tabHidden = $state(false);
	$effect(() => {
		const sync = () => (tabHidden = document.visibilityState !== 'visible');
		sync();
		document.addEventListener('visibilitychange', sync);
		return () => document.removeEventListener('visibilitychange', sync);
	});
	const sceneStill = $derived(tabHidden || viewport.phone);

	// --- Fitting the problems --------------------------------------------------
	// From 1024 px the board is a fixed-height column: the problems get what the
	// tiles of the monitors below do not need. Below, the page scrolls and
	// nothing is capped.
	const DEVICES_MIN_PX = 56;
	let leftRoom = $state(0);
	let problemListed = $state(new Set<number>());
	let wide = $state(false);
	$effect(() => {
		const query = window.matchMedia('(min-width: 1024px)');
		const sync = () => (wide = query.matches);
		sync();
		query.addEventListener('change', sync);
		return () => query.removeEventListener('change', sync);
	});
	const problemsRoom = $derived(wide && leftRoom > 0 ? Math.max(120, leftRoom - DEVICES_MIN_PX - 16) : Infinity);

	// The tools show for a few seconds when the wall opens, then on hover, focus or touch.
	let toolsHint = $state(true);
	$effect(() => {
		const timer = setTimeout(() => (toolsHint = false), 7000);
		return () => clearTimeout(timer);
	});
</script>

<svelte:head><title>{m.wall_page_title()}</title></svelte:head>
<svelte:window onkeydown={onKeydown} />

<div
	class="wall fixed inset-0 z-[60] box-border bg-canvas pt-[var(--safe-top)] pr-[var(--safe-right)] pb-[var(--safe-bottom)] pl-[var(--safe-left)] text-ink"
	class:has-world={sceneCity !== null}
	class:wall-day={daySkin}
	class:tokens-night={nightSkin}
>
	<div class="wall-stage relative h-full">
		{#if sceneCity}
			<div class="scene-box" aria-hidden="true">
				{#if World}
					<World
						city={sceneCity}
						theme={worldTheme}
						{now}
						{weather}
						problems={firstLoad ? 0 : sky.attention}
						{startle}
						paused={sceneStill}
						pulse={live.pulse}
						sparkle={onTheHour}
						focus={1360}
					/>
				{/if}
				<div class="scene-fade"></div>
			</div>
			<div class="veil veil-left" aria-hidden="true"></div>
			<div class="veil veil-right" aria-hidden="true"></div>
		{/if}

		<div class="board">
			<div class="board-left" class:rise-in={animate} style="--rise-delay: 60ms">
				{#if firstLoad}
					<div class="grid gap-6">
						<Skeleton class="h-20 w-3/4 lg:h-28" />
						<Skeleton class="h-8 w-1/2" />
						<Skeleton class="h-40 w-full" />
					</div>
				{:else}
					<p class="instance" data-wall-info>{instanceStore.displayName}</p>
					{#key verdict.text}
						<header class="verdict flex items-center gap-4 lg:gap-6" class:verdict-in={animate} data-wall-info>
							<span class="status-mark status-mark--{verdict.tone}" aria-hidden="true">
								<Icon class="size-1/2" strokeWidth={2.4} />
							</span>
							<h1 class="headline min-w-0" aria-live="polite">{verdict.text}</h1>
						</header>
					{/key}
					<p class="sentence mt-3 lg:mt-4" data-wall-info>{sky.sentence}</p>
					<div class="mt-4 flex flex-wrap items-center gap-2.5" data-wall-info>
						{#each sky.plates as plate (plate.label)}
							<Plate tone={plate.tone} label={plate.label} bare={plate.bare} size="md" />
						{/each}
					</div>

					<div class="board-list mt-7 flex min-h-0 flex-1 flex-col gap-4 lg:mt-9" bind:clientHeight={leftRoom}>
						<WallProblems {sky} {now} room={problemsRoom} bind:listed={problemListed} />
						<WallDevices {targets} {probes} {sky} skip={problemListed} />
					</div>
				{/if}
			</div>

			<div class="board-right" class:rise-in={animate} style="--rise-delay: 140ms">
				<time class="clock tnum" datetime={now.toISOString()} data-wall-info>{clock}</time>
				<p class="date" data-wall-info>{dateLabel}</p>
				<p class="updated tnum" aria-live="off" data-wall-info>
					<span aria-hidden="true" class="size-1.5 rounded-full bg-signal {animate ? 'ob-live' : ''}"></span>
					{updated}
				</p>
				{#if onCallRows.length > 0 || !auth.scoped}
					<section class="oncall" aria-labelledby="wall-oncall" data-wall-info>
						<h2 id="wall-oncall" class="m-0 text-sm font-semibold text-ink-2">{m.console_panel_oncall()}</h2>
						{#if onCallRows.length === 0}
							<p class="m-0 mt-1.5 text-sm text-ink-2">{m.wall_ent_oncall_none()}</p>
						{/if}
						<ul class="m-0 mt-2 grid list-none gap-2 p-0">
							{#each onCallRows as row (row.key)}
								<li class="flex items-center gap-3">
									<span aria-hidden="true" class="grid size-9 shrink-0 place-items-center rounded-full bg-info-soft text-sm font-semibold text-info-ink">{row.initial}</span>
									<span class="min-w-0 text-left">
										<span class="block truncate text-base font-medium text-ink">{row.name}</span>
										<span class="block truncate text-xs text-ink-2">
											{row.entry.until
												? m.console_panel_oncall_until({ schedule: row.entry.schedule_name, time: formatDateTime(row.entry.until) })
												: row.entry.schedule_name}
										</span>
									</span>
								</li>
							{/each}
						</ul>
					</section>
				{/if}
			</div>

			<div class="board-band" class:rise-in={animate} style="--rise-delay: 220ms">
				<WallBand {figures} river={riverRows} hint={riverHint} limit={sceneCity ? 3 : 8} />
			</div>
		</div>

		<!-- Backdrop, Theme and Exit: out of sight on a display nobody touches, there on hover, focus or touch. -->
		<div
			class="wall-tools fixed top-3 right-3 z-10 flex items-start gap-1 rounded-xl bg-surface/85 p-1 shadow-lift lg:top-4 lg:right-4"
			class:tools-hint={toolsHint}
			data-wall-info
		>
			<WallSceneControl
				scene={forcedScene ?? sceneChoice}
				sceneForced={forcedScene !== null}
				onscene={setScene}
				theme={forcedTheme ?? themeChoice}
				themeForced={forcedTheme !== null}
				{dayAvailable}
				showTheme={sceneCity !== null}
				ontheme={setTheme}
			/>
			<Button variant="ghost" size="sm" onclick={exit} aria-label={m.wall_exit_aria()} class="min-h-10">
				<X class="size-4" aria-hidden="true" />
				{m.wall_exit()}
				<kbd class="ml-1 rounded-md border border-line-strong bg-surface-2 px-1.5 text-[0.6875rem] text-ink-3 max-sm:hidden">esc</kbd>
			</Button>
		</div>
	</div>
</div>

<style>
	/*
	 * Day: chart paper, navy ink (the status pages' light, as tokens). Night is
	 * `tokens-night` (app.css); a plain board keeps the console's own black.
	 */
	.wall-day {
		color-scheme: light;
		--c-canvas: #f3efe6;
		--c-canvas-deep: #ebe6d9;
		--c-surface: #fbf9f4;
		--c-surface-2: #f0ebe0;
		--c-line: rgb(22 33 58 / 0.14);
		--c-line-strong: rgb(22 33 58 / 0.28);
		--c-ink: #16213a;
		--c-ink-2: #4a5a78;
		--c-ink-3: #586783;
		--c-ghost: rgb(22 33 58 / 0.1);
		--c-signal: #0f8f86;
		--c-signal-ink: #0b6d66;
		--c-signal-soft: rgb(15 143 134 / 0.12);
		--c-advisory: #b7791f;
		--c-advisory-ink: #8f5d12;
		--c-advisory-soft: rgb(183 121 31 / 0.14);
		--c-warning: #c8402f;
		--c-warning-ink: #a32f21;
		--c-warning-soft: rgb(200 64 47 / 0.12);
		--c-info: #2f6db5;
		--c-info-ink: #24568f;
		--c-info-soft: rgb(47 109 181 / 0.12);
		--c-on-signal: #f3efe6;
		--c-primary: var(--c-signal);
		--c-on-primary: var(--c-on-signal);
		--c-focus: var(--c-signal);
		--c-river-up: rgb(15 143 134 / 0.55);
		--c-river-warn: #b7791f;
		--c-river-down: #c8402f;
		--c-river-none: rgb(22 33 58 / 0.1);
		--shadow-lift: 0 1px 2px rgb(22 33 58 / 0.06), 0 6px 18px -8px rgb(22 33 58 / 0.22);
		--shadow-float: 0 2px 6px rgb(22 33 58 / 0.08), 0 18px 40px -16px rgb(22 33 58 / 0.32);
	}

	/*
	 * Layout. Phone and tablet: the world is a band at the top and the board
	 * scrolls under it. From 1024 px the world fills the screen and the board
	 * sits in it: verdict and tiles on the left, clock and on-call on the right,
	 * the middle left to the monument, and the foot of the screen to the
	 * parapet, its pigeons and the quiet strip of figures.
	 */
	.wall-stage {
		overflow-y: auto;
	}
	.scene-box {
		position: relative;
		height: max(240px, 40vh);
		overflow: hidden;
		isolation: isolate;
	}
	/* On a small screen the band sinks into the page under the headline. */
	.scene-fade {
		position: absolute;
		inset-inline: 0;
		bottom: 0;
		height: 45%;
		background: linear-gradient(180deg, transparent, var(--c-canvas));
	}
	.veil {
		display: none;
	}
	.board {
		display: flex;
		flex-direction: column;
		gap: 2rem;
		padding: 1.5rem 1rem 0;
	}
	/* A world behind: the board lifts over the band's lower edge on a small screen. */
	@media (max-width: 1023px) {
		.has-world .board {
			margin-top: -4rem;
			position: relative;
		}
	}
	.board-left,
	.board-right {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.board-right {
		order: -1;
	}
	.board-band {
		margin-inline: -1rem;
	}
	.instance {
		margin: 0 0 0.5rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 0.9375rem;
		color: var(--c-ink-2);
	}
	.headline {
		margin: 0;
		white-space: nowrap;
		font-size: clamp(2.5rem, 11vw, 3.5rem);
		font-weight: 600;
		line-height: 1;
		letter-spacing: -0.04em;
		color: var(--c-ink);
	}
	.status-mark {
		display: grid;
		place-items: center;
		flex-shrink: 0;
		width: clamp(2.75rem, 12vw, 3.5rem);
		aspect-ratio: 1;
		border-radius: 50%;
		border: 2px solid currentColor;
		background: var(--c-surface);
		box-shadow: var(--shadow-lift);
	}
	.status-mark--signal {
		color: var(--c-signal-ink);
	}
	.status-mark--warning {
		color: var(--c-warning-ink);
	}
	.status-mark--advisory {
		color: var(--c-advisory-ink);
	}
	.status-mark--ghost {
		color: var(--c-ink-2);
	}
	.sentence {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 500;
		color: var(--c-ink-2);
	}
	.clock {
		font-size: 3.25rem;
		font-weight: 600;
		letter-spacing: -0.04em;
		line-height: 1;
		color: var(--c-ink);
	}
	.date {
		margin: 0.375rem 0 0;
		font-size: 1.25rem;
		font-weight: 600;
		color: var(--c-ink-2);
		text-transform: capitalize;
	}
	.updated {
		margin: 0.5rem 0 0;
		display: flex;
		align-items: center;
		gap: 0.625rem;
		font-family: var(--font-code, ui-monospace, monospace);
		font-size: 0.8125rem;
		color: var(--c-ink-2);
	}
	.oncall {
		width: 100%;
		margin-top: 1.25rem;
		padding: 0.875rem 1rem;
		border: 1px solid var(--c-line);
		border-radius: var(--radius-card, 14px);
		background: color-mix(in srgb, var(--c-surface) 88%, transparent);
		box-shadow: var(--shadow-lift);
		text-align: left;
	}
	/* The verdict changes in a short, soft arrival, never a flash. */
	.verdict-in {
		animation: verdict-in 240ms cubic-bezier(0.2, 0, 0, 1) both;
	}
	@keyframes verdict-in {
		from {
			opacity: 0;
			transform: translateY(6px);
		}
	}

	@media (min-width: 1024px) {
		.wall-stage {
			overflow: hidden;
		}
		.scene-box {
			position: absolute;
			inset: 0;
			height: auto;
		}
		.scene-fade {
			display: none;
		}
		/* A veil behind each column's words: legible over any hour, any cloud. */
		.veil {
			display: block;
			position: absolute;
			inset-block: 0;
			pointer-events: none;
		}
		.veil-left {
			left: 0;
			width: min(54rem, 52vw);
			background: linear-gradient(
				90deg,
				color-mix(in srgb, var(--c-canvas) 64%, transparent),
				color-mix(in srgb, var(--c-canvas) 44%, transparent) 62%,
				transparent
			);
		}
		.veil-right {
			right: 0;
			width: min(34rem, 32vw);
			background: linear-gradient(
				270deg,
				color-mix(in srgb, var(--c-canvas) 60%, transparent),
				color-mix(in srgb, var(--c-canvas) 38%, transparent) 62%,
				transparent
			);
		}
		.board {
			position: absolute;
			inset: 0;
			margin-top: 0;
			display: grid;
			grid-template-columns: minmax(0, 38rem) minmax(0, 1fr) minmax(0, 24rem);
			grid-template-rows: minmax(0, 1fr) auto;
			gap: 0 2rem;
			/* The words come first: the world's props give way to them, never the reverse. */
			padding: 3rem 3.5rem 0;
		}
		.board-left {
			grid-column: 1;
			grid-row: 1;
			min-height: 0;
		}
		.board-right {
			order: 0;
			grid-column: 3;
			grid-row: 1;
			align-items: flex-end;
			text-align: right;
			padding-top: 1.75rem;
			min-height: 0;
		}
		.board-band {
			grid-column: 1 / -1;
			grid-row: 2;
			margin-inline: -3.5rem;
		}
		.headline {
			font-size: clamp(4.25rem, 5vw, 6rem);
		}
		.status-mark {
			width: clamp(4rem, 4.8vw, 5.75rem);
			border-width: 3px;
		}
		.sentence {
			font-size: 1.75rem;
		}
		.clock {
			font-size: 6.5rem;
		}
		.date {
			font-size: 1.75rem;
		}
		.updated {
			font-size: 0.9375rem;
		}
		.oncall {
			margin-top: 1.5rem;
		}
	}
	/* The words sit on a scene: a soft halo keeps them readable over its details. */
	.has-world .headline,
	.has-world .clock,
	.has-world .date,
	.has-world .sentence,
	.has-world .updated,
	.has-world .instance {
		text-shadow: 0 0 14px color-mix(in srgb, var(--c-canvas) 85%, transparent);
	}
	/* 720p to 900p: a narrower board leaves the middle of the scene to its monument. */
	@media (min-width: 1024px) and (max-width: 1599px) {
		.board {
			grid-template-columns: minmax(0, 27rem) minmax(0, 1fr) minmax(0, 20rem);
			gap: 0 1.5rem;
			padding-inline: 2.5rem;
		}
		.board-band {
			margin-inline: -2.5rem;
		}
		.headline {
			font-size: clamp(3.5rem, 5.4vw, 4.5rem);
		}
		.clock {
			font-size: 4.75rem;
		}
	}
	/* A short screen: the right column trims its spacing so the tiles keep their room. */
	@media (min-width: 1024px) and (max-height: 940px) {
		.board {
			padding-top: 2.25rem;
		}
		.instance,
		.updated {
			display: none;
		}
		.date {
			font-size: 1.25rem;
			margin-top: 0.125rem;
		}
		.sentence {
			font-size: 1.25rem;
			margin-top: 0.5rem;
		}
		.board-list {
			margin-top: 1.25rem;
		}
	}
	/* A 4K panel read from the same sofa: the board grows with the screen. */
	@media (min-width: 2400px) {
		.board,
		.wall-tools {
			zoom: 1.33;
		}
	}
	@media (min-width: 3200px) {
		.board,
		.wall-tools {
			zoom: 2;
		}
	}

	/* Backdrop, Theme and Exit: out of sight on a display nobody touches. */
	.wall-tools {
		opacity: 0;
		transition: opacity 200ms ease;
	}
	.wall:hover .wall-tools,
	.wall-tools:focus-within,
	.wall-tools:has([aria-expanded='true']),
	.wall-tools.tools-hint {
		opacity: 1;
	}
	@media (hover: none) {
		.wall-tools {
			opacity: 1;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.wall-tools {
			transition: none;
		}
		.verdict-in {
			animation: none;
		}
	}
</style>
