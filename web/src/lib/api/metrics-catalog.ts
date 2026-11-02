/** The metrics catalogue behind the explorer. Mirrors `crates/server/src/api/metrics.rs`. */
import { request } from './client';
import type { MetricsCatalog } from './types';

export function getMetricsCatalog(signal?: AbortSignal): Promise<MetricsCatalog> {
	return request<MetricsCatalog>('/metrics/catalog', { signal });
}
