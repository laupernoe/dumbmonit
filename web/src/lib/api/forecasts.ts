/**
 * Fill-up forecasts of a device: one entry per capacity entity (mountpoint,
 * datastore, pool, volume, licence…) with its growth per day, "full in N days"
 * and the confidence of the fit. Mirrors `crates/server/src/api/forecasts.rs`;
 * the numbers come from the same engine as the `capacity_forecast` rule.
 */
import { request } from './client';
import type { CapacityForecast, TargetId } from './types';

/** Forecasts of one device, soonest to fill first. */
export function getTargetForecasts(id: TargetId, signal?: AbortSignal): Promise<CapacityForecast[]> {
	return request<CapacityForecast[]>(`/targets/${id}/forecasts`, { signal });
}
