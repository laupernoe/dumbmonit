<script lang="ts">
	/**
	 * Wall mode — a Paris rooftop for a display that stays on for days.
	 *
	 * Half TV dashboard, half ambient picture. The whole screen is a living
	 * scene (`ParisScene`): the Eiffel Tower, Haussmann roofs, a sky that
	 * follows the real hour, and the DumbMonit pigeons strolling, pecking and
	 * napping on the parapet. The infrastructure is woven into it — clouds
	 * gather, it drizzles, Gaston holds up a sign with the number of problems —
	 * and stated plainly on top of it, for across the room: the headline
	 * ("All good" / "2 problems") with its icon, the problems as big tiles,
	 * every device as a small tile (problems first), the clock and the date,
	 * and what is playing.
	 *
	 * Same truth as the Overview (`readSky` over targets, probes, alerts and
	 * rules), refreshed every 20 s; the screen is kept awake; Escape or "Exit"
	 * goes back to the overview. No chrome: a fixed overlay covers the nav.
	 *
	 * Music sits under the clock (`MusicDock`). What plays on the connected
	 * Spotify account shows there wherever it plays; a link sent to the walls
	 * plays in the service's own player; and the display can be a Spotify
	 * Connect speaker ("DumbMonit Wall", the name chosen in Settings, or
	 * `?speaker=Living room` for this display alone). The first click or key
	 * press anywhere on the wall unlocks its sound; when the speaker cannot
	 * work here, the dock says why, on the wall itself. Nothing third-party
	 * loads until Spotify is connected or a link is set.
	 *
	 * Theme: Auto / Day / Night / OLED per display (`?theme=` forces one),
	 * with burn-in care and optional night dimming on OLED.
	 */
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { AlertTriangle, CircleAlert, CircleCheck, CircleDashed, X } from 'lucide-svelte';
	import {
		ApiError,
		listTargets,
		listAlerts,
		listAlertRules,
		type Alert,
		type AlertRule,
		type Target,
		type TargetId
	} from '#lib/api/index.js';
	import { formatRelative, type ProbeStatus } from '#lib/format.js';
	import { loadProbeStatuses } from '#lib/metrics.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { theme, type ThemePreference } from '#lib/stores/theme.svelte.js';
	import { palette } from '#lib/stores/palette.svelte.js';
	import { Button, Plate, Skeleton, ErrorNotice } from '#lib/ui/index.js';
	import { readSky, skyCondition } from '#lib/components/overview/sky.js';
	import ParisScene from '#lib/components/wall/paris/ParisScene.svelte';
	import type { SceneMood, SceneTheme } from '#lib/components/wall/paris/daylight.js';
	import WallProblems from '#lib/components/wall/WallProblems.svelte';
	import WallDevices from '#lib/components/wall/WallDevices.svelte';
	import WallThemeControl from '#lib/components/wall/WallThemeControl.svelte';
	import MusicControl from '#lib/components/wall/MusicControl.svelte';
	import MusicDock from '#lib/components/wall/MusicDock.svelte';
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
	} from '#lib/components/wall/wallTheme.js';
	import { getWallMusic, playOnWall, stopWallLink, type WallMusic } from '#lib/api/music.js';
	import { parseMusicLink } from '#lib/wall/music.js';
	import { cardVisible, displaySpeakerName, pickPlaying } from '#lib/wall/spotify.js';
	import { WallSpeaker } from '#lib/wall/speaker.svelte.js';

	const REFRESH_MS = 20_000;

	let targets = $state<Target[]>([]);
	let alerts = $state<Alert[]>([]);
	let rules = $state<AlertRule[]>([]);
	let probes = $state<Map<TargetId, ProbeStatus>>(new Map());
	let loading = $state(true);
	let error = $state<unknown>(null);
	let lastChecked = $state<Date | null>(null);
	let now = $state(new Date());

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
	/** `?speaker=Kitchen` names this display alone, and is remembered by it (`?speaker=` forgets). */
	const SPEAKER_NAME_KEY = 'dumbmonit-wall-speaker-name';
	let rememberedName = $state<string | null>(readRememberedName());
	function readRememberedName(): string | null {
		try {
			return localStorage.getItem(SPEAKER_NAME_KEY);
		} catch {
			return null;
		}
	}
	$effect(() => {
		const wanted = page.url.searchParams.get('speaker');
		if (wanted === null) return;
		const name = wanted.trim();
		rememberedName = name || null;
		try {
			if (name) localStorage.setItem(SPEAKER_NAME_KEY, name);
			else localStorage.removeItem(SPEAKER_NAME_KEY);
		} catch {
			// Not remembered: the address still names it while it is open.
		}
	});
	const speakerName = $derived(
		displaySpeakerName(page.url.searchParams.get('speaker'), rememberedName, wallMusic?.speaker_name ?? 'DumbMonit Wall')
	);

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

	// The first click, tap or key press anywhere on the wall (a TV remote's OK
	// button included) unlocks the sound: browsers want a gesture, not a
	// particular button.
	$effect(() => {
		const unlock = () => {
			if (speakerOn && speaker.needsTap) speaker.activate();
		};
		document.addEventListener('pointerdown', unlock, { capture: true });
		document.addEventListener('keydown', unlock, { capture: true });
		return () => {
			document.removeEventListener('pointerdown', unlock, { capture: true });
			document.removeEventListener('keydown', unlock, { capture: true });
		};
	});

	/** Why this display cannot be a speaker, on the wall itself (not only in the Music panel). */
	const speakerTrouble = $derived(
		speakerOn && spotifyConnected && (speaker.phase === 'unsupported' || speaker.phase === 'error') && speaker.problem
			? { problem: speaker.problem, fix: speaker.fix, retrying: speaker.retryAt !== null }
			: null
	);

	const playing = $derived(pickPlaying(speaker.local, wallMusic?.spotify.now_playing ?? null));
	const playingReadAt = $derived(playing?.source === 'speaker' ? speaker.localAt : musicReadAt);
	let lastPlayingAt = $state<number | null>(null);
	$effect(() => {
		if (playing?.now.playing) lastPlayingAt = untrack(() => now.getTime());
	});
	const shownPlaying = $derived(playing && cardVisible(playing.now, lastPlayingAt, now.getTime()) ? playing : null);

	const sky = $derived(readSky({ targets, probes, alerts, rules }));
	const condition = $derived(skyCondition(sky));

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

	// --- The scene ----------------------------------------------------------------

	/** A hidden tab pauses the scene outright: nothing to look at, nothing to spend. */
	let hidden = $state(typeof document !== 'undefined' && document.visibilityState !== 'visible');
	$effect(() => {
		const onVisibility = () => (hidden = document.visibilityState !== 'visible');
		document.addEventListener('visibilitychange', onVisibility);
		return () => document.removeEventListener('visibilitychange', onVisibility);
	});

	const sceneTheme = $derived<SceneTheme>(oledActive ? 'oled' : theme.resolved);
	const mood = $derived<SceneMood>(
		condition === 'storm' ? 'storm' : condition === 'cloudy' || condition === 'overcast' ? 'clouded' : 'calm'
	);

	/**
	 * A new problem startles the pigeons once (and drops a few feathers).
	 * Nothing startles on the very first load: the jump from "nothing loaded"
	 * to the real board is the page finishing, not news.
	 */
	let startle = $state(0);
	let attentionBaseline: number | null = null;
	$effect(() => {
		const attention = sky.attention;
		if (loading) return;
		if (attentionBaseline !== null && attention > attentionBaseline) untrack(() => (startle += 1));
		attentionBaseline = attention;
	});

	// --- The board ------------------------------------------------------------------

	type HeadTone = 'signal' | 'warning' | 'advisory' | 'ghost';
	const status = $derived.by((): { tone: HeadTone; text: string } => {
		if (sky.counts.devices === 0) return { tone: 'ghost', text: 'Nothing to watch yet' };
		if (sky.attention > 0) {
			const severe = sky.counts.warnings > 0 || sky.counts.unreachable > 0;
			return {
				tone: severe ? 'warning' : 'advisory',
				text: `${sky.attention} ${sky.attention === 1 ? 'problem' : 'problems'}`
			};
		}
		if (sky.counts.reporting === 0) return { tone: 'ghost', text: 'Waiting for reports' };
		return { tone: 'signal', text: 'All good' };
	});
	const STATUS_ICON = {
		signal: CircleCheck,
		warning: AlertTriangle,
		advisory: CircleAlert,
		ghost: CircleDashed
	};
	const StatusIcon = $derived(STATUS_ICON[status.tone]);

	const musicVisible = $derived(
		!!shownPlaying || !!music || (speakerOn && speaker.needsTap) || !!speakerTrouble
	);

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
	const dateLabel = $derived(
		now.toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'long' })
	);
	const updatedLabel = $derived.by(() => {
		if (!lastChecked) return 'Waiting for the first check…';
		const seconds = Math.max(0, Math.round((now.getTime() - lastChecked.getTime()) / 1000));
		return seconds < 60 ? `Updated ${seconds} s ago` : `Updated ${formatRelative(lastChecked)}`;
	});
	const progress = $derived(
		lastChecked ? Math.min(1, (now.getTime() - lastChecked.getTime()) / REFRESH_MS) : 0
	);

	const firstLoad = $derived(loading && targets.length === 0);
</script>

<svelte:head><title>Wall · DumbMonit</title></svelte:head>
<svelte:window onkeydown={onKeydown} />

<div class="wall fixed inset-0 z-40 bg-canvas text-ink" data-wall-theme={oledActive ? 'oled' : undefined}>
	<!--
		Everything lives in this stage. On OLED it is nudged a few pixels every
		few minutes (burn-in care), scene included: nothing stays lit in one spot.
	-->
	<div
		class="wall-stage relative h-full"
		style:transform={oledActive ? `translate(${oledShift[0]}px, ${oledShift[1]}px)` : undefined}
	>
		<div class="scene-box">
			<ParisScene
				theme={sceneTheme}
				{now}
				{mood}
				problems={firstLoad ? 0 : sky.attention}
				{startle}
				paused={hidden}
				focus={1360}
			/>
		</div>

		<div class="board">
			{#if error && targets.length === 0}
				<div class="board-left">
					<div class="max-w-xl rounded-[var(--radius-card)] bg-surface p-2 shadow-float">
						<ErrorNotice
							{error}
							title="Could not load the wall"
							onretry={() => {
								loading = true;
								void load();
							}}
						/>
					</div>
					<div class="mt-4">
						<Button variant="secondary" onclick={exit}>Back to the overview</Button>
					</div>
				</div>
			{:else if firstLoad}
				<div class="board-left gap-6">
					<Skeleton class="h-20 w-3/4 lg:h-28" />
					<Skeleton class="h-8 w-1/2" />
					<Skeleton class="h-40 w-full" />
				</div>
			{:else}
				<div class="board-left">
					<header class="flex items-center gap-4 lg:gap-6">
						<span class="status-mark status-mark--{status.tone}" aria-hidden="true">
							<StatusIcon class="size-1/2" strokeWidth={2.4} />
						</span>
						<h1 class="headline display min-w-0" aria-live="polite">{status.text}</h1>
					</header>
					<p class="sentence mt-3 lg:mt-4">{sky.sentence}</p>
					<div class="plates mt-4 flex flex-wrap items-center gap-2.5">
						{#each sky.plates as plate (plate.label)}
							<Plate tone={plate.tone} label={plate.label} bare={plate.bare} size="md" />
						{/each}
					</div>

					<div class="mt-7 flex min-h-0 flex-1 flex-col gap-4 lg:mt-9">
						<WallProblems {sky} {now} />
						<WallDevices {targets} {probes} {sky} />
					</div>
				</div>
			{/if}

			<div class="board-right">
				<time class="clock display tnum" datetime={now.toISOString()}>{clock}</time>
				<p class="date">{dateLabel}</p>
				<p class="updated tnum" aria-live="off">
					<span class="refresh" aria-hidden="true"><span style:transform="scaleX({progress})"></span></span>
					{updatedLabel}
				</p>
				{#if error && targets.length > 0}
					<p class="mt-1 text-base text-warning-ink" role="status">Last refresh failed, showing the previous state.</p>
				{/if}

				{#if musicVisible}
					<div class="music-slot">
						<MusicDock
							playing={shownPlaying}
							{speakerName}
							readAt={playingReadAt}
							clock={now.getTime()}
							embed={music}
							autoplay={musicAutoplay}
							needsTap={speakerOn && speaker.needsTap}
							trouble={speakerTrouble}
							onactivate={() => speaker.activate()}
							onretry={() => void speaker.start(speakerName, { retry: true })}
							onstop={auth.isAdmin || !sharedLink ? () => void stopLink() : undefined}
						/>
					</div>
				{/if}
			</div>
		</div>

		<!-- Tools, top right; Music and Theme show on hover, focus or touch. -->
		<div class="wall-tools absolute top-3 right-3 z-10 flex items-center rounded-xl bg-surface/85 p-1 shadow-lift lg:top-4 lg:right-4">
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
	</div>
</div>

<style>
	/*
	 * Layout. Phone and tablet: the scene is a band at the top, the board
	 * scrolls under it. From 1024 px: the scene fills the screen and the board
	 * sits in it — headline, problems and devices on the left third, clock
	 * and music on the right, the middle left to Paris and the bottom eighth
	 * to the parapet and its pigeons.
	 */
	.wall-stage {
		overflow-y: auto;
		/* OLED burn-in care: the stage eases to its next offset over a few seconds. */
		transition: transform 4s var(--ease-out-expo, ease);
	}
	.scene-box {
		position: relative;
		height: max(240px, 40vh);
		overflow: hidden;
	}
	.board {
		display: flex;
		flex-direction: column;
		gap: 2rem;
		padding: 1.5rem 1rem 2.5rem;
	}
	.board-left,
	.board-right {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.headline {
		font-size: clamp(2.5rem, 11vw, 3.5rem);
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
		font-size: 1.25rem;
		font-weight: 500;
		color: var(--c-ink-2);
	}
	.clock {
		font-size: 3rem;
		color: var(--c-ink);
		line-height: 1;
	}
	.date {
		margin-top: 0.375rem;
		font-size: 1.25rem;
		font-weight: 600;
		color: var(--c-ink-2);
	}
	.updated {
		margin-top: 0.5rem;
		display: flex;
		align-items: center;
		gap: 0.625rem;
		font-size: 0.9375rem;
		color: var(--c-ink-2);
	}
	/* Fills up to the next refresh: a quiet second hand for the data. */
	.refresh {
		position: relative;
		width: 3rem;
		height: 3px;
		border-radius: 2px;
		background: var(--c-line-strong);
		overflow: hidden;
	}
	.refresh span {
		position: absolute;
		inset: 0;
		background: var(--c-signal);
		transform-origin: left;
		transition: transform 1s linear;
	}
	.music-slot {
		position: relative;
		width: 100%;
		height: 9.5rem;
		margin-top: 1.25rem;
	}
	.music-slot :global(.music-dock) {
		inset-inline: 0;
		bottom: 0;
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
		.board {
			position: absolute;
			inset: 0;
			display: grid;
			grid-template-columns: minmax(0, 42rem) minmax(0, 1fr) minmax(0, 30rem);
			gap: 2rem;
			/* The bottom eighth stays the pigeons' parapet. */
			padding: 3.25rem 3.5rem 12.5vh;
		}
		.board-left {
			grid-column: 1;
			min-height: 0;
		}
		.board-right {
			grid-column: 3;
			align-items: flex-end;
			text-align: right;
			padding-top: 2.25rem;
			min-height: 0;
		}
		.headline {
			font-size: clamp(4rem, 5vw, 6rem);
		}
		.status-mark {
			width: clamp(4rem, 4.6vw, 5.5rem);
			border-width: 3px;
		}
		.sentence {
			font-size: 1.75rem;
		}
		.clock {
			font-size: 6rem;
		}
		.date {
			font-size: 1.75rem;
		}
		.updated {
			font-size: 1.0625rem;
		}
		.music-slot {
			height: 11.5rem;
			margin-top: 2rem;
		}
		.music-slot :global(.music-dock) {
			justify-content: flex-end;
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
	@media (prefers-reduced-motion: reduce) {
		.wall-stage,
		.refresh span,
		.wall :global(.music-toggle),
		.wall :global(.theme-toggle) {
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
	 * of it (the `dark` class still applies underneath). The scene paints its
	 * own OLED palette (`daylight.ts`).
	 */
	.wall[data-wall-theme='oled'] {
		--c-canvas: #000000;
		--c-canvas-deep: #000000;
		--c-surface: #050608;
		--c-surface-2: #0a0a0a;
		--c-line: rgb(255 255 255 / 0.1);
		--c-line-strong: rgb(255 255 255 / 0.18);
		--c-ink: #a9b2c2;
		--c-ink-2: #6f7889;
		--c-ink-3: #4a505d;
		--c-ghost: rgb(255 255 255 / 0.08);
		--c-signal: #1a9e94;
		--c-signal-ink: #5cbcb3;
		--c-warning: #c4574a;
		--c-warning-ink: #d98a80;
		--c-advisory: #b98a36;
		--c-advisory-ink: #cfa55c;
		--shadow-lift: none;
		--shadow-float: none;
	}
</style>
