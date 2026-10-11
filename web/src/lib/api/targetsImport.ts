/**
 * Bulk import of websites from a CSV. Mirrors
 * `crates/server/src/api/targets_import.rs`. Administrators only.
 *
 * The CSV travels in a `{"csv": "…"}` envelope so the request helper keeps its
 * single JSON path (and its anti-CSRF header).
 */
import { request } from './client';
import type { TargetImportReport } from './types';

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
	'url,name,folder,tags,interval,expect_text\n' +
	'https://example.com,Example,Clients/Acme,env=prod;team=web,5m,Welcome\n' +
	'blog.example.org,,,,,\n';
