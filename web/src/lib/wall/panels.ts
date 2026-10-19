/**
 * The wall's information panels, as pure data (no Svelte, no Paraglide: tested
 * by `tests/wall-panels.test.mjs`). One panel at a time sits under the clock,
 * still for a few seconds, then the next one takes its place: a page of
 * headlines, an article's picture, or a widget's card (coins, exchange rates,
 * stocks, critical vulnerabilities, Claude or OpenAI usage). A list longer than
 * the room is cut into pages of the rows that fit; nothing scrolls, nothing is
 * clipped.
 */
import type { WallWidgets } from '../api/types';
import type { NewsItem } from '../api/wallNews';

export type WidgetKind = 'crypto' | 'fx' | 'stocks' | 'cve' | 'claude' | 'openai';
/** Panels made of rows: their pages are cut by the rows that fit the room. */
export type ListKind = WidgetKind | 'headlines';

export type Panel =
	| { key: string; kind: 'picture'; item: NewsItem }
	| { key: string; kind: 'headlines'; items: NewsItem[]; page: number; pages: number }
	| { key: string; kind: WidgetKind; from: number; to: number; page: number; pages: number };

export const WIDGET_KINDS: readonly WidgetKind[] = ['crypto', 'fx', 'stocks', 'cve', 'claude', 'openai'];
export const LIST_KINDS: readonly ListKind[] = ['headlines', ...WIDGET_KINDS];

/** Rows per page while the room is not measured (a phone, where the page scrolls). */
export const DEFAULT_ROWS: Record<ListKind, number> = {
	headlines: 5,
	crypto: 6,
	fx: 6,
	stocks: 6,
	cve: 4,
	claude: 4,
	openai: 4
};
/** Pictures per round, the most recent first: a wall is glanced at, not read through. */
export const MAX_PICTURES = 8;
/** Headlines per round, beyond the pictured articles. */
export const MAX_HEADLINES = 20;

export type Rows = Partial<Record<ListKind, number>>;

/**
 * How many rows a card may show in a room `room` pixels high, once its chrome
 * (header, footer, padding) and one row's height are known. Unmeasured: the
 * fallback. Always at least one row, so a card never goes blank.
 */
export function rowsThatFit(room: number, chrome: number, row: number, fallback: number): number {
	if (!(room > 0) || !(row > 0) || !(chrome >= 0)) return fallback;
	return Math.max(1, Math.floor((room - chrome) / row));
}

/** The chrome and row height of a list card from two probes: one row high, two rows high. */
export function cardMetrics(oneRow: number, twoRows: number): { chrome: number; row: number } | null {
	if (!(oneRow > 0) || !(twoRows > oneRow)) return null;
	const row = twoRows - oneRow;
	return { chrome: oneRow - row, row };
}

/** Rows a widget would show; 0 when it has nothing to show on a wall. */
export function widgetRows(widgets: WallWidgets | null, kind: WidgetKind): number {
	const section = widgets?.[kind];
	// Not configured, waiting for a key or failing with nothing to show: no panel at all.
	if (!section || section.needs_key || !section.data) return 0;
	switch (kind) {
		case 'crypto':
			return section.data.coins.length;
		case 'fx':
			return section.data.rates.length;
		case 'stocks':
			return section.data.quotes.length;
		case 'cve':
			return section.data.length;
		default:
			// The usage card always has its total; the people below it are the rows.
			return Math.max(1, section.data.people?.length ?? 0);
	}
}

/** Interleaves two lists, one from each in turn, the longer one finishing alone. */
export function interleave<T>(a: readonly T[], b: readonly T[]): T[] {
	const out: T[] = [];
	for (let i = 0; i < Math.max(a.length, b.length); i += 1) {
		if (i < a.length) out.push(a[i]);
		if (i < b.length) out.push(b[i]);
	}
	return out;
}

function paged<T>(count: number, perPage: number, make: (from: number, to: number, page: number, pages: number) => T): T[] {
	const size = Math.max(1, Math.floor(perPage));
	const pages = Math.ceil(count / size);
	const out: T[] = [];
	for (let page = 0; page < pages; page += 1) {
		out.push(make(page * size, Math.min(count, (page + 1) * size), page, pages));
	}
	return out;
}

export interface PanelInput {
	/** The news items, most recent first; empty when the news is off. */
	news: readonly NewsItem[];
	/** Pictures are on: an article with a picture gets a panel of its own. */
	pictures: boolean;
	/** Picture addresses that did not load: their articles go back to the headlines. */
	failed?: ReadonlySet<string>;
	widgets: WallWidgets | null;
	/** Rows per page by kind, from the measured room; missing kinds use the defaults. */
	rows?: Rows;
}

/**
 * The round of panels, in order. News and widgets alternate so neither waits
 * for the other; within the news, a picture and a page of headlines alternate.
 * An article without a picture (or whose picture failed) is always a headline:
 * turning pictures on never hides a feed.
 */
export function buildPanels(input: PanelInput): Panel[] {
	const rows = { ...DEFAULT_ROWS, ...input.rows };
	const failed = input.failed ?? new Set<string>();
	const pictured = input.pictures
		? input.news.filter((item) => item.image && !failed.has(item.image)).slice(0, MAX_PICTURES)
		: [];
	const shown = new Set(pictured.map((item) => item.link));
	const text = input.news.filter((item) => !shown.has(item.link)).slice(0, MAX_HEADLINES);
	const pictures: Panel[] = pictured.map((item) => ({ key: `picture:${item.link}`, kind: 'picture', item }));
	const headlines = paged<Panel>(text.length, rows.headlines, (from, to, page, pages) => ({
		key: `headlines:${page}`,
		kind: 'headlines',
		items: text.slice(from, to),
		page,
		pages
	}));
	const widgets: Panel[] = [];
	for (const kind of WIDGET_KINDS) {
		const count = widgetRows(input.widgets, kind);
		if (count === 0) continue;
		widgets.push(
			...paged<Panel>(count, rows[kind], (from, to, page, pages) => ({ key: `${kind}:${page}`, kind, from, to, page, pages }))
		);
	}
	return interleave(interleave(pictures, headlines), widgets);
}

/** The next panel's index; stays valid when the round shrinks. */
export function nextPanel(index: number, count: number): number {
	if (count <= 0) return 0;
	return (index + 1) % count;
}
