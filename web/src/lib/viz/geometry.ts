/** Pure helpers of the visual kit (no DOM, no `#lib` import: `node --test` runs them). */
import type { Cell } from './types.ts';

export function clamp(value: number, min: number, max: number): number {
	return Math.min(max, Math.max(min, value));
}

/** Position of `value` in [min, max], clamped to 0..1; 0 when the range is empty. */
export function unit01(value: number, min: number, max: number): number {
	return max > min ? clamp((value - min) / (max - min), 0, 1) : 0;
}

/** Share of the signal token mixed into the surface, per step of the sequential ramp. */
export const RAMP_STEPS = [8, 24, 42, 62, 84] as const;

/** Step 0..4 of the ramp for a position 0..1. */
export function rampLevel(t: number): number {
	return Math.min(RAMP_STEPS.length - 1, Math.floor(clamp(t, 0, 1) * RAMP_STEPS.length));
}

/**
 * Sequential colour: the signal token blended into the surface, so the ramp
 * follows the theme (light, dark, wall) with no token of its own.
 */
export function rampColor(t: number): string {
	return `color-mix(in oklab, var(--c-signal) ${RAMP_STEPS[rampLevel(t)]}%, var(--c-surface))`;
}

/** Min and max of the finite values; null when there are none. */
export function extent(values: Iterable<number | null | undefined>): [number, number] | null {
	let lo = Infinity;
	let hi = -Infinity;
	for (const v of values) {
		if (v == null || !Number.isFinite(v)) continue;
		if (v < lo) lo = v;
		if (v > hi) hi = v;
	}
	return lo <= hi ? [lo, hi] : null;
}

export interface Matrix {
	rows: string[];
	cols: string[];
	/** `values[r][c]`, null = no data. */
	values: (number | null)[][];
	/** How many source columns each displayed column averages (1 = none). */
	merged: number;
}

/**
 * Lays cells out as a matrix. Above `maxCells` the columns are averaged in
 * buckets so the grid stays a few thousand nodes at most (SVG/DOM stay
 * accessible and exportable; a canvas is not needed for that).
 */
export function toMatrix(rows: string[], cols: string[], cells: Cell[], maxCells = 2000): Matrix {
	const rowIndex = new Map(rows.map((k, i) => [k, i]));
	const colIndex = new Map(cols.map((k, i) => [k, i]));
	const full: (number | null)[][] = rows.map(() => cols.map(() => null));
	for (const cell of cells) {
		const r = rowIndex.get(cell.row);
		const c = colIndex.get(cell.col);
		if (r === undefined || c === undefined) continue;
		full[r][c] = cell.value != null && Number.isFinite(cell.value) ? cell.value : null;
	}
	const total = rows.length * cols.length;
	if (total <= maxCells || cols.length <= 1) return { rows, cols, values: full, merged: 1 };
	const merged = Math.ceil(total / Math.max(1, maxCells));
	const outCols: string[] = [];
	for (let c = 0; c < cols.length; c += merged) outCols.push(cols[c]);
	const values = full.map((line) => {
		const out: (number | null)[] = [];
		for (let c = 0; c < line.length; c += merged) {
			let sum = 0;
			let n = 0;
			for (const v of line.slice(c, c + merged)) {
				if (v != null) {
					sum += v;
					n += 1;
				}
			}
			out.push(n ? sum / n : null);
		}
		return out;
	});
	return { rows, cols: outCols, values, merged };
}

export interface SparkGeometry {
	/** Polyline path; empty under two points. */
	path: string;
	/** Closed path under the line, for a faint fill. */
	area: string;
	min: number;
	max: number;
	last: [number, number] | null;
	/** Points as [x, y] in the box. */
	points: [number, number][];
}

/** Geometry of a sparkline in a `width` × `height` box; y grows downwards. */
export function sparkGeometry(
	values: readonly number[],
	width: number,
	height: number,
	pad = 2,
	floor?: number,
	ceil?: number
): SparkGeometry {
	const finite = values.filter((v) => Number.isFinite(v));
	if (finite.length === 0) return { path: '', area: '', min: 0, max: 0, last: null, points: [] };
	const min = floor ?? Math.min(...finite);
	const max = ceil ?? Math.max(...finite);
	const span = max - min || 1;
	const inner = height - pad * 2;
	const n = finite.length;
	const points = finite.map((v, i): [number, number] => [
		n === 1 ? width / 2 : (i * width) / (n - 1),
		pad + inner - clamp((v - min) / span, 0, 1) * inner
	]);
	const path =
		n < 2 ? '' : points.map(([x, y], i) => `${i === 0 ? 'M' : 'L'}${x.toFixed(1)} ${y.toFixed(1)}`).join(' ');
	const area = path ? `${path} L${width} ${height} L0 ${height} Z` : '';
	return { path, area, min: Math.min(...finite), max: Math.max(...finite), last: points[n - 1], points };
}

/** Point on a circle; angle 0 = left, 1 = right of a semicircle drawn over the top. */
export function semiPoint(cx: number, cy: number, r: number, t: number): [number, number] {
	const angle = Math.PI * (1 - clamp(t, 0, 1));
	return [cx + r * Math.cos(angle), cy - r * Math.sin(angle)];
}

/** SVG arc of the gauge between two positions 0..1. */
export function arcPath(cx: number, cy: number, r: number, from: number, to: number): string {
	const [x1, y1] = semiPoint(cx, cy, r, from);
	const [x2, y2] = semiPoint(cx, cy, r, to);
	return `M${x1.toFixed(2)} ${y1.toFixed(2)} A${r} ${r} 0 0 1 ${x2.toFixed(2)} ${y2.toFixed(2)}`;
}

export interface Delta {
	/** Signed ratio against the previous value; null when it cannot be computed. */
	ratio: number | null;
	direction: 'up' | 'down' | 'flat';
}

/** Change against a previous value. A previous of 0 gives no ratio, only a direction. */
export function delta(current: number, previous: number | null | undefined): Delta {
	if (previous == null || !Number.isFinite(previous)) return { ratio: null, direction: 'flat' };
	const diff = current - previous;
	const direction = Math.abs(diff) < 1e-9 ? 'flat' : diff > 0 ? 'up' : 'down';
	return { ratio: previous === 0 ? null : diff / Math.abs(previous), direction };
}
