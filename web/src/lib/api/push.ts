/**
 * Heartbeat (push) monitors: the URL a cron job or script calls, and its last
 * call. Mirrors `crates/server/src/api/push.rs` (UI side only; the public
 * `/api/push/<token>` route is what the job calls, never the browser).
 */
import { request } from './client';
import type { PushMonitor, PushRunsPage, TargetId } from './types';

/** The monitor of a heartbeat device, created on first read if missing. */
export function getPushMonitor(id: TargetId, signal?: AbortSignal): Promise<PushMonitor> {
	return request<PushMonitor>(`/targets/${id}/push`, { signal });
}

/** New token: the previous URL stops answering at once. */
export function regeneratePushToken(id: TargetId): Promise<PushMonitor> {
	return request<PushMonitor>(`/targets/${id}/push/regenerate`, { method: 'POST' });
}

/** A page of the run history, newest first; `before` is the previous page's `next_before`. */
export function listPushRuns(
	id: TargetId,
	options: { before?: number | null; limit?: number } = {},
	signal?: AbortSignal
): Promise<PushRunsPage> {
	const query = new URLSearchParams();
	if (options.limit) query.set('limit', String(options.limit));
	if (options.before) query.set('before', String(options.before));
	const suffix = query.size ? `?${query}` : '';
	return request<PushRunsPage>(`/targets/${id}/push/runs${suffix}`, { signal });
}
