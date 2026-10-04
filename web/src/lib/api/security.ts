/**
 * Security score per device (PingCastle / Secure Score style): the checks run
 * for one device, and the fleet-wide summary. Mirrors
 * `crates/server/src/api/security.rs`. Both routes can answer 502 when
 * VictoriaMetrics is unreachable; the caller shows `ErrorNotice`.
 */
import { request } from './client';
import type { SecurityReport, SecuritySummary, TargetId } from './types';

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
