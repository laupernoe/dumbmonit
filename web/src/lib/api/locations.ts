/**
 * World map locations and the map background. Mirrors
 * `crates/server/src/api/locations.rs`; types live in `types.ts`.
 */
import { request } from './client';
import type { Basemap, LocationId, LocationPayload, Locations, MapLocation, TargetId, WallPlace } from './types';

export function listLocations(signal?: AbortSignal): Promise<Locations> {
	return request<Locations>('/locations', { signal });
}

export function createLocation(payload: LocationPayload): Promise<MapLocation> {
	return request<MapLocation>('/locations', { method: 'POST', body: payload });
}

export function updateLocation(id: LocationId, payload: LocationPayload): Promise<MapLocation> {
	return request<MapLocation>(`/locations/${id}`, { method: 'PUT', body: payload });
}

export function deleteLocation(id: LocationId): Promise<void> {
	return request<void>(`/locations/${id}`, { method: 'DELETE' });
}

/** Places devices at a location in one go; `null` takes them off. */
export function assignLocation(location: LocationId | null, targets: TargetId[]): Promise<{ updated: number }> {
	return request<{ updated: number }>('/locations/assign', {
		method: 'POST',
		body: { location_id: location, target_ids: targets }
	});
}

/** Place search, asked by the server (Open-Meteo geocoding), never by the browser. */
export function geocode(query: string, lang: string, signal?: AbortSignal): Promise<WallPlace[]> {
	return request<WallPlace[]>('/locations/geocode', { query: { q: query, lang }, signal });
}

export function getBasemap(signal?: AbortSignal): Promise<Basemap> {
	return request<Basemap>('/locations/basemap', { signal });
}

export function saveBasemap(basemap: Basemap): Promise<Basemap> {
	return request<Basemap>('/locations/basemap', { method: 'PUT', body: basemap });
}
