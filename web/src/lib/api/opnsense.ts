/**
 * OPNsense: the device page's gateways, traffic and health. Mirrors
 * `crates/server/src/api/opnsense.rs`. Everything is read from what the probe
 * stored — opening the page never queries the firewall itself, which is the
 * machine routing every packet in the house.
 */
import { request } from './client';
import type { GeoipStatus, OpnsenseGateways, OpnsenseHealth, OpnsenseLive, OpnsenseTraffic, TargetId } from './types';

/**
 * The live map: the last fifteen minutes of the firewall log reduced to
 * countries, rules and ports (never an address), plus the events after
 * `after`. Only read while the panel is open; the server polls the firewall
 * meanwhile, the browser never does.
 */
export function getOpnsenseLive(id: TargetId, after: number, signal?: AbortSignal): Promise<OpnsenseLive> {
	return request<OpnsenseLive>(`/targets/${id}/opnsense/live`, { query: { after }, signal });
}

/** The IP → country database (DB-IP Lite) and its switch. */
export function getGeoip(signal?: AbortSignal): Promise<GeoipStatus> {
	return request<GeoipStatus>('/geoip', { signal });
}

export function setGeoip(enabled: boolean): Promise<GeoipStatus> {
	return request<GeoipStatus>('/geoip', { method: 'PUT', body: { enabled } });
}

/** Gateways, their delay and loss, and the WAN addresses of the moment. */
export function getOpnsenseGateways(id: TargetId, signal?: AbortSignal): Promise<OpnsenseGateways> {
	return request<OpnsenseGateways>(`/targets/${id}/opnsense/gateways`, { signal });
}

/** The pf state table, the interface counters and the DHCP lease counts. */
export function getOpnsenseTraffic(id: TargetId, signal?: AbortSignal): Promise<OpnsenseTraffic> {
	return request<OpnsenseTraffic>(`/targets/${id}/opnsense/traffic`, { signal });
}

/** Services, VPN tunnels, CARP, firmware, the resolver and the machine itself. */
export function getOpnsenseHealth(id: TargetId, signal?: AbortSignal): Promise<OpnsenseHealth> {
	return request<OpnsenseHealth>(`/targets/${id}/opnsense/health`, { signal });
}
