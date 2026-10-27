/**
 * Configuration in YAML files (GitOps). Mirrors `crates/server/src/api/gitops.rs`.
 *
 * A plan never writes anything; an apply is one transaction. The server only
 * resolves secret references (`${NAME}`) for its watched folder: a file pasted
 * here is planned with those references assumed unchanged, and can only be
 * applied once it holds none.
 */
import { request } from './client';
import type { ConfigExport, ConfigPlan, ConfigStatus } from './types';

export function getConfigStatus(signal?: AbortSignal): Promise<ConfigStatus> {
	return request<ConfigStatus>('/config/status', { signal });
}

export function exportConfig(): Promise<ConfigExport> {
	return request<ConfigExport>('/config/export', { query: { format: 'json' } });
}

export function planConfig(yaml: string, prune: boolean): Promise<ConfigPlan> {
	return request<ConfigPlan>('/config/plan', { method: 'POST', body: { yaml, prune } });
}

export function applyConfig(yaml: string, prune: boolean): Promise<ConfigPlan> {
	return request<ConfigPlan>('/config/apply', { method: 'POST', body: { yaml, prune } });
}

/** Hands every file-managed object back to manual editing. Deletes nothing. */
export function releaseConfig(): Promise<{ released: number }> {
	return request<{ released: number }>('/config/release', { method: 'POST' });
}

/** Hands the YAML to the browser as a file. */
export function saveYaml(yaml: string): void {
	const blob = new Blob([yaml], { type: 'application/yaml' });
	const url = URL.createObjectURL(blob);
	const anchor = document.createElement('a');
	anchor.href = url;
	anchor.download = 'dumbmonit.yaml';
	document.body.append(anchor);
	anchor.click();
	anchor.remove();
	setTimeout(() => URL.revokeObjectURL(url), 1000);
}
