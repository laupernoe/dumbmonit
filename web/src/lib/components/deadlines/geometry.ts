/**
 * Pure maths of the deadline timeline: which tier a number of days falls in,
 * where a marker sits on the 90-day axis and on which lane, so neighbours do
 * not hide each other. No imports from the app: tested on its own.
 */
import type { Tone } from '../../viz/types.ts';

/** Horizon of the timeline; later deadlines only appear in the list. */
export const HORIZON_DAYS = 90;
/** Alert tiers drawn on the axis (the server rules fire at 30, 14 and 7 days). */
export const TIER_DAYS = [7, 14, 30, 60] as const;

/**
 * Tone of a deadline: red at 7 days or less (or expired), amber up to 30, blue
 * up to 60, calm beyond. The glyph and the words carry the same information.
 */
export function deadlineTone(daysLeft: number): Tone {
	if (daysLeft <= 7) return 'warning';
	if (daysLeft <= 30) return 'advisory';
	if (daysLeft <= 60) return 'info';
	return 'signal';
}

/** Position on the axis, 0 to 1. Expired items stick to the left edge. */
export function axisPosition(daysLeft: number, horizon = HORIZON_DAYS): number {
	if (!Number.isFinite(daysLeft) || horizon <= 0) return 0;
	return Math.min(1, Math.max(0, daysLeft / horizon));
}

export interface Placed<T> {
	item: T;
	/** 0 to 1 along the axis. */
	at: number;
	lane: number;
}

/**
 * Greedy lane assignment: items come sorted by date; each takes the first lane
 * whose last marker is at least `gap` (a fraction of the axis) to its left.
 * Beyond `maxLanes` the last lane is shared rather than growing the band.
 */
export function placeOnLanes<T>(
	items: T[],
	daysOf: (item: T) => number,
	options: { horizon?: number; gap?: number; maxLanes?: number } = {}
): Placed<T>[] {
	const { horizon = HORIZON_DAYS, gap = 0.06, maxLanes = 4 } = options;
	const lastAt: number[] = [];
	const out: Placed<T>[] = [];
	for (const item of items) {
		const days = daysOf(item);
		if (days > horizon) continue;
		const at = axisPosition(days, horizon);
		let lane = lastAt.findIndex((last) => at - last >= gap);
		if (lane === -1) lane = lastAt.length < maxLanes ? lastAt.length : maxLanes - 1;
		lastAt[lane] = at;
		out.push({ item, at, lane });
	}
	return out;
}
