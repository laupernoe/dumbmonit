/**
 * Traefik, Caddy, Nginx Proxy Manager and domain registrations. Mirrors
 * `crates/server/src/api/proxies.rs`, whose view has the same shape as the
 * log and metrics servers' one. Everything is read from what the probe
 * stored — opening the page never queries the proxy or the registry.
 */
import { request } from './client';
import type { ObservabilityOverview, TargetId } from './types';

export function getProxyOverview(id: TargetId, signal?: AbortSignal): Promise<ObservabilityOverview> {
	return request<ObservabilityOverview>(`/targets/${id}/proxy`, { signal });
}
