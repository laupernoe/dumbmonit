/**
 * Music on the wall: turns a share link into the provider's official embed.
 *
 * Only three players are ever framed — Spotify, Deezer and YouTube (YouTube
 * Music included) — and the embed URL is rebuilt from the few parts that were
 * validated (kind and id), never copied from what was pasted. Anything else is
 * refused with a sentence the user can act on. The server's CSP allows exactly
 * the three embed origins below in `frame-src`; keep both lists in step.
 *
 * Pure module, no imports: `npm test` (tests/music.test.mjs) runs it as is.
 */

export type MusicProvider = 'spotify' | 'deezer' | 'youtube';

export interface MusicEmbed {
	provider: MusicProvider;
	/** Provider name shown to the user ("Spotify", "YouTube Music"…). */
	label: string;
	/** What was linked, in words: "track", "playlist", "video"… */
	kind: string;
	/** The iframe `src`: always on one of `EMBED_ORIGINS`. */
	src: string;
}

/** Why a link was refused; the UI says it in the user's language (`wall/messages.ts`). */
export type MusicErrorCode =
	| 'empty'
	| 'not_link'
	| 'unsupported'
	| 'spotify_short'
	| 'deezer_short'
	| 'youtube_page'
	| 'spotify_kinds'
	| 'spotify_cut'
	| 'deezer_kinds'
	| 'deezer_cut'
	| 'youtube_cut'
	| 'youtube_empty';

/** `message` is the English fallback; screens show the translation of `code`. */
export type MusicParse = { ok: true; embed: MusicEmbed } | { ok: false; code: MusicErrorCode; message: string };

/** The only origins a wall ever frames (mirrors the server's `frame-src`). */
export const EMBED_ORIGINS = [
	'https://open.spotify.com',
	'https://widget.deezer.com',
	'https://www.youtube-nocookie.com'
] as const;

const SPOTIFY_KINDS = ['track', 'album', 'playlist', 'episode'] as const;
const DEEZER_KINDS = ['track', 'album', 'playlist'] as const;

const SPOTIFY_ID = /^[A-Za-z0-9]{22}$/;
const DEEZER_ID = /^\d{1,20}$/;
const YOUTUBE_VIDEO = /^[A-Za-z0-9_-]{11}$/;
const YOUTUBE_LIST = /^[A-Za-z0-9_-]{2,80}$/;

const UNSUPPORTED =
	'Paste a Spotify, Deezer or YouTube link: a track, album, playlist, episode or video.';

function fail(code: MusicErrorCode, message: string): MusicParse {
	return { ok: false, code, message };
}

function includes<T extends string>(list: readonly T[], value: string): value is T {
	return (list as readonly string[]).includes(value);
}

/** Parses a pasted link. Accepts it with or without `https://`. */
export function parseMusicLink(input: string): MusicParse {
	const raw = input.trim();
	if (!raw) return fail('empty', 'Paste a link first.');

	// Spotify's app copies URIs as well as links: spotify:playlist:<id>.
	const uri = /^spotify:([a-z]+):([A-Za-z0-9]+)$/.exec(raw);
	if (uri) return spotify([uri[1], uri[2]]);

	let url: URL;
	try {
		url = new URL(/^[a-z][a-z0-9+.-]*:/i.test(raw) ? raw : `https://${raw}`);
	} catch {
		return fail('not_link', `That is not a link. ${UNSUPPORTED}`);
	}
	if (url.protocol !== 'https:' && url.protocol !== 'http:') return fail('unsupported', UNSUPPORTED);
	if (url.username || url.password || (url.port && url.port !== '443' && url.port !== '80')) {
		return fail('unsupported', UNSUPPORTED);
	}

	const host = url.hostname.toLowerCase();
	const segments = url.pathname.split('/').filter(Boolean);

	switch (host) {
		case 'open.spotify.com':
		case 'play.spotify.com': {
			// Localised links carry a prefix: /intl-fr/track/<id>. Embeds too: /embed/track/<id>.
			const rest = segments.filter((s, i) => !(i === 0 && (/^intl-[a-z-]+$/i.test(s) || s === 'embed')));
			return spotify(rest);
		}
		case 'spotify.link':
			return fail('spotify_short', 'Short Spotify links cannot be read here: open it, then copy the full open.spotify.com link.');
		case 'deezer.com':
		case 'www.deezer.com': {
			// Localised links carry a language: /fr/playlist/<id>.
			const rest = segments.filter((s, i) => !(i === 0 && /^[a-z]{2}(-[a-z]{2})?$/i.test(s)));
			return deezer(rest);
		}
		case 'widget.deezer.com': {
			// Already an embed: /widget/<theme>/<kind>/<id>.
			if (segments[0] !== 'widget') return fail('unsupported', UNSUPPORTED);
			return deezer(segments.slice(2));
		}
		case 'deezer.page.link':
		case 'link.deezer.com':
			return fail('deezer_short', 'Short Deezer links cannot be read here: open it, then copy the full deezer.com link.');
		case 'youtu.be':
			return youtube(segments[0], url.searchParams.get('list'), false);
		case 'youtube.com':
		case 'www.youtube.com':
		case 'm.youtube.com':
		case 'youtube-nocookie.com':
		case 'www.youtube-nocookie.com':
		case 'music.youtube.com': {
			const music = host === 'music.youtube.com';
			const list = url.searchParams.get('list');
			const [first, second] = segments;
			if (first === 'watch') return youtube(url.searchParams.get('v'), list, music);
			if (first === 'playlist') return youtube(null, list, music);
			if ((first === 'embed' || first === 'shorts' || first === 'live') && second) {
				if (second === 'videoseries') return youtube(null, list, music);
				return youtube(second, list, music);
			}
			return fail('youtube_page', 'Paste the link of a YouTube video or playlist, not of a channel or a page.');
		}
		default:
			return fail('unsupported', UNSUPPORTED);
	}
}

function spotify([kind, id]: string[]): MusicParse {
	if (!kind || !id) return fail('unsupported', UNSUPPORTED);
	if (!includes(SPOTIFY_KINDS, kind)) {
		return fail('spotify_kinds', 'Spotify links to a track, album, playlist or episode can play on the wall.');
	}
	if (!SPOTIFY_ID.test(id)) return fail('spotify_cut', 'This Spotify link looks cut short.');
	return {
		ok: true,
		embed: {
			provider: 'spotify',
			label: 'Spotify',
			kind,
			src: `https://open.spotify.com/embed/${kind}/${id}`
		}
	};
}

function deezer([kind, id]: string[]): MusicParse {
	if (!kind || !id) return fail('unsupported', UNSUPPORTED);
	if (!includes(DEEZER_KINDS, kind)) {
		return fail('deezer_kinds', 'Deezer links to a track, album or playlist can play on the wall.');
	}
	if (!DEEZER_ID.test(id)) return fail('deezer_cut', 'This Deezer link looks cut short.');
	return {
		ok: true,
		embed: {
			provider: 'deezer',
			label: 'Deezer',
			kind,
			src: `https://widget.deezer.com/widget/auto/${kind}/${id}`
		}
	};
}

function youtube(video: string | null | undefined, list: string | null, music: boolean): MusicParse {
	const label = music ? 'YouTube Music' : 'YouTube';
	const validList = list && YOUTUBE_LIST.test(list) ? list : null;
	if (video) {
		if (!YOUTUBE_VIDEO.test(video)) return fail('youtube_cut', 'This YouTube link looks cut short.');
		const query = validList ? `?list=${validList}` : '';
		return {
			ok: true,
			embed: {
				provider: 'youtube',
				label,
				kind: validList ? 'playlist' : music ? 'song' : 'video',
				src: `https://www.youtube-nocookie.com/embed/${video}${query}`
			}
		};
	}
	if (!validList) return fail('youtube_empty', 'This YouTube link has no video or playlist in it.');
	return {
		ok: true,
		embed: {
			provider: 'youtube',
			label,
			kind: 'playlist',
			src: `https://www.youtube-nocookie.com/embed/videoseries?list=${validList}`
		}
	};
}

/**
 * The frame address, asking the player to start on its own where the provider
 * allows it (YouTube, Deezer; Spotify's embed never does). Browsers still keep
 * a frame silent until someone has touched the wall once: the wall asks for
 * that tap. Same origin as `embed.src`, only a query parameter is added.
 */
export function embedSrc(embed: MusicEmbed, options: { autoplay?: boolean } = {}): string {
	if (!options.autoplay) return embed.src;
	const join = embed.src.includes('?') ? '&' : '?';
	switch (embed.provider) {
		case 'youtube':
			return `${embed.src}${join}autoplay=1`;
		case 'deezer':
			return `${embed.src}${join}autoplay=true`;
		default:
			return embed.src;
	}
}
