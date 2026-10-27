/**
 * Layout of a dashboard: a 12-column grid, widgets measured in cells.
 *
 * Pure functions, no DOM: the grid component only turns pointer and key
 * events into calls here. Layout is vertical-gravity like most dashboard
 * grids: after a change every widget sits as high as it can without
 * overlapping the ones already placed, and the widget being moved wins any
 * conflict.
 */

export const COLUMNS = 12;
export const MAX_HEIGHT = 24;

export interface Cell {
	id: string;
	x: number;
	y: number;
	w: number;
	h: number;
}

export function overlaps(a: Cell, b: Cell): boolean {
	return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

/** Keeps a cell inside the grid and at a sane size. */
export function clampCell<T extends Cell>(cell: T, minW = 1, minH = 1): T {
	const w = Math.min(COLUMNS, Math.max(minW, Math.round(cell.w)));
	const h = Math.min(MAX_HEIGHT, Math.max(minH, Math.round(cell.h)));
	const x = Math.min(COLUMNS - w, Math.max(0, Math.round(cell.x)));
	const y = Math.max(0, Math.round(cell.y));
	return { ...cell, x, y, w, h };
}

/** Reading order: top to bottom, then left to right. */
export function byPosition<T extends Cell>(a: T, b: T): number {
	return a.y - b.y || a.x - b.x;
}

/**
 * Packs widgets upwards. `pinned` (the one being dragged or resized) keeps its
 * cell; the others are placed in reading order at the first free row.
 */
export function compact<T extends Cell>(cells: readonly T[], pinned?: string): T[] {
	const placed: T[] = [];
	const first = pinned ? cells.find((c) => c.id === pinned) : undefined;
	if (first) placed.push(clampCell(first));
	const rest = cells.filter((c) => c.id !== pinned).sort(byPosition);
	for (const cell of rest) {
		const next = clampCell(cell);
		let y = 0;
		while (placed.some((p) => overlaps({ ...next, y }, p))) y += 1;
		placed.push({ ...next, y });
	}
	// Same order as the input, so callers can keep their keyed list stable.
	return cells.map((c) => placed.find((p) => p.id === c.id)!);
}

export function rows(cells: readonly Cell[]): number {
	return cells.reduce((max, c) => Math.max(max, c.y + c.h), 0);
}

/** Moves one widget to a cell; the others make room. */
export function moveTo<T extends Cell>(cells: readonly T[], id: string, x: number, y: number): T[] {
	return compact(
		cells.map((c) => (c.id === id ? { ...c, x, y } : c)),
		id
	);
}

/** Resizes one widget; the others make room. */
export function resizeTo<T extends Cell>(
	cells: readonly T[],
	id: string,
	w: number,
	h: number,
	min: { w: number; h: number } = { w: 1, h: 1 }
): T[] {
	return compact(
		cells.map((c) => {
			if (c.id !== id) return c;
			const width = Math.min(COLUMNS - c.x, Math.max(min.w, Math.round(w)));
			return { ...c, w: width, h: Math.min(MAX_HEIGHT, Math.max(min.h, Math.round(h))) };
		}),
		id
	);
}

/** First position for a new widget: below everything, full left. */
export function slotFor(cells: readonly Cell[]): { x: number; y: number } {
	return { x: 0, y: rows(cells) };
}

/** Widgets in the order a single column shows them. */
export function stackOrder<T extends Cell>(cells: readonly T[]): T[] {
	return [...cells].sort(byPosition);
}

/**
 * Lays widgets out again in a given reading order, keeping their sizes: each
 * one goes to the first free spot after the previous one. This is how the
 * single-column (phone) view reorders without wrecking the wide layout.
 */
export function flow<T extends Cell>(order: readonly T[]): T[] {
	const placed: T[] = [];
	let cursor = { x: 0, y: 0 };
	for (const cell of order) {
		const next = clampCell(cell);
		let { x, y } = cursor;
		if (x + next.w > COLUMNS) {
			x = 0;
			y += 1;
		}
		for (;;) {
			const candidate = { ...next, x, y };
			if (!placed.some((p) => overlaps(candidate, p))) {
				placed.push(candidate);
				cursor = { x, y };
				break;
			}
			x += 1;
			if (x + next.w > COLUMNS) {
				x = 0;
				y += 1;
			}
		}
	}
	return order.map((c) => placed.find((p) => p.id === c.id)!);
}

/** Swaps a widget with its neighbour in reading order (`-1` up, `1` down). */
export function shift<T extends Cell>(cells: readonly T[], id: string, by: -1 | 1): T[] {
	const order = stackOrder(cells);
	const from = order.findIndex((c) => c.id === id);
	const to = from + by;
	if (from < 0 || to < 0 || to >= order.length) return [...cells];
	[order[from], order[to]] = [order[to], order[from]];
	return flow(order);
}
