/**
 * Bulk import of websites from a CSV. Mirrors
 * `crates/server/src/api/targets_import.rs`. Administrators only.
 *
 * The CSV travels in a `{"csv": "…"}` envelope so the request helper keeps its
 * single JSON path (and its anti-CSRF header).
 */
import { request } from './client';
import { rowsParam } from './rows-param';
import type { KumaImportReport, TargetImportReport } from './types';

/** `dryRun` previews every row (including per-row errors) without creating anything. */
export function importTargets(csv: string, dryRun: boolean): Promise<TargetImportReport> {
	return request<TargetImportReport>('/targets/import', {
		method: 'POST',
		body: { csv },
		query: { dry_run: dryRun ? true : undefined }
	});
}

/** The template offered for download: header, one fully filled row, one minimal row. */
export const IMPORT_TEMPLATE_CSV =
	'url,name,folder,tags,interval,expect_text,category\n' +
	'https://example.com,Example,Clients/Acme,env=prod;team=web,5m,Welcome,\n' +
	'shop.example.org,,,,,,E-commerce\n' +
	'blog.example.org,,,,,,\n';

/** Largest backup accepted by the server; checked here first for a clear message. */
export const KUMA_MAX_BYTES = 4 * 1024 * 1024;

/** Largest `kuma.db` accepted by the server (it keeps the whole heartbeat history). */
export const KUMA_DB_MAX_BYTES = 512 * 1024 * 1024;

/**
 * Import of an Uptime Kuma 1.x backup (the parsed JSON file). `dryRun` previews only;
 * `rows` (row numbers of the preview) limits the import to the ticked ones.
 */
export function importKuma(backup: unknown, dryRun: boolean, rows?: number[]): Promise<KumaImportReport> {
	return request<KumaImportReport>('/targets/import/kuma', {
		method: 'POST',
		body: backup,
		query: { dry_run: dryRun ? true : undefined, rows: rowsParam(rows) }
	});
}

/**
 * Import of an Uptime Kuma 1.x or 2.x database (`data/kuma.db`, uploaded as is). Preview and
 * import read the same file and number the rows the same way, so `rows` can be the ticked ones.
 */
export function importKumaDb(file: File, dryRun: boolean, rows?: number[]): Promise<KumaImportReport> {
	return request<KumaImportReport>('/targets/import/kuma-db', {
		method: 'POST',
		body: file,
		query: { dry_run: dryRun ? true : undefined, rows: rowsParam(rows) }
	});
}

/**
 * Imports from UptimeRobot (API key or exported CSV), Gatus (config.yaml) and
 * Healthchecks.io (API key). Same preview/import contract as Uptime Kuma: the
 * server fetches or parses, converts and validates, and answers with the same
 * report. Keys are sent once and never stored.
 */
export type ServiceImport =
	| { provider: 'uptimerobot'; apiKey: string }
	| { provider: 'uptimerobot'; csv: string }
	| { provider: 'gatus'; yaml: string }
	| { provider: 'healthchecks'; apiKey: string; baseUrl?: string };

/** Largest CSV/YAML accepted by the server. */
export const SERVICE_IMPORT_MAX_BYTES = 4 * 1024 * 1024;

export function importFromService(source: ServiceImport, dryRun: boolean, rows?: number[]): Promise<KumaImportReport> {
	let body: Record<string, unknown>;
	if (source.provider === 'gatus') body = { yaml: source.yaml };
	else if (source.provider === 'healthchecks') body = { api_key: source.apiKey, base_url: source.baseUrl || undefined };
	else body = 'csv' in source ? { csv: source.csv } : { api_key: source.apiKey };
	return request<KumaImportReport>(`/targets/import/${source.provider}`, {
		method: 'POST',
		body,
		query: { dry_run: dryRun ? true : undefined, rows: rowsParam(rows) }
	});
}
