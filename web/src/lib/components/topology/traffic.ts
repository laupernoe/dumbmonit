/**
 * Weathermap: what a link's traffic looks like on the map. Pure functions (no
 * DOM, no Svelte, no messages), so the canvas, the list, the phone fallback
 * and the tests share one reading of `EdgeTraffic`.
 *
 * Load is never shown by colour alone: the thickness follows it, the label
 * spells the rate and the percentage, a link that loses packets is dashed.
 */
import type { PortTraffic, TopologyEdgeTraffic } from '#lib/api/index.js';

/** Load bands, from the legend: 0–25, 25–50, 50–75, 75–100 %. `unknown`: no port speed to compare with. */
export type Band = 'idle' | 'low' | 'mid' | 'high' | 'full' | 'unknown';

/** Upper limits (percent) of the first three bands; above the last, `full`. */
export const BAND_LIMITS = [25, 50, 75] as const;

/** Below this rate a link is idle: nothing flows on it. */
const IDLE_BPS = 1000;
/** Errors and discards per second from which a link is drawn dashed (about 3 a minute). */
export const ERROR_RATE = 0.05;

export interface LinkLoad {
	/** Busiest direction against the port speed, in percent; `null` without a speed. */
	pct: number | null;
	/** Rate of the busiest direction, bits per second. */
	bps: number;
	band: Band;
	/** Direction of the dominant flow along the drawn path (`from` to `to`); `null` when idle. */
	forward: boolean | null;
	/** The port loses frames. */
	errors: boolean;
	/** Stroke width of the link. */
	width: number;
}

export function bandOf(pct: number | null): Band {
	if (pct === null) return 'unknown';
	if (pct < 1) return 'idle';
	if (pct < BAND_LIMITS[0]) return 'low';
	if (pct < BAND_LIMITS[1]) return 'mid';
	if (pct < BAND_LIMITS[2]) return 'high';
	return 'full';
}

/** Width of a link: from a hairline for an idle one to a thick pipe at 100 %. */
export function widthFor(pct: number | null): number {
	if (pct === null) return 2.2;
	return Math.round((1.6 + (Math.min(100, Math.max(0, pct)) / 100) * 4.4) * 10) / 10;
}

/** How a link looks: `from` is the link's upstream node id (the side the path starts at). */
export function linkLoad(traffic: TopologyEdgeTraffic, from: string): LinkLoad {
	const inbound = traffic.utilization_in;
	const outbound = traffic.utilization_out;
	const pct = inbound === null || outbound === null ? null : Math.max(inbound, outbound) * 100;
	const bps = Math.max(traffic.bps_in, traffic.bps_out);
	const out = traffic.bps_out >= traffic.bps_in;
	const measuredFrom = traffic.measured_on === from;
	// Sent by the `from` side goes forward; received by it comes backward. Mirrored when measured on `to`.
	const forward = bps < IDLE_BPS ? null : out === measuredFrom;
	return {
		pct,
		bps,
		band: bps < IDLE_BPS && pct === null ? 'idle' : bandOf(pct),
		forward,
		errors: traffic.errors_per_sec >= ERROR_RATE,
		width: widthFor(pct)
	};
}

const UNITS = ['b/s', 'kb/s', 'Mb/s', 'Gb/s', 'Tb/s'] as const;

/** `412 Mb/s`: decimal units, three significant digits at most. */
export function formatRate(bps: number, locale?: string): string {
	let value = Math.max(0, bps);
	let unit = 0;
	while (value >= 1000 && unit < UNITS.length - 1) {
		value /= 1000;
		unit++;
	}
	const digits = unit === 0 ? 0 : value >= 100 ? 0 : value >= 10 ? 1 : 2;
	return `${new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(value)} ${UNITS[unit]}`;
}

/** `41 %` (the locale decides the spacing). */
export function formatPct(pct: number, locale?: string): string {
	return new Intl.NumberFormat(locale, { style: 'percent', maximumFractionDigits: pct < 10 ? 1 : 0 }).format(pct / 100);
}

/** `412 Mb/s · 41 %`, or the rate alone when the port speed is unknown. */
export function loadText(load: LinkLoad, locale?: string): string {
	const rate = formatRate(load.bps, locale);
	return load.pct === null ? rate : `${rate} · ${formatPct(load.pct, locale)}`;
}

/** A link with traffic, for the ranked list. */
export interface RankedLink<T> {
	edge: T;
	load: LinkLoad;
}

/** The busiest links first (by load, then by rate), at most `limit`; links without traffic are left out. */
export function mostLoaded<T extends { from: string; traffic?: TopologyEdgeTraffic | null }>(edges: T[], limit = Infinity): RankedLink<T>[] {
	return edges
		.filter((e): e is T & { traffic: TopologyEdgeTraffic } => !!e.traffic)
		.map((edge) => ({ edge, load: linkLoad(edge.traffic, edge.from) }))
		.sort((a, b) => (b.load.pct ?? -1) - (a.load.pct ?? -1) || b.load.bps - a.load.bps)
		.slice(0, limit);
}

/** The two polylines of a one-hour curve, in a `width` by `height` box sharing one scale. */
export function sparkPaths(points: PortTraffic['points'], width: number, height: number): { inbound: string; outbound: string; max: number } {
	if (points.length === 0) return { inbound: '', outbound: '', max: 0 };
	const max = Math.max(1, ...points.flatMap((p) => [p.bps_in, p.bps_out]));
	const first = points[0].t;
	const span = Math.max(1, points[points.length - 1].t - first);
	const line = (pick: (p: PortTraffic['points'][number]) => number) =>
		points
			.map((p, i) => {
				const x = points.length === 1 ? width : ((p.t - first) / span) * width;
				const y = height - (pick(p) / max) * height;
				return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
			})
			.join(' ');
	return { inbound: line((p) => p.bps_in), outbound: line((p) => p.bps_out), max };
}
