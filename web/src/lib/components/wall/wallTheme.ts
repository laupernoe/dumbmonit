/**
 * The wall's own theme preference: Auto / Day / Night / OLED.
 *
 * Separate from the app-wide `theme` store (`#lib/stores/theme.svelte.js`),
 * which only knows Auto/Day/Night — OLED is a wall-only idea (a display left
 * on for days, never the rest of the product). `auto` means "whatever the
 * app is showing"; the other three force a palette for this display only,
 * remembered per browser like the Spotify-speaker switch.
 */

export type WallThemeChoice = 'auto' | 'light' | 'dark' | 'oled';

const STORAGE_KEY = 'dumbmonit-wall-theme';
const NIGHT_DIM_KEY = 'dumbmonit-wall-night-dim';

function isWallThemeChoice(value: string | null): value is WallThemeChoice {
	return value === 'auto' || value === 'light' || value === 'dark' || value === 'oled';
}

/** The saved preference, or `auto` if nothing was saved (or storage is blocked). */
export function readWallTheme(): WallThemeChoice {
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		return isWallThemeChoice(stored) ? stored : 'auto';
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
	return isWallThemeChoice(value) && value !== 'auto' ? value : null;
}

export function readNightDim(): boolean {
	try {
		return localStorage.getItem(NIGHT_DIM_KEY) === 'on';
	} catch {
		return false;
	}
}

export function writeNightDim(on: boolean): void {
	try {
		if (on) localStorage.setItem(NIGHT_DIM_KEY, 'on');
		else localStorage.removeItem(NIGHT_DIM_KEY);
	} catch {
		// Not remembered: off again on the next visit.
	}
}

/** True from 22:00 to 07:00, local time. */
export function isNightHour(date: Date): boolean {
	const hour = date.getHours();
	return hour >= 22 || hour < 7;
}

/**
 * Burn-in care: a handful of whole-layout offsets, a couple of pixels each,
 * cycled every few minutes so no pixel sits lit at the same spot for long.
 */
export const OLED_SHIFT_OFFSETS: readonly [number, number][] = [
	[0, 0],
	[2, -1],
	[-2, 1],
	[1, 2],
	[-1, -2],
	[2, 1],
	[-2, -1]
];

export const OLED_SHIFT_INTERVAL_MS = 4 * 60_000;
