/**
 * Log and metrics servers: VictoriaMetrics, VictoriaLogs, Loki and Graylog.
 * Mirrors `crates/server/src/api/observability.rs`. Everything is read from
 * what the probe stored — opening the page never queries the server itself.
 */
import { request } from './client';
import type { ObservabilityOverview, TargetId } from './types';

export function getObservabilityOverview(id: TargetId, signal?: AbortSignal): Promise<ObservabilityOverview> {
	return request<ObservabilityOverview>(`/targets/${id}/observability`, { signal });
}
