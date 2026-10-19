/**
 * News on the wall: headlines of RSS/Atom feeds, read by the server. Mirrors `crates/server/src/api/wall_news.rs` and its
 * presets. Off by default.
 */
import { request } from './client';

export interface NewsFeed {
	url: string;
	/** Shown before each headline; empty: the feed's domain. */
	label: string;
}

/** `GET|PUT /api/wall/news/settings` (administrators). */
export interface NewsSettings {
	ticker: boolean;
	feeds: NewsFeed[];
}

export interface NewsItem {
	title: string;
	link: string;
	source: string;
	/** UTC without suffix; null when the feed does not date the article. */
	published: string | null;
}

/** `GET /api/wall/news`: what the wall polls every five minutes. */
export interface WallNews {
	ticker: boolean;
	items: NewsItem[];
	/** Feeds that did not answer, with why. */
	problems: { feed: string; message: string }[];
}

export interface NewsCountry {
	/** ISO 3166-1 alpha-2, or `INT` for international sources. */
	code: string;
	feeds: { label: string; url: string }[];
}

export interface FeedTest {
	ok: boolean;
	error: string | null;
	count: number;
	first: NewsItem | null;
}

export const DEFAULT_NEWS_SETTINGS: NewsSettings = {
	ticker: false,
	feeds: []
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
