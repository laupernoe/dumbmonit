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
	 * and what is playing. The widgets (weather, headlines, vulnerabilities,
	 * coins, stocks, rates, AI usage) sit where the admin put them — a thin bar
	 * at the top or the bottom, or the right column under the clock — all at
	 * once, small and still (`WallZone`): nothing turns, nothing scrolls.
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
	import { m } from '#lib/paraglide/messages.js';
	import { untrack } from 'svelte';
	import { getLocale } from '#lib/paraglide/runtime.js';
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
	import { viewport } from '#lib/stores/viewport.svelte.js';
	import { theme, type ThemePreference } from '#lib/stores/theme.svelte.js';
	import { palette } from '#lib/stores/palette.svelte.js';
	import { Button, Plate, Skeleton, ErrorNotice } from '#lib/ui/index.js';
	import { readSky, skyCondition } from '#lib/components/overview/sky.js';
	import { useSceneLive } from '#lib/scene/live.svelte.js';
	import { toWallMood } from '#lib/scene/mood.js';
	import type { SceneMood, SceneTheme } from '#lib/components/wall/paris/daylight.js';
	import WallProblems from '#lib/components/wall/WallProblems.svelte';
	import WallDevices from '#lib/components/wall/WallDevices.svelte';
	import {
		WALL_CITIES,
		readWallCity,
		writeWallCity,
		parseForcedWallCity,
		resolveWallCity,
		type WallCityChoice
	} from '#lib/components/wall/wallCity.js';
	import WallThemeControl from '#lib/components/wall/WallThemeControl.svelte';
	import WallStorm from '#lib/components/wall/WallStorm.svelte';
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
	import { getWallNews, type WallNews } from '#lib/api/wallNews.js';
	import WallZone from '#lib/components/wall/WallZone.svelte';
	import { resolveLayout, type WallWidgetId } from '#lib/wall/layout.js';
	import { getWallWidgets } from '#lib/api/wallWidgets.js';
	import type { WallWidgets as WallWidgetsData } from '#lib/api/types.js';
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

	// --- News -------------------------------------------------------------------

	/**
	 * Headlines, text only, off unless an admin turned them on in Settings. The server reads the feeds and caches them for five
	 * minutes; the wall asks as often, only while visible, and keeps the last
	 * answer when a read fails.
	 */
	const NEWS_MS = 5 * 60_000;
	let news = $state<WallNews | null>(null);
	const headlines = $derived(news && news.ticker ? news.items : []);

	$effect(() => {
		const controller = new AbortController();
		let timer: ReturnType<typeof setTimeout> | undefined;
		const poll = async () => {
			if (document.visibilityState === 'visible') {
				try {
					news = await getWallNews(controller.signal);
				} catch (cause) {
					if (cause instanceof DOMException && cause.name === 'AbortError') return;
				}
			}
			if (!controller.signal.aborted) timer = setTimeout(poll, NEWS_MS);
		};
		void poll();
		return () => {
			controller.abort();
			clearTimeout(timer);
		};
	});

	// --- Widgets ----------------------------------------------------------------

	/**
	 * Coins, rates, stocks, CVEs and AI usage as panels of the right rail, and
	 * the weather now under the clock; off unless an admin turned them on.
	 * The server reads and caches every source (a minute for coins, an hour for
	 * rates); the wall asks every minute while visible and keeps the last answer
	 * when a read fails. An older server without the route: no more asking.
	 */
	const WIDGETS_MS = 60_000;
	let widgets = $state<WallWidgetsData | null>(null);
	/** Where each widget goes (Settings → Wall widgets → Layout); only those with something to show. */
	const layout = $derived(resolveLayout(widgets?.layout));
	function shows(id: WallWidgetId): boolean {
		if (id === 'news') return headlines.length > 0;
		if (id === 'text') return !!widgets?.text;
		return !!widgets?.[id]?.data;
	}
	const topIds = $derived(layout.top.filter(shows));
	const bottomIds = $derived(layout.bottom.filter(shows));
	const rightIds = $derived(layout.right.filter(shows));

	$effect(() => {
		const controller = new AbortController();
		let timer: ReturnType<typeof setTimeout> | undefined;
		const poll = async () => {
			if (document.visibilityState === 'visible') {
				try {
					widgets = await getWallWidgets(controller.signal);
				} catch (cause) {
					if (cause instanceof DOMException && cause.name === 'AbortError') return;
					if (cause instanceof ApiError && cause.missing) return;
				}
			}
			if (!controller.signal.aborted) timer = setTimeout(poll, WIDGETS_MS);
		};
		void poll();
		return () => {
			controller.abort();
			clearTimeout(timer);
		};
	});

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
	let wallCity = $state<WallCityChoice>(readWallCity());
	const activeCity = $derived(
		parseForcedWallCity(page.url.searchParams.get('city')) ?? resolveWallCity(wallCity)
	);
	/** Scenes with no sky of their own to storm in: indoors, under water, in space. */
	const STORMLESS = new Set<string>(['orbit', 'aquarium', 'hamsters', 'cats', 'arcade', 'raclette', 'chalet', 'mars', 'desert']);
	/** Scenes that are night whatever the hour or theme. */
	const NIGHT_SCENES = new Set<string>(['orbit']);
	/** Scenes whose picture goes dark under a mood: the Day palette's ink would sink into it. */
	function darkUnder(city: string, tone: string): boolean {
		if (city === 'aquarium') return tone !== 'calm';
		if (city === 'forest' || city === 'mars') return tone === 'storm';
		return false;
	}
	const CityScene = $derived(WALL_CITIES.find((c) => c.value === activeCity)!.scene);
	function setWallCity(choice: WallCityChoice) {
		wallCity = choice;
		writeWallCity(choice);
	}
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
	// A scene that is always night (the station in orbit) never rides the Day
	// palette: dark ink on black space would be unreadable.
	$effect(() => {
		const forcedNight = nightScene && (effectiveWallTheme === 'auto' || effectiveWallTheme === 'light');
		if (effectiveWallTheme === 'auto' && !forcedNight) return;
		const base: ThemePreference = forcedNight
			? 'dark'
			: effectiveWallTheme === 'oled'
				? 'dark'
				: (effectiveWallTheme as ThemePreference);
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
	// A wall on a phone is a glance, not a room screen: the scene holds still.
	const sceneStill = $derived(hidden || viewport.phone);
	$effect(() => {
		const onVisibility = () => (hidden = document.visibilityState !== 'visible');
		document.addEventListener('visibilitychange', onVisibility);
		return () => document.removeEventListener('visibilitychange', onVisibility);
	});

	const sceneTheme = $derived<SceneTheme>(oledActive ? 'oled' : theme.resolved);
	const live = useSceneLive();
	const mood = $derived<SceneMood>(
		live.tone
			? toWallMood(live.tone)
			: condition === 'storm'
				? 'storm'
				: condition === 'cloudy' || condition === 'overcast'
					? 'clouded'
					: 'calm'
	);

	const nightScene = $derived(NIGHT_SCENES.has(activeCity) || darkUnder(activeCity, mood));

	// Gestures: a device going down or coming back, an alert resolving. The first
	// load is only a baseline; a refetch from scratch (loading) starts over.
	$effect(() => {
		if (loading) {
			live.reset();
			return;
		}
		const down = new Set<string>();
		const issues = new Set<string>();
		for (const row of sky.needsYou) {
			if (row.kind === 'device' && row.tone === 'warning') down.add(row.key);
			else if (row.kind === 'alert' && row.alert.effective_phase === 'firing' && row.alert.rule_uid.startsWith('host_down')) down.add(row.key);
			if (row.tone === 'warning' || row.tone === 'advisory') issues.add(row.key);
		}
		live.feed({ down, issues });
	});

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
		if (sky.counts.devices === 0) return { tone: 'ghost', text: m.wall_status_empty() };
		if (sky.attention > 0) {
			const severe = sky.counts.warnings > 0 || sky.counts.unreachable > 0;
			return {
				tone: severe ? 'warning' : 'advisory',
				text: sky.attention === 1 ? m.wall_status_problem_one() : m.wall_status_problems({ count: sky.attention })
			};
		}
		if (sky.counts.reporting === 0) return { tone: 'ghost', text: m.wall_status_waiting() };
		return { tone: 'signal', text: m.wall_status_ok() };
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
		now.toLocaleTimeString(getLocale(), { hour: '2-digit', minute: '2-digit', hour12: false })
	);
	const dateLabel = $derived(
		now.toLocaleDateString(getLocale(), { weekday: 'long', day: 'numeric', month: 'long' })
	);
	const updatedLabel = $derived.by(() => {
		if (!lastChecked) return m.wall_updated_waiting();
		const seconds = Math.max(0, Math.round((now.getTime() - lastChecked.getTime()) / 1000));
		return seconds < 60 ? m.wall_updated_seconds({ seconds }) : m.wall_updated_relative({ when: formatRelative(lastChecked) });
	});
	const progress = $derived(
		lastChecked ? Math.min(1, (now.getTime() - lastChecked.getTime()) / REFRESH_MS) : 0
	);

	const firstLoad = $derived(loading && targets.length === 0);

	/** Nothing but the clock on the right: the rail slims down and the problems take the room. */
	const leanRail = $derived(rightIds.length === 0);

	/**
	 * From 1024 px the board is a fixed-height column: the problems get what the
	 * devices below do not need. One row of chips is always kept, and the
	 * devices whose problem did not fit come first in it. Below, the page
	 * scrolls and nothing is capped.
	 */
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

	/**
	 * The scene's props (the pigeons and their sign, the station clock, the
	 * split-flap board, a chalkboard) never sit under the board's words nor
	 * past the screen's edge: each one that would (`data-wall-prop` against
	 * every `data-wall-info` box) is hidden. Checked on resize and every few
	 * seconds; a prop hidden stays hidden until the board itself changes, so
	 * a pigeon strolling along the parapet does not blink in and out.
	 */
	let stage = $state<HTMLElement | null>(null);
	/** What a prop actually paints: an `<svg>` box is its whole viewBox, its shapes are smaller. */
	function paintedRect(el: Element): { left: number; top: number; right: number; bottom: number; width: number; height: number } {
		if (!(el instanceof SVGSVGElement)) return el.getBoundingClientRect();
		let left = Infinity;
		let top = Infinity;
		let right = -Infinity;
		let bottom = -Infinity;
		for (const child of el.children) {
			const r = child.getBoundingClientRect();
			if (r.width === 0 && r.height === 0) continue;
			left = Math.min(left, r.left);
			top = Math.min(top, r.top);
			right = Math.max(right, r.right);
			bottom = Math.max(bottom, r.bottom);
		}
		if (left === Infinity) return el.getBoundingClientRect();
		return { left, top, right, bottom, width: right - left, height: bottom - top };
	}
	$effect(() => {
		const root = stage;
		if (!root) return;
		let layout = '';
		const fit = () => {
			const screen = root.getBoundingClientRect();
			// On a 4K panel the board is zoomed; browsers before the standard CSS
			// zoom give its boxes unzoomed, so they are scaled back to the screen
			// (the board covers the whole stage: its width tells the factor).
			const board = root.querySelector(':scope > .board')?.getBoundingClientRect();
			const k = board && board.width > 0 ? screen.width / board.width : 1;
			const infos = Array.from(root.querySelectorAll('[data-wall-info]'))
				.map((el) => {
					const r = el.getBoundingClientRect();
					return { left: r.left * k, top: r.top * k, right: r.right * k, bottom: r.bottom * k, width: r.width * k, height: r.height * k };
				})
				.filter((rect) => rect.width > 0 && rect.height > 0);
			const next = infos.map((r) => `${Math.round(r.left)},${Math.round(r.top)},${Math.round(r.width)},${Math.round(r.height)}`).join('|') + `@${Math.round(screen.width)}x${Math.round(screen.height)}`;
			const sticky = next === layout;
			layout = next;
			const gap = 4;
			for (const prop of root.querySelectorAll('[data-wall-prop]')) {
				const r = paintedRect(prop);
				if (r.width === 0 && r.height === 0) continue;
				const cut = r.left < screen.left - 1 || r.right > screen.right + 1 || r.top < screen.top - 1 || r.bottom > screen.bottom + 1;
				const covered = infos.some(
					(i) => r.left < i.right + gap && r.right > i.left - gap && r.top < i.bottom + gap && r.bottom > i.top - gap
				);
				prop.toggleAttribute('data-wall-prop-off', cut || covered || (sticky && prop.hasAttribute('data-wall-prop-off')));
			}
		};
		// After the first paint, then on every resize and every few seconds.
		const first = requestAnimationFrame(fit);
		const timer = setInterval(fit, 2500);
		const observer = new ResizeObserver(() => fit());
		observer.observe(root);
		return () => {
			cancelAnimationFrame(first);
			clearInterval(timer);
			observer.disconnect();
		};
	});
</script>

<svelte:head><title>{m.wall_page_title()}</title></svelte:head>
<svelte:window onkeydown={onKeydown} />

<div class="wall fixed inset-0 z-[60] box-border bg-canvas pt-[var(--safe-top)] pr-[var(--safe-right)] pb-[var(--safe-bottom)] pl-[var(--safe-left)] text-ink" data-wall-theme={oledActive ? 'oled' : undefined}>
	<!--
		Everything lives in this stage. On OLED it is nudged a few pixels every
		few minutes (burn-in care), scene included: nothing stays lit in one spot.
	-->
	<div
		class="wall-stage relative h-full"
		class:lean={leanRail}
		bind:this={stage}
		style:transform={oledActive ? `translate(${oledShift[0]}px, ${oledShift[1]}px)` : undefined}
	>
		<div class="scene-box">
			<CityScene
				theme={sceneTheme}
				{now}
				{mood}
				pulse={live.pulse}
				problems={firstLoad ? 0 : sky.attention}
				{startle}
				paused={sceneStill}
				focus={1360}
			/>
			{#if mood === 'storm' && !STORMLESS.has(activeCity)}
				<WallStorm theme={sceneTheme} paused={sceneStill} />
			{/if}
		</div>

		<div class="board">
			{#if error && targets.length === 0}
				<div class="board-left">
					<div class="max-w-xl rounded-[var(--radius-card)] bg-surface p-2 shadow-float">
						<ErrorNotice
							{error}
							title={m.wall_load_error()}
							onretry={() => {
								loading = true;
								void load();
							}}
						/>
					</div>
					<div class="mt-4">
						<Button variant="secondary" onclick={exit}>{m.wall_back()}</Button>
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
					<header class="flex items-center gap-4 lg:gap-6" data-wall-info>
						<span class="status-mark status-mark--{status.tone}" aria-hidden="true">
							<StatusIcon class="size-1/2" strokeWidth={2.4} />
						</span>
						<h1 class="headline display min-w-0" aria-live="polite">{status.text}</h1>
					</header>
					<p class="sentence mt-3 lg:mt-4" data-wall-info>{sky.sentence}</p>
					<div class="plates mt-4 flex flex-wrap items-center gap-2.5" data-wall-info>
						{#each sky.plates as plate (plate.label)}
							<Plate tone={plate.tone} label={plate.label} bare={plate.bare} size="md" />
						{/each}
					</div>

					<div class="board-list mt-7 flex min-h-0 flex-1 flex-col gap-4 lg:mt-9" bind:clientHeight={leftRoom}>
						<WallProblems {sky} {now} room={problemsRoom} bind:listed={problemListed} />
						<WallDevices {targets} {probes} {sky} skip={problemListed} />
					</div>
				</div>
			{/if}

			<div class="board-right">
				<time class="clock display tnum" datetime={now.toISOString()} data-wall-info>{clock}</time>
				<p class="date" data-wall-info>{dateLabel}</p>
				<p class="updated tnum" aria-live="off" data-wall-info>
					<span class="refresh" aria-hidden="true"><span style:transform="scaleX({progress})"></span></span>
					{updatedLabel}
				</p>
				{#if error && targets.length > 0}
					<p class="mt-1 text-base text-warning-ink" role="status">{m.wall_refresh_failed()}</p>
				{/if}

				{#if musicVisible}
					<div class="music-slot" data-wall-info>
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

				{#if rightIds.length > 0}
					<WallZone ids={rightIds} variant="column" {widgets} news={headlines} {now} fit={wide} class="right-zone" />
				{/if}
			</div>

			{#if topIds.length > 0}
				<div class="wall-bar wall-bar--top" data-wall-info>
					<WallZone ids={topIds} variant="bar" {widgets} news={headlines} {now} fit={wide} />
				</div>
			{/if}
			{#if bottomIds.length > 0}
				<div class="wall-bar wall-bar--bottom" data-wall-info>
					<WallZone ids={bottomIds} variant="bar" {widgets} news={headlines} {now} fit={wide} />
				</div>
			{/if}
		</div>

		<!-- Tools, top right; Music and Theme show on hover, focus or touch. -->
		<div data-wall-info class="wall-tools absolute top-3 right-3 z-10 flex max-w-[calc(100vw-1.5rem)] flex-wrap items-center justify-end rounded-xl bg-surface/85 p-1 shadow-lift lg:top-4 lg:right-4">
			<WallThemeControl
				value={wallTheme}
				forced={forcedWallTheme}
				{nightDim}
				bind:open={themeOpen}
				onchange={setWallTheme}
				onnightdim={setNightDim}
				city={wallCity}
				oncity={setWallCity}
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
			<Button variant="ghost" size="sm" onclick={exit} aria-label={m.wall_exit_aria()} class="min-h-10 lg:min-h-0">
				<X class="size-4" aria-hidden="true" />
				{m.wall_exit()}
				<kbd class="ml-1 max-sm:hidden rounded-md border border-line bg-surface-2 px-1.5 text-[0.6875rem] text-ink-3">esc</kbd>
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
		/* The scene and its storm stay under the board, whatever z-index they use. */
		isolation: isolate;
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
	/* Fills up to the next refresh, one step a second: a quiet second hand for the data. */
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
	}
	/* The widgets: a quieter voice than the monitors (see `WallWidget`). */
	.board-right :global(.right-zone) {
		width: 100%;
		margin-top: 1.25rem;
		--w-size: 1rem;
	}
	.wall-bar {
		min-width: 0;
		padding: 0.5rem 1rem;
		border-radius: 12px;
		background: color-mix(in srgb, var(--c-surface) 72%, transparent);
		--w-size: 0.9375rem;
	}
	.wall-bar--top {
		order: -1;
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
			grid-template-columns: minmax(0, 38rem) minmax(0, 1fr) minmax(0, 30rem);
			/* A bar on top, the board, a bar at the bottom: each bar only when it has widgets. */
			grid-template-rows: auto minmax(0, 1fr) auto;
			gap: 0 2rem;
			/* Top to bottom: the scene's props give way to the words, never the reverse. */
			padding: 3.25rem 3.5rem 2.5rem;
		}
		.board-left {
			grid-column: 1;
			grid-row: 2;
			min-height: 0;
		}
		.wall-bar--top {
			grid-column: 1 / 3;
			grid-row: 1;
			margin-bottom: 1.25rem;
		}
		.wall-bar--bottom {
			grid-column: 1 / -1;
			grid-row: 3;
			margin-top: 1.25rem;
		}
		/* Nothing but the clock on the right: the rail slims down, the problems get the room. */
		.lean .board {
			grid-template-columns: minmax(0, 46rem) minmax(0, 1fr) minmax(0, 22rem);
		}
		.board-right {
			grid-column: 3;
			grid-row: 1 / 3;
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
		/* The right column's widgets take what the clock and the music leave, and fit in it. */
		.board-right :global(.right-zone) {
			flex: 1 1 0;
			min-height: 0;
		}
		.music-slot :global(.music-dock) {
			justify-content: flex-end;
		}
	}
	/* 720p to 900p: a narrower board leaves the middle of the scene to its monument. */
	@media (min-width: 1024px) and (max-width: 1599px) {
		.board {
			grid-template-columns: minmax(0, 27rem) minmax(0, 1fr) minmax(0, 22rem);
			gap: 1.5rem;
			padding-inline: 2.5rem;
		}
		.lean .board {
			grid-template-columns: minmax(0, 36rem) minmax(0, 1fr) minmax(0, 18rem);
		}
		.clock {
			font-size: 4.5rem;
		}
	}
	/* A 4K panel read from the same sofa: the board grows with the screen. */
	@media (min-width: 2400px) {
		.wall-stage {
			--zoom: 1.33;
		}
		.board,
		.wall-tools {
			zoom: 1.33;
		}
	}
	@media (min-width: 3200px) {
		.wall-stage {
			--zoom: 2;
		}
		.board,
		.wall-tools {
			zoom: 2;
		}
	}
	/* A short screen (720p): the rail trims its spacing so the panels keep their room. */
	@media (min-width: 1024px) and (max-height: 940px) {
		.updated {
			display: none;
		}
		.date {
			font-size: 1.25rem;
			margin-top: 0.125rem;
		}
		.updated {
			margin-top: 0.25rem;
		}
		.board-right :global(.right-zone) {
			margin-top: 0.75rem;
		}
		/* The headline gives the problems their room: smaller type, tighter spacing. */
		.board {
			padding-top: 2.25rem;
		}
		.headline {
			font-size: clamp(3rem, 4vw, 4rem);
		}
		.status-mark {
			width: clamp(3rem, 3.6vw, 4rem);
		}
		.sentence {
			font-size: 1.25rem;
			margin-top: 0.5rem;
		}
		.plates {
			margin-top: 0.625rem;
			gap: 0.375rem;
		}
		.board-list {
			margin-top: 1.25rem;
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
		.wall :global(.music-toggle),
		.wall :global(.theme-toggle) {
			transition: none;
		}
	}

	/* A prop of the scene that would sit under the board's words, or past the screen's edge. */
	.wall :global([data-wall-prop-off]) {
		visibility: hidden;
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
