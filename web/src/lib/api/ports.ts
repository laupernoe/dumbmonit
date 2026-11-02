/**
 * Network ports to open in the firewall (Settings → System).
 * Administrator only; read-only.
 */
import { request } from './client';
import type { PortsView } from './types';

export function getPorts(signal?: AbortSignal): Promise<PortsView> {
	return request<PortsView>('/security/ports', { signal });
}
