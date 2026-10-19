/**
 * Leaflet (BSD-2-Clause) and the built-in world outlines, loaded on demand:
 * pages without a world map never download either.
 *
 * The outlines are Natural Earth's 1:110m countries (public domain), reduced
 * to a name, an ISO code and a label point per country, coordinates rounded to
 * two decimals, Antarctica left out. With them the map needs no server at all.
 */
import type * as Leaflet from 'leaflet';
import type { FeatureCollection, MultiPolygon, Polygon } from 'geojson';

export type LeafletModule = typeof Leaflet;

export interface CountryProps {
	/** ISO 3166-1 alpha-2, empty for the few areas without one. */
	iso: string;
	name: string;
	/** Label point (Natural Earth `LABEL_X`, `LABEL_Y`). */
	lx: number;
	ly: number;
}

export type World = FeatureCollection<Polygon | MultiPolygon, CountryProps>;

let loading: Promise<{ L: LeafletModule; world: World }> | null = null;

export function loadLeaflet(): Promise<{ L: LeafletModule; world: World }> {
	loading ??= Promise.all([
		import('leaflet'),
		import('./world-110m.json'),
		import('leaflet/dist/leaflet.css')
	]).then(([lib, data]) => {
		const L = ((lib as { default?: LeafletModule }).default ?? lib) as LeafletModule;
		const world = ((data as { default?: World }).default ?? data) as World;
		return { L, world };
	}).catch((error: unknown) => {
		// A failed chunk download (server restarted mid-update): try again next time.
		loading = null;
		throw error;
	});
	return loading;
}
