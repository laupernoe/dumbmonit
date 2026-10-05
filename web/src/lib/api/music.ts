/**
 * Music on the wall: what plays on the connected Spotify account, the shared
 * link the walls play, and connecting Spotify. Mirrors
 * `crates/server/src/api/music.rs` and `crates/server/src/music/`.
 *
 * Tokens never come back here, except the short-lived access token the wall's
 * Spotify player (Web Playback SDK) needs — and only to a browser session.
 */
import { request } from './client';

/** `off`: no account; `expired`: Spotify ended the connection, reconnect. */
export type SpotifyConnection = 'off' | 'connected' | 'expired';

/** What plays, reduced to what the wall shows. */
export interface NowPlaying {
	/** False while paused. */
	playing: boolean;
	/** `track`, `episode`, `ad` or `unknown`. */
	kind: string;
	title: string;
	/** Artists of a track; the show of an episode. */
	artists: string[];
	album: string | null;
	/** Always on `https://i.scdn.co/` (the only image origin the CSP allows). */
	cover_url: string | null;
	duration_ms: number;
	progress_ms: number;
	/** The output device: "Living room TV", "DumbMonit Wall"… */
	device_name: string | null;
	/** Spotify's device type: `Computer`, `Smartphone`, `Speaker`, `TV`… */
	device_type: string | null;
}

export interface SpotifyNow {
	status: SpotifyConnection;
	now_playing: NowPlaying | null;
	/** A passing problem, or what to do, in one sentence. */
	error: string | null;
}

/** The link every wall plays ("Play on the wall"). */
export interface WallLink {
	link: string;
	set_by: string;
	/** Server time, UTC without suffix. */
	set_at: string;
}

/** `GET /api/music/now`: what the walls poll every few seconds. */
export interface WallMusic {
	spotify: SpotifyNow;
	link: WallLink | null;
	/** The Spotify Connect device name the wall announces. */
	speaker_name: string;
}

/** The connected account as Settings shows it: never a token. */
export interface SpotifyAccount {
	status: SpotifyConnection;
	client_id: string | null;
	account_name: string | null;
	connected_at: string | null;
	/** Spotify stops honouring the connection six months after approval. */
	reconnect_by: string | null;
	missing_scopes: string[];
	/** Why the connection ended, when it did. */
	last_error: string | null;
	speaker_name: string;
	scopes: string[];
	/** `http://127.0.0.1:8888/callback`: the redirect for plain-HTTP installs. */
	loopback_redirect_uri: string;
	/** `/api/music/spotify/callback`: the direct redirect, on an HTTPS origin. */
	callback_path: string;
}

export interface SpotifyAuthorization {
	/** Spotify's approval page. */
	authorize_url: string;
	redirect_uri: string;
	/** Seconds the approval may take. */
	expires_in: number;
}

export interface SpotifyAccessToken {
	access_token: string;
	expires_in: number;
}

export function getWallMusic(signal?: AbortSignal): Promise<WallMusic> {
	return request<WallMusic>('/music/now', { signal, anticipated: true });
}

/** Every wall plays this link (admin). */
export function playOnWall(link: string): Promise<WallLink> {
	return request<WallLink>('/music/link', { method: 'PUT', body: { link } });
}

/** The walls stop the embedded player (admin). */
export function stopWallLink(): Promise<void> {
	return request<void>('/music/link', { method: 'DELETE' });
}

export function getSpotifyAccount(signal?: AbortSignal): Promise<SpotifyAccount> {
	return request<SpotifyAccount>('/music/spotify', { signal, anticipated: true });
}

/** Starts a connection (admin); the browser then goes to `authorize_url`. */
export function startSpotifyConnection(clientId: string, redirectUri: string): Promise<SpotifyAuthorization> {
	return request<SpotifyAuthorization>('/music/spotify/authorize', {
		method: 'POST',
		body: { client_id: clientId, redirect_uri: redirectUri }
	});
}

/** Finishes a loopback connection with the address the browser landed on. */
export function completeSpotifyConnection(url: string): Promise<SpotifyAccount> {
	return request<SpotifyAccount>('/music/spotify/complete', { method: 'POST', body: { url } });
}

export function disconnectSpotify(): Promise<void> {
	return request<void>('/music/spotify', { method: 'DELETE' });
}

/** A short-lived token for the wall's Spotify player. 409: nothing connected. */
export function getSpotifyToken(): Promise<SpotifyAccessToken> {
	return request<SpotifyAccessToken>('/music/spotify/token');
}

// --- The speaker -------------------------------------------------------------

/** A Spotify Connect device of the account (`GET /v1/me/player/devices`). */
export interface SpeakerDevice {
	id: string | null;
	name: string;
	/** `Computer`, `Smartphone`, `Speaker`, `TV`… */
	type: string;
	is_active: boolean;
	is_restricted: boolean;
	volume_percent: number | null;
}

export type SpeakerPhaseName = 'off' | 'unsupported' | 'starting' | 'ready' | 'error';

/** What a wall says about its speaker (sent every minute and on every change). */
export interface SpeakerReportBody {
	/** Random id kept by the wall's browser. */
	display: string;
	name: string;
	phase: SpeakerPhaseName;
	activated: boolean;
	device_id: string | null;
	problem: string | null;
	/** "Chrome 130 on Linux". */
	browser: string | null;
	/** false: Spotify said the account has no Premium; true: the player is ready. */
	premium: boolean | null;
}

/** A wall as Settings shows it. */
export interface WallReport extends SpeakerReportBody {
	/** Spotify lists this wall's device (`null`: could not be checked). */
	listed: boolean | null;
	/** Server time, UTC without suffix. */
	seen_at: string;
}

/** `GET /api/music/speaker`. */
export interface WallSpeaker {
	speaker_name: string;
	status: SpotifyConnection;
	account_name: string | null;
	/** `null`: unknown until Spotify or a wall says. */
	premium: boolean | null;
	devices: SpeakerDevice[];
	/** Spotify lists the speaker among the account's devices. */
	listed: boolean;
	/** Walls that reported in the last fifteen minutes, newest first. */
	walls: WallReport[];
	/** Why the device list could not be read. */
	error: string | null;
}

export function getWallSpeaker(signal?: AbortSignal): Promise<WallSpeaker> {
	return request<WallSpeaker>('/music/speaker', { signal, anticipated: true });
}

/** Renames the speaker every wall announces (admin); `null` goes back to the default. */
export function setSpeakerName(name: string | null): Promise<{ speaker_name: string }> {
	return request<{ speaker_name: string }>('/music/speaker', { method: 'PUT', body: { name } });
}

/** A wall's report; the reply says whether Spotify lists its device. */
export function reportSpeaker(report: SpeakerReportBody): Promise<{ listed: boolean | null }> {
	return request<{ listed: boolean | null }>('/music/speaker/report', { method: 'POST', body: report, anticipated: true });
}

/**
 * Plays the account on the speaker: this wall's device when given ("Play
 * here"), else the device named like the speaker ("Test sound").
 */
export function playOnSpeaker(deviceId?: string | null): Promise<{ device_id: string; speaker_name: string }> {
	return request<{ device_id: string; speaker_name: string }>('/music/speaker/play', {
		method: 'POST',
		body: { device_id: deviceId ?? null }
	});
}
