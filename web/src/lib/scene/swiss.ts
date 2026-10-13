/**
 * Swiss extras shared by the Swiss scenes: the station clock's time (Europe/Zurich)
 * and which holiday or season the day belongs to. Pure functions, no DOM.
 * `?date=YYYY-MM-DD` on the page URL forces the calendar day to check a scene by eye.
 */

export interface SwissDay {
	/** 1 August, the national day: fireworks. */
	nationalDay: boolean;
	/** Around 12 December, the Escalade: the chocolate cauldron (Geneva). */
	escalade: boolean;
	/** December to February: snow. */
	winter: boolean;
}

export interface CalendarDay {
	y: number;
	m: number;
	d: number;
}

const ZONE = 'Europe/Zurich';

const parts = new Intl.DateTimeFormat('en-GB', {
	timeZone: ZONE,
	year: 'numeric',
	month: 'numeric',
	day: 'numeric',
	hour: 'numeric',
	minute: 'numeric',
	second: 'numeric',
	hourCycle: 'h23'
});

function zurichParts(date: Date): Record<string, number> {
	const out: Record<string, number> = {};
	for (const p of parts.formatToParts(date)) if (p.type !== 'literal') out[p.type] = Number(p.value);
	return out;
}

/** Wall-clock time in Zurich; `ms` is the sub-second part of the instant. */
export function zurichClock(date: Date): { h: number; m: number; s: number; ms: number } {
	const p = zurichParts(date);
	return { h: p.hour % 24, m: p.minute, s: p.second, ms: ((date.getTime() % 1000) + 1000) % 1000 };
}

/** The calendar day in Zurich. */
export function zurichDay(date: Date): CalendarDay {
	const p = zurichParts(date);
	return { y: p.year, m: p.month, d: p.day };
}

/** `YYYY-MM-DD` (a real date) from a query string such as `?date=2026-08-01`, else null. */
export function parseForcedDate(search: string): CalendarDay | null {
	const raw = new URLSearchParams(search).get('date');
	const match = raw && /^(\d{4})-(\d{2})-(\d{2})$/.exec(raw);
	if (!match) return null;
	const [y, m, d] = [Number(match[1]), Number(match[2]), Number(match[3])];
	const probe = new Date(Date.UTC(y, m - 1, d));
	if (probe.getUTCFullYear() !== y || probe.getUTCMonth() !== m - 1 || probe.getUTCDate() !== d)
		return null;
	return { y, m, d };
}

/** The Escalade is celebrated on the weekend around 12 December: 10 to 14. */
export const ESCALADE_FROM = 10;
export const ESCALADE_TO = 14;

export function swissDayFor({ m, d }: CalendarDay): SwissDay {
	return {
		nationalDay: m === 8 && d === 1,
		escalade: m === 12 && d >= ESCALADE_FROM && d <= ESCALADE_TO,
		winter: m === 12 || m === 1 || m === 2
	};
}

/** Today's flags, unless the query string forces a day. */
export function resolveSwissDay(now: Date, search = ''): SwissDay {
	return swissDayFor(parseForcedDate(search) ?? zurichDay(now));
}

/** Seconds hand: a lap in 58 s, then a stop at 12 for the rest of the minute. */
export const SECONDS_LAP_MS = 58_000;

/** Clock angles in degrees (0 = 12 o'clock, clockwise). */
export function clockAngles(date: Date, smooth = true) {
	const { h, m, s, ms } = zurichClock(date);
	const sinceMinute = s * 1000 + ms;
	const lap = Math.min(sinceMinute / SECONDS_LAP_MS, 1);
	return {
		hour: ((h % 12) + m / 60) * 30,
		minute: m * 6,
		// Without animation the hand moves once a second, as on a still picture.
		second: smooth ? lap * 360 : Math.min(s / 58, 1) * 360
	};
}
