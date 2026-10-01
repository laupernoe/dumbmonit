/**
 * The wall as a Spotify Connect speaker ("DumbMonit Wall"), through Spotify's
 * Web Playback SDK: once it is ready, the phone's Spotify app lists the wall
 * under Devices and the sound comes out of the display.
 *
 * What it takes, checked first and said plainly when missing: a secure page
 * (HTTPS or localhost), a browser with protected-media support, Spotify
 * Premium on the connected account, and one tap on the wall per page load so
 * the browser lets it make sound (`activate`, from a click handler).
 *
 * The access token comes from DumbMonit (`GET /api/music/spotify/token`); the
 * SDK asks again when it expires. The SDK script is loaded only when Spotify is
 * connected and this display has not opted out.
 */
import { getSpotifyToken } from '$lib/api/music';
import type { NowPlaying } from '$lib/api/music';
import { SDK_URL, fromSdkState, speakerSupport, type SdkState } from './spotify';

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
 * - `starting`: loading the SDK and registering the device.
 * - `ready`: listed in Spotify's devices; `activated` tells whether sound is unlocked.
 * - `error`: Spotify refused (no Premium, token refused…); `problem` says why.
 */
export type SpeakerPhase = 'off' | 'unsupported' | 'starting' | 'ready' | 'error';

let sdkLoading: Promise<SpotifyNamespace> | null = null;

/** Injects the SDK script once; resolves when it has called back. */
function loadSdk(): Promise<SpotifyNamespace> {
	if (window.Spotify) return Promise.resolve(window.Spotify);
	if (sdkLoading) return sdkLoading;
	sdkLoading = new Promise<SpotifyNamespace>((resolve, reject) => {
		window.onSpotifyWebPlaybackSDKReady = () => {
			if (window.Spotify) resolve(window.Spotify);
			else reject(new Error('The Spotify player did not start.'));
		};
		const script = document.createElement('script');
		script.src = SDK_URL;
		script.async = true;
		script.onerror = () => {
			sdkLoading = null;
			script.remove();
			reject(new Error('The Spotify player could not be downloaded (sdk.scdn.co unreachable).'));
		};
		document.head.appendChild(script);
	});
	return sdkLoading;
}

export class WallSpeaker {
	phase = $state<SpeakerPhase>('off');
	/** What is missing or went wrong, in a sentence. */
	problem = $state<string | null>(null);
	/** How to fix `problem`, when there is a known fix. */
	fix = $state<string | null>(null);
	/** Sound unlocked by a tap on this page load. */
	activated = $state(false);
	/** What this display plays, when it is the output device. */
	local = $state<NowPlaying | null>(null);
	/** When `local` was read (`Date.now()`), to move its progress bar along. */
	localAt = $state(0);

	#player: SpotifyPlayer | null = null;
	#name = '';
	#generation = 0;

	/** Ready to be picked in Spotify, but the browser still wants a tap. */
	get needsTap(): boolean {
		return this.phase === 'ready' && !this.activated;
	}

	/**
	 * Registers the device. A display that cannot (`unsupported`) or was
	 * refused (`error`) is not retried on its own: only with `retry`.
	 */
	async start(name: string, { retry = false } = {}): Promise<void> {
		if (this.phase === 'starting' || this.phase === 'ready') return;
		if (!retry && (this.phase === 'unsupported' || this.phase === 'error')) return;
		const generation = ++this.#generation;
		this.#name = name;
		this.problem = null;
		this.fix = null;
		const support = await speakerSupport(window);
		if (generation !== this.#generation) return;
		if (!support.ok) {
			this.phase = 'unsupported';
			this.problem = support.reason;
			this.fix = support.fix;
			return;
		}
		this.phase = 'starting';
		try {
			const Spotify = await loadSdk();
			if (generation !== this.#generation) return;
			const player = new Spotify.Player({
				name,
				volume: 0.8,
				getOAuthToken: (callback) => {
					getSpotifyToken()
						.then((token) => callback(token.access_token))
						.catch((cause: unknown) => this.#fail(cause instanceof Error ? cause.message : 'No Spotify token.'));
				}
			});
			this.#player = player;
			player.addListener('ready', () => {
				this.phase = 'ready';
				this.problem = null;
			});
			player.addListener('not_ready', () => {
				// Offline for a moment: the SDK reconnects by itself.
				if (this.phase === 'ready') this.phase = 'starting';
			});
			player.addListener('player_state_changed', (state: SdkState | null) => {
				this.local = fromSdkState(state, this.#name);
				this.localAt = Date.now();
			});
			player.addListener('autoplay_failed', () => {
				// The browser blocked the sound: one more tap is needed.
				this.activated = false;
			});
			player.addListener('initialization_error', ({ message }: { message: string }) =>
				this.#fail(`This browser could not start the Spotify player (${message}).`)
			);
			player.addListener('authentication_error', ({ message }: { message: string }) =>
				this.#fail(`Spotify refused the connection (${message}). An admin may need to reconnect it.`)
			);
			player.addListener('account_error', () =>
				this.#fail('Spotify Premium is required to play on this display.', 'Now playing still shows what plays on your other devices.')
			);
			player.addListener('playback_error', ({ message }: { message: string }) => {
				this.problem = `Spotify could not play this track (${message}).`;
			});
			const connected = await player.connect();
			if (!connected && generation === this.#generation) this.#fail('Spotify did not accept this display as a speaker.');
		} catch (cause) {
			if (generation === this.#generation) this.#fail(cause instanceof Error ? cause.message : 'The Spotify player did not start.');
		}
	}

	/**
	 * Unlocks the sound. Must run inside a click or key handler: browsers only
	 * let a page make sound after a gesture, and the SDK needs it ahead of a
	 * transfer from the phone.
	 */
	activate(): void {
		this.activated = true;
		void this.#player?.activateElement().catch(() => (this.activated = false));
	}

	stop(): void {
		this.#generation++;
		this.#player?.disconnect();
		this.#player = null;
		this.phase = 'off';
		this.local = null;
		this.problem = null;
		this.fix = null;
	}

	#fail(problem: string, fix: string | null = null): void {
		this.#player?.disconnect();
		this.#player = null;
		this.phase = 'error';
		this.local = null;
		this.problem = problem;
		this.fix = fix;
	}
}
