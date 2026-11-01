/**
 * The wall's theme: Night (the default) or Day. Chosen in Settings or from the
 * wall's own "Theme" control, remembered per browser; a `?theme=` on a shared
 * link overrides it for that display without saving. The retired OLED theme
 * reads as Night, and an old "Auto" choice (follow the app) reads as Night too:
 * the console itself is always dark.
 *
 * Pure module (no Svelte): also loaded by `tests/wall-theme.test.mjs`.
 */

/** `light` is Day, `dark` is Night (the names the scenes' palettes use). */
export type WallTheme = 'light' | 'dark';

export const DEFAULT_WALL_THEME: WallTheme = 'dark';

const STORAGE_KEY = 'dumbmonit-wall-theme';

/** A stored or linked value as a theme, or null when it names none. */
export function toWallTheme(value: string | null): WallTheme | null {
	if (value === null) return null;
	if (value === 'light' || value === 'day') return 'light';
	if (value === 'dark' || value === 'night' || value === 'oled' || value === 'auto') return 'dark';
	return null;
}

/** The saved theme; Night when nothing is saved (or storage is blocked). */
export function readWallTheme(fallback: WallTheme = DEFAULT_WALL_THEME): WallTheme {
	try {
		return toWallTheme(localStorage.getItem(STORAGE_KEY)) ?? fallback;
	} catch {
		return fallback;
	}
}

export function writeWallTheme(theme: WallTheme): void {
	try {
		localStorage.setItem(STORAGE_KEY, theme);
	} catch {
		// Not remembered: it applies until the page reloads.
	}
}

/** A `?theme=` value, or null when it does not name a wall theme. */
export function parseForcedWallTheme(value: string | null): WallTheme | null {
	return value === 'auto' ? null : toWallTheme(value);
}
