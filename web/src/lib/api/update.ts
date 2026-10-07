/**
 * Guided update: the latest published release, read by the server from the
 * public GitHub API at most once a day. Mirrors `crates/server/src/api/update.rs`.
 */
import { request } from './client';
import type { UpdateInfo } from './types';

/** Cached answer; the server only goes to GitHub when its cache is a day old. */
export function getUpdate(signal?: AbortSignal, quiet = false): Promise<UpdateInfo> {
	return request<UpdateInfo>('/update', { signal, allowUnauthorized: quiet });
}

/** "Check now": bypasses the cache; 409 with the wait when asked twice within a minute. */
export function checkUpdateNow(): Promise<UpdateInfo> {
	return request<UpdateInfo>('/update/check', { method: 'POST', body: {} });
}

/** Administrators only. */
export function setUpdateCheck(enabled: boolean): Promise<UpdateInfo> {
	return request<UpdateInfo>('/update/settings', { method: 'PUT', body: { enabled } });
}
