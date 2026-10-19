/**
 * The state of a location on the world map, from the states of its devices.
 * Pure: shared by the map, its list and the tests (`tests/world-map.test.mjs`).
 */

/** The device states the interface knows (`TargetState` in `format.ts`). */
export type DeviceState = 'online' | 'offline' | 'misconfigured' | 'pending' | 'disabled' | 'down' | 'unknown';

export interface Tally {
	total: number;
	/** Unreachable or down: someone should look. */
	down: number;
	/** Misconfigured or waiting for a first answer. */
	warn: number;
	ok: number;
	/** Paused or never probed: says nothing either way. */
	idle: number;
}

/** What the pin says, worst first. `empty`: no device there yet. */
export type Verdict = 'down' | 'warn' | 'ok' | 'idle' | 'empty';

export function tally(states: readonly DeviceState[]): Tally {
	const t: Tally = { total: states.length, down: 0, warn: 0, ok: 0, idle: 0 };
	for (const state of states) {
		if (state === 'down' || state === 'offline') t.down++;
		else if (state === 'misconfigured' || state === 'pending') t.warn++;
		else if (state === 'online') t.ok++;
		else t.idle++;
	}
	return t;
}

export function verdict(t: Tally): Verdict {
	if (t.total === 0) return 'empty';
	if (t.down > 0) return 'down';
	if (t.warn > 0) return 'warn';
	if (t.ok > 0) return 'ok';
	return 'idle';
}

/** The figure on the pin: problems out of the total, or just the total when all is well. */
export function pinCount(t: Tally): string {
	const problems = t.down + t.warn;
	return problems > 0 ? `${problems}/${t.total}` : String(t.total);
}

/** Worst first, then by name: the list beside the map reads like the alerts page. */
export function sortByVerdict<T extends { name: string; verdict: Verdict }>(items: readonly T[]): T[] {
	const rank: Record<Verdict, number> = { down: 0, warn: 1, ok: 2, idle: 3, empty: 4 };
	return [...items].sort((a, b) => rank[a.verdict] - rank[b.verdict] || a.name.localeCompare(b.name));
}

/** Longitude brought back into [-180, 180] (Leaflet lets the world wrap). */
export function wrapLongitude(lon: number): number {
	if (lon >= -180 && lon <= 180) return lon;
	const wrapped = ((((lon + 180) % 360) + 360) % 360) - 180;
	return wrapped === -180 && lon > 0 ? 180 : wrapped;
}

/** Coordinates typed by hand: "46.52, 6.63" or "46.52 6.63"; null when not two numbers in range. */
export function parseCoordinates(text: string): { latitude: number; longitude: number } | null {
	const parts = text
		.trim()
		.split(/[\s,;]+/)
		.filter(Boolean);
	if (parts.length !== 2) return null;
	const [latitude, longitude] = parts.map(Number);
	if (!Number.isFinite(latitude) || !Number.isFinite(longitude)) return null;
	if (Math.abs(latitude) > 90 || Math.abs(longitude) > 180) return null;
	return { latitude, longitude };
}
