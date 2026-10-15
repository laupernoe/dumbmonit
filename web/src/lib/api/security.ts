/**
 * Security score per device (PingCastle / Secure Score style), opt-in from
 * Settings: the checks run for one device, the fleet-wide summary the device
 * list reads its badges from, and the on/off switch. Mirrors
 * `crates/server/src/api/security.rs`. The two reads can answer 502 when
 * VictoriaMetrics is unreachable; turned off, they answer `enabled: false`
 * without touching it.
 */
import { request } from './client';
import type { SecurityReport, SecuritySettings, SecuritySummary, TargetId } from './types';

/**
 * The security report of one device. 404 only when the device itself does
 * not exist; a kind with no checks still answers 200 with `supported: false`.
 */
export function getTargetSecurity(id: TargetId, signal?: AbortSignal): Promise<SecurityReport> {
	return request<SecurityReport>(`/targets/${id}/security`, { signal });
}

/** Every device whose kind has security checks, worst first. */
export function getSecuritySummary(signal?: AbortSignal): Promise<SecuritySummary> {
	return request<SecuritySummary>('/security/summary', { signal });
}

/** Is the security score turned on for this instance? */
export function getSecuritySettings(signal?: AbortSignal): Promise<SecuritySettings> {
	return request<SecuritySettings>('/security/settings', { signal });
}

/** Turn the security score on or off (administrators). */
export function setSecuritySettings(enabled: boolean): Promise<SecuritySettings> {
	return request<SecuritySettings>('/security/settings', { method: 'PUT', body: { enabled } });
}
