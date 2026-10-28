/**
 * iSCSI LUNs (Synology) and zvols (TrueNAS) of a NAS, the volumes that host
 * them, and the manual link "this LUN is entity X of monitor Y". Mirrors
 * `crates/server/src/api/luns.rs`; nothing here asks the NAS itself.
 */
import { request } from './client';
import type { LunLinkPayload, TargetId, TargetLuns } from './types';

export async function getTargetLuns(id: TargetId, signal?: AbortSignal): Promise<TargetLuns> {
	const view = await request<TargetLuns>(`/targets/${id}/luns`, { signal });
	return { ...view, luns: view.luns ?? [], volumes: view.volumes ?? [] };
}

/** Saves which monitor uses a LUN; replaces any previous link. */
export function linkLunConsumer(id: TargetId, lun: string, payload: LunLinkPayload): Promise<void> {
	return request<void>(`/targets/${id}/luns/${encodeURIComponent(lun)}/consumer`, {
		method: 'PUT',
		body: payload
	});
}

export function unlinkLunConsumer(id: TargetId, lun: string): Promise<void> {
	return request<void>(`/targets/${id}/luns/${encodeURIComponent(lun)}/consumer`, { method: 'DELETE' });
}
