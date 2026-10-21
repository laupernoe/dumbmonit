/**
 * What period the history bars of a public status page really cover, and how
 * one bar is named. The server picks the bar size (`step_secs`) from the page's
 * period and the age of its data; this only turns that into words and keys.
 */

export interface PeriodParts {
	unit: 'hours' | 'days';
	count: number;
}

/** The period in whole hours up to two days, in days beyond. */
export function periodParts(windowSecs: number): PeriodParts {
	const hours = Math.max(1, Math.round(windowSecs / 3600));
	return hours <= 48 ? { unit: 'hours', count: hours } : { unit: 'days', count: Math.round(windowSecs / 86400) };
}

/** One bar per day (or more): the bar carries a date, not a time. */
export function isDayBar(stepSecs: number): boolean {
	return stepSecs >= 86400;
}

/** `YYYY-MM-DD` (UTC) of a bar, to ask for that day's detail. */
export function barDay(date: string): string {
	return date.slice(0, 10);
}

/** Periods a page can choose, in days: 24 hours, then 7, 30, 60 and 90 days. */
export const PERIOD_CHOICES = [1, 7, 30, 60, 90] as const;
