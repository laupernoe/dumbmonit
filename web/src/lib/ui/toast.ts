/** Pure helpers for toast merging: identical toasts collapse into one with a counter. */

/** Counter of a toast state after another identical one arrives (`prev` null/error: restart at 1). */
export function bumpToastCount(prev: { count: number } | { error: string } | null | undefined): number {
	return prev && "count" in prev && prev.count > 0 ? prev.count + 1 : 1;
}

/** "Saved" then "Saved ×3": the visible/announced text of a toast. */
export function toastText(message: string, count = 1): string {
	return count > 1 ? `${message} ×${count}` : message;
}
