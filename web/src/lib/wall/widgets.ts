/**
 * Pure helpers for the wall widgets: which cards to show and in what order,
 * rises and falls (never by colour alone: an arrow and a sign), prices and
 * rates formatted for across the room, and WMO weather codes. No Svelte, no
 * Paraglide: tested by `tests/wall-widgets.test.mjs`.
 */
import type { WallWidgets } from '../api/types';

export type Trend = 'up' | 'down' | 'flat';

/** A change in percent: below a hundredth of a percent it reads as flat. */
export function trend(percent: number | null | undefined): Trend | null {
	if (percent === null || percent === undefined || !Number.isFinite(percent)) return null;
	if (Math.abs(percent) < 0.005) return 'flat';
	return percent > 0 ? 'up' : 'down';
}

export const TREND_ARROW: Record<Trend, string> = { up: '▲', down: '▼', flat: '▶' };

/** `+1.04 %`, `−0.57 %` in the reader's locale; the sign is always written. */
export function formatPercent(percent: number, locale: string): string {
	return new Intl.NumberFormat(locale, {
		style: 'percent',
		signDisplay: 'exceptZero',
		minimumFractionDigits: 2,
		maximumFractionDigits: 2
	}).format(percent / 100);
}

/** Fraction digits that suit a price: none for thousands, more for pennies. */
export function priceDigits(value: number): { min: number; max: number } {
	const abs = Math.abs(value);
	if (abs >= 1000) return { min: 0, max: 0 };
	if (abs >= 1) return { min: 2, max: 2 };
	if (abs >= 0.01) return { min: 4, max: 4 };
	return { min: 6, max: 8 };
}

/** A price in a currency (`73 119 €`, `$0.1834`); an unknown code still prints as its code. */
export function formatPrice(value: number, currency: string, locale: string): string {
	const { min, max } = priceDigits(value);
	try {
		return new Intl.NumberFormat(locale, {
			style: 'currency',
			currency: currency.toUpperCase(),
			minimumFractionDigits: min,
			maximumFractionDigits: max
		}).format(value);
	} catch {
		return `${formatNumber(value, locale)} ${currency.toUpperCase()}`;
	}
}

/** An exchange rate: two decimals, up to four below ten (`0.94`, `0.9421`, `188.70`). */
export function formatRate(value: number, locale: string): string {
	return new Intl.NumberFormat(locale, { minimumFractionDigits: 2, maximumFractionDigits: value < 10 ? 4 : 2 }).format(value);
}

export function formatNumber(value: number, locale: string): string {
	const { min, max } = priceDigits(value);
	return new Intl.NumberFormat(locale, { minimumFractionDigits: min, maximumFractionDigits: max }).format(value);
}

/**
 * An exchange rate the way people read one: what a unit of the foreign
 * currency costs in the base (1 USD = 0.89 EUR). Currencies worth little
 * (yen, forint…) are shown per hundred. The change compares with the previous
 * business day.
 */
export function fxQuote(rate: number, previous: number | null): { per: number; value: number; change: number | null } {
	const inverse = 1 / rate;
	const per = inverse < 0.05 ? 100 : 1;
	const change = previous && previous > 0 ? ((1 / rate - 1 / previous) / (1 / previous)) * 100 : null;
	return { per, value: inverse * per, change };
}

/** What a WMO weather code looks like, for the icon and the words. */
export type WeatherKind = 'clear' | 'partly' | 'cloudy' | 'fog' | 'drizzle' | 'rain' | 'snow' | 'storm';

export function weatherKind(code: number): WeatherKind {
	if (code <= 1) return 'clear';
	if (code === 2) return 'partly';
	if (code === 3) return 'cloudy';
	if (code === 45 || code === 48) return 'fog';
	if (code >= 51 && code <= 57) return 'drizzle';
	if ((code >= 61 && code <= 67) || (code >= 80 && code <= 82)) return 'rain';
	if ((code >= 71 && code <= 77) || code === 85 || code === 86) return 'snow';
	if (code >= 95) return 'storm';
	return 'cloudy';
}

/** A temperature for across the room: whole degrees, `18°`. */
export function roundTemperature(value: number): string {
	return `${Math.round(value)}°`;
}

/**
 * One card of the right rail. Coins and exchange rates are not cards: they
 * live in the market bar (`marketItems`). Long lists are split into pages.
 */
export type WidgetCard = {
	key: string;
	kind: 'cve' | 'claude' | 'openai' | 'stocks';
	from: number;
	to: number;
	page: number;
	pages: number;
};

/** Rows per card, so a card always fits the rail and reads from the sofa. */
export const ROWS_PER_CARD = { cve: 5, claude: 4, openai: 4, stocks: 5 } as const;

/** A dollar amount: cents under a hundred dollars, whole dollars above. */
export function formatUsd(value: number, locale: string): string {
	const digits = Math.abs(value) >= 100 ? 0 : 2;
	return new Intl.NumberFormat(locale, {
		style: 'currency',
		currency: 'USD',
		minimumFractionDigits: digits,
		maximumFractionDigits: digits
	}).format(value);
}

/** Token counts for across the room: `1.2M`, `340K`, `980`. */
export function formatTokens(value: number, locale: string): string {
	return new Intl.NumberFormat(locale, { notation: 'compact', maximumFractionDigits: 1 }).format(value);
}

/** Rows a card would show; 0 when the widget has nothing to show on a wall. */
function rowsOf(widgets: WallWidgets, kind: keyof typeof ROWS_PER_CARD): number {
	const section = widgets[kind];
	// Not configured, waiting for a key or failing with nothing to show: no card at all.
	if (!section || section.needs_key || !section.data) return 0;
	if (kind === 'stocks') return (section.data as { quotes: unknown[] }).quotes.length;
	if (kind === 'claude' || kind === 'openai') return Math.max(1, (section.data as { people: unknown[] | null }).people?.length ?? 0);
	return (section.data as unknown[]).length;
}

/**
 * The rail's cards, in a fixed order: critical vulnerabilities, Claude and OpenAI usage, stocks. Only
 * widgets that are turned on and have something to show get a card: never a "key required" card on a wall.
 */
export function widgetCards(widgets: WallWidgets | null, maxRows = Infinity): WidgetCard[] {
	if (!widgets) return [];
	const cards: WidgetCard[] = [];
	for (const kind of ['cve', 'claude', 'openai', 'stocks'] as const) {
		const rows = rowsOf(widgets, kind);
		if (rows === 0) continue;
		const size = Math.max(1, Math.min(ROWS_PER_CARD[kind], maxRows));
		const pages = Math.ceil(rows / size);
		for (let page = 0; page < pages; page += 1) {
			cards.push({ key: `${kind}:${page}`, kind, from: page * size, to: Math.min(rows, (page + 1) * size), page, pages });
		}
	}
	return cards;
}

/** One figure of the market bar: a coin or an exchange rate. */
export type MarketItem = {
	key: string;
	group: 'crypto' | 'fx';
	/** `BTC`, `EUR`, `100 JPY`. */
	symbol: string;
	price: number;
	/** The currency `price` is written in. */
	currency: string;
	/** Percent. */
	change: number | null;
};

/** The market bar's figures: coins first, then the world's currencies against the base. */
export function marketItems(widgets: WallWidgets | null): MarketItem[] {
	const items: MarketItem[] = [];
	const crypto = widgets?.crypto;
	if (crypto && !crypto.needs_key && crypto.data) {
		for (const coin of crypto.data.coins) {
			items.push({ key: `c:${coin.id}`, group: 'crypto', symbol: coin.symbol.toUpperCase(), price: coin.price, currency: crypto.data.currency, change: coin.change_24h });
		}
	}
	const fx = widgets?.fx;
	if (fx && !fx.needs_key && fx.data) {
		for (const rate of fx.data.rates) {
			const quote = fxQuote(rate.rate, rate.previous);
			items.push({ key: `f:${rate.code}`, group: 'fx', symbol: quote.per === 1 ? rate.code : `${quote.per} ${rate.code}`, price: quote.value, currency: fx.data.base, change: quote.change });
		}
	}
	return items;
}

/**
 * Greedy pages for a column: how many cards fit under each other in `avail`
 * pixels. Order is kept; a card taller than the room still gets a page of its own.
 */
export function packColumn(heights: number[], avail: number, gap: number): number[][] {
	const pages: number[][] = [];
	let used = 0;
	for (let i = 0; i < heights.length; i += 1) {
		const page = pages[pages.length - 1];
		if (page && used + gap + heights[i] <= avail) {
			page.push(i);
			used += gap + heights[i];
		} else {
			pages.push([i]);
			used = heights[i];
		}
	}
	return pages;
}

/**
 * Greedy pages for a row: items side by side in `avail` pixels, a group's
 * label written wherever a page starts or the group changes. Order is kept;
 * an item wider than the room still gets a page of its own.
 */
export function packRow(widths: number[], groups: string[], labels: Record<string, number>, avail: number, gap: number): number[][] {
	const pages: number[][] = [];
	let used = 0;
	for (let i = 0; i < widths.length; i += 1) {
		const page = pages[pages.length - 1];
		const prev = page ? groups[page[page.length - 1]] : null;
		const extra = prev === groups[i] ? 0 : (labels[groups[i]] ?? 0) + gap;
		if (page && used + gap + extra + widths[i] <= avail) {
			page.push(i);
			used += gap + extra + widths[i];
		} else {
			pages.push([i]);
			used = (labels[groups[i]] ?? 0) + gap + widths[i];
		}
	}
	return pages;
}

/**
 * How many rows a card may show in a rail `room` pixels high (about 44 px a row, 60 for the title and
 * the source line): on a short screen, a long list is cut in more pages instead of being clipped.
 */
export function rowsThatFit(room: number): number {
	if (!(room > 0)) return Infinity;
	return Math.max(1, Math.floor((room - 60) / 44));
}

/** How much a page may grow (to fill the rail) or must shrink (to fit it). */
export function fitScale(content: number, avail: number, min = 0.75, max = 1.2): number {
	if (!(content > 0) || !(avail > 0)) return 1;
	return Math.min(max, Math.max(min, avail / content));
}

/**
 * Initials for a person's badge: one letter from each of the first two parts
 * of a name (`Ada Lovelace`, `ada.lovelace@x.io` give `AL`), two from a single one.
 */
export function initialsOf(name: string): string {
	const parts = name.split(/[\s._@+-]+/u).filter(Boolean);
	if (parts.length === 0) return '?';
	const letters = parts.length > 1 ? [parts[0], parts[1]].map((part) => Array.from(part)[0]) : Array.from(parts[0]).slice(0, 2);
	return letters.join('').toLocaleUpperCase();
}

/**
 * A hue (0-359) derived from a name: the same on every display and every day.
 * The badge is dark enough for white initials at every hue (see `PersonBadge`).
 */
export function hueOf(name: string): number {
	let hash = 2166136261;
	for (const char of name.trim().toLowerCase()) {
		hash ^= char.codePointAt(0) ?? 0;
		hash = Math.imul(hash, 16777619) >>> 0;
	}
	return hash % 360;
}

/** The next card's index; keeps a valid index when the deck shrinks. */
export function nextCard(index: number, count: number): number {
	if (count <= 0) return 0;
	return (index + 1) % count;
}

/** True when a section's data is older than three times its usual refresh. */
export function isStale(updated: string | null, now: number, refreshMs: number): boolean {
	if (!updated) return false;
	const at = Date.parse(`${updated}Z`);
	return Number.isFinite(at) && now - at > refreshMs * 3;
}
