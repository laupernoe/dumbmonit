/**
 * How /targets lays out the monitors: full rack faceplates, a grid of tiles,
 * or one dense line each. A per-viewer convenience kept in `localStorage`
 * (every access guarded: private windows and blocked storage throw), never
 * sent to the server.
 */
export type RackView = 'rows' | 'tiles' | 'compact';

export const RACK_VIEWS: readonly RackView[] = ['rows', 'tiles', 'compact'];

const STORAGE_KEY = 'dumbmonit-targets-view';

function isView(value: unknown): value is RackView {
	return typeof value === 'string' && (RACK_VIEWS as readonly string[]).includes(value);
}

export function loadRackView(): RackView {
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		return isView(stored) ? stored : 'rows';
	} catch {
		return 'rows';
	}
}

export function saveRackView(view: RackView): void {
	try {
		if (view === 'rows') localStorage.removeItem(STORAGE_KEY);
		else localStorage.setItem(STORAGE_KEY, view);
	} catch {
		// Not remembered: the next visit opens on the default view.
	}
}
