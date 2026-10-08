/**
 * Spotify on the wall, the pure parts: can this display be a speaker, what is
 * playing (from the wall's own player or from the account, wherever it plays),
 * and how to say it from across the room.
 *
 * No runtime imports: `npm test` (tests/wall-spotify.test.mjs) runs it as is.
 */
import type { NowPlaying } from '../api/music';

/** The SDK script, the only third-party script the CSP lets in (`script-src`). */
export const SDK_URL = 'https://sdk.scdn.co/spotify-player.js';

/** The only origin a cover may come from (`img-src`). */
export const COVER_ORIGIN = 'https://i.scdn.co';

/**
 * A cover on `i.scdn.co`, or null. Spotify also serves its images from
 * `image-cdn-*.spotifycdn.com` under the same id: those are brought back to
 * `i.scdn.co` rather than opening the CSP to a wildcard (same rule as the
 * server's `cover_url`).
 */
export function coverUrl(url: string | null | undefined): string | null {
	if (typeof url !== 'string') return null;
	let parsed: URL;
	try {
		parsed = new URL(url);
	} catch {
		return null;
	}
	const host = parsed.hostname;
	const fromCdn = host === 'i.scdn.co' || (host.startsWith('image-cdn-') && host.endsWith('.spotifycdn.com'));
	const id = parsed.pathname.startsWith('/image/') ? parsed.pathname.slice('/image/'.length) : '';
	const clean = parsed.protocol === 'https:' && fromCdn && !parsed.port && !parsed.search && /^[A-Za-z0-9]{1,64}$/.test(id);
	return clean ? `${COVER_ORIGIN}/image/${id}` : null;
}

/** How long a paused track stays on the wall before the card goes away. */
export const PAUSE_GRACE_MS = 30_000;

/** `code` picks the translated text (`wall/messages.ts`); `reason` and `fix` are the English fallback. */
export type SpeakerSupportCode = 'insecure' | 'no_eme' | 'no_drm';

export type SpeakerSupport = { ok: true } | { ok: false; code: SpeakerSupportCode; reason: string; fix: string };

/** The few things `speakerSupport` reads, so it can be tested without a browser. */
export interface SpeakerEnv {
	isSecureContext: boolean;
	navigator: {
		requestMediaKeySystemAccess?: (keySystem: string, configs: MediaKeySystemConfiguration[]) => Promise<unknown>;
	};
}

/** Audio the way Spotify streams it to browsers; any one supported is enough. */
const AUDIO_CONFIGS: MediaKeySystemConfiguration[] = [
	{
		initDataTypes: ['cenc'],
		audioCapabilities: [
			{ contentType: 'audio/mp4; codecs="mp4a.40.2"' },
			{ contentType: 'audio/webm; codecs="vorbis"' },
			{ contentType: 'audio/webm; codecs="opus"' }
		]
	}
];

/**
 * Widevine (Chrome, Edge, Firefox, Android), PlayReady (Edge on Windows),
 * then FairPlay (Safari): the key systems Spotify's player tries.
 */
const KEY_SYSTEMS = ['com.widevine.alpha', 'com.microsoft.playready', 'com.apple.fps', 'com.apple.fps.1_0'];

/** What to do when this display cannot play Spotify: the honest alternatives. */
export const NO_DRM_FIX =
	'Use Chrome, Edge or Firefox on a computer plugged into the TV (Firefox: allow “Play DRM-controlled content”; Chromium on a Raspberry Pi: install Widevine, the libwidevinecdm0 package). Most smart-TV and kiosk browsers cannot. Otherwise play from the TV’s own Spotify app or a Chromecast: the wall still shows what plays.';

/**
 * Can this browser be a Spotify Connect speaker? The Web Playback SDK needs a
 * secure context (HTTPS or localhost) and protected-media support (EME with a
 * DRM module). Says what is missing and how to fix it, instead of failing in
 * silence later.
 */
export async function speakerSupport(env: SpeakerEnv): Promise<SpeakerSupport> {
	if (!env.isSecureContext) {
		return {
			ok: false,
			code: 'insecure',
			reason: 'This display opened DumbMonit over plain HTTP.',
			fix: 'Spotify only plays in a secure page: open DumbMonit over HTTPS (a reverse proxy), or as http://localhost on the display itself.'
		};
	}
	const request = env.navigator.requestMediaKeySystemAccess;
	if (typeof request !== 'function') {
		return {
			ok: false,
			code: 'no_eme',
			reason: 'This browser cannot play Spotify: it has no Encrypted Media Extensions (DRM).',
			fix: NO_DRM_FIX
		};
	}
	for (const keySystem of KEY_SYSTEMS) {
		try {
			await request.call(env.navigator, keySystem, AUDIO_CONFIGS);
			return { ok: true };
		} catch {
			// Try the next one.
		}
	}
	return {
		ok: false,
		code: 'no_drm',
		reason: 'This browser cannot play Spotify: it has no DRM module (Widevine).',
		fix: NO_DRM_FIX
	};
}

/** Why the Web Playback SDK gave up, in words, and whether trying again can help. */
export interface SdkFailure {
	/** Which SDK event; `wall/messages.ts` turns it into translated text. */
	code?: 'initialization_error' | 'authentication_error' | 'account_error';
	/** The SDK's own message, appended in parentheses. */
	detail?: string;
	problem: string;
	fix: string | null;
	/** Nothing changes by waiting: only a change on the display or the account helps. */
	permanent: boolean;
	/** The SDK said the account has no Premium. */
	noPremium?: boolean;
}

/**
 * The SDK's error events, said plainly. `initialization_error` is what a
 * browser without usable DRM (EME/Widevine) raises, even when it claims EME
 * support: many TV, kiosk and Linux Chromium builds do.
 */
export function sdkFailure(event: 'initialization_error' | 'authentication_error' | 'account_error', message = ''): SdkFailure {
	const detail = message ? ` (${message})` : '';
	switch (event) {
		case 'initialization_error':
			return {
				code: event,
				detail: message,
				problem: `This browser cannot play Spotify: its DRM (Widevine) did not start${detail}.`,
				fix: NO_DRM_FIX,
				permanent: true
			};
		case 'account_error':
			return {
				code: event,
				problem: 'Spotify Premium is required for the wall to be a speaker.',
				fix: 'Connect a Premium account in Settings → Wall music. Now playing still shows what plays on your other devices.',
				permanent: true,
				noPremium: true
			};
		default:
			return {
				code: event,
				detail: message,
				problem: `Spotify refused the wall’s connection${detail}.`,
				fix: 'Trying again by itself. If it keeps failing, reconnect Spotify in Settings → Wall music.',
				permanent: false
			};
	}
}

/** Waits between automatic retries: 15 s, 30 s, 1 min, 2 min, then every 5 min. */
export function retryDelay(attempt: number): number {
	return Math.min(15_000 * 2 ** Math.max(0, attempt), 300_000);
}

/** The bits of `navigator` that say whether a page may make sound without a tap. */
export interface AutoplayEnv {
	getAutoplayPolicy?: (type: 'mediaelement' | 'audiocontext') => string;
}

/**
 * The browser already lets this page play sound (a kiosk started with
 * `--autoplay-policy=no-user-gesture-required`, Firefox with autoplay allowed
 * for the site…): no tap needed. Only Firefox and recent Chromium say so;
 * elsewhere the answer is "unknown", and the wall asks for a tap.
 */
export function autoplayAllowed(env: AutoplayEnv): boolean {
	try {
		return typeof env.getAutoplayPolicy === 'function' && env.getAutoplayPolicy('mediaelement') === 'allowed';
	} catch {
		return false;
	}
}

/** "Chrome 130 on Linux", "Firefox 131 on Windows", "Samsung Internet 25 on Tizen": to recognise a display. */
export function browserLabel(userAgent: string): string {
	const ua = userAgent || '';
	const version = (pattern: RegExp) => ua.match(pattern)?.[1]?.split('.')[0] ?? '';
	let browser = 'A browser';
	if (/SamsungBrowser\//.test(ua)) browser = `Samsung Internet ${version(/SamsungBrowser\/([\d.]+)/)}`;
	else if (/Edg\//.test(ua)) browser = `Edge ${version(/Edg\/([\d.]+)/)}`;
	else if (/OPR\//.test(ua)) browser = `Opera ${version(/OPR\/([\d.]+)/)}`;
	else if (/Firefox\//.test(ua)) browser = `Firefox ${version(/Firefox\/([\d.]+)/)}`;
	else if (/Chrom(e|ium)\//.test(ua)) browser = `${/Chromium\//.test(ua) ? 'Chromium' : 'Chrome'} ${version(/Chrom(?:e|ium)\/([\d.]+)/)}`;
	else if (/Safari\//.test(ua) && /Version\//.test(ua)) browser = `Safari ${version(/Version\/([\d.]+)/)}`;
	let system = '';
	if (/Tizen/.test(ua)) system = 'Tizen';
	else if (/Web0S|webOS/.test(ua)) system = 'webOS';
	else if (/Android/.test(ua)) system = /TV|AFT|BRAVIA|SMART-TV/i.test(ua) ? 'Android TV' : 'Android';
	else if (/CrOS/.test(ua)) system = 'ChromeOS';
	else if (/Windows/.test(ua)) system = 'Windows';
	else if (/iPhone|iPad/.test(ua)) system = 'iOS';
	else if (/Mac OS X/.test(ua)) system = 'macOS';
	else if (/Linux/.test(ua)) system = 'Linux';
	return `${browser.trim()}${system ? ` on ${system}` : ''}`;
}

/**
 * The name this display announces: `?speaker=` in its address (remembered by
 * the display), else the name chosen in Settings.
 */
export function displaySpeakerName(fromUrl: string | null, remembered: string | null, fromServer: string): string {
	const clean = (value: string | null) => {
		const name = (value ?? '').replace(/\p{Cc}/gu, '').trim();
		return name.length > 0 && name.length <= 64 ? name : null;
	};
	return clean(fromUrl) ?? clean(remembered) ?? fromServer;
}

/** The Web Playback SDK's state, as far as the wall reads it. */
export interface SdkState {
	paused: boolean;
	position: number;
	duration: number;
	track_window?: {
		current_track?: SdkTrack | null;
	};
}

export interface SdkTrack {
	name: string;
	type?: string;
	duration_ms?: number;
	artists?: { name: string }[];
	album?: { name?: string; images?: SdkImage[] };
}

export interface SdkImage {
	url: string;
	width?: number | null;
	height?: number | null;
	size?: string | null;
}

/** The smallest cover of at least 300 px, else the largest; only from Spotify's CDN. */
export function pickCover(images: SdkImage[] | undefined): string | null {
	const allowed = (images ?? []).flatMap((image) => {
		const url = coverUrl(image.url);
		return url ? [{ ...image, url }] : [];
	});
	if (allowed.length === 0) return null;
	const width = (image: SdkImage) => image.width ?? image.height ?? (image.size === 'LARGE' ? 640 : image.size === 'SMALL' ? 64 : 300);
	const large = allowed.filter((image) => width(image) >= 300).sort((a, b) => width(a) - width(b));
	if (large.length > 0) return large[0].url;
	return [...allowed].sort((a, b) => width(b) - width(a))[0].url;
}

/** What the wall's own player is playing, in the server's shape. */
export function fromSdkState(state: SdkState | null | undefined, speakerName: string): NowPlaying | null {
	const track = state?.track_window?.current_track;
	if (!state || !track || !track.name) return null;
	const kind = track.type ?? 'track';
	return {
		playing: !state.paused,
		kind,
		title: track.name,
		artists: (track.artists ?? []).map((artist) => artist.name).filter(Boolean),
		album: track.album?.name || null,
		cover_url: pickCover(track.album?.images),
		duration_ms: state.duration || track.duration_ms || 0,
		progress_ms: Math.max(0, state.position || 0),
		device_name: speakerName,
		device_type: 'Computer'
	};
}

export interface Playing {
	now: NowPlaying;
	/** `speaker`: this display plays it; `account`: somewhere else on the account. */
	source: 'speaker' | 'account';
}

/**
 * What the card shows: the wall's own player while it plays (instant), else
 * what the account plays anywhere (polled).
 */
export function pickPlaying(local: NowPlaying | null, account: NowPlaying | null): Playing | null {
	if (local?.playing) return { now: local, source: 'speaker' };
	if (account) return { now: account, source: 'account' };
	if (local) return { now: local, source: 'speaker' };
	return null;
}

/**
 * Shown while playing, and for a short grace after a pause (a skip or a
 * phone call should not make the card blink away).
 */
export function cardVisible(now: NowPlaying | null, lastPlayingAt: number | null, at: number): boolean {
	if (!now || now.kind === 'ad') return false;
	if (now.playing) return true;
	return lastPlayingAt !== null && at - lastPlayingAt < PAUSE_GRACE_MS;
}

/** Position now, from the position read at `readAt` (ms clock of the caller). */
export function progressAt(now: NowPlaying, readAt: number, at: number): number {
	const position = now.playing ? now.progress_ms + Math.max(0, at - readAt) : now.progress_ms;
	return now.duration_ms > 0 ? Math.min(position, now.duration_ms) : position;
}

/** `3:07`, `1:02:45`. */
export function formatTime(ms: number): string {
	const total = Math.max(0, Math.floor(ms / 1000));
	const hours = Math.floor(total / 3600);
	const minutes = Math.floor((total % 3600) / 60);
	const seconds = String(total % 60).padStart(2, '0');
	return hours > 0 ? `${hours}:${String(minutes).padStart(2, '0')}:${seconds}` : `${minutes}:${seconds}`;
}

/** "Living room TV", "this display", "a phone"… for "Playing on …". */
export function deviceLabel(now: NowPlaying, source: Playing['source'], speakerName: string): string {
	if (source === 'speaker' || (now.device_name && now.device_name === speakerName)) return 'this display';
	if (now.device_name) return now.device_name;
	const words: Record<string, string> = {
		Smartphone: 'a phone',
		Tablet: 'a tablet',
		Computer: 'a computer',
		Speaker: 'a speaker',
		TV: 'a TV',
		CastVideo: 'a Chromecast',
		CastAudio: 'a cast speaker',
		AVR: 'an amplifier',
		GameConsole: 'a game console'
	};
	return words[now.device_type ?? ''] ?? 'another device';
}
