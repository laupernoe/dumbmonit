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

export type SpeakerSupport = { ok: true } | { ok: false; reason: string; fix: string };

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

/** Widevine (Chrome, Edge, Firefox, Android), then FairPlay (Safari). */
const KEY_SYSTEMS = ['com.widevine.alpha', 'com.apple.fps', 'com.apple.fps.1_0'];

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
			reason: 'This display opened DumbMonit over plain HTTP.',
			fix: 'Spotify only plays in a secure page: open DumbMonit over HTTPS (a reverse proxy), or as http://localhost on the display itself.'
		};
	}
	const request = env.navigator.requestMediaKeySystemAccess;
	if (typeof request !== 'function') {
		return {
			ok: false,
			reason: 'This browser cannot play protected audio (no Encrypted Media Extensions).',
			fix: 'Use Chrome, Edge, Firefox or Safari on this display.'
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
		reason: 'This browser has no DRM module for Spotify (Widevine).',
		fix: 'Firefox: allow “Play DRM-controlled content” in its settings. Chromium on a Raspberry Pi: install Widevine (the libwidevinecdm0 package). Smart-TV browsers usually cannot.'
	};
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
