/**
 * Dashboards and the metrics catalogue. Mirrors
 * `crates/server/src/api/dashboards.rs` and `metrics.rs`; types live in `types.ts`.
 */
import { request } from './client';
import type { Dashboard, DashboardId, DashboardPayload, MetricsCatalog } from './types';

export function listDashboards(signal?: AbortSignal): Promise<Dashboard[]> {
	return request<Dashboard[]>('/dashboards', { signal });
}

export function createDashboard(payload: DashboardPayload): Promise<Dashboard> {
	return request<Dashboard>('/dashboards', { method: 'POST', body: payload });
}

export function updateDashboard(id: DashboardId, payload: DashboardPayload): Promise<Dashboard> {
	return request<Dashboard>(`/dashboards/${id}`, { method: 'PUT', body: payload });
}

export function deleteDashboard(id: DashboardId): Promise<void> {
	return request<void>(`/dashboards/${id}`, { method: 'DELETE' });
}

export function getMetricsCatalog(signal?: AbortSignal): Promise<MetricsCatalog> {
	return request<MetricsCatalog>('/metrics/catalog', { signal });
}
