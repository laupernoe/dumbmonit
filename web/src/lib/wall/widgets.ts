/**
 * Pure helpers for the wall widgets: rises and falls (never by colour alone:
 * an arrow and a sign), prices and rates formatted for across the room, WMO
 * weather codes, people's badges. Which panels show, and in what order, is
 * `panels.ts`. No Svelte, no Paraglide: tested by `tests/wall-widgets.test.mjs`.
 */

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

/** True when a section's data is older than three times its usual refresh. */
export function isStale(updated: string | null, now: number, refreshMs: number): boolean {
	if (!updated) return false;
	const at = Date.parse(`${updated}Z`);
	return Number.isFinite(at) && now - at > refreshMs * 3;
}
