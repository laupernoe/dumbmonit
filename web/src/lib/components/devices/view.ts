/**
 * How /targets lays out the monitors: one dense line each (the default) or a
 * grid of tiles. A per-viewer convenience kept in `localStorage` (every
 * access guarded: private windows and blocked storage throw), never sent to
 * the server.
 */
export type RackView = 'compact' | 'tiles';

export const RACK_VIEWS: readonly RackView[] = ['compact', 'tiles'];

const STORAGE_KEY = 'dumbmonit-targets-view';

/**
 * Reads a view name, from storage or the URL. `rows`, the faceplate layout
 * retired in favour of compact, is read as `compact` so old preferences and
 * old links keep working.
 */
export function asRackView(value: unknown): RackView | null {
	if (value === 'rows') return 'compact';
	return typeof value === 'string' && (RACK_VIEWS as readonly string[]).includes(value)
		? (value as RackView)
		: null;
}

export function loadRackView(): RackView {
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		// A remembered `rows` is now the default: forget it rather than keep a stale value.
		if (stored === 'rows') localStorage.removeItem(STORAGE_KEY);
		return asRackView(stored) ?? 'compact';
	} catch {
		return 'compact';
	}
}

export function saveRackView(view: RackView): void {
	try {
		if (view === 'compact') localStorage.removeItem(STORAGE_KEY);
		else localStorage.setItem(STORAGE_KEY, view);
	} catch {
		// Not remembered: the next visit opens on the default view.
	}
}
