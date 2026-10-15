<script lang="ts">
	/**
	 * Overview — a briefing from the pigeon, not a dashboard.
	 *
	 * The sky says how things are right now; "Since you last looked" tells
	 * what happened while you were away, in sentences; "Needs you" lists what
	 * to act on, one line each, as many as fit; "Last 7 days" reads three
	 * figures off the week's alert history (how long
	 * everything has been reporting, the quietest device, the one with the most
	 * alert events). The last visit lives in the browser, so the story starts
	 * where the reader left it.
	 *
	 * Alerts, devices and probe states come from the shared store (polled
	 * app-wide, see `alertsStore`); everything else is refreshed here every
	 * 30 s. No device list: the rack lives on /targets.
	 *
	 * The pigeon in the sky lives the weather with the reader: it sleeps on a
	 * wire on a quiet night, startles when something new goes wrong, and loops
	 * under confetti when the sky clears again.
	 */
	import { onMount, untrack } from 'svelte';
	import { page } from '$app/state';
	import {
		listAlertRules,
		listAlertHistory,
		listChannels,
		listCollectors,
		getOnboarding,
		type AlertRule,
		type AlertHistoryEntry,
		type OnboardingState
	} from '#lib/api/index.js';
	import type { Alert } from '#lib/api/index.js';
	import { displayState, formatRelative } from '#lib/format.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, EmptyState, Plate, Skeleton, ErrorNotice, DecryptText, ClickSpark, reducedMotion } from '#lib/ui/index.js';
	import SkyScene, { type PigeonMood } from '#lib/components/overview/SkyScene.svelte';
	import Briefing from '#lib/components/overview/Briefing.svelte';
	import Streaks from '#lib/components/overview/Streaks.svelte';
	import FirstRun from '#lib/components/overview/FirstRun.svelte';
	import { isAckedRow, isSnoozedRow, readSky, skyCondition } from '#lib/components/overview/sky.js';
	import { buildBriefing } from '#lib/components/overview/briefing.js';
	import { computeStreaks } from '#lib/components/overview/streaks.js';
	import { readLastVisit, writeLastVisit } from '#lib/components/overview/lastVisit.js';
	import NeedsYouCompact from '#lib/components/overview/NeedsYouCompact.svelte';
	import { m } from '#lib/paraglide/messages.js';

	const DAY_MS = 24 * 3600 * 1000;
	const HISTORY_DAYS = 7;
	const POLL_MS = 30_000;
	const VISIT_MS = 60_000;
	/** Entrance stagger, one step per section top to bottom. */
	const STAGGER_MS = 60;

	let rules = $state<AlertRule[]>([]);
	let history = $state<AlertHistoryEntry[]>([]);
	/** Enabled channel names; `undefined` until read (or when the read failed). */
	let channels = $state<string[] | undefined>(undefined);
	let hasDemoKind = $state(false);
	/** The first-run guide, as the server keeps it; `null` while unread or unavailable. */
	let onboarding = $state<OnboardingState | null>(null);
	/**
	 * Whether the guide is on screen. Decided once per page load and then held:
	 * the last step turns green at the same moment the server latches the guide
	 * as finished, and a panel that vanished on its own payoff would be unkind.
	 */
	let showGuide = $state(false);
	let guideDecided = false;
	let loading = $state(true);
	let error = $state<unknown>(null);
	let lastChecked = $state<Date | null>(null);
	let now = $state(new Date());
	/** Start of the briefing window; `null` until read, and on a first visit. */
	let lastVisit = $state<Date | null>(null);

	// `?demo=empty` shows the first-run screen on a populated server. Harmless, kept for review.
	const demoEmpty = $derived(page.url.searchParams.get('demo') === 'empty');
	// Targets and probe states are read from the shared store (polled app-wide
	// by the root layout) instead of fetched again here: see `alertsStore`.
	const targets = $derived(alertsStore.targets);
	const probes = $derived(alertsStore.probes);
	const shownTargets = $derived(demoEmpty ? [] : targets);
	const alerts = $derived<Alert[]>(demoEmpty ? [] : alertsStore.alerts);
	const noDevices = $derived(shownTargets.length === 0);

	// The one truth model, shared with /alerts and /wall.
	const sky = $derived(readSky({ targets: shownTargets, probes, alerts, rules }));
	const condition = $derived(skyCondition(sky));
	const checkedLabel = $derived(lastChecked ? formatRelative(lastChecked) : '');

	/** Devices that cannot be reached right now, whoever voices it. */
	const unreachable = $derived(
		shownTargets.filter((target) => {
			const state = displayState(target, probes.get(target.id));
			return state === 'offline' || state === 'down';
		})
	);

	// --- The pigeon's mood -----------------------------------------------------
	// A quiet night (22:00–06:00 here, nothing worse than a cloud) sends it to
	// sleep on its wire. A new problem startles it; the sky clearing after
	// trouble makes it loop under confetti. Reactions only follow changes seen
	// while the page is open — never the first reading — and stay off under
	// reduced motion. `?pigeon=sleep|startle|celebrate` shows a mood for review.
	const night = $derived(now.getHours() >= 22 || now.getHours() < 6);
	let reaction = $state<'startle' | 'celebrate' | null>(null);
	let reactionTimer: ReturnType<typeof setTimeout> | undefined;
	const reviewMood = $derived.by(() => {
		const asked = page.url.searchParams.get('pigeon');
		return asked === 'sleep' || asked === 'startle' || asked === 'celebrate' ? (asked as PigeonMood) : null;
	});
	const pigeon = $derived<PigeonMood>(
		reviewMood ?? reaction ?? (night && (condition === 'clear' || condition === 'cloudy') ? 'sleep' : 'fly')
	);

	function react(kind: 'startle' | 'celebrate') {
		if (reducedMotion()) return;
		clearTimeout(reactionTimer);
		reaction = kind;
		reactionTimer = setTimeout(() => (reaction = null), kind === 'celebrate' ? 3400 : 1500);
	}

	/** Armed a few seconds after the first full reading: what loads in is not news. */
	let watching = $state(false);
	let seenAttention = 0;
	let seenCondition: string | null = null;
	$effect(() => {
		const attention = sky.attention;
		const current = condition;
		const armed = watching;
		untrack(() => {
			if (armed && seenCondition !== null && !demoEmpty) {
				if (attention > seenAttention || (current === 'storm' && seenCondition !== 'storm')) {
					react('startle');
				} else if (current === 'clear' && ['storm', 'overcast', 'cloudy'].includes(seenCondition)) {
					react('celebrate');
				}
			}
			seenAttention = attention;
			seenCondition = current;
		});
	});

	const briefing = $derived(
		buildBriefing({ history, targets: shownTargets, rules, alerts, unreachable, channels, lastVisit, now })
	);
	const streaks = $derived(
		computeStreaks({ history, targets: shownTargets, rules, unreachable, now, windowDays: HISTORY_DAYS })
	);

	/**
	 * Reads the first-run guide. Silent on failure: a server without the route,
	 * or a read that did not come back, simply means no guide — never an error
	 * banner over a bulletin that is otherwise fine.
	 */
	async function readOnboarding(signal?: AbortSignal) {
		// A viewer can neither add a device nor create a channel: offering the
		// guide would be offering three buttons the server would refuse.
		if (!auth.isAdmin) return;
		if (guideDecided && !showGuide) return;
		try {
			const next = await getOnboarding(signal);
			onboarding = next;
			if (!guideDecided) {
				guideDecided = true;
				showGuide = !next.skipped && !next.complete;
			} else if (next.skipped) {
				showGuide = false;
			}
		} catch {
			guideDecided = true;
		}
	}

	async function load(signal?: AbortSignal) {
		error = null;
		const at = new Date();
		// The history reaches back to the last visit or seven days, whichever is
		// older, so one read feeds both the briefing and the last-7-days figures.
		const sinceMs = Math.min(lastVisit?.getTime() ?? Infinity, at.getTime() - HISTORY_DAYS * DAY_MS);
		// Targets and probe states come from the shared store, polled app-wide;
		// this covers only what it doesn't hold. Secondary readings leave with
		// the lists: their failure is not blocking.
		const side = Promise.all([
			listAlertHistory({ limit: 500, since: new Date(sinceMs).toISOString() }, signal).catch(
				() => [] as AlertHistoryEntry[]
			),
			channels === undefined
				? listChannels(signal)
						.then((list) => list.filter((channel) => channel.enabled).map((channel) => channel.name))
						.catch(() => undefined)
				: Promise.resolve(channels)
		]);
		try {
			rules = await listAlertRules(signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
			return;
		} finally {
			loading = false;
		}
		[history, channels] = await side;
		if (shownTargets.length === 0 || demoEmpty) {
			hasDemoKind = await listCollectors(signal)
				.then((kinds) => kinds.some((kind) => kind.kind === 'dummy'))
				.catch(() => false);
		}
		now = at;
		lastChecked = new Date();
		void readOnboarding(signal);
		if (!watching && !armTimer) armTimer = setTimeout(() => (watching = true), 3000);
	}
	let armTimer: ReturnType<typeof setTimeout> | undefined;

	/** Resets the briefing window to now: the story restarts from here. */
	function markRead() {
		const at = new Date();
		lastVisit = at;
		now = at;
		writeLastVisit(at);
	}

	// Loaded on open, then every 30 seconds. The last visit is read once before
	// the first load and written every minute while open, and on the way out.
	onMount(() => {
		lastVisit = readLastVisit();
		const controller = new AbortController();
		void load(controller.signal);
		const poll = setInterval(() => void load(controller.signal), POLL_MS);
		const visit = setInterval(() => writeLastVisit(new Date()), VISIT_MS);
		const leave = () => writeLastVisit(new Date());
		window.addEventListener('pagehide', leave);
		return () => {
			controller.abort();
			clearInterval(poll);
			clearInterval(visit);
			clearTimeout(armTimer);
			clearTimeout(reactionTimer);
			window.removeEventListener('pagehide', leave);
			leave();
		};
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

	const firstLoad = $derived((loading || alertsStore.loading) && targets.length === 0);
	// This page's own error (the rules read) takes precedence; absent that, a
	// failure on the shared store (targets, the badge's alerts) is this page's
	// problem too, since the sky model needs both.
	const pageError = $derived(error ?? (!alertsStore.available ? alertsStore.lastError : null));

	// --- Fitting the window ------------------------------------------------------
	// On a computer the page never scrolls: its root takes the height left under
	// the top bar (minus the main area's bottom padding) and "Needs you" shows as
	// many whole lines as its box holds, counting the rest. Phones keep the
	// natural flow, with the list cut to a handful of lines.
	const FIT_QUERY = '(min-width: 1024px) and (min-height: 560px)';
	/** One compact line (h-14) and the gap between two. */
	const ROW_PX = 56;
	const GAP_PX = 6;
	const PHONE_ROWS = 5;
	/** Part of the main area's bottom padding the page takes back when it fits: the window is the frame. */
	const RECLAIM_PX = 32;
	let root = $state<HTMLDivElement | null>(null);
	let fit = $state(false);
	let fitHeight = $state(0);
	let listHeight = $state(0);
	const needsRows = $derived(
		fit ? Math.max(1, Math.floor((listHeight + GAP_PX) / (ROW_PX + GAP_PX))) : PHONE_ROWS
	);
	/** Problems still open (known ones, acked or snoozed, left out), for the heading's count. */
	const needsCount = $derived(
		sky.needsYou.filter((row) => !isAckedRow(row) && !isSnoozedRow(row)).length
	);

	function measure() {
		if (!root) return;
		fit = window.matchMedia(FIT_QUERY).matches;
		if (!fit) return;
		// Below the page: the main area's bottom padding and whatever the layout
		// puts in the flow after it (the version tag).
		// `main` is the layout column's flex-1, so the gap between their bottoms
		// is exactly what follows it, margins included.
		const main = root.closest('main');
		const column = main?.parentElement;
		const below =
			main && column
				? (parseFloat(getComputedStyle(main).paddingBottom) || 0) +
					Math.max(0, column.getBoundingClientRect().bottom - main.getBoundingClientRect().bottom)
				: 0;
		const top = root.getBoundingClientRect().top + window.scrollY;
		// One pixel spare: fractional heights (the version tag) round the document up.
		fitHeight = Math.max(0, Math.floor(window.innerHeight - top - below + RECLAIM_PX) - 1);
	}

	$effect(() => {
		if (!root) return;
		measure();
		// The bars above (demo banner, nav) can change height: the body follows them.
		const observer = new ResizeObserver(() => measure());
		observer.observe(document.body);
		window.addEventListener('resize', measure);
		return () => {
			observer.disconnect();
			window.removeEventListener('resize', measure);
		};
	});
</script>

<svelte:head><title>{m.overview_page_title()} · DumbMonit</title></svelte:head>

<!--
	The whole page fits the window on a computer (≥ 1024 × 560): the sky band
	on top, then "Needs you" (left, 7/12) beside "Since you last looked" and
	"Last 7 days" (right, 5/12). The list is cut to the lines that fit and
	counts the rest. On phones the same blocks stack in that order of urgency.
-->
<div
	bind:this={root}
	class="flex min-w-0 flex-col gap-5 lg:gap-4"
	style:height={fit && fitHeight > 0 ? `${fitHeight}px` : null}
	style:margin-bottom={fit && fitHeight > 0 ? `-${RECLAIM_PX}px` : null}
>
	{#if pageError && targets.length === 0}
		<ErrorNotice
			error={pageError}
			title={m.overview_page_error_load()}
			onretry={() => {
				loading = true;
				void load();
				void alertsStore.refresh();
			}}
		/>
	{:else if firstLoad}
		<!-- Skeleton shaped like the page: the sky band, then the list beside the briefing. -->
		<Skeleton class="h-[200px] w-full shrink-0 rounded-[var(--radius-card)] lg:h-[clamp(160px,25dvh,250px)]" />
		<div class="grid min-h-0 flex-1 gap-5 lg:grid-cols-12 lg:overflow-hidden">
			<div class="space-y-1.5 lg:col-span-7">
				<Skeleton class="mb-3 h-5 w-24" />
				{#each { length: 4 } as _, i (i)}
					<Skeleton class="h-14 w-full" />
				{/each}
			</div>
			<div class="space-y-2.5 lg:col-span-5">
				<Skeleton class="h-5 w-44" />
				<Skeleton class="h-5 w-3/4" />
				<Skeleton class="h-4 w-2/3" />
			</div>
		</div>
	{:else}
		<!-- The sky, full width; the plate sits at the bottom of the band. -->
		<section
			data-tour="weather"
			class="rise-in relative flex min-h-[140px] shrink-0 flex-col justify-end overflow-hidden rounded-[var(--radius-card)] border border-line shadow-lift sm:min-h-[200px] lg:h-[clamp(160px,25dvh,250px)] lg:min-h-0"
		>
			<SkyScene {condition} mood={pigeon} playful frame={false} class="absolute inset-0 h-full w-full" />

			{#if !noDevices && !showGuide}
				<!--
					One primary per view; on phones it sits at the end of the page. While
					the first-run guide is up, its current step is the one thing to do.
				-->
				<div class="absolute top-4 right-4 hidden sm:block">
					<ClickSpark>
						<Button variant="primary" href="/targets/new">{m.overview_first_device_action()}</Button>
					</ClickSpark>
				</div>
			{/if}

			<div
				class="relative m-3 mt-10 rounded-lg bg-surface/85 px-4 py-3 shadow-lift backdrop-blur sm:m-5 sm:mt-20 sm:max-w-[680px] sm:px-5 sm:py-4 lg:mt-5"
			>
				<DecryptText
					tag="h1"
					text={sky.sentence}
					speed={16}
					hold={2}
					class="sky-sentence display text-[1.75rem] leading-[1.05] text-ink sm:text-[2.25rem] xl:text-[2.5rem]"
				/>
				{#if !noDevices}
					<div class="mt-2 flex flex-wrap items-center gap-2">
						{#each sky.plates as plate (plate.label)}
							<Plate tone={plate.tone} label={plate.label} bare={plate.bare} />
						{/each}
						{#if checkedLabel}
							<span class="tnum text-[0.8125rem] text-ink-2">{m.overview_page_checked({ when: checkedLabel })}</span>
						{/if}
					</div>
				{/if}
			</div>
		</section>

		{#if showGuide && onboarding}
			<!-- The guide replaces the lower blocks until it is done: one thing to do at a time. -->
			<div class="min-h-0 flex-1 lg:overflow-y-auto">
				<FirstRun
					guide={onboarding}
					hasDemo={hasDemoKind}
					onchange={(next) => {
						onboarding = next;
						if (next.skipped) showGuide = false;
					}}
				/>
			</div>
		{:else if noDevices}
			<!-- The sky above already says there is nothing to watch: one way in is enough. -->
			<div class="min-h-0 flex-1">
				<EmptyState
					title={m.overview_page_empty_title()}
					description={m.overview_page_empty_text()}
					mascot="watch"
					class="lg:h-full lg:!py-6"
				>
					{#snippet action()}
						<ClickSpark>
							<Button variant="primary" href="/targets/new">{m.overview_first_device_action()}</Button>
						</ClickSpark>
						{#if auth.isAdmin}
							<p class="mt-4 text-sm text-ink-2">
								{orParts[0]}<a href="/targets/new?kind=agent" class="inline-block py-1 font-semibold text-ink underline decoration-line-strong underline-offset-2 hover:decoration-ink">{orParts[1]}</a>{orParts[2]}<a href="/targets/new?kind=agent&via=relay" class="inline-block py-1 font-semibold text-ink underline decoration-line-strong underline-offset-2 hover:decoration-ink">{orParts[3]}</a>{orParts[4]}
							</p>
						{/if}
					{/snippet}
				</EmptyState>
			</div>
		{:else}
			<div class="grid min-h-0 flex-1 grid-cols-1 gap-x-8 gap-y-6 lg:grid-cols-12">
				<!-- Needs you: what to act on comes first. -->
				<section class="rise-in flex min-h-0 min-w-0 flex-col lg:col-span-7" style={`--rise-delay: ${STAGGER_MS}ms`}>
					<h2 class="mb-2 flex min-h-8 items-center gap-2 text-base font-semibold tracking-tight text-ink">
						{m.overview_page_needs_you()}
						{#if needsCount > 0}
							<span class="tnum rounded-[var(--radius-plate)] border border-line bg-surface-2 px-1.5 text-[0.75rem] font-semibold text-ink-2">{needsCount}</span>
						{/if}
					</h2>
					<div class="min-h-0 flex-1" bind:clientHeight={listHeight}>
						<NeedsYouCompact {sky} {checkedLabel} rows={needsRows} fill={fit} />
					</div>
				</section>

				<div class="flex min-h-0 min-w-0 flex-col gap-6 lg:col-span-5 lg:gap-4">
					<!-- Since you last looked -->
					<section class="rise-in min-w-0" style={`--rise-delay: ${STAGGER_MS * 2}ms`}>
						<div class="mb-2 flex min-h-8 items-center justify-between gap-3">
							<h2 class="text-base font-semibold tracking-tight text-ink">{m.overview_page_since()}</h2>
							<Button variant="ghost" size="sm" onclick={markRead} class="max-sm:!h-10">{m.overview_page_mark_read()}</Button>
						</div>
						<Briefing sentences={briefing} />
					</section>

					<!-- Last 7 days: three figures, only once there is an alert history to read them from. -->
					{#if streaks.length > 0}
						<section class="rise-in min-w-0" style={`--rise-delay: ${STAGGER_MS * 3}ms`}>
							<h2 class="mb-1 flex min-h-8 items-center text-base font-semibold tracking-tight text-ink">
								{m.overview_page_last_days({ days: HISTORY_DAYS })}
							</h2>
							<Streaks {streaks} />
						</section>
					{/if}
				</div>
			</div>

			{#if !showGuide}
				<div class="sm:hidden">
					<ClickSpark>
						<Button variant="primary" href="/targets/new" class="w-full">{m.overview_first_device_action()}</Button>
					</ClickSpark>
				</div>
			{/if}
		{/if}
	{/if}
</div>

<style>
	/* Short computer screens: a smaller sentence keeps the sky band low and leaves the room to the list. */
	@media (min-width: 1024px) and (max-height: 800px) {
		:global(.sky-sentence) {
			font-size: 2rem;
		}
	}
</style>
