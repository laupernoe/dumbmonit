/**
 * DNS blocklist checks: mail server reputation and IP range blocklist.
 * Mirrors `crates/server/src/api/blacklist.rs`. Everything is read from what
 * the probe stored — opening the page never queries a blocklist.
 */
import { request } from './client';
import type { BlacklistOverview, TargetId } from './types';

export function getBlacklistOverview(id: TargetId, signal?: AbortSignal): Promise<BlacklistOverview> {
	return request<BlacklistOverview>(`/targets/${id}/blacklist`, { signal });
}
