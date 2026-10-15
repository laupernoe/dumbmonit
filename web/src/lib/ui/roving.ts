/** Pure WAI-ARIA roving-tabindex logic for horizontal tablists and segmented controls. */

/** Index to focus after `key` from `current` among `count` items, or null if the key is not handled. */
export function rovingIndex(key: string, current: number, count: number): number | null {
	if (count <= 0) return null;
	switch (key) {
		case 'ArrowRight':
			return (current + 1) % count;
		case 'ArrowLeft':
			return (current - 1 + count) % count;
		case 'Home':
			return 0;
		case 'End':
			return count - 1;
		default:
			return null;
	}
}

/** tabindex of item `i`: only the selected one (or the first when none is) is in the tab order. */
export function rovingTabindex(i: number, selected: number): 0 | -1 {
	return i === (selected < 0 ? 0 : selected) ? 0 : -1;
}
