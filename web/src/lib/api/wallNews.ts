/**
 * News on the wall: the headline ticker (RSS/Atom feeds read by the server) and
 * the live video tile. Mirrors `crates/server/src/api/wall_news.rs` and its
 * presets. Off by default.
 */
import { request } from './client';

export interface NewsFeed {
	url: string;
	/** Shown before each headline; empty: the feed's domain. */
	label: string;
}

export type VideoPosition = 'right' | 'center';
export type VideoSize = 'small' | 'medium' | 'large';

/** `GET|PUT /api/wall/news/settings` (administrators). */
export interface NewsSettings {
	ticker: boolean;
	/** Articles with their picture, in rotation on the wall (same feeds). */
	gallery: boolean;
	feeds: NewsFeed[];
	video: boolean;
	/** The link as pasted: YouTube, Dailymotion or Twitch. */
	video_url: string;
	video_position: VideoPosition;
	video_size: VideoSize;
}

export interface NewsItem {
	title: string;
	link: string;
	source: string;
	/** UTC without suffix; null when the feed does not date the article. */
	published: string | null;
	/** The article's picture, relayed by the server (`/api/wall/news/image/<id>`); null without one. */
	image: string | null;
}

export interface NewsVideo {
	/** Always on a fixed https origin the CSP allows. Twitch needs `parent` added by the page. */
	embed_url: string;
	position: VideoPosition;
	size: VideoSize;
}

/** `GET /api/wall/news`: what the wall polls every five minutes. */
export interface WallNews {
	ticker: boolean;
	/** The picture carousel is on. */
	gallery: boolean;
	items: NewsItem[];
	/** Feeds that did not answer, with why. */
	problems: { feed: string; message: string }[];
	video: NewsVideo | null;
}

export interface NewsCountry {
	/** ISO 3166-1 alpha-2, or `INT` for international sources. */
	code: string;
	feeds: { label: string; url: string }[];
	videos: { label: string; url: string }[];
}

export interface FeedTest {
	ok: boolean;
	error: string | null;
	count: number;
	first: NewsItem | null;
}

export const DEFAULT_NEWS_SETTINGS: NewsSettings = {
	ticker: false,
	gallery: false,
	feeds: [],
	video: false,
	video_url: '',
	video_position: 'right',
	video_size: 'medium'
};

export function getWallNews(signal?: AbortSignal): Promise<WallNews> {
	return request<WallNews>('/wall/news', { signal, anticipated: true });
}

export function getNewsSettings(signal?: AbortSignal): Promise<NewsSettings> {
	return request<NewsSettings>('/wall/news/settings', { signal, anticipated: true });
}

export function saveNewsSettings(settings: NewsSettings): Promise<NewsSettings> {
	return request<NewsSettings>('/wall/news/settings', { method: 'PUT', body: settings });
}

export function getNewsPresets(signal?: AbortSignal): Promise<NewsCountry[]> {
	return request<NewsCountry[]>('/wall/news/presets', { signal, anticipated: true });
}

export function testNewsFeed(url: string): Promise<FeedTest> {
	return request<FeedTest>('/wall/news/test', { method: 'POST', body: { url } });
}
