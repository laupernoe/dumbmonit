/**
 * The wall's own theme preference: Auto / Day / Night.
 *
 * Separate from the app-wide `theme` store, so the wall can force a palette
 * for this display only, remembered per browser like the Spotify-speaker
 * switch. `auto` means "whatever the app is showing". A saved or linked
 * `oled` (the retired true-black theme) reads as `dark`.
 */

export type WallThemeChoice = 'auto' | 'light' | 'dark';

const STORAGE_KEY = 'dumbmonit-wall-theme';

/** A stored or linked value as a choice; the retired `oled` folds onto `dark`. */
function toWallThemeChoice(value: string | null): WallThemeChoice | null {
	if (value === 'oled') return 'dark';
	return value === 'auto' || value === 'light' || value === 'dark' ? value : null;
}

/** The saved preference, or `auto` if nothing was saved (or storage is blocked). */
export function readWallTheme(): WallThemeChoice {
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		return toWallThemeChoice(stored) ?? 'auto';
	} catch {
		return 'auto';
	}
}

export function writeWallTheme(choice: WallThemeChoice): void {
	try {
		if (choice === 'auto') localStorage.removeItem(STORAGE_KEY);
		else localStorage.setItem(STORAGE_KEY, choice);
	} catch {
		// Not remembered: it applies until the page reloads.
	}
}

/** A `?theme=` query value, or null when it does not name a wall theme. */
export function parseForcedWallTheme(value: string | null): WallThemeChoice | null {
	const choice = toWallThemeChoice(value);
	return choice === 'auto' ? null : choice;
}
