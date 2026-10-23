/**
 * CVE & vulnerabilities (opt-in): the software inventory of the devices matched
 * against NVD, OSV.dev, the Debian security tracker and CISA KEV. Mirrors
 * `crates/server/src/api/cves.rs`. Writes are for administrators.
 */
import { request } from './client';
import type {
	CveFilters,
	CveInventory,
	CveList,
	CveSettings,
	CveSettingsInput,
	CveSummary,
	TargetId
} from './types';

export function getCveSettings(signal?: AbortSignal): Promise<CveSettings> {
	return request<CveSettings>('/security/cves/settings', { signal });
}

/** Turn the feature on or off and/or set the NVD key (`""` clears it, omitted keeps it). */
export function setCveSettings(input: CveSettingsInput): Promise<CveSettings> {
	return request<CveSettings>('/security/cves/settings', { method: 'PUT', body: input });
}

export function getCveSummary(target?: TargetId, signal?: AbortSignal): Promise<CveSummary> {
	return request<CveSummary>('/security/cves/summary', { query: { target }, signal });
}

export function listCves(filters: CveFilters = {}, signal?: AbortSignal): Promise<CveList> {
	return request<CveList>('/security/cves', { query: { ...filters }, signal });
}

export function getCveInventory(target?: TargetId, signal?: AbortSignal): Promise<CveInventory> {
	return request<CveInventory>('/security/cves/inventory', { query: { target }, signal });
}

/** Start a refresh now (409 when the feature is off). */
export function refreshCves(): Promise<{ refreshing: boolean }> {
	return request<{ refreshing: boolean }>('/security/cves/refresh', { method: 'POST' });
}

/** Accept the risk of one CVE on one device. */
export function ignoreCve(targetId: TargetId, cveId: string, note?: string): Promise<void> {
	return request<void>('/security/cves/ignores', {
		method: 'POST',
		body: { target_id: targetId, cve_id: cveId, note: note || undefined }
	});
}

export function restoreCve(targetId: TargetId, cveId: string): Promise<void> {
	return request<void>(
		`/security/cves/ignores/${targetId}/${encodeURIComponent(cveId)}`,
		{ method: 'DELETE' }
	);
}
