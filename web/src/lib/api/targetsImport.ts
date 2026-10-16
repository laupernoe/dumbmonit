/**
 * Bulk import of websites from a CSV. Mirrors
 * `crates/server/src/api/targets_import.rs`. Administrators only.
 *
 * The CSV travels in a `{"csv": "…"}` envelope so the request helper keeps its
 * single JSON path (and its anti-CSRF header).
 */
import { request } from './client';
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

/** Import of an Uptime Kuma 1.x backup (the parsed JSON file). `dryRun` previews only. */
export function importKuma(backup: unknown, dryRun: boolean): Promise<KumaImportReport> {
	return request<KumaImportReport>('/targets/import/kuma', {
		method: 'POST',
		body: backup,
		query: { dry_run: dryRun ? true : undefined }
	});
}
