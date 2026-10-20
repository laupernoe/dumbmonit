/**
 * Optional features a missing privilege made the last probe skip (Proxmox VE,
 * Backup Server, Datacenter Manager). Mirrors
 * `crates/server/src/api/skipped_features.rs`; read from what the probe left.
 */
import { request } from './client';
import type { SkippedFeatures, TargetId } from './types';

export function getSkippedFeatures(id: TargetId, signal?: AbortSignal): Promise<SkippedFeatures> {
	return request<SkippedFeatures>(`/targets/${id}/skipped-features`, { signal });
}
