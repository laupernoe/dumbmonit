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

/** `2026-10-16T19:00` (the place's own clock) as an hour in the reader's locale. */
export function localHour(time: string, locale: string): string {
	const date = new Date(`${time.slice(0, 16)}:00Z`);
	if (Number.isNaN(date.getTime())) return time.slice(11, 16);
	return date.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit', timeZone: 'UTC' });
}

/** `2026-10-17` as a short weekday in the reader's locale. */
export function weekday(date: string, locale: string): string {
	const day = new Date(`${date.slice(0, 10)}T12:00:00Z`);
	if (Number.isNaN(day.getTime())) return date;
	return day.toLocaleDateString(locale, { weekday: 'short', timeZone: 'UTC' });
}

export function roundTemperature(value: number): string {
	return `${Math.round(value)}°`;
}

/** One card of the rotating deck. Long lists are split into pages. */
export type WidgetCard =
	| { key: string; kind: 'weather'; place: number }
	| { key: string; kind: 'crypto' | 'stocks' | 'fx' | 'cve'; from: number; to: number; page: number; pages: number };

/** Rows per card, so a page always fits the column and reads from the sofa. */
export const ROWS_PER_CARD = { crypto: 5, stocks: 5, fx: 5, cve: 4 } as const;

function rowsOf(widgets: WallWidgets, kind: keyof typeof ROWS_PER_CARD): number | null {
	const section = widgets[kind];
	if (!section) return null;
	const data = section.data;
	if (!data) return 0;
	if (kind === 'crypto') return (data as { coins: unknown[] }).coins.length;
	if (kind === 'stocks') return (data as { quotes: unknown[] }).quotes.length;
	if (kind === 'fx') return (data as { rates: unknown[] }).rates.length;
	return (data as unknown[]).length;
}

/**
 * The deck, in a fixed order: each place's weather, then coins, stocks, rates
 * and vulnerabilities. A widget turned on but unavailable still gets its card
 * (it says so); a widget turned off gets none.
 */
export function widgetCards(widgets: WallWidgets | null): WidgetCard[] {
	if (!widgets) return [];
	const cards: WidgetCard[] = [];
	if (widgets.weather) {
		const places = widgets.weather.data?.places.length ?? 0;
		if (places === 0) cards.push({ key: 'weather:0', kind: 'weather', place: 0 });
		for (let i = 0; i < places; i += 1) cards.push({ key: `weather:${i}`, kind: 'weather', place: i });
	}
	for (const kind of ['crypto', 'stocks', 'fx', 'cve'] as const) {
		const rows = rowsOf(widgets, kind);
		if (rows === null) continue;
		const size = ROWS_PER_CARD[kind];
		const pages = Math.max(1, Math.ceil(rows / size));
		for (let page = 0; page < pages; page += 1) {
			cards.push({ key: `${kind}:${page}`, kind, from: page * size, to: Math.min(rows, (page + 1) * size), page, pages });
		}
	}
	return cards;
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
