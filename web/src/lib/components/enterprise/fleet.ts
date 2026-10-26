/**
 * Fleet-wide figures for the Enterprise console, a handful of queries for the
 * whole estate rather than one per monitor: the health river (every monitor,
 * slotted over the range), latency sparklines, p95 and 30-day availability.
 *
 * Every series carries the `target` label (the target id); multi-location
 * monitors have one series per place, folded here with the worst value.
 */
import { queryInstant, queryRange, type AlertHistoryEntry, type MetricSeries, type TargetId } from '#lib/api/index.js';
import { parseServerDate } from '#lib/format.js';

/** One slot of the river: every check passed, some failed, all failed, nothing measured. */
export type RiverCell = 'up' | 'warn' | 'down' | 'none';

export const RIVER_SLOTS = 72;
export const SPARK_POINTS = 28;

/** Probes publish `probe_success`; devices only `up`. Both are 1 when healthy. */
const HEALTH = '{__name__=~"dumbmonit_probe_success|dumbmonit_up"}';

function targetOf(serie: MetricSeries): TargetId | null {
	const id = Number(serie.metric?.target);
	return Number.isFinite(id) && serie.metric?.target !== undefined ? id : null;
}

function numbers(serie: MetricSeries): [number, number][] {
	const points: [number, number][] = [];
	for (const [ts, raw] of serie.values ?? []) {
		const value = Number(raw);
		if (Number.isFinite(value)) points.push([ts, value]);
	}
	return points.sort((a, b) => a[0] - b[0]);
}

function instant(series: MetricSeries[]): Map<TargetId, number> {
	const out = new Map<TargetId, number>();
	for (const serie of series) {
		const id = targetOf(serie);
		const value = Number(serie.values?.[0]?.[1]);
		if (id === null || !Number.isFinite(value)) continue;
		out.set(id, value);
	}
	return out;
}

export function cellOf(value: number | undefined): RiverCell {
	if (value === undefined) return 'none';
	if (value >= 0.999) return 'up';
	if (value <= 0.001) return 'down';
	return 'warn';
}

/** Every monitor's health over the range, in `RIVER_SLOTS` equal slots, oldest first. */
export async function loadRiver(rangeSeconds: number, signal?: AbortSignal): Promise<Map<TargetId, RiverCell[]>> {
	const step = Math.max(15, Math.round(rangeSeconds / RIVER_SLOTS));
	const end = Date.now();
	const start = end - step * RIVER_SLOTS * 1000;
	const series = await queryRange(
		{ query: `min by (target) (avg_over_time(${HEALTH}[${step}s]))`, start, end, step },
		signal
	);
	const first = Math.floor(start / 1000 / step) * step;
	const out = new Map<TargetId, RiverCell[]>();
	for (const serie of series) {
		const id = targetOf(serie);
		if (id === null) continue;
		const bySlot = new Map<number, number>();
		for (const [ts, value] of numbers(serie)) {
			const slot = Math.floor((ts - first) / step);
			const known = bySlot.get(slot);
			bySlot.set(slot, known === undefined ? value : Math.min(known, value));
		}
		const cells: RiverCell[] = [];
		for (let i = 1; i <= RIVER_SLOTS; i += 1) cells.push(cellOf(bySlot.get(i)));
		out.set(id, cells);
	}
	return out;
}

/** Response time of each monitor over the range, in milliseconds, about `SPARK_POINTS` points. */
export async function loadLatency(rangeSeconds: number, signal?: AbortSignal): Promise<Map<TargetId, number[]>> {
	const step = Math.max(15, Math.round(rangeSeconds / SPARK_POINTS));
	const end = Date.now();
	const series = await queryRange(
		{
			query: 'max by (target) (dumbmonit_probe_duration_seconds)',
			start: end - rangeSeconds * 1000,
			end,
			step
		},
		signal
	);
	const out = new Map<TargetId, number[]>();
	for (const serie of series) {
		const id = targetOf(serie);
		if (id === null) continue;
		const points = numbers(serie).map(([, value]) => value * 1000);
		if (points.length > 0) out.set(id, points);
	}
	return out;
}

export interface FleetFigures {
	/** p95 response time per monitor over the range, in milliseconds. */
	p95: Map<TargetId, number>;
	/** Availability per monitor over 30 days, in percent. */
	uptime30: Map<TargetId, number>;
	/** p95 across the fleet over the range, in milliseconds; `null` without probes. */
	fleetP95: number | null;
}

export async function loadFigures(rangeSeconds: number, signal?: AbortSignal): Promise<FleetFigures> {
	const window = `${Math.max(60, Math.round(rangeSeconds))}s`;
	const [p95, uptime, fleet] = await Promise.all([
		queryInstant(`max by (target) (quantile_over_time(0.95, dumbmonit_probe_duration_seconds[${window}])) * 1000`, signal),
		queryInstant(`min by (target) (avg_over_time(${HEALTH}[30d])) * 100`, signal),
		queryInstant(`quantile(0.95, quantile_over_time(0.95, dumbmonit_probe_duration_seconds[${window}])) * 1000`, signal)
	]);
	const fleetValue = Number(fleet[0]?.values?.[0]?.[1]);
	return {
		p95: instant(p95),
		uptime30: instant(uptime),
		fleetP95: Number.isFinite(fleetValue) ? fleetValue : null
	};
}

/** Mean of the values present; `null` when there are none. */
export function mean(values: Iterable<number>): number | null {
	let sum = 0;
	let count = 0;
	for (const value of values) {
		if (!Number.isFinite(value)) continue;
		sum += value;
		count += 1;
	}
	return count === 0 ? null : sum / count;
}

/**
 * Mean time to resolve, in seconds, from the alert history: for each alert,
 * the time from its firing to its resolution. `null` when nothing resolved.
 */
export function meanTimeToResolve(history: AlertHistoryEntry[]): number | null {
	const firedAt = new Map<string, number>();
	const spans: number[] = [];
	const ordered = [...history].sort((a, b) => (parseServerDate(a.at)?.getTime() ?? 0) - (parseServerDate(b.at)?.getTime() ?? 0));
	for (const entry of ordered) {
		const at = parseServerDate(entry.at)?.getTime();
		if (at === undefined) continue;
		if (entry.to_phase === 'firing') firedAt.set(entry.fingerprint, at);
		else if (entry.to_phase === 'resolved') {
			const since = firedAt.get(entry.fingerprint);
			if (since !== undefined && at >= since) spans.push((at - since) / 1000);
			firedAt.delete(entry.fingerprint);
		}
	}
	return mean(spans);
}

/** SVG path of a sparkline in a `width` × `height` box; empty without two points. */
export function sparkPath(values: number[], width: number, height: number, pad = 2): string {
	if (values.length < 2) return '';
	const min = Math.min(...values);
	const max = Math.max(...values);
	const span = max - min || 1;
	const inner = height - pad * 2;
	return values
		.map((value, i) => {
			const x = (i * width) / (values.length - 1);
			const y = pad + inner - ((value - min) / span) * inner;
			return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)} ${y.toFixed(1)}`;
		})
		.join(' ');
}
