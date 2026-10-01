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
