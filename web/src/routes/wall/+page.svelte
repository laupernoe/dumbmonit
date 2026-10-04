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
	 * on a phone. Music lives in that window's bottom corner (`MusicDock`), laid
	 * over the sky and never in the grid: it can't shrink or push the bulletin.
	 * What plays on the connected Spotify account shows there wherever it plays;
	 * a link sent to the walls plays in the service's own player; and the
	 * display can be a Spotify Connect speaker ("DumbMonit Wall"). Nothing
	 * third-party loads until Spotify is connected or a link is set.
	 */
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { X } from 'lucide-svelte';
	import {
		ApiError,
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
	import { auth } from '$lib/stores/auth.svelte';
	import { theme, type ThemePreference } from '$lib/stores/theme.svelte';
	import { palette } from '$lib/stores/palette.svelte';
	import { Button, Plate, Skeleton, ErrorNotice, DecryptText } from '$lib/ui';
	import { readSky } from '$lib/components/overview/sky';
	import SkyScene from '$lib/components/overview/SkyScene.svelte';
	import NeedsYouList from '$lib/components/alerts/NeedsYouList.svelte';
	import { quickSilencePayload } from '$lib/components/alerts/helpers';
	import WallReadout from '$lib/components/wall/WallReadout.svelte';
	import WallAmbient from '$lib/components/wall/WallAmbient.svelte';
	import WallThemeControl from '$lib/components/wall/WallThemeControl.svelte';
	import MusicControl from '$lib/components/wall/MusicControl.svelte';
	import MusicDock from '$lib/components/wall/MusicDock.svelte';
	import { skyCondition } from '$lib/components/overview/sky';
	import {
		readWallTheme,
		writeWallTheme,
		parseForcedWallTheme,
		readNightDim,
		writeNightDim,
		isNightHour,
		OLED_SHIFT_OFFSETS,
		OLED_SHIFT_INTERVAL_MS,
		type WallThemeChoice
	} from '$lib/components/wall/wallTheme';
	import { getWallMusic, playOnWall, stopWallLink, type WallMusic } from '$lib/api/music';
	import { parseMusicLink } from '$lib/wall/music';
	import { cardVisible, pickPlaying } from '$lib/wall/spotify';
	import { WallSpeaker } from '$lib/wall/speaker.svelte';

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

	// --- Music ------------------------------------------------------------------

	/**
	 * What the walls play, polled every 5 s while the display is visible: the
	 * Spotify account's playback (wherever it plays) and the shared link. The
	 * server caches Spotify for a few seconds, whatever the number of walls.
	 */
	const MUSIC_MS = 5_000;
	let wallMusic = $state<WallMusic | null>(null);
	let musicReadAt = $state(0);
	let musicOpen = $state(false);
	const speakerName = $derived(wallMusic?.speaker_name ?? 'DumbMonit Wall');

	/**
	 * Before September 2026 a wall kept its link in this browser only. It still
	 * plays here until the walls share one; an admin's wall hands it over to
	 * the server once, then forgets it.
	 */
	const LEGACY_KEY = 'dumbmonit-wall-music';
	let legacyLink = $state<string | null>(readLegacyLink());

	function readLegacyLink(): string | null {
		try {
			return localStorage.getItem(LEGACY_KEY) || null;
		} catch {
			return null;
		}
	}

	function forgetLegacyLink() {
		legacyLink = null;
		try {
			localStorage.removeItem(LEGACY_KEY);
		} catch {
			// Nothing stored, or storage blocked: nothing to forget.
		}
	}

	const sharedLink = $derived(wallMusic?.link?.link ?? null);
	const musicLink = $derived(sharedLink ?? (wallMusic ? legacyLink : null));
	// Re-checked on every read: a stored value that no longer parses plays nothing.
	const music = $derived.by(() => {
		if (!musicLink) return null;
		const parsed = parseMusicLink(musicLink);
		return parsed.ok ? parsed.embed : null;
	});
	// The link present when the wall opened plays when asked; one sent while it
	// is open (from a phone) starts by itself, where the provider allows it.
	let openingLink = $state<string | null | undefined>(undefined);
	const musicAutoplay = $derived(openingLink !== undefined && musicLink !== null && musicLink !== openingLink);

	async function loadMusic(signal?: AbortSignal): Promise<boolean> {
		try {
			const next = await getWallMusic(signal);
			if (openingLink === undefined) openingLink = next.link?.link ?? legacyLink;
			wallMusic = next;
			musicReadAt = Date.now();
			if (!next.link && legacyLink && auth.isAdmin) {
				// Hand the old per-browser link over to every wall, once.
				const link = legacyLink;
				openingLink = link;
				try {
					wallMusic = { ...next, link: await playOnWall(link) };
					forgetLegacyLink();
				} catch (cause) {
					// Refused (no longer a playable link): drop it rather than retry forever.
					if (cause instanceof ApiError && cause.status === 400) forgetLegacyLink();
				}
			} else if (next.link && legacyLink) {
				forgetLegacyLink();
			}
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return false;
			// An older server without the route: no music, and no more asking.
			if (cause instanceof ApiError && cause.missing) return false;
			// Otherwise keep what is on screen and try again at the next tick.
		}
		return true;
	}

	$effect(() => {
		const controller = new AbortController();
		let timer: ReturnType<typeof setTimeout> | undefined;
		const poll = async () => {
			const again = document.visibilityState === 'visible' ? await loadMusic(controller.signal) : true;
			if (again && !controller.signal.aborted) timer = setTimeout(poll, MUSIC_MS);
		};
		void poll();
		const onVisible = () => {
			if (document.visibilityState !== 'visible' || controller.signal.aborted) return;
			clearTimeout(timer);
			void poll();
		};
		document.addEventListener('visibilitychange', onVisible);
		return () => {
			controller.abort();
			clearTimeout(timer);
			document.removeEventListener('visibilitychange', onVisible);
		};
	});

	async function sendLink(link: string) {
		const saved = await playOnWall(link);
		if (wallMusic) wallMusic = { ...wallMusic, link: saved };
		forgetLegacyLink();
	}

	async function stopLink() {
		if (sharedLink) await stopWallLink();
		if (wallMusic) wallMusic = { ...wallMusic, link: null };
		forgetLegacyLink();
	}

	/** This display as a Spotify Connect speaker; each display may opt out. */
	const SPEAKER_KEY = 'dumbmonit-wall-speaker';
	const speaker = new WallSpeaker();
	let speakerOn = $state(readSpeakerOn());

	function readSpeakerOn(): boolean {
		try {
			return localStorage.getItem(SPEAKER_KEY) !== 'off';
		} catch {
			return true;
		}
	}

	function setSpeakerOn(on: boolean) {
		speakerOn = on;
		try {
			if (on) localStorage.removeItem(SPEAKER_KEY);
			else localStorage.setItem(SPEAKER_KEY, 'off');
		} catch {
			// Not remembered: it applies until the page reloads.
		}
	}

	const spotifyConnected = $derived(wallMusic?.spotify.status === 'connected');
	$effect(() => {
		const wanted = spotifyConnected && speakerOn;
		const name = speakerName;
		untrack(() => {
			if (wanted) void speaker.start(name);
			else if (speaker.phase !== 'off') speaker.stop();
		});
	});
	$effect(() => () => speaker.stop());

	const playing = $derived(pickPlaying(speaker.local, wallMusic?.spotify.now_playing ?? null));
	const playingReadAt = $derived(playing?.source === 'speaker' ? speaker.localAt : musicReadAt);
	let lastPlayingAt = $state<number | null>(null);
	$effect(() => {
		if (playing?.now.playing) lastPlayingAt = untrack(() => now.getTime());
	});
	const shownPlaying = $derived(playing && cardVisible(playing.now, lastPlayingAt, now.getTime()) ? playing : null);

	const sky = $derived(readSky({ targets, probes, alerts, rules }));
	const condition = $derived(skyCondition(sky));

	// --- Ambient background -----------------------------------------------------

	/**
	 * A change worth a glance gets three slow pulses from the ambient backdrop
	 * (`WallAmbient`), then it holds still again: a new line in "Needs you",
	 * or the sky turning to storm. Nothing pulses on the very first load.
	 */
	let ambientPulse = $state(false);
	let ambientBaseline: { attention: number; condition: typeof condition } | null = null;
	$effect(() => {
		const attention = sky.attention;
		const cond = condition;
		// Ignore everything before the first load resolves: targets/alerts
		// start empty, so the jump from "empty" to the real bulletin is not a
		// change worth a pulse — it is simply the page finishing its first load.
		if (loading) return;
		if (ambientBaseline && (attention > ambientBaseline.attention || (cond === 'storm' && ambientBaseline.condition !== 'storm'))) {
			ambientPulse = false;
			requestAnimationFrame(() => (ambientPulse = true));
		}
		ambientBaseline = { attention, condition: cond };
	});

	// --- Theme --------------------------------------------------------------

	/**
	 * The wall's own Auto / Day / Night / OLED choice, remembered per browser
	 * like the Spotify-speaker switch. A `?theme=` on a shared link overrides
	 * it for this display only, the same way it already did for day/night.
	 */
	let wallTheme = $state<WallThemeChoice>(readWallTheme());
	let themeOpen = $state(false);
	let nightDim = $state(readNightDim());

	function setWallTheme(choice: WallThemeChoice) {
		wallTheme = choice;
		writeWallTheme(choice);
	}
	function setNightDim(on: boolean) {
		nightDim = on;
		writeNightDim(on);
	}

	const forcedWallTheme = $derived(parseForcedWallTheme(page.url.searchParams.get('theme')));
	const effectiveWallTheme = $derived(forcedWallTheme ?? wallTheme);
	const oledActive = $derived(effectiveWallTheme === 'oled');
	const nightDimActive = $derived(oledActive && nightDim && isNightHour(now));

	// Auto rides whatever the rest of DumbMonit shows. Day/Night/OLED force
	// the app-wide theme for this display only — never saved there, and
	// restored on exit; OLED rides the Night palette, then this page paints
	// true black and dimmed text over it (see the `oled` styles below).
	$effect(() => {
		if (effectiveWallTheme === 'auto') return;
		const base: ThemePreference = effectiveWallTheme === 'oled' ? 'dark' : effectiveWallTheme;
		const previous: ThemePreference = untrack(() => theme.preference);
		theme.preference = base;
		return () => {
			theme.preference = previous;
		};
	});

	// Burn-in care: nudge the whole layout a few pixels every few minutes so
	// no static bright element (the clock, the tool pill) sits on the same
	// pixels for long.
	let oledShiftIndex = $state(0);
	$effect(() => {
		if (!oledActive) {
			oledShiftIndex = 0;
			return;
		}
		const timer = setInterval(() => {
			oledShiftIndex = (oledShiftIndex + 1) % OLED_SHIFT_OFFSETS.length;
		}, OLED_SHIFT_INTERVAL_MS);
		return () => clearInterval(timer);
	});
	const oledShift = $derived(OLED_SHIFT_OFFSETS[oledShiftIndex]);

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

	function exit() {
		void goto('/');
	}

	function onKeydown(event: KeyboardEvent) {
		// The palette owns Escape while it is open; so do the music and theme panels.
		if (event.key === 'Escape' && (musicOpen || themeOpen)) {
			event.preventDefault();
			musicOpen = false;
			themeOpen = false;
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

<div
	class="wall fixed inset-0 z-40 flex flex-col bg-canvas text-ink"
	data-wall-theme={oledActive ? 'oled' : undefined}
>
	<!-- The calm backdrop, behind everything, painted first so it never
		 covers a click. -->
	<WallAmbient {condition} pulse={ambientPulse} onpulseend={() => (ambientPulse = false)} />

	<!--
		Everything the room actually reads lives in this stage. On OLED it is
		nudged a few pixels every few minutes (burn-in care) — the backdrop
		above does not move with it, so the shift is never seen, only felt as
		"nothing stays lit in one spot".
	-->
	<div
		class="wall-stage relative flex min-h-0 flex-1 flex-col"
		style:transform={oledActive ? `translate(${oledShift[0]}px, ${oledShift[1]}px)` : undefined}
	>
		<!-- Tools float over the weather window's corner, on a frosted pill. -->
		<div
			class="wall-tools absolute top-6 right-6 z-10 flex items-center rounded-xl bg-surface/80 p-1 shadow-lift backdrop-blur sm:top-[2.125rem] sm:right-[2.625rem] lg:top-[2.625rem] lg:right-[3.625rem]"
		>
			<WallThemeControl
				value={wallTheme}
				forced={forcedWallTheme}
				{nightDim}
				bind:open={themeOpen}
				onchange={setWallTheme}
				onnightdim={setNightDim}
			/>
			<MusicControl
				link={musicLink}
				embed={music}
				bind:open={musicOpen}
				isAdmin={auth.isAdmin}
				spotify={wallMusic?.spotify ?? null}
				{speaker}
				{speakerName}
				{speakerOn}
				onsave={sendLink}
				onclear={stopLink}
				onspeaker={setSpeakerOn}
				onretry={() => void speaker.start(speakerName, { retry: true })}
			/>
			<Button variant="ghost" size="sm" onclick={exit} aria-label="Exit wall mode">
				<X class="size-4" aria-hidden="true" />
				Exit
				<kbd class="ml-1 rounded-md border border-line bg-surface-2 px-1.5 text-[0.6875rem] text-ink-3">esc</kbd>
			</Button>
		</div>
		{#if nightDimActive}
			<div class="wall-night-dim" aria-hidden="true"></div>
		{/if}
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
					window as the hero on the right. Phone: the window first, as a band,
					then the sentence and readouts. Music sits inside the window.
				-->
				<section class="hero grid gap-x-10 gap-y-6 lg:grid-cols-[minmax(0,1.12fr)_minmax(0,1fr)] xl:gap-x-14">
					<div class="sky-cell relative lg:col-start-2 lg:row-start-1">
						<SkyScene
							{condition}
							frame={false}
							class="h-[200px] w-full rounded-[var(--radius-card)] border border-line shadow-float sm:h-[260px] lg:h-full lg:min-h-[clamp(300px,42vh,580px)]"
						/>
						<MusicDock
							playing={shownPlaying}
							{speakerName}
							readAt={playingReadAt}
							clock={now.getTime()}
							embed={music}
							autoplay={musicAutoplay}
							needsTap={speakerOn && speaker.needsTap}
							onactivate={() => speaker.activate()}
							onstop={auth.isAdmin || !sharedLink ? () => void stopLink() : undefined}
						/>
					</div>

					<div class="flex min-w-0 flex-col lg:col-start-1 lg:row-start-1 lg:py-2">
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
</div>

<style>
	/*
	 * "Music" and "Theme" stay out of sight on a display nobody touches: they
	 * show when a pointer moves over the wall, when reached with the
	 * keyboard, and always on touch screens (no hover to reveal them).
	 */
	.wall :global(.music-toggle),
	.wall :global(.theme-toggle) {
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
	.wall :global(.music-toggle.is-open),
	.wall:hover :global(.theme-toggle),
	.wall :global(.theme-toggle:focus-visible),
	.wall :global(.theme-toggle.is-open) {
		max-width: 8rem;
		padding-inline: 0.75rem;
		opacity: 1;
	}
	@media (hover: none) {
		.wall :global(.music-toggle),
		.wall :global(.theme-toggle) {
			max-width: 8rem;
			padding-inline: 0.75rem;
			opacity: 1;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.wall :global(.music-toggle),
		.wall :global(.theme-toggle) {
			transition: none;
		}
	}
	/* The music corner's fold and stop buttons follow the same rule. */
	.wall :global(.wall-reveal) {
		opacity: 0;
		transition: opacity 200ms ease;
	}
	.wall:hover :global(.wall-reveal),
	.wall :global(.wall-reveal:focus-within),
	.wall :global(.wall-reveal:focus-visible) {
		opacity: 1;
	}
	@media (hover: none) {
		.wall :global(.wall-reveal) {
			opacity: 1;
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

	/*
	 * OLED burn-in care: the whole stage eases to its next offset over a few
	 * seconds — slow enough that the shift itself is never seen, only ever
	 * felt as "nothing stays lit in one spot" days later.
	 */
	.wall-stage {
		transition: transform 4s var(--ease-out-expo, ease);
	}
	@media (prefers-reduced-motion: reduce) {
		.wall-stage {
			transition: none;
		}
	}

	/* Optional night dimming on OLED: a plain black veil, opacity only. */
	.wall-night-dim {
		position: absolute;
		inset: 0;
		z-index: 15;
		background: #000;
		opacity: 0;
		pointer-events: none;
		animation: wall-dim-in 4s ease forwards;
	}
	@keyframes wall-dim-in {
		to {
			opacity: 0.45;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.wall-night-dim {
			animation: none;
			opacity: 0.45;
		}
	}

	/*
	 * OLED: true black, dimmed text, no bright surfaces. Scoped to the wall
	 * so the rest of the app keeps its ordinary night palette; built on top
	 * of it (the `dark` class still applies underneath, for anything this
	 * does not override).
	 */
	.wall[data-wall-theme='oled'] {
		--c-canvas: #000000;
		--c-canvas-deep: #000000;
		--c-surface: #000000;
		--c-surface-2: #0a0a0a;
		--c-line: rgb(255 255 255 / 0.1);
		--c-line-strong: rgb(255 255 255 / 0.18);
		--c-ink: #a9b2c2;
		--c-ink-2: #6b7384;
		--c-ink-3: #454b58;
		--c-ghost: rgb(255 255 255 / 0.08);
		--shadow-lift: none;
		--shadow-float: none;
		/* A touch dimmer still: OLED has no room for "a bit bright". Read by
		   `WallAmbient`, which inherits it — it never sets this itself. */
		--amb-scale: 0.7;
	}
	/*
	 * The weather window (`SkyScene`) is shared with the Overview and keeps
	 * its own palette and lightning flash — nothing here edits it. A single
	 * static filter (no animated property, so it costs nothing beyond the
	 * scene's own already-animated content) is the only OLED-safe way left
	 * to take a bright sky and a white flash down to "no large bright area".
	 */
	.wall[data-wall-theme='oled'] :global(.sky) {
		filter: brightness(0.5);
	}
</style>
