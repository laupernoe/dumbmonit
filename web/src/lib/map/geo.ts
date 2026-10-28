/**
 * Pure helpers of the generic world map (`GeoMap.svelte`) and the maps built
 * on it — threats by country, cloud regions, sites and their relays:
 * choropleth steps, cloud region lookup, the rows of the text list that
 * stands for the picture. No import, so `tests/geo-map.test.mjs` runs them
 * as they are; the region table is handed in by the caller.
 */

export type LatLon = [number, number];

/** Number of choropleth steps above "nothing". */
export const LEVELS = 5;

/**
 * The choropleth step of a value, 0 for nothing, 1..LEVELS otherwise. On a
 * logarithmic scale: one country with thousands of hits must not leave all
 * the others in the palest step.
 */
export function choroplethLevel(value: number, max: number): number {
	if (!(value > 0) || !(max > 0)) return 0;
	const ratio = Math.log1p(value) / Math.log1p(Math.max(max, value));
	return Math.min(LEVELS, 1 + Math.floor(ratio * LEVELS));
}

/** The smallest value of each step 1..LEVELS, for the legend (rounded up, at least 1). */
export function levelFloors(max: number): number[] {
	if (!(max > 0)) return [];
	const floors: number[] = [];
	for (let level = 1; level <= LEVELS; level++) {
		const floor = Math.max(1, Math.ceil(Math.expm1(((level - 1) / LEVELS) * Math.log1p(max))));
		floors.push(Math.min(floor, Math.ceil(max)));
	}
	return floors;
}

export interface CountryValue {
	code: string;
	value: number;
}

/** Countries with something, busiest first, then by code: the list beside a choropleth. */
export function rankCountries(values: Readonly<Record<string, number>>): CountryValue[] {
	return Object.entries(values)
		.filter(([code, value]) => /^[A-Z]{2}$/.test(code) && value > 0)
		.map(([code, value]) => ({ code, value }))
		.sort((a, b) => b.value - a.value || a.code.localeCompare(b.code));
}

// --- Cloud regions -------------------------------------------------------------

export interface Region {
	name: string;
	lat: number;
	lon: number;
}

/** `lib/map/regions.json`: provider → region code → place. */
export type RegionTable = Readonly<Record<string, Readonly<Record<string, Region>>>>;

/**
 * The key a provider's region goes by in the table. OVHcloud writes its
 * datacenters `gra7`, `GRA`, or `Region OpenStack: os-gra7`: all are `gra`.
 * Azure accepts `West Europe` for `westeurope`.
 */
export function regionKey(provider: string, raw: string): string {
	const key = raw.trim().toLowerCase();
	if (provider === 'ovh') {
		const bare = key.replace(/^.*\bos-/, '');
		return /^[a-z]+/.exec(bare)?.[0] ?? bare;
	}
	if (provider === 'azure') return key.replace(/[\s_-]+/g, '');
	return key;
}

/** A region's place, or null when the table does not know it. */
export function regionFor(table: RegionTable, provider: string, raw: string): (Region & { code: string }) | null {
	const entries = table[provider];
	if (!entries || !raw.trim()) return null;
	const code = regionKey(provider, raw);
	const hit = Object.hasOwn(entries, code) ? entries[code] : undefined;
	return hit ? { ...hit, code } : null;
}

export interface CloudPlace {
	code: string;
	name: string;
	lat: number;
	lon: number;
	/** Resources located there. */
	total: number;
	/** Of which not running. */
	down: number;
}

/** A stored series, as `queryInstant` hands it over (only the labels matter here). */
export interface LabelledSeries {
	metric: Record<string, string>;
}

/** Which series locates a provider's resources, by which label, and when one is up. */
const CLOUD_SOURCES: Record<string, { metric: string; label: string; up: (labels: Record<string, string>) => boolean }[]> = {
	hetzner: [{ metric: 'dumbmonit_hetzner_server_info', label: 'location', up: (l) => l.status === 'running' }],
	ovh: [
		{ metric: 'dumbmonit_ovh_vps_info', label: 'zone', up: (l) => l.state === 'running' || l.state === 'backuping' },
		{ metric: 'dumbmonit_ovh_dedicated_info', label: 'datacenter', up: (l) => l.state === 'ok' && l.power !== 'poweroff' }
	]
};

/** The series names to ask for a provider's places; empty when it has none. */
export function cloudMetrics(provider: string): string[] {
	return (CLOUD_SOURCES[provider] ?? []).map((source) => source.metric);
}

/**
 * Where a cloud account's resources are, one place per known region, busiest
 * first. `region` is the account's own region when it has one (the AWS
 * `region` setting): it is a place even without any resource listed.
 * Resources in a region the table does not know are counted in `unknown`.
 */
export function cloudPlaces(
	table: RegionTable,
	provider: string,
	series: readonly LabelledSeries[],
	region?: string
): { places: CloudPlace[]; unknown: number } {
	const places = new Map<string, CloudPlace>();
	let unknown = 0;
	const at = (raw: string): CloudPlace | null => {
		const hit = regionFor(table, provider, raw);
		if (!hit) return null;
		let place = places.get(hit.code);
		if (!place) {
			place = { code: hit.code, name: hit.name, lat: hit.lat, lon: hit.lon, total: 0, down: 0 };
			places.set(hit.code, place);
		}
		return place;
	};
	if (region) at(region);
	for (const source of CLOUD_SOURCES[provider] ?? []) {
		for (const one of series) {
			if (one.metric.__name__ !== source.metric) continue;
			const place = at(one.metric[source.label] ?? '');
			if (!place) {
				unknown++;
				continue;
			}
			place.total++;
			if (!source.up(one.metric)) place.down++;
		}
	}
	const sorted = [...places.values()].sort((a, b) => b.down - a.down || b.total - a.total || a.name.localeCompare(b.name));
	return { places: sorted, unknown };
}
