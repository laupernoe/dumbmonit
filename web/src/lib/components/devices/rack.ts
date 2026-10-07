/**
 * Ordering of the rack: which unit sits where, and how deep in its bay.
 *
 * Parents come first, children stack right under them. Among siblings the
 * units that need attention rise to the top, then names sort alphabetically:
 * on a wall screen the trouble is always at the top of the rack.
 */
import type { Target, TargetId } from '#lib/api/index.js';
import type { TargetState } from '#lib/format.js';

export interface RackRow {
	target: Target;
	state: TargetState;
	depth: number;
	/** An ancestor is unreachable: this unit's alerts are suppressed. */
	shadowed: boolean;
}

/** A row's live screen position, read once when a drag starts (rows never reflow mid-drag). */
export interface RowRect {
	id: TargetId;
	top: number;
	height: number;
}

/**
 * The row a dragged pointer sits nearest to, and whether it should land
 * above or below it — the geometry half of pointer-based drag-and-drop.
 * `rects` must be in on-screen (top to bottom) order. `null` when there is
 * nothing to drop against.
 */
export function nearestRow(rects: RowRect[], pointerY: number): { id: TargetId; before: boolean } | null {
	for (const r of rects) {
		if (pointerY < r.top + r.height / 2) return { id: r.id, before: true };
	}
	const last = rects[rects.length - 1];
	return last ? { id: last.id, before: false } : null;
}

/** States the operator has to act on. `pending` only waits. */
export function needsAttention(state: TargetState): boolean {
	return state === 'offline' || state === 'down' || state === 'misconfigured';
}

export const RANK: Record<TargetState, number> = {
	offline: 0,
	down: 0,
	misconfigured: 0,
	pending: 1,
	unknown: 1,
	online: 2,
	disabled: 3
};

/**
 * Orders two siblings (same parent, same folder): state tier first — a
 * device needing attention always floats above a quiet one, manual order
 * notwithstanding — then the manual `position`, then the name.
 */
export function compareSiblings(
	a: Target,
	b: Target,
	states: Map<TargetId, TargetState>
): number {
	const rank = RANK[states.get(a.id)!] - RANK[states.get(b.id)!];
	if (rank !== 0) return rank;
	const position = a.position - b.position;
	if (position !== 0) return position;
	return a.name.localeCompare(b.name, 'en', { sensitivity: 'base' });
}

/**
 * Flattens the device tree into display rows.
 *
 * `visible` is the set kept by the current filters. A hidden parent does not
 * hide its children: they are promoted to the depth of the nearest visible
 * ancestor (or the top of the rack), so a filter never loses a device.
 */
export function buildRack(
	targets: Target[],
	stateOf: (target: Target) => TargetState,
	visible: Set<TargetId>
): RackRow[] {
	const byId = new Map(targets.map((t) => [t.id, t]));
	const children = new Map<TargetId | null, Target[]>();
	for (const target of targets) {
		// A parent that no longer exists is treated as no parent at all.
		const parent = target.parent_id !== null && byId.has(target.parent_id) ? target.parent_id : null;
		const list = children.get(parent) ?? [];
		list.push(target);
		children.set(parent, list);
	}

	const states = new Map(targets.map((t) => [t.id, stateOf(t)]));
	const sort = (list: Target[]) => list.sort((a, b) => compareSiblings(a, b, states));

	const rows: RackRow[] = [];
	const seen = new Set<TargetId>();

	const walk = (parent: TargetId | null, depth: number, shadowed: boolean) => {
		for (const target of sort(children.get(parent) ?? [])) {
			// Guards against a cycle the server would not normally allow.
			if (seen.has(target.id)) continue;
			seen.add(target.id);
			const state = states.get(target.id)!;
			const shown = visible.has(target.id);
			if (shown) rows.push({ target, state, depth, shadowed });
			walk(target.id, shown ? depth + 1 : depth, shadowed || state === 'offline');
		}
	};
	walk(null, 0, false);
	return rows;
}
