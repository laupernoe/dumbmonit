/**
 * Data & retention (administrators) and the log search over the embedded
 * log store. Mirrors `crates/server/src/api/storage.rs` and `api/logs.rs`.
 */
import { request } from './client';
import type {
	AgentLogs,
	AgentLogsSetting,
	LogInputs,
	LogSearchParams,
	LogSearchResult,
	StorageUpdate,
	StorageView
} from './types';

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

/** Syslog and SNMP trap receivers: what listens, what arrived. */
export function getLogInputs(signal?: AbortSignal): Promise<LogInputs> {
	return request<LogInputs>('/logs/inputs', { signal });
}

/** Log collection setting of an agent device. */
export function getAgentLogs(id: number, signal?: AbortSignal): Promise<AgentLogs> {
	return request<AgentLogs>(`/logs/agents/${id}`, { signal });
}

/** The agent applies it on the fly at its next report. */
export function setAgentLogs(id: number, body: AgentLogsSetting): Promise<AgentLogs> {
	return request<AgentLogs>(`/logs/agents/${id}`, { method: 'PUT', body });
}
