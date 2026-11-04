/**
 * Multi-product network discovery (admin): a scan is a job the server runs
 * in the background; the UI polls it. Mirrors `crates/server/src/api/discovery.rs`.
 * Subnet suggestions stay in `index.ts` (`localSubnets`).
 */
import { request } from './client';
import type {
	DiscoveryJob,
	DiscoveryJobRequest,
	DiscoveryJobSummary,
	NetscanBulkAction,
	NetscanBulkReport,
	NetscanDevice,
	NetscanReview,
	NetscanSchedule,
	NetscanScheduleInput
} from './types';

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

// --- Scheduled scans and inventory (admin) ------------------------------------

export async function listNetscanSchedules(signal?: AbortSignal): Promise<NetscanSchedule[]> {
	const raw = await request<{ schedules?: NetscanSchedule[] }>('/discovery/schedules', { signal });
	return Array.isArray(raw?.schedules) ? raw.schedules : [];
}

/** 400 over 1024 hosts or a bad interval, 409 when that subnet is already scheduled for that scanner. */
export function createNetscanSchedule(payload: NetscanScheduleInput): Promise<NetscanSchedule> {
	return request<NetscanSchedule>('/discovery/schedules', { method: 'POST', body: payload });
}

export function updateNetscanSchedule(id: number, payload: NetscanScheduleInput): Promise<NetscanSchedule> {
	return request<NetscanSchedule>(`/discovery/schedules/${id}`, { method: 'PUT', body: payload });
}

/** Also removes its inventory. */
export function deleteNetscanSchedule(id: number): Promise<void> {
	return request<void>(`/discovery/schedules/${id}`, { method: 'DELETE' });
}

/** Asks for a scan at the server's next cycle (a few seconds). */
export function runNetscanSchedule(id: number): Promise<void> {
	return request<void>(`/discovery/schedules/${id}/run`, { method: 'POST' });
}

/** Known devices and open ports, unknown ones first. */
export async function listNetscanInventory(signal?: AbortSignal): Promise<NetscanDevice[]> {
	const raw = await request<{ devices?: NetscanDevice[] }>('/discovery/inventory', { signal });
	return Array.isArray(raw?.devices) ? raw.devices : [];
}

/** Marks a device and all its ports as known. */
export function markNetscanDeviceKnown(id: number): Promise<void> {
	return request<void>(`/discovery/inventory/devices/${id}/known`, { method: 'POST' });
}

/** Ignores a device: it counts as known and never alerts again. Marking it known puts it back. */
export function ignoreNetscanDevice(id: number): Promise<void> {
	return request<void>(`/discovery/inventory/devices/${id}/ignore`, { method: 'POST' });
}

export function markNetscanPortKnown(id: number): Promise<void> {
	return request<void>(`/discovery/inventory/ports/${id}/known`, { method: 'POST' });
}

/** Marks every unknown device and port as known; resolves with how many there were. */
export async function markAllNetscanKnown(): Promise<number> {
	const raw = await request<{ marked?: number }>('/discovery/inventory/known', { method: 'POST' });
	return raw?.marked ?? 0;
}

/** Most devices one bulk call accepts. */
export const BULK_LIMIT = 200;

/**
 * Adds, ignores or restores several devices at once; one result per device.
 * Longer lists are cut into calls of {@link BULK_LIMIT} and the reports merged.
 */
export async function bulkNetscanDevices(action: NetscanBulkAction, ids: number[]): Promise<NetscanBulkReport> {
	const merged: NetscanBulkReport = { results: [], added: 0, pending_credentials: 0, already_present: 0, errors: 0 };
	for (let i = 0; i < ids.length; i += BULK_LIMIT) {
		const part = await request<NetscanBulkReport>('/discovery/inventory/bulk', {
			method: 'POST',
			body: { action, device_ids: ids.slice(i, i + BULK_LIMIT) }
		});
		merged.results.push(...part.results);
		merged.added += part.added;
		merged.pending_credentials += part.pending_credentials;
		merged.already_present += part.already_present;
		merged.errors += part.errors;
	}
	return merged;
}

/** Devices waiting for review, and whether the latest batch still has to be announced. */
export function getNetscanReview(signal?: AbortSignal): Promise<NetscanReview> {
	return request<NetscanReview>('/discovery/review', { signal });
}

/** Acknowledges the latest batch: it is not announced again. */
export function markNetscanReviewSeen(): Promise<void> {
	return request<void>('/discovery/review/seen', { method: 'POST' });
}
