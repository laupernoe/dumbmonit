/**
 * The cloud region table (`regions.json`: AWS, Azure, Hetzner, OVHcloud and
 * Infomaniak regions with public coordinates), loaded on demand like the
 * world outlines: pages without a region map never download it.
 */
import type { RegionTable } from './geo.js';

let loading: Promise<RegionTable> | null = null;

export function loadRegions(): Promise<RegionTable> {
	loading ??= import('./regions.json')
		.then((data) => ((data as { default?: RegionTable }).default ?? data) as RegionTable)
		.catch((error: unknown) => {
			// A failed chunk download: try again next time.
			loading = null;
			throw error;
		});
	return loading;
}
