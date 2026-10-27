/**
 * The wall's optional backdrop: one of the four night cities of the status
 * pages (Paris, New York, Tokyo, Dubai) as a band behind the verdict and the
 * clock, or none (the default). `auto` picks the closest city from the
 * browser's time zone. Remembered per browser; a `?city=` on a shared link
 * overrides it for that display without saving. Retired backdrops fold onto
 * a kept one (Zermatt and the Matterhorn onto Dubai, as on the status pages),
 * so an old saved choice or link still lands somewhere.
 *
 * Pure module (no Svelte): also loaded by `tests/wall-scene.test.mjs`.
 */

export type WallCity = 'paris' | 'newyork' | 'tokyo' | 'dubai';
export type WallSceneChoice = 'none' | 'auto' | WallCity;

export const WALL_CITY_IDS: readonly WallCity[] = ['paris', 'newyork', 'tokyo', 'dubai'];

/** Retired backdrops → the one kept in their place. */
const LEGACY_CITIES: Record<string, WallCity> = {
	zermatt: 'dubai',
	matterhorn: 'dubai',
	london: 'paris',
	rome: 'paris',
	hongkong: 'tokyo',
	kyoto: 'tokyo',
	rio: 'newyork'
};

const STORAGE_KEY = 'dumbmonit-wall-city';

/** A saved or linked value as a choice, or null when it names nothing known. */
export function toWallSceneChoice(value: string | null): WallSceneChoice | null {
	if (value === null) return null;
	if (value === 'none' || value === 'auto') return value;
	if ((WALL_CITY_IDS as readonly string[]).includes(value)) return value as WallCity;
	return LEGACY_CITIES[value] ?? null;
}

export function readWallScene(): WallSceneChoice {
	try {
		return toWallSceneChoice(localStorage.getItem(STORAGE_KEY)) ?? 'none';
	} catch {
		return 'none';
	}
}

export function writeWallScene(choice: WallSceneChoice): void {
	try {
		if (choice === 'none') localStorage.removeItem(STORAGE_KEY);
		else localStorage.setItem(STORAGE_KEY, choice);
	} catch {
		// Not remembered: it applies until the page reloads.
	}
}

/** A `?city=` value, or null when it names no backdrop. */
export function parseForcedWallScene(value: string | null): WallSceneChoice | null {
	return toWallSceneChoice(value);
}

/** The closest city for a time zone (Paris when nothing fits). */
export function cityForTimeZone(zone: string | undefined): WallCity {
	const z = zone ?? '';
	if (['Asia/Dubai', 'Asia/Muscat', 'Asia/Qatar', 'Asia/Bahrain', 'Asia/Kuwait', 'Asia/Riyadh'].includes(z)) {
		return 'dubai';
	}
	if (z.startsWith('Asia/') || z.startsWith('Australia/') || z.startsWith('Pacific/')) return 'tokyo';
	if (z.startsWith('America/')) return 'newyork';
	return 'paris';
}

/** The city to draw for a choice, or null for none. */
export function resolveWallScene(choice: WallSceneChoice, zone?: string): WallCity | null {
	if (choice === 'none') return null;
	if (choice !== 'auto') return choice;
	let z = zone;
	if (z === undefined) {
		try {
			z = Intl.DateTimeFormat().resolvedOptions().timeZone;
		} catch {
			z = undefined;
		}
	}
	return cityForTimeZone(z);
}
