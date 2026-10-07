/**
 * Folders: a flat, non-nested grouping of top-level devices on `/targets`.
 *
 * A folder is named by `Target.group_name` (empty: no folder). Only a device's
 * own ancestor-free position in the tree decides which folder section it
 * belongs to — a child always follows its parent, whatever its own
 * `group_name` says, so there is never a second tree alongside parent/child.
 *
 * `buildRack` already produces the correct row order for the whole fleet
 * (state tier, then manual position, then name) in one pass; this module
 * partitions that flat list into folders without touching that order, since a
 * stable partition keeps each folder's internal order exactly as computed.
 */
import type { Target, TargetId } from '#lib/api/index.js';
import type { TargetState } from '#lib/format.js';
// Explicit extension: this module is also loaded straight by Node (`npm test`),
// whose ESM resolver — unlike Vite's — requires it for a relative import.
import { RANK, buildRack, needsAttention, type RackRow } from './rack.ts';

export interface FolderSection {
	/** Empty string: the "no folder" section, always listed first. */
	key: string;
	label: string;
	rows: RackRow[];
}

/** Topmost ancestor of every target (itself, if it has no live parent). */
export function rootOf(targets: Target[]): Map<TargetId, Target> {
	const byId = new Map(targets.map((t) => [t.id, t]));
	const roots = new Map<TargetId, Target>();
	for (const target of targets) {
		let current = target;
		const seen = new Set<TargetId>();
		while (
			current.parent_id !== null &&
			byId.has(current.parent_id) &&
			!seen.has(current.id)
		) {
			seen.add(current.id);
			current = byId.get(current.parent_id)!;
		}
		roots.set(target.id, current);
	}
	return roots;
}

/** Worst (most urgent) state among a folder's rows, for its header badge. */
export function worstState(rows: RackRow[]): TargetState {
	let worst = rows[0]?.state ?? 'disabled';
	for (const row of rows) if (RANK[row.state] < RANK[worst]) worst = row.state;
	return worst;
}

/**
 * Display order of folder sections: the "no folder" section first, then
 * named folders — those needing attention before the quiet ones,
 * alphabetically among themselves. Exported so the page can slot in a
 * freshly-created, still-empty folder at the right spot.
 */
export function compareFolderSections(a: FolderSection, b: FolderSection): number {
	if (a.key === '') return -1;
	if (b.key === '') return 1;
	const attentionA = a.rows.some((r) => needsAttention(r.state));
	const attentionB = b.rows.some((r) => needsAttention(r.state));
	if (attentionA !== attentionB) return attentionA ? -1 : 1;
	return a.label.localeCompare(b.label, 'en', { sensitivity: 'base' });
}

/**
 * Partitions the rack into folder sections, in display order (see
 * `compareFolderSections`).
 */
export function buildFolders(
	targets: Target[],
	stateOf: (target: Target) => TargetState,
	visible: Set<TargetId>
): FolderSection[] {
	const rows = buildRack(targets, stateOf, visible);
	const roots = rootOf(targets);

	const buckets = new Map<string, RackRow[]>();
	for (const row of rows) {
		const key = roots.get(row.target.id)?.group_name || '';
		const bucket = buckets.get(key);
		if (bucket) bucket.push(row);
		else buckets.set(key, [row]);
	}

	const sections = [...buckets.entries()].map(([key, bucketRows]) => ({
		key,
		label: key || 'No folder',
		rows: bucketRows
	}));

	sections.sort(compareFolderSections);
	return sections;
}

/** Folder names already in use, for the "move to folder" picker. */
export function knownFolders(targets: Target[]): string[] {
	return [...new Set(targets.map((t) => t.group_name).filter(Boolean))].sort((a, b) =>
		a.localeCompare(b, 'en', { sensitivity: 'base' })
	);
}

/** The folder a target's own position in the tree puts it in (its topmost ancestor's `group_name`). */
export function folderKeyOf(targets: Target[], target: Target): string {
	return rootOf(targets).get(target.id)?.group_name || '';
}

/**
 * Siblings a device can be reordered against: other children of the same
 * parent, or — for a top-level device — other top-level devices of the same
 * folder. Crossing a folder or a parent boundary is a "move to folder" or
 * "set parent" action, never a reorder.
 */
export function reorderScope(targets: Target[], target: Target): Target[] {
	const byId = new Map(targets.map((t) => [t.id, t]));
	const parentOf = (t: Target): TargetId | null =>
		t.parent_id !== null && byId.has(t.parent_id) ? t.parent_id : null;
	const parent = parentOf(target);
	if (parent !== null) return targets.filter((t) => parentOf(t) === parent);
	const group = target.group_name || '';
	return targets.filter((t) => parentOf(t) === null && (t.group_name || '') === group);
}

/**
 * Moves a device one step up or down among its reorder siblings (same
 * parent, same folder for a root device), never across a state tier — a
 * device needing attention cannot be pushed below a quiet one, or the other
 * way around.
 *
 * Returns the sibling ids in their new order (positions to send to
 * `reorderTargets`), or `null` when the move is not possible.
 */
export function moveWithinScope(
	targets: Target[],
	stateOf: (target: Target) => TargetState,
	id: TargetId,
	delta: -1 | 1
): TargetId[] | null {
	const target = targets.find((t) => t.id === id);
	if (!target) return null;
	const states = new Map(targets.map((t) => [t.id, stateOf(t)]));
	const scope = reorderScope(targets, target)
		.slice()
		.sort((a, b) => {
			const rank = RANK[states.get(a.id)!] - RANK[states.get(b.id)!];
			if (rank !== 0) return rank;
			const position = a.position - b.position;
			if (position !== 0) return position;
			return a.name.localeCompare(b.name, 'en', { sensitivity: 'base' });
		});
	const i = scope.findIndex((t) => t.id === id);
	const j = i + delta;
	if (i < 0 || j < 0 || j >= scope.length) return null;
	if (RANK[states.get(scope[i].id)!] !== RANK[states.get(scope[j].id)!]) return null;
	const next = scope.slice();
	[next[i], next[j]] = [next[j], next[i]];
	return next.map((t) => t.id);
}

/**
 * Drag-and-drop counterpart of `moveWithinScope`: moves `draggedId` to sit
 * where `overId` is, among their shared reorder scope. `null` when the two
 * are not reorderable against each other — a different parent, a different
 * folder, or a different state tier (a device needing attention cannot be
 * dragged below a quiet one).
 *
 * Returns the scope's ids in their new order, ready for `reorderTargets`.
 */
export function reorderByDrop(
	targets: Target[],
	stateOf: (target: Target) => TargetState,
	draggedId: TargetId,
	overId: TargetId
): TargetId[] | null {
	if (draggedId === overId) return null;
	const dragged = targets.find((t) => t.id === draggedId);
	const over = targets.find((t) => t.id === overId);
	if (!dragged || !over) return null;
	const scope = reorderScope(targets, dragged);
	if (!scope.some((t) => t.id === overId)) return null;

	const states = new Map(targets.map((t) => [t.id, stateOf(t)]));
	if (RANK[states.get(draggedId)!] !== RANK[states.get(overId)!]) return null;

	const sorted = scope
		.slice()
		.sort((a, b) => {
			const rank = RANK[states.get(a.id)!] - RANK[states.get(b.id)!];
			if (rank !== 0) return rank;
			const position = a.position - b.position;
			if (position !== 0) return position;
			return a.name.localeCompare(b.name, 'en', { sensitivity: 'base' });
		})
		.map((t) => t.id);

	const from = sorted.indexOf(draggedId);
	const to = sorted.indexOf(overId);
	sorted.splice(from, 1);
	sorted.splice(to, 0, draggedId);
	return sorted;
}
