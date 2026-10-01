/**
 * Integration packs: device types declared in YAML. Mirrors
 * `crates/server/src/api/packs.rs`. Administrators only.
 *
 * The YAML travels in a `{"yaml": "…"}` envelope so the request helper keeps
 * its single JSON path (and its anti-CSRF header).
 */
import { request } from './client';
import type { PackInstallReport, PackView } from './types';

/** Where packs are documented. The interface links to it, never serves it. */
export const PACKS_DOC_URL = 'https://dumbmonit.readthedocs.io/en/latest/packs/';

export function listPacks(signal?: AbortSignal): Promise<PackView[]> {
	return request<PackView[]>('/packs', { signal });
}

/** Installs a pack, or updates the one with the same id. A 400 lists every validation error. */
export function installPack(yaml: string): Promise<PackInstallReport> {
	return request<PackInstallReport>('/packs', { method: 'POST', body: { yaml } });
}

export function setPackEnabled(id: string, enabled: boolean): Promise<PackView> {
	return request<PackView>(`/packs/${encodeURIComponent(id)}/${enabled ? 'enable' : 'disable'}`, {
		method: 'PUT'
	});
}

/** Refused with a 409 while devices still use the pack. */
export function uninstallPack(id: string): Promise<void> {
	return request<void>(`/packs/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

/** Kinds added by a pack are named `pack.<id>`. */
export function isPackKind(kind: string): boolean {
	return kind.startsWith('pack.');
}
