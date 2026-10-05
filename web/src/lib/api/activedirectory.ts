/**
 * Active Directory: the device page's domain, controllers, privileged groups
 * and findings. Mirrors `crates/server/src/api/activedirectory.rs`. Everything
 * is read from what the probe stored — opening the page never queries the
 * domain controller.
 */
import { request } from './client';
import type { AdFindings, AdOverview, TargetId } from './types';

/** The whole domain view, findings included. */
export function getAdOverview(id: TargetId, signal?: AbortSignal): Promise<AdOverview> {
	return request<AdOverview>(`/targets/${id}/ad`, { signal });
}

/** The raw security findings, most severe first. */
export function getAdFindings(id: TargetId, signal?: AbortSignal): Promise<AdFindings> {
	return request<AdFindings>(`/targets/${id}/ad/findings`, { signal });
}
