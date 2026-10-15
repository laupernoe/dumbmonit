/**
 * Health of DumbMonit itself and the outgoing heartbeat. Mirrors
 * `crates/server/src/api/self_monitoring.rs`. The heartbeat URL often holds
 * its own secret: it is sent, never read back (`has_url`, `url_host`).
 */
import { request } from './client';
import type { Heartbeat, HeartbeatTestResult, HeartbeatUpdate, SelfMonitoring } from './types';

export function getSelfMonitoring(signal?: AbortSignal): Promise<SelfMonitoring> {
	return request<SelfMonitoring>('/self-monitoring', { signal });
}

/** Save the heartbeat setting (administrators). */
export function setHeartbeat(update: HeartbeatUpdate): Promise<Heartbeat> {
	return request<Heartbeat>('/self-monitoring/heartbeat', { method: 'PUT', body: update });
}

/** One "up" call now, to `url` or to the stored URL. */
export function testHeartbeat(url?: string): Promise<HeartbeatTestResult> {
	return request<HeartbeatTestResult>('/self-monitoring/heartbeat/test', {
		method: 'POST',
		body: url ? { url } : {}
	});
}
