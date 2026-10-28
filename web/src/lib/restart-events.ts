/**
 * Restarts read from stored series, for the markers of a device page's charts.
 * Pure: the page fetches the range, this module reads it. Tested in
 * `tests/restart-events.test.mjs`.
 *
 * - host (agent): `dumbmonit_uptime_seconds` is a gauge of seconds since boot; a
 *   sample lower than the one before is a reboot, dated by its boot instant
 *   (`sample time - uptime`) when that falls between the two samples;
 * - container: `dumbmonit_container_restart_count` (label `container`) goes up
 *   at each restart of the same container; the marker sits on the first sample that
 *   shows the new count. A count that falls means a recreated container: no marker.
 */
export const UPTIME_METRIC = 'dumbmonit_uptime_seconds';
export const CONTAINER_RESTART_METRIC = 'dumbmonit_container_restart_count';

/** Series of a range query, as `queryRange` hands them over. */
export interface RangeSeries {
	metric: Record<string, string>;
	values: [number, string][];
}

export interface RestartEvent {
	/** Unix seconds. */
	at: number;
	kind: 'host' | 'container';
	/** The container's name; empty for the host. */
	name: string;
}

/** More markers than this only clutter a chart: the newest are kept. */
export const MAX_RESTARTS = 40;
/** A fall under this many seconds is sampling noise, not a reboot. */
const UPTIME_SLACK = 60;

function points(series: RangeSeries): [number, number][] {
	const out: [number, number][] = [];
	for (const [t, raw] of series.values) {
		const v = Number(raw);
		if (Number.isFinite(t) && Number.isFinite(v)) out.push([t, v]);
	}
	return out.sort((a, b) => a[0] - b[0]);
}

export function restartEvents(series: RangeSeries[]): RestartEvent[] {
	const events: RestartEvent[] = [];
	for (const one of series) {
		const name = one.metric.__name__ ?? '';
		const pts = points(one);
		if (name === UPTIME_METRIC) {
			for (let i = 1; i < pts.length; i++) {
				const [t, v] = pts[i];
				const [before, previous] = pts[i - 1];
				if (v >= previous - UPTIME_SLACK) continue;
				const boot = t - v;
				events.push({ at: Math.round(boot > before && boot <= t ? boot : t), kind: 'host', name: '' });
			}
		} else if (name === CONTAINER_RESTART_METRIC) {
			const container = one.metric.container ?? '';
			for (let i = 1; i < pts.length; i++) {
				if (pts[i][1] > pts[i - 1][1]) events.push({ at: Math.round(pts[i][0]), kind: 'container', name: container });
			}
		}
	}
	events.sort((a, b) => a.at - b.at);
	return events.slice(-MAX_RESTARTS);
}
