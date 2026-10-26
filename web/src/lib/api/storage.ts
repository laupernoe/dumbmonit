/**
 * Data & retention (administrators) and the log search over the embedded
 * log store. Mirrors `crates/server/src/api/storage.rs` and `api/logs.rs`.
 */
import { request } from './client';
import type { LogSearchParams, LogSearchResult, StorageUpdate, StorageView } from './types';

export function getStorage(signal?: AbortSignal): Promise<StorageView> {
	return request<StorageView>('/system/storage', { signal });
}

/** Save; the answer carries `restarted`. May restart a process (a few seconds). */
export function updateStorage(body: StorageUpdate): Promise<StorageView> {
	return request<StorageView>('/system/storage', { method: 'PUT', body });
}

export function searchLogs(params: LogSearchParams, signal?: AbortSignal): Promise<LogSearchResult> {
	return request<LogSearchResult>('/logs/search', { query: { ...params }, signal });
}
