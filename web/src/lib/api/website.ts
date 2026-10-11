/**
 * Website health & SEO device: last reading and confirmed changes. Mirrors
 * `crates/server/src/api/website.rs`.
 */
import { request } from './client';
import type { TargetId, WebsiteChange, WebsiteOverview } from './types';

export function getWebsiteOverview(id: TargetId, signal?: AbortSignal): Promise<WebsiteOverview> {
	return request<WebsiteOverview>(`/targets/${id}/website`, { signal });
}

export function listWebsiteChanges(id: TargetId, limit = 20, signal?: AbortSignal): Promise<WebsiteChange[]> {
	return request<WebsiteChange[]>(`/targets/${id}/website/changes`, { query: { limit }, signal });
}
