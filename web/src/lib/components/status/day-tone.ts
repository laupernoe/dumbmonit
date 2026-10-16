/**
 * Tone of a day in the public history bar, from its uptime. The thresholds are
 * shared with the colour key under the services, so the legend always states
 * the bands the bar actually paints.
 */
export type DayTone = 'signal' | 'advisory' | 'warning' | 'ghost';

/** At or above: a good day. */
export const DAY_GOOD = 99.5;
/** At or above (and under `DAY_GOOD`): a day with some downtime; below: a poor one. */
export const DAY_POOR = 95;

export function dayTone(uptime: number | null): DayTone {
	if (uptime === null) return 'ghost';
	if (uptime >= DAY_GOOD) return 'signal';
	if (uptime >= DAY_POOR) return 'advisory';
	return 'warning';
}

