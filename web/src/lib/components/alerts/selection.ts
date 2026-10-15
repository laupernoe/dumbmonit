/**
 * Multi-selection of the active alerts list: pure helpers, no DOM, no network.
 * A selectable row is identified by its `key` (a folded row stands for all the
 * sibling alerts it groups); the bulk actions loop over the existing
 * per-alert API calls and report what failed.
 */

export type BulkAction = 'ack' | 'unack' | 'ignore';

/** What the selection needs to know about one visible row. */
export interface SelectableItem {
	key: string;
	/** Fingerprints of the row's alert and of its folded siblings. */
	fingerprints: string[];
	acked: boolean;
	ruleId?: number;
	targetId?: number | null;
}

export type SelectionState = 'none' | 'some' | 'all';

/** A new set with `key` flipped (or forced with `on`). */
export function toggleKey(selected: ReadonlySet<string>, key: string, on?: boolean): Set<string> {
	const next = new Set(selected);
	const want = on ?? !next.has(key);
	if (want) next.add(key);
	else next.delete(key);
	return next;
}

/** Select, or clear, every key of `keys` (other keys of the selection are kept). */
export function setMany(
	selected: ReadonlySet<string>,
	keys: readonly string[],
	on: boolean
): Set<string> {
	const next = new Set(selected);
	for (const key of keys) {
		if (on) next.add(key);
		else next.delete(key);
	}
	return next;
}

/** Whether none, some or all of `keys` are selected (an empty list is 'none'). */
export function selectionState(
	selected: ReadonlySet<string>,
	keys: readonly string[]
): SelectionState {
	let count = 0;
	for (const key of keys) if (selected.has(key)) count++;
	if (count === 0) return 'none';
	return count === keys.length ? 'all' : 'some';
}

/** Drop the selected keys that are no longer on screen (the list refreshed). */
export function pruneSelection(
	selected: ReadonlySet<string>,
	visible: readonly string[]
): Set<string> {
	const present = new Set(visible);
	const next = new Set<string>();
	for (const key of selected) if (present.has(key)) next.add(key);
	return next;
}

/**
 * The selected items an action applies to: ack skips the already acknowledged,
 * unack keeps only those, ignore needs a known rule and device and is done once
 * per (rule, device) pair even when several rows share it.
 */
export function eligibleItems(
	action: BulkAction,
	items: readonly SelectableItem[],
	selected: ReadonlySet<string>
): SelectableItem[] {
	const chosen = items.filter((item) => selected.has(item.key));
	if (action === 'ack') return chosen.filter((item) => !item.acked);
	if (action === 'unack') return chosen.filter((item) => item.acked);
	const seen = new Set<string>();
	return chosen.filter((item) => {
		if (item.ruleId === undefined || item.targetId === undefined || item.targetId === null) {
			return false;
		}
		const id = `${item.ruleId}:${item.targetId}`;
		if (seen.has(id)) return false;
		seen.add(id);
		return true;
	});
}

export interface BulkFailure {
	key: string;
	message: string;
}

export interface BulkResult {
	done: number;
	failed: BulkFailure[];
}

/**
 * Run `fn` on every item one after the other (the server is not hammered, and
 * the order stays readable), never stopping at the first failure.
 */
export async function runBulk(
	items: readonly SelectableItem[],
	fn: (item: SelectableItem) => Promise<unknown>
): Promise<BulkResult> {
	const result: BulkResult = { done: 0, failed: [] };
	for (const item of items) {
		try {
			await fn(item);
			result.done++;
		} catch (cause) {
			result.failed.push({
				key: item.key,
				message: cause instanceof Error ? cause.message : String(cause)
			});
		}
	}
	return result;
}

/** The selection left after a run: only the rows that failed, to retry them. */
export function afterRun(result: BulkResult): Set<string> {
	return new Set(result.failed.map((f) => f.key));
}
