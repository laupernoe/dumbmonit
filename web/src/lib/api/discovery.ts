/**
 * Multi-product network discovery (admin): a scan is a job the server runs
 * in the background; the UI polls it. Mirrors `crates/server/src/api/discovery.rs`.
 * Subnet suggestions stay in `index.ts` (`localSubnets`).
 */
import { request } from './client';
import type { DiscoveryJob, DiscoveryJobRequest, DiscoveryJobSummary } from './types';

/** Starts a scan. 400 over 1024 hosts, 409 when one already runs on that agent/server. */
export function startDiscoveryJob(payload: DiscoveryJobRequest): Promise<{ id: string }> {
	return request<{ id: string }>('/discovery/jobs', { method: 'POST', body: payload });
}

/** The last 20 jobs (kept in memory by the server, lost on restart). */
export async function listDiscoveryJobs(signal?: AbortSignal): Promise<DiscoveryJobSummary[]> {
	const raw = await request<{ jobs?: DiscoveryJobSummary[] }>('/discovery/jobs', { signal });
	return Array.isArray(raw?.jobs) ? raw.jobs : [];
}

export function getDiscoveryJob(id: string, signal?: AbortSignal): Promise<DiscoveryJob> {
	return request<DiscoveryJob>(`/discovery/jobs/${encodeURIComponent(id)}`, { signal });
}

/** Cancels a running job (a finished one is left as is). */
export function cancelDiscoveryJob(id: string): Promise<void> {
	return request<void>(`/discovery/jobs/${encodeURIComponent(id)}`, { method: 'DELETE' });
}
