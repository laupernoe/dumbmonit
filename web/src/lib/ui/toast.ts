/** Pure helpers for toast merging: identical plain toasts collapse into one with a counter. */

/**
 * Counter of a toast state after another one arrives. A plain toast (no action) merges with the
 * previous one (`prev` null/error: restart at 1). A toast with an action (Undo) is never merged:
 * it always starts at 1, so each one keeps its own action instead of only the latest being undoable.
 */
export function bumpToastCount(
	prev: { count: number } | { error: string } | null | undefined,
	withAction = false
): number {
	if (withAction) return 1;
	return prev && "count" in prev && prev.count > 0 ? prev.count + 1 : 1;
}

/** "Saved" then "Saved ×3": the visible/announced text of a toast. */
export function toastText(message: string, count = 1): string {
	return count > 1 ? `${message} ×${count}` : message;
}
