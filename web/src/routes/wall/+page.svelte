<script lang="ts">
	/**
	 * Wall mode — the bulletin, full screen, for a display that stays on for days.
	 *
	 * Same truth as the Overview (`readSky` over targets, probes, alerts and
	 * rules), read from across the room: the sky sentence in display type, the
	 * weather window, three big readouts, then "Needs you" in two columns. No
	 * chrome: the page covers the nav with a fixed overlay; Escape or "Exit"
	 * goes back to the overview. Refreshes every 20 s, keeps the screen awake.
	 *
	 * The weather window is the hero: half the width on a desktop, its own band
	 * on a phone. A display can also play music (Spotify, Deezer, YouTube): the
	 * link is kept in this browser only and nothing third-party loads until
	 * one is set.
	 */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { X } from 'lucide-svelte';
	import {
		listTargets,
		listAlerts,
		listAlertRules,
		createSilence,
		type Alert,
		type AlertRule,
		type Target,
		type TargetId
	} from '$lib/api';
	import { formatRelative, type ProbeStatus } from '$lib/format';
	import { loadProbeStatuses } from '$lib/metrics';
	import { theme, type ThemePreference } from '$lib/stores/theme.svelte';
	import { palette } from '$lib/stores/palette.svelte';
	import { Button, Plate, Skeleton, ErrorNotice, DecryptText } from '$lib/ui';
	import { readSky } from '$lib/components/overview/sky';
	import SkyScene from '$lib/components/overview/SkyScene.svelte';
	import NeedsYouList from '$lib/components/alerts/NeedsYouList.svelte';
	import { quickSilencePayload } from '$lib/components/alerts/helpers';
	import WallReadout from '$lib/components/wall/WallReadout.svelte';
	import MusicControl from '$lib/components/wall/MusicControl.svelte';
	import MusicPlayer from '$lib/components/wall/MusicPlayer.svelte';
	import { skyCondition } from '$lib/components/overview/sky';
	import { parseMusicLink } from '$lib/wall/music';

	const REFRESH_MS = 20_000;

	let targets = $state<Target[]>([]);
	let alerts = $state<Alert[]>([]);
	let rules = $state<AlertRule[]>([]);
	let probes = $state<Map<TargetId, ProbeStatus>>(new Map());
	let loading = $state(true);
	let error = $state<unknown>(null);
	let lastChecked = $state<Date | null>(null);
	let now = $state(new Date());
	let silencingKey = $state<string | null>(null);
	let silenceError = $state<string | null>(null);

	/** Music for this display: the share link as pasted, per browser. */
	const MUSIC_KEY = 'dumbmonit-wall-music';
	let musicLink = $state<string | null>(readMusicLink());
	let musicOpen = $state(false);
	// Re-checked on every read: a stored value that no longer parses plays nothing.
	const music = $derived.by(() => {
		if (!musicLink) return null;
		const parsed = parseMusicLink(musicLink);
		return parsed.ok ? parsed.embed : null;
	});

	function readMusicLink(): string | null {
		try {
			return localStorage.getItem(MUSIC_KEY);
		} catch {
			return null; // Storage blocked: the wall simply has no music.
		}
	}

	function saveMusic(link: string | null) {
		musicLink = link;
		try {
			if (link) localStorage.setItem(MUSIC_KEY, link);
			else localStorage.removeItem(MUSIC_KEY);
		} catch {
			// Not persisted (private window): it still plays until the tab closes.
		}
	}

	const sky = $derived(readSky({ targets, probes, alerts, rules }));
	const condition = $derived(skyCondition(sky));

	async function load(signal?: AbortSignal) {
		const probesPromise = loadProbeStatuses(signal).catch(() => new Map<TargetId, ProbeStatus>());
		try {
			const [nextTargets, nextAlerts, nextRules] = await Promise.all([
				listTargets(signal),
				listAlerts(signal),
				listAlertRules(signal)
			]);
			targets = nextTargets;
			alerts = nextAlerts;
			rules = nextRules;
			error = null;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			// A refresh that fails keeps the last good bulletin on screen.
			error = cause;
			return;
		} finally {
			loading = false;
		}
		probes = await probesPromise;
		lastChecked = new Date();
	}

	async function silence(alert: Alert, target: Target) {
		silencingKey = alert.fingerprint;
		silenceError = null;
		try {
			await createSilence(quickSilencePayload(target));
			await load();
		} catch (cause) {
			silenceError = cause instanceof Error ? cause.message : 'Could not create the silence.';
		} finally {
			silencingKey = null;
		}
	}

	// Loaded on open, then every 20 seconds.
	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void load(controller.signal), REFRESH_MS);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	// One tick per second drives the clock, "updated … ago" and the progress bar.
	$effect(() => {
		const timer = setInterval(() => (now = new Date()), 1000);
		return () => clearInterval(timer);
	});

	// Keep the screen on. Browsers release the lock when the tab is hidden, so
	// it is requested again when the display comes back.
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

	// `?theme=dark|light` forces a light for this display without touching the
	// saved preference; leaving the wall restores it.
	$effect(() => {
		const wanted = page.url.searchParams.get('theme');
		if (wanted !== 'dark' && wanted !== 'light') return;
		const previous: ThemePreference = theme.preference;
		theme.preference = wanted;
		return () => {
			theme.preference = previous;
		};
	});

	function exit() {
		void goto('/');
	}

	function onKeydown(event: KeyboardEvent) {
		// The palette owns Escape while it is open; so does the music panel.
		if (event.key === 'Escape' && musicOpen) {
			event.preventDefault();
			musicOpen = false;
			return;
		}
		if (event.key === 'Escape' && !palette.isOpen) {
			event.preventDefault();
			exit();
		}
	}

	const clock = $derived(
		now.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', hour12: false })
	);
	const updatedLabel = $derived.by(() => {
		if (!lastChecked) return 'Waiting for the first check…';
		const seconds = Math.max(0, Math.round((now.getTime() - lastChecked.getTime()) / 1000));
		return seconds < 60 ? `Updated ${seconds} s ago` : `Updated ${formatRelative(lastChecked)}`;
	});
	const progress = $derived(
		lastChecked ? Math.min(1, (now.getTime() - lastChecked.getTime()) / REFRESH_MS) : 0
	);
	const checkedLabel = $derived(lastChecked ? formatRelative(lastChecked) : '');

	const firstLoad = $derived(loading && targets.length === 0);
</script>

<svelte:head><title>Wall · DumbMonit</title></svelte:head>
<svelte:window onkeydown={onKeydown} />

<div class="wall fixed inset-0 z-40 flex flex-col bg-canvas text-ink">
	<!-- Tools float over the weather window's corner, on a frosted pill. -->
	<div
		class="wall-tools absolute top-6 right-6 z-10 flex items-center rounded-xl bg-surface/80 p-1 shadow-lift backdrop-blur sm:top-[2.125rem] sm:right-[2.625rem] lg:top-[2.625rem] lg:right-[3.625rem]"
	>
		<MusicControl
			link={musicLink}
			embed={music}
			bind:open={musicOpen}
			onsave={(link) => saveMusic(link)}
			onclear={() => saveMusic(null)}
		/>
		<Button variant="ghost" size="sm" onclick={exit} aria-label="Exit wall mode">
			<X class="size-4" aria-hidden="true" />
			Exit
			<kbd class="ml-1 rounded-md border border-line bg-surface-2 px-1.5 text-[0.6875rem] text-ink-3">esc</kbd>
		</Button>
	</div>
	<div class="min-h-0 flex-1 overflow-y-auto px-4 pt-4 pb-6 sm:px-8 sm:pt-6 lg:px-12 lg:pt-8">
		{#if error && targets.length === 0}
			<div class="mx-auto max-w-xl pt-[20vh]">
				<ErrorNotice
					{error}
					title="Could not load the bulletin"
					onretry={() => {
						loading = true;
						void load();
					}}
				/>
				<div class="mt-4">
					<Button variant="ghost" onclick={exit}>Back to the overview</Button>
				</div>
			</div>
		{:else if firstLoad}
			<div class="grid gap-8 lg:grid-cols-[minmax(0,1.12fr)_minmax(0,1fr)]">
				<Skeleton class="h-[200px] w-full sm:h-[260px] lg:order-last lg:h-[clamp(320px,46vh,620px)]" />
				<div>
					<Skeleton class="h-24 w-3/4" />
					<Skeleton class="mt-6 h-8 w-1/3" />
				</div>
			</div>
			<div class="mt-10 flex gap-16">
				{#each { length: 3 } as _, i (i)}
					<Skeleton class="h-28 w-40" />
				{/each}
			</div>
		{:else}
			<!--
				Bulletin. Desktop: the sentence and readouts on the left, the weather
				window as the hero on the right (music under it when set). Phone: the
				window first, as a band, then the sentence, readouts and music.
			-->
			<section
				class="hero grid gap-x-10 gap-y-6 lg:grid-cols-[minmax(0,1.12fr)_minmax(0,1fr)] lg:grid-rows-[minmax(0,1fr)_auto] xl:gap-x-14"
			>
				<div class="sky-cell relative lg:col-start-2 lg:row-start-1">
					<SkyScene
						{condition}
						frame={false}
						class="h-[200px] w-full rounded-[var(--radius-card)] border border-line shadow-float sm:h-[260px] lg:h-full lg:min-h-[clamp(300px,42vh,580px)]"
					/>
				</div>

				<div class="flex min-w-0 flex-col lg:col-start-1 lg:row-span-2 lg:row-start-1 lg:py-2">
					<DecryptText
						tag="h1"
						text={sky.sentence}
						speed={16}
						hold={2}
						class="display text-[clamp(2.5rem,4.4vw,5.5rem)] text-ink"
					/>
					<div class="mt-5 flex flex-wrap items-center gap-2.5 sm:gap-3">
						{#each sky.plates as plate (plate.label)}
							<Plate tone={plate.tone} label={plate.label} bare={plate.bare} size="md" />
						{/each}
					</div>

					<!-- Three readouts, read from across the room; they sit on the window's baseline. -->
					<div class="graticule mt-8 flex flex-wrap gap-x-8 gap-y-4 sm:gap-x-14 lg:mt-auto lg:pt-10">
						<WallReadout label="Reporting" value={sky.counts.reporting} tone="signal" />
						<WallReadout
							label="Needs you"
							value={sky.attention}
							tone={sky.attention > 0 ? 'warning' : 'ink'}
						/>
						<WallReadout
							label="Forecasts"
							value={sky.forecasts}
							tone={sky.forecasts > 0 ? 'advisory' : 'ink'}
						/>
					</div>
				</div>

				{#if music}
					<MusicPlayer embed={music} class="lg:col-start-2 lg:row-start-2" />
				{/if}
			</section>

			<!-- Needs you -->
			<section class="mt-8 lg:mt-10">
				<h2 class="mb-4 text-lg font-semibold tracking-tight text-ink lg:text-xl">Needs you</h2>
				{#if silenceError}
					<p class="mb-3 text-sm text-warning-ink" role="alert" aria-live="polite">{silenceError}</p>
				{/if}
				<div class={sky.quiet ? 'wall-quiet' : 'wall-needs'}>
					<NeedsYouList
						{sky}
						{checkedLabel}
						{silencingKey}
						onsilence={silence}
						onackchange={() => void load()}
					/>
				</div>
			</section>
		{/if}
	</div>

	<!-- Clock and freshness, bottom-left; the bar fills up to the next refresh. -->
	<footer class="relative shrink-0 border-t border-line bg-canvas px-4 py-3 sm:px-8 lg:px-12">
		<div
			class="absolute inset-x-0 top-0 h-px origin-left bg-signal transition-transform duration-1000 ease-linear"
			style:transform={`scaleX(${progress})`}
			aria-hidden="true"
		></div>
		<div class="flex flex-wrap items-baseline gap-x-6 gap-y-1">
			<time class="display tnum text-[clamp(1.75rem,3vw,2.75rem)] text-ink" datetime={now.toISOString()}>{clock}</time>
			<span class="tnum text-sm text-ink-2 lg:text-base" aria-live="off">{updatedLabel}</span>
			{#if error}
				<span class="text-sm text-warning-ink lg:text-base" role="status">Last refresh failed, showing the previous bulletin.</span>
			{/if}
		</div>
	</footer>
</div>

<style>
	/*
	 * "Music" stays out of sight on a display nobody touches: it shows when a
	 * pointer moves over the wall, when it is reached with the keyboard, and
	 * always on touch screens (no hover to reveal it).
	 */
	.wall :global(.music-toggle) {
		max-width: 0;
		padding-inline: 0;
		opacity: 0;
		overflow: hidden;
		transition:
			max-width 240ms var(--ease-out-expo, ease),
			padding 240ms var(--ease-out-expo, ease),
			opacity 200ms ease;
	}
	.wall:hover :global(.music-toggle),
	.wall :global(.music-toggle:focus-visible),
	.wall :global(.music-toggle.is-open) {
		max-width: 8rem;
		padding-inline: 0.75rem;
		opacity: 1;
	}
	@media (hover: none) {
		.wall :global(.music-toggle) {
			max-width: 8rem;
			padding-inline: 0.75rem;
			opacity: 1;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.wall :global(.music-toggle) {
			transition: none;
		}
	}

	/*
	 * The "Needs you" rows are shared with the Overview and set their own type
	 * sizes; on a wall they are read from further away, so the list is scaled
	 * up as a whole and laid out in two columns on wide screens.
	 */
	.wall-needs :global(> div) {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 0.75rem;
		align-items: start;
	}
	.wall-needs :global(> div > div) {
		margin: 0 !important;
	}
	@media (min-width: 1280px) {
		.wall-needs :global(> div) {
			grid-template-columns: repeat(2, minmax(0, 1fr));
			gap: 1rem;
		}
		.wall-needs,
		.wall-quiet {
			zoom: 1.2;
		}
	}
	@media (min-width: 1800px) {
		.wall-needs,
		.wall-quiet {
			zoom: 1.35;
		}
	}
</style>
