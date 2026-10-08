/**
 * The wall as a Spotify Connect speaker ("DumbMonit Wall", or the name chosen
 * in Settings), through Spotify's Web Playback SDK: once it is ready, the
 * Spotify app of any phone signed in to the same account lists the wall under
 * Devices and the sound comes out of the display.
 *
 * What it takes, checked first and said plainly when missing: a secure page
 * (HTTPS or localhost), a browser whose DRM works (Widevine, PlayReady or
 * FairPlay), Spotify Premium on the connected account, and one gesture on the
 * wall per page load so the browser lets it make sound (`activate`, from any
 * click or key press — unless the browser already allows sound).
 *
 * A wall stays on for days, so the player looks after itself: the SDK asks
 * DumbMonit for a fresh token when its own expires; a device that drops out
 * (`not_ready`, network loss) is rebuilt if the SDK does not come back by
 * itself; a passing failure is retried with a growing delay; and the wall
 * reports where it stands to the server every minute, which answers whether
 * Spotify still lists it — a wall that Spotify forgot rebuilds its device.
 * Only what waiting cannot fix (no DRM, no Premium) stops for good, until
 * "Try again".
 */
import { m } from '#lib/paraglide/messages.js';
import { sdkFailureText, speakerSupportText } from './messages';
import { ApiError } from '#lib/api/client.js';
import { getSpotifyToken, playOnSpeaker, reportSpeaker } from '#lib/api/music.js';
import type { NowPlaying, SpeakerReportBody } from '#lib/api/music.js';
import {
	SDK_URL,
	autoplayAllowed,
	browserLabel,
	fromSdkState,
	retryDelay,
	sdkFailure,
	speakerSupport,
	type AutoplayEnv,
	type SdkFailure,
	type SdkState
} from './spotify';

interface SpotifyPlayer {
	connect(): Promise<boolean>;
	disconnect(): void;
	activateElement(): Promise<void>;
	addListener(event: string, callback: (payload: never) => void): boolean;
}

interface SpotifyNamespace {
	Player: new (options: {
		name: string;
		getOAuthToken: (callback: (token: string) => void) => void;
		volume?: number;
	}) => SpotifyPlayer;
}

declare global {
	interface Window {
		Spotify?: SpotifyNamespace;
		onSpotifyWebPlaybackSDKReady?: () => void;
	}
}

/**
 * - `off`: not started (Spotify not connected, or turned off on this display).
 * - `unsupported`: this browser or page cannot play Spotify (`problem` says why).
 * - `starting`: loading the SDK and registering the device (or reconnecting).
 * - `ready`: listed in Spotify's devices; `activated` tells whether sound is unlocked.
 * - `error`: Spotify refused (no Premium, token refused…); `problem` says why,
 *   `retryAt` when it tries again by itself (null: it will not).
 */
export type SpeakerPhase = 'off' | 'unsupported' | 'starting' | 'ready' | 'error';

/** A device that went `not_ready` and did not come back by then is rebuilt. */
const NOT_READY_GRACE_MS = 30_000;
/** How often the wall tells the server where its speaker stands. */
const REPORT_MS = 60_000;
/** Reports in a row in which Spotify does not list a "ready" device before it is rebuilt. */
const UNLISTED_LIMIT = 2;
/** Where this display keeps its random id, so Settings can tell walls apart. */
const DISPLAY_KEY = 'dumbmonit-wall-display';

let sdkLoading: Promise<SpotifyNamespace> | null = null;

/** Injects the SDK script once; resolves when it has called back. */
function loadSdk(): Promise<SpotifyNamespace> {
	if (window.Spotify) return Promise.resolve(window.Spotify);
	if (sdkLoading) return sdkLoading;
	sdkLoading = new Promise<SpotifyNamespace>((resolve, reject) => {
		window.onSpotifyWebPlaybackSDKReady = () => {
			if (window.Spotify) resolve(window.Spotify);
			else reject(new Error(m.misc_speaker_not_started()));
		};
		const script = document.createElement('script');
		script.src = SDK_URL;
		script.async = true;
		script.onerror = () => {
			sdkLoading = null;
			script.remove();
			reject(new Error(m.misc_speaker_download_failed()));
		};
		document.head.appendChild(script);
	});
	return sdkLoading;
}

function displayId(): string {
	try {
		const stored = localStorage.getItem(DISPLAY_KEY);
		if (stored && /^[A-Za-z0-9-]{8,64}$/.test(stored)) return stored;
		const fresh = `wall-${crypto.randomUUID()}`;
		localStorage.setItem(DISPLAY_KEY, fresh);
		return fresh;
	} catch {
		// Storage blocked: a new id per page load, still enough to report.
		return `wall-${Math.random().toString(36).slice(2, 12)}`;
	}
}

export class WallSpeaker {
	phase = $state<SpeakerPhase>('off');
	/** What is missing or went wrong, in a sentence. */
	problem = $state<string | null>(null);
	/** How to fix `problem`, when there is a known fix. */
	fix = $state<string | null>(null);
	/** Sound unlocked by a gesture on this page load (or allowed by the browser). */
	activated = $state(false);
	/** The Spotify Connect device id, once ready. */
	deviceId = $state<string | null>(null);
	/** Spotify lists this device among the account's (`null`: not checked yet). */
	listed = $state<boolean | null>(null);
	/** `Date.now()` of the next automatic retry, when one is planned. */
	retryAt = $state<number | null>(null);
	/** What this display plays, when it is the output device. */
	local = $state<NowPlaying | null>(null);
	/** When `local` was read (`Date.now()`), to move its progress bar along. */
	localAt = $state(0);

	#player: SpotifyPlayer | null = null;
	#name = '';
	#generation = 0;
	#attempt = 0;
	#premium: boolean | null = null;
	#retryTimer: ReturnType<typeof setTimeout> | undefined;
	#notReadyTimer: ReturnType<typeof setTimeout> | undefined;
	#reportTimer: ReturnType<typeof setInterval> | undefined;
	#unlisted = 0;
	#display = '';
	#reporting = true;
	#onOnline = () => {
		if (this.phase === 'error' && this.retryAt !== null) void this.start(this.#name, { retry: true });
		else if (this.phase === 'starting' && this.#player) this.#rebuild();
	};

	/** Ready to be picked in Spotify, but the browser still wants a gesture. */
	get needsTap(): boolean {
		return this.phase === 'ready' && !this.activated;
	}

	/** The name this display announces. */
	get name(): string {
		return this.#name;
	}

	/**
	 * Registers the device under `name`. A display that cannot (`unsupported`)
	 * or was refused for good (`error` without a retry planned) is not retried
	 * on its own: only with `retry`. A new name rebuilds the device.
	 */
	async start(name: string, { retry = false } = {}): Promise<void> {
		const renamed = this.#name !== '' && name !== this.#name;
		if (!renamed && (this.phase === 'starting' || this.phase === 'ready')) return;
		if (!renamed && !retry && (this.phase === 'unsupported' || this.phase === 'error')) return;
		this.#teardown();
		if (retry && this.retryAt === null) this.#attempt = 0;
		const generation = ++this.#generation;
		this.#name = name;
		this.problem = null;
		this.fix = null;
		this.retryAt = null;
		this.#startReports();
		window.addEventListener('online', this.#onOnline);
		const support = await speakerSupport(window);
		if (generation !== this.#generation) return;
		if (!support.ok) {
			this.phase = 'unsupported';
			const text = speakerSupportText(support.code);
			this.problem = text.reason;
			this.fix = text.fix;
			this.#report();
			return;
		}
		this.phase = 'starting';
		this.#report();
		try {
			const Spotify = await loadSdk();
			if (generation !== this.#generation) return;
			const player = new Spotify.Player({
				name,
				volume: 0.8,
				getOAuthToken: (callback) => {
					getSpotifyToken()
						.then((token) => callback(token.access_token))
						.catch((cause: unknown) => {
							if (generation !== this.#generation) return;
							// 409: nothing (or nothing valid) connected; the wall's poll says it too.
							const permanent = cause instanceof ApiError && cause.status === 409;
							this.#fail(
								{
									problem: cause instanceof Error ? cause.message : m.misc_speaker_no_token(),
									fix: permanent ? m.misc_speaker_fix_reconnect() : null,
									permanent
								},
								generation
							);
						});
				}
			});
			this.#player = player;
			player.addListener('ready', ({ device_id }: { device_id: string }) => {
				if (generation !== this.#generation) return;
				clearTimeout(this.#notReadyTimer);
				this.phase = 'ready';
				this.problem = null;
				this.fix = null;
				this.deviceId = device_id;
				this.listed = null;
				this.#unlisted = 0;
				this.#attempt = 0;
				this.#premium = true;
				// A browser that already allows sound needs no tap.
				if (!this.activated && autoplayAllowed(navigator as unknown as AutoplayEnv)) this.activated = true;
				this.#report();
			});
			player.addListener('not_ready', () => {
				if (generation !== this.#generation) return;
				// Offline for a moment: the SDK usually reconnects by itself; if
				// it has not within the grace period, the device is rebuilt.
				this.phase = 'starting';
				this.listed = null;
				clearTimeout(this.#notReadyTimer);
				this.#notReadyTimer = setTimeout(() => {
					if (generation === this.#generation && this.phase === 'starting') this.#rebuild();
				}, NOT_READY_GRACE_MS);
				this.#report();
			});
			player.addListener('player_state_changed', (state: SdkState | null) => {
				if (generation !== this.#generation) return;
				this.local = fromSdkState(state, this.#name);
				this.localAt = Date.now();
			});
			player.addListener('autoplay_failed', () => {
				// The browser blocked the sound: one more gesture is needed.
				this.activated = false;
				this.#report();
			});
			player.addListener('initialization_error', ({ message }: { message: string }) =>
				this.#fail(sdkFailureText(sdkFailure('initialization_error', message)), generation)
			);
			player.addListener('authentication_error', ({ message }: { message: string }) =>
				this.#fail(sdkFailureText(sdkFailure('authentication_error', message)), generation)
			);
			player.addListener('account_error', ({ message }: { message: string }) =>
				this.#fail(sdkFailureText(sdkFailure('account_error', message)), generation)
			);
			player.addListener('playback_error', ({ message }: { message: string }) => {
				this.problem = m.misc_speaker_playback_error({ message });
			});
			const connected = await player.connect();
			if (!connected && generation === this.#generation) {
				this.#fail({ problem: m.misc_speaker_not_accepted(), fix: null, permanent: false }, generation);
			}
		} catch (cause) {
			if (generation === this.#generation) {
				this.#fail(
					{ problem: cause instanceof Error ? cause.message : m.misc_speaker_not_started(), fix: null, permanent: false },
					generation
				);
			}
		}
	}

	/**
	 * Unlocks the sound. Must run inside a click or key handler: browsers only
	 * let a page make sound after a gesture, and the SDK needs it ahead of a
	 * transfer from the phone.
	 */
	activate(): void {
		if (this.activated || !this.#player) return;
		this.activated = true;
		void this.#player
			.activateElement()
			.then(() => this.#report())
			.catch(() => (this.activated = false));
	}

	/** "Play here": unlocks the sound (a gesture) and moves the account's playback to this display. */
	async playHere(): Promise<void> {
		this.activate();
		if (!this.deviceId) throw new Error(m.misc_speaker_not_ready());
		await playOnSpeaker(this.deviceId);
	}

	stop(): void {
		this.#generation++;
		this.#teardown();
		clearInterval(this.#reportTimer);
		this.#reportTimer = undefined;
		window.removeEventListener('online', this.#onOnline);
		if (this.phase !== 'off') {
			this.phase = 'off';
			this.#report();
		}
		this.local = null;
		this.problem = null;
		this.fix = null;
		this.retryAt = null;
	}

	/** Disconnects the player and cancels timers, without changing the phase. */
	#teardown(): void {
		clearTimeout(this.#retryTimer);
		clearTimeout(this.#notReadyTimer);
		this.#player?.disconnect();
		this.#player = null;
		this.deviceId = null;
		this.listed = null;
	}

	/** Throws the device away and registers a new one, under the same name. */
	#rebuild(): void {
		const name = this.#name;
		this.#teardown();
		this.phase = 'off';
		void this.start(name, { retry: true });
	}

	#fail(failure: SdkFailure, generation: number): void {
		if (generation !== this.#generation) return;
		this.#generation++;
		this.#teardown();
		this.phase = 'error';
		this.local = null;
		this.problem = failure.problem;
		this.fix = failure.fix;
		if (failure.noPremium) this.#premium = false;
		if (failure.permanent) {
			this.retryAt = null;
		} else {
			const wait = retryDelay(this.#attempt++);
			this.retryAt = Date.now() + wait;
			const name = this.#name;
			this.#retryTimer = setTimeout(() => void this.start(name, { retry: true }), wait);
		}
		this.#report();
	}

	#startReports(): void {
		if (this.#reportTimer !== undefined) return;
		this.#display ||= displayId();
		this.#reportTimer = setInterval(() => this.#report(), REPORT_MS);
	}

	/** Tells the server where this speaker stands; rebuilds a device Spotify forgot. */
	#report(): void {
		if (!this.#reporting || !this.#display) return;
		const body: SpeakerReportBody = {
			display: this.#display,
			name: this.#name,
			phase: this.phase,
			activated: this.activated,
			device_id: this.deviceId,
			problem: this.problem,
			browser: browserLabel(navigator.userAgent),
			premium: this.#premium
		};
		const generation = this.#generation;
		reportSpeaker(body)
			.then(({ listed }) => {
				if (generation !== this.#generation || this.phase !== 'ready') return;
				this.listed = listed;
				this.#unlisted = listed === false ? this.#unlisted + 1 : 0;
				if (this.#unlisted >= UNLISTED_LIMIT) this.#rebuild();
			})
			.catch((cause: unknown) => {
				// An older server without the route: stop reporting. Anything else: next time.
				if (cause instanceof ApiError && cause.missing) this.#reporting = false;
			});
	}
}
