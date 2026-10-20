/**
 * Server logs: local files, export and forwarding destinations (administrators).
 * Mirrors `crates/server/src/api/logs.rs`. Secrets are sent, never read back.
 */
import { request } from './client';
import type {
	LogDestination,
	LogDestinationPayload,
	LogExportEstimate,
	LogKindInfo,
	LogsInfo,
	LogsSettingsUpdate,
	LogTestResult
} from './types';

export function getLogs(signal?: AbortSignal): Promise<LogsInfo> {
	return request<LogsInfo>('/logs', { signal });
}

export function setLogsSettings(update: LogsSettingsUpdate): Promise<LogsInfo> {
	return request<LogsInfo>('/logs/settings', { method: 'PUT', body: update });
}

/** Estimated size of the export of the last `days` days. */
export function getExportEstimate(days: number, signal?: AbortSignal): Promise<LogExportEstimate> {
	return request<LogExportEstimate>('/logs/export/estimate', { query: { days }, signal });
}

/** Link to download (plain GET, cookie session): put it in `<a href download>`. */
export function exportUrl(days: number): string {
	return `/api/logs/export?days=${encodeURIComponent(days)}`;
}

export function listLogKinds(signal?: AbortSignal): Promise<LogKindInfo[]> {
	return request<LogKindInfo[]>('/logs/kinds', { signal });
}

export function listLogDestinations(signal?: AbortSignal): Promise<LogDestination[]> {
	return request<LogDestination[]>('/logs/destinations', { signal });
}

export function createLogDestination(payload: LogDestinationPayload): Promise<LogDestination> {
	return request<LogDestination>('/logs/destinations', { method: 'POST', body: payload });
}

export function updateLogDestination(id: number, payload: LogDestinationPayload): Promise<LogDestination> {
	return request<LogDestination>(`/logs/destinations/${id}`, { method: 'PUT', body: payload });
}

export function deleteLogDestination(id: number): Promise<void> {
	return request<void>(`/logs/destinations/${id}`, { method: 'DELETE' });
}

export function testLogDestination(id: number): Promise<LogTestResult> {
	return request<LogTestResult>(`/logs/destinations/${id}/test`, { method: 'POST' });
}
