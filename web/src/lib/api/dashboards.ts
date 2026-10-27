/**
 * Dashboards and the metrics catalogue. Mirrors
 * `crates/server/src/api/dashboards.rs` and `metrics.rs`; types live in `types.ts`.
 */
import { request } from './client';
import type {
	Dashboard,
	DashboardId,
	DashboardPayload,
	DashboardTemplateFilter,
	DashboardTemplateList,
	DashboardTemplatePreview,
	MetricsCatalog
} from './types';

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

export function listDashboardTemplates(signal?: AbortSignal): Promise<DashboardTemplateList> {
	return request<DashboardTemplateList>('/dashboards/templates', { signal });
}

export function previewDashboardTemplate(id: string, filter: DashboardTemplateFilter, signal?: AbortSignal): Promise<DashboardTemplatePreview> {
	const query = new URLSearchParams();
	if (filter.kind) query.set('kind', filter.kind);
	if (filter.folder) query.set('folder', filter.folder);
	if (filter.site !== undefined) query.set('site', String(filter.site));
	if (filter.client !== undefined) query.set('client', String(filter.client));
	const text = query.toString();
	const suffix = text ? `?${text}` : '';
	return request<DashboardTemplatePreview>(`/dashboards/templates/${encodeURIComponent(id)}/preview${suffix}`, { signal });
}
