/**
 * Redis / Valkey, MongoDB, RabbitMQ and CrowdSec. Mirrors
 * `crates/server/src/api/backends.rs`, whose view has the same shape as the
 * log and metrics servers' one. Everything is read from what the probe
 * stored — opening the page never queries the service itself.
 */
import { request } from './client';
import type { ObservabilityOverview, TargetId } from './types';

export function getBackendOverview(id: TargetId, signal?: AbortSignal): Promise<ObservabilityOverview> {
	return request<ObservabilityOverview>(`/targets/${id}/backend`, { signal });
}
