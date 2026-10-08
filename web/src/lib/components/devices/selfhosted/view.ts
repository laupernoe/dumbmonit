/**
 * Self-hosted applications (Nextcloud, Immich, Paperless-ngx, Jellyfin, Plex,
 * GitLab, Forgejo/Gitea): what the latest stored measurement says, folded
 * into sentences, figures and short lists. Pure functions over the series of
 * one device, so the panel only lays them out.
 *
 * The collector (`crates/collectors/src/selfhosted`) writes one family per
 * fact, prefixed by the kind: `dumbmonit_nextcloud_maintenance`,
 * `dumbmonit_immich_queue_waiting{queue}`… Every state is a word as well as a
 * colour; a fact that was not measured is left out rather than shown as fine.
 */
import type { MetricSeries } from '#lib/api/index.js';
import { formatBytes } from '../docker/api';
import { formatSpan } from '../pbs/format';
import { m } from '#lib/paraglide/messages.js';

export type CheckState = 'ok' | 'advisory' | 'warning';

export interface Check {
	state: CheckState;
	label: string;
	detail: string;
}

export interface FigureView {
	label: string;
	value: string | null;
}

export interface Row {
	label: string;
	value?: string;
	state?: CheckState;
}

export interface Breakdown {
	title: string;
	note?: string;
	rows: Row[];
}

export interface AppView {
	version: string | null;
	checks: Check[];
	figures: FigureView[];
	breakdowns: Breakdown[];
	/** True once any series of this kind exists: the probe has run. */
	measured: boolean;
}

export const TITLES: Record<string, string> = {
	nextcloud: 'Nextcloud',
	immich: 'Immich',
	paperless: 'Paperless-ngx',
	jellyfin: 'Jellyfin',
	plex: 'Plex Media Server',
	gitlab: 'GitLab',
	forgejo: 'Forgejo / Gitea'
};

export const SELFHOSTED_KINDS = Object.keys(TITLES);

interface Point {
	labels: Record<string, string>;
	value: number;
}

/** The series of one device, by family name without the `dumbmonit_<kind>_` prefix. */
class Reading {
	private families = new Map<string, Point[]>();

	constructor(kind: string, series: MetricSeries[]) {
		const prefix = `dumbmonit_${kind}_`;
		for (const serie of series) {
			const name = serie.metric.__name__ ?? '';
			if (!name.startsWith(prefix)) continue;
			const value = Number(serie.values.at(-1)?.[1]);
			if (!Number.isFinite(value)) continue;
			const family = name.slice(prefix.length);
			const list = this.families.get(family) ?? [];
			list.push({ labels: serie.metric, value });
			this.families.set(family, list);
		}
	}

	get size(): number {
		return this.families.size;
	}

	all(family: string): Point[] {
		return this.families.get(family) ?? [];
	}

	one(family: string): number | null {
		return this.all(family)[0]?.value ?? null;
	}

	label(family: string, key: string): string | null {
		return this.all(family)[0]?.labels[key] || null;
	}

	flag(family: string): boolean | null {
		const value = this.one(family);
		return value === null ? null : value >= 1;
	}
}

function count(value: number | null): string | null {
	if (value === null) return null;
	return Math.round(value).toLocaleString('en');
}

function bytes(value: number | null): string | null {
	return value === null ? null : formatBytes(value);
}

function percent(value: number | null): string | null {
	if (value === null) return null;
	return `${value < 10 ? value.toFixed(1) : Math.round(value)}%`;
}

function updateCheck(r: Reading, product: string, versionLabel = 'latest_version'): Check | null {
	const available = r.flag('update_available');
	if (available === null) return null;
	const latest = r.label('update_available', versionLabel) ?? r.label('update_available', 'available_version');
	return available
		? { state: 'advisory', label: m.devicesb_selfhosted_view_update_available(), detail: latest ? m.devicesb_selfhosted_view_update_named({ product, version: latest }) : m.devicesb_selfhosted_view_update_generic({ product }) }
		: { state: 'ok', label: m.devicesb_selfhosted_view_up_to_date(), detail: m.devicesb_selfhosted_view_up_to_date_detail({ product }) };
}

function nextcloud(r: Reading): Omit<AppView, 'measured'> {
	const checks: Check[] = [];
	if (r.flag('maintenance')) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_maintenance_mode(), detail: m.devicesb_selfhosted_view_nc_maintenance_detail() });
	}
	if (r.flag('needs_db_upgrade')) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_upgrade_pending(), detail: m.devicesb_selfhosted_view_nc_upgrade_detail() });
	}
	if (r.flag('maintenance') === false && r.flag('needs_db_upgrade') === false) {
		checks.push({ state: 'ok', label: m.devicesb_selfhosted_view_serving(), detail: m.devicesb_selfhosted_view_serving_detail() });
	}
	const update = updateCheck(r, 'Nextcloud', 'available_version');
	if (update) checks.push(update);
	const apps = r.one('app_updates_available');
	if (apps !== null && apps > 0) {
		checks.push({ state: 'advisory', label: m.devicesb_selfhosted_view_app_updates(), detail: apps === 1 ? m.devicesb_selfhosted_view_app_updates_detail_one() : m.devicesb_selfhosted_view_app_updates_detail_other({ count: apps }) });
	}
	if (r.flag('opcache_enabled') === false) {
		checks.push({ state: 'advisory', label: m.devicesb_selfhosted_view_opcache_off(), detail: m.devicesb_selfhosted_view_opcache_off_detail() });
	} else if (r.flag('opcache_full')) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_opcache_full(), detail: m.devicesb_selfhosted_view_opcache_full_detail() });
	}
	const active = (window: string) => r.all('active_users').find((p) => p.labels.window === window)?.value ?? null;
	const figures: FigureView[] = [
		{ label: m.devicesb_selfhosted_view_fig_active_5m(), value: count(active('5m')) },
		{ label: m.devicesb_selfhosted_view_fig_active_24h(), value: count(active('24h')) },
		{ label: m.devicesb_selfhosted_view_fig_free_space(), value: bytes(r.one('free_space_bytes')) },
		{ label: m.devicesb_selfhosted_view_fig_db_size(), value: bytes(r.one('database_size_bytes')) },
		{ label: m.devicesb_selfhosted_view_fig_users(), value: count(r.one('users')) },
		{ label: m.devicesb_selfhosted_view_fig_files(), value: count(r.one('files')) },
		{ label: m.devicesb_selfhosted_view_fig_opcache_used(), value: percent(r.one('opcache_memory_used_percent')) },
		{ label: m.devicesb_selfhosted_view_fig_opcache_hit(), value: percent(r.one('opcache_hit_rate_percent')) }
	];
	const breakdowns: Breakdown[] = [];
	const appRows = r.all('app_update_available').map((p) => ({ label: p.labels.app ?? '', value: p.labels.available_version ?? '' }));
	if (appRows.length > 0) breakdowns.push({ title: m.devicesb_selfhosted_view_app_updates(), rows: appRows });
	const platform = r.all('platform_info')[0]?.labels;
	if (platform) {
		breakdowns.push({
			title: m.devicesb_selfhosted_view_platform(),
			rows: [
				{ label: m.devicesb_selfhosted_view_database(), value: [platform.database, platform.database_version].filter(Boolean).join(' ') },
				{ label: 'PHP', value: platform.php_version ?? '—' },
				{ label: m.devicesb_selfhosted_view_php_memory_limit(), value: bytes(r.one('php_memory_limit_bytes')) ?? '—' }
			]
		});
	}
	return { version: r.label('version_info', 'version'), checks, figures, breakdowns };
}

const IMMICH_QUEUES: Record<string, () => string> = {
	thumbnailGeneration: () => m.devicesb_selfhosted_view_queue_thumbnail_generation(),
	metadataExtraction: () => m.devicesb_selfhosted_view_queue_metadata_extraction(),
	videoConversion: () => m.devicesb_selfhosted_view_queue_video_conversion(),
	faceDetection: () => m.devicesb_selfhosted_view_queue_face_detection(),
	facialRecognition: () => m.devicesb_selfhosted_view_queue_facial_recognition(),
	smartSearch: () => m.devicesb_selfhosted_view_queue_smart_search(),
	duplicateDetection: () => m.devicesb_selfhosted_view_queue_duplicate_detection(),
	backgroundTask: () => m.devicesb_selfhosted_view_queue_background_task(),
	storageTemplateMigration: () => m.devicesb_selfhosted_view_queue_storage_template_migration(),
	migration: () => m.devicesb_selfhosted_view_queue_migration(),
	search: () => m.devicesb_selfhosted_view_queue_search(),
	sidecar: () => m.devicesb_selfhosted_view_queue_sidecar(),
	library: () => m.devicesb_selfhosted_view_queue_library(),
	notifications: () => m.devicesb_selfhosted_view_queue_notifications(),
	backupDatabase: () => m.devicesb_selfhosted_view_queue_backup_database(),
	ocr: () => m.devicesb_selfhosted_view_queue_ocr(),
	workflow: () => m.devicesb_selfhosted_view_queue_workflow(),
	integrityCheck: () => m.devicesb_selfhosted_view_queue_integrity_check(),
	editor: () => m.devicesb_selfhosted_view_queue_editor()
};

function immich(r: Reading): Omit<AppView, 'measured'> {
	const checks: Check[] = [];
	const queues = new Map<string, { waiting: number; active: number; failed: number; paused: boolean }>();
	const queue = (name: string) => {
		let q = queues.get(name);
		if (!q) {
			q = { waiting: 0, active: 0, failed: 0, paused: false };
			queues.set(name, q);
		}
		return q;
	};
	for (const p of r.all('queue_waiting')) queue(p.labels.queue ?? '').waiting = p.value;
	for (const p of r.all('queue_active')) queue(p.labels.queue ?? '').active = p.value;
	for (const p of r.all('queue_failed')) queue(p.labels.queue ?? '').failed = p.value;
	for (const p of r.all('queue_paused')) queue(p.labels.queue ?? '').paused = p.value >= 1;
	const stalled = [...queues.entries()].filter(([, q]) => q.waiting > 0 && q.active === 0 && !q.paused);
	const paused = [...queues.entries()].filter(([, q]) => q.paused);
	if (r.flag('maintenance_mode')) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_maintenance_mode(), detail: m.devicesb_selfhosted_view_immich_maintenance_detail() });
	}
	if (stalled.length > 0) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_jobs_waiting(), detail: stalled.length === 1 ? m.devicesb_selfhosted_view_stalled_one() : m.devicesb_selfhosted_view_stalled_other({ count: stalled.length }) });
	}
	if (paused.length > 0) {
		checks.push({ state: 'advisory', label: m.devicesb_selfhosted_view_queue_paused(), detail: m.devicesb_selfhosted_view_queue_paused_detail({ queues: paused.map(([name]) => IMMICH_QUEUES[name]?.() ?? name).join(', ') }) });
	}
	if (queues.size > 0 && stalled.length === 0 && paused.length === 0) {
		checks.push({ state: 'ok', label: m.devicesb_selfhosted_view_jobs_flowing(), detail: m.devicesb_selfhosted_view_jobs_flowing_detail() });
	}
	const update = updateCheck(r, 'Immich');
	if (update) checks.push(update);
	if (r.flag('statistics_readable') === false) {
		checks.push({ state: 'advisory', label: m.devicesb_selfhosted_view_counts_skipped(), detail: m.devicesb_selfhosted_view_counts_skipped_detail() });
	}
	const figures: FigureView[] = [
		{ label: m.devicesb_selfhosted_view_fig_photos(), value: count(r.one('photos')) },
		{ label: m.devicesb_selfhosted_view_fig_videos(), value: count(r.one('videos')) },
		{ label: m.devicesb_selfhosted_view_fig_library_size(), value: bytes(r.one('usage_bytes')) },
		{ label: m.devicesb_selfhosted_view_fig_disk_used(), value: percent(r.one('storage_used_percent')) },
		{ label: m.devicesb_selfhosted_view_jobs_waiting(), value: count(r.one('jobs_waiting')) },
		{ label: m.devicesb_selfhosted_view_fig_jobs_running(), value: count(r.one('jobs_active')) }
	];
	const busy = [...queues.entries()]
		.filter(([, q]) => q.waiting > 0 || q.active > 0 || q.failed > 0 || q.paused)
		.sort((a, b) => b[1].waiting - a[1].waiting);
	const breakdowns: Breakdown[] = [];
	if (busy.length > 0) {
		breakdowns.push({
			title: m.devicesb_selfhosted_view_job_queues(),
			note: m.devicesb_selfhosted_view_job_queues_note(),
			rows: busy.map(([name, q]) => ({
				label: IMMICH_QUEUES[name]?.() ?? name,
				value: q.failed > 0
					? m.devicesb_selfhosted_view_queue_row_failed({ waiting: count(q.waiting) ?? '0', active: count(q.active) ?? '0', failed: count(q.failed) ?? '0' })
					: m.devicesb_selfhosted_view_queue_row({ waiting: count(q.waiting) ?? '0', active: count(q.active) ?? '0' }),
				state: q.paused ? 'advisory' : q.waiting > 0 && q.active === 0 ? 'warning' : undefined
			}))
		});
	}
	return { version: r.label('version_info', 'version'), checks, figures, breakdowns };
}

const PAPERLESS_COMPONENTS: Record<string, () => string> = {
	database: () => m.devicesb_selfhosted_view_component_database(),
	redis: () => m.devicesb_selfhosted_view_component_redis(),
	celery: () => m.devicesb_selfhosted_view_component_celery(),
	index: () => m.devicesb_selfhosted_view_component_index(),
	classifier: () => m.devicesb_selfhosted_view_component_classifier(),
	sanity_check: () => m.devicesb_selfhosted_view_component_sanity_check(),
	llm_index: () => m.devicesb_selfhosted_view_component_llm_index()
};

function paperless(r: Reading): Omit<AppView, 'measured'> {
	const checks: Check[] = [];
	const components = r.all('component_status');
	const broken = components.filter((p) => p.value >= 2).map((p) => PAPERLESS_COMPONENTS[p.labels.component ?? '']?.() ?? p.labels.component);
	if (broken.length > 0) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_component_error(), detail: m.devicesb_selfhosted_view_component_error_detail({ components: broken.join(', ') }) });
	} else if (components.length > 0) {
		checks.push({ state: 'ok', label: m.devicesb_selfhosted_view_components(), detail: m.devicesb_selfhosted_view_components_detail() });
	}
	const failed = r.one('tasks_failed_recent');
	if (failed !== null && failed > 0) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_failed_tasks(), detail: failed === 1 ? m.devicesb_selfhosted_view_failed_tasks_detail_one() : m.devicesb_selfhosted_view_failed_tasks_detail_other({ count: failed }) });
	}
	const migrations = r.one('unapplied_migrations');
	if (migrations !== null && migrations > 0) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_migrations_pending(), detail: migrations === 1 ? m.devicesb_selfhosted_view_migrations_detail_one() : m.devicesb_selfhosted_view_migrations_detail_other({ count: migrations }) });
	}
	const update = updateCheck(r, 'Paperless-ngx');
	if (update) checks.push(update);
	if (r.flag('status_readable') === false) {
		checks.push({ state: 'advisory', label: m.devicesb_selfhosted_view_health_not_readable(), detail: m.devicesb_selfhosted_view_health_not_readable_detail() });
	}
	const figures: FigureView[] = [
		{ label: m.devicesb_selfhosted_view_fig_documents(), value: count(r.one('documents')) },
		{ label: m.devicesb_selfhosted_view_fig_inbox(), value: count(r.one('documents_inbox')) },
		{ label: m.devicesb_selfhosted_view_fig_tasks_pending(), value: count(r.one('tasks_pending')) },
		{ label: m.devicesb_selfhosted_view_fig_failed_30d(), value: count(r.one('tasks_failed_window')) },
		{ label: m.devicesb_selfhosted_view_fig_disk_used(), value: percent(r.one('storage_used_percent')) },
		{ label: m.devicesb_selfhosted_view_fig_disk_free(), value: bytes(r.one('storage_available_bytes')) }
	];
	const breakdowns: Breakdown[] = [];
	if (components.length > 0) {
		breakdowns.push({
			title: m.devicesb_selfhosted_view_components(),
			rows: components.map((p) => ({
				label: PAPERLESS_COMPONENTS[p.labels.component ?? '']?.() ?? p.labels.component ?? '',
				state: p.value >= 2 ? 'warning' : p.value >= 1 ? 'advisory' : 'ok'
			}))
		});
	}
	const failedRows = r.all('task_failed').map((p) => ({ label: p.labels.file || p.labels.task || m.devicesb_selfhosted_view_task_fallback(), value: p.labels.task ?? '', state: 'warning' as const }));
	if (failedRows.length > 0) breakdowns.push({ title: m.devicesb_selfhosted_view_failed_tasks(), rows: failedRows });
	return { version: r.label('version_info', 'version'), checks, figures, breakdowns };
}

function jellyfin(r: Reading): Omit<AppView, 'measured'> {
	const checks: Check[] = [];
	const failed = r.one('scheduled_tasks_failed');
	if (failed !== null && failed > 0) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_task_failed(), detail: failed === 1 ? m.devicesb_selfhosted_view_task_failed_detail_one() : m.devicesb_selfhosted_view_task_failed_detail_other({ count: failed }) });
	} else if (failed !== null) {
		checks.push({ state: 'ok', label: m.devicesb_selfhosted_view_scheduled_tasks(), detail: m.devicesb_selfhosted_view_scheduled_tasks_detail() });
	}
	const plugins = r.one('plugins_broken');
	if (plugins !== null && plugins > 0) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_plugin_broken(), detail: plugins === 1 ? m.devicesb_selfhosted_view_plugin_broken_detail_one() : m.devicesb_selfhosted_view_plugin_broken_detail_other({ count: plugins }) });
	}
	if (r.flag('pending_restart')) {
		checks.push({ state: 'advisory', label: m.devicesb_selfhosted_view_restart_pending(), detail: m.devicesb_selfhosted_view_restart_pending_detail() });
	}
	const scanAge = r.one('library_scan_age_seconds');
	const figures: FigureView[] = [
		{ label: m.devicesb_selfhosted_view_fig_streams(), value: count(r.one('streams')) },
		{ label: m.devicesb_selfhosted_view_fig_transcoding(), value: count(r.one('transcodes')) },
		{ label: m.devicesb_selfhosted_view_fig_sessions(), value: count(r.one('sessions')) },
		{ label: m.devicesb_selfhosted_view_fig_last_scan(), value: scanAge === null ? null : m.devicesb_selfhosted_view_scan_ago({ span: formatSpan(scanAge) }) }
	];
	const breakdowns: Breakdown[] = [];
	const rows: Row[] = [
		...r.all('scheduled_task_failed').map((p) => ({ label: p.labels.task ?? '', value: p.labels.error ?? '', state: 'warning' as const })),
		...r.all('plugin_broken').map((p) => ({ label: m.devicesb_selfhosted_view_plugin_row({ name: p.labels.plugin ?? '' }), value: p.labels.status ?? '', state: 'warning' as const }))
	];
	if (rows.length > 0) breakdowns.push({ title: m.devicesb_selfhosted_view_failures(), rows });
	return { version: r.label('version_info', 'version'), checks, figures, breakdowns };
}

function plex(r: Reading): Omit<AppView, 'measured'> {
	const checks: Check[] = [];
	const update = updateCheck(r, 'Plex Media Server');
	if (update) checks.push(update);
	if (r.flag('claimed') === false) {
		checks.push({ state: 'advisory', label: m.devicesb_selfhosted_view_not_claimed(), detail: m.devicesb_selfhosted_view_not_claimed_detail() });
	}
	const scanning = r.one('libraries_scanning');
	if (scanning !== null && scanning > 0) {
		checks.push({ state: 'ok', label: m.devicesb_selfhosted_view_scanning(), detail: scanning === 1 ? m.devicesb_selfhosted_view_scanning_detail_one() : m.devicesb_selfhosted_view_scanning_detail_other({ count: scanning }) });
	}
	const bandwidth = r.one('stream_bandwidth_bits_per_second');
	const figures: FigureView[] = [
		{ label: m.devicesb_selfhosted_view_fig_streams(), value: count(r.one('streams')) },
		{ label: m.devicesb_selfhosted_view_fig_transcoding(), value: count(r.one('transcodes')) },
		{ label: m.devicesb_selfhosted_view_fig_hw_transcodes(), value: count(r.one('transcodes_hardware')) },
		{ label: m.devicesb_selfhosted_view_fig_remote_streams(), value: count(r.one('streams_remote')) },
		{ label: m.devicesb_selfhosted_view_fig_bandwidth(), value: bandwidth === null ? null : `${(bandwidth / 1e6).toFixed(1)} Mbit/s` },
		{ label: m.devicesb_selfhosted_view_fig_libraries(), value: count(r.one('libraries')) }
	];
	return { version: r.label('version_info', 'version'), checks, figures, breakdowns: [] };
}

function gitlab(r: Reading): Omit<AppView, 'measured'> {
	const checks: Check[] = [];
	const readiness = r.all('readiness_check');
	const failingReadiness = readiness.filter((p) => p.value >= 1).map((p) => p.labels.check ?? '');
	if (failingReadiness.length > 0) {
		checks.push({
			state: 'warning',
			label: m.devicesb_selfhosted_view_readiness_failing(),
			detail: failingReadiness.length === 1
				? m.devicesb_selfhosted_view_readiness_failing_detail_one({ checks: failingReadiness.join(', ') })
				: m.devicesb_selfhosted_view_readiness_failing_detail_other({ checks: failingReadiness.join(', ') })
		});
	} else if (readiness.length > 0) {
		checks.push({ state: 'ok', label: m.devicesb_selfhosted_view_readiness(), detail: m.devicesb_selfhosted_view_readiness_detail() });
	}
	const runnersTotal = r.one('runners_total');
	const runnersOnline = r.one('runners_online');
	const runnersOffline = r.one('runners_offline') ?? 0;
	if (runnersTotal !== null && runnersTotal > 0 && runnersOnline === 0) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_runners_all_offline(), detail: m.devicesb_selfhosted_view_gitlab_runners_detail() });
	} else if (runnersOffline > 0) {
		checks.push({ state: 'advisory', label: m.devicesb_selfhosted_view_runners_offline(), detail: runnersOffline === 1 ? m.devicesb_selfhosted_view_runners_offline_detail_one() : m.devicesb_selfhosted_view_runners_offline_detail_other({ count: runnersOffline }) });
	}
	const migrations = r.one('migrations_pending');
	if (migrations !== null && migrations > 0) {
		checks.push({ state: 'advisory', label: m.devicesb_selfhosted_view_migrations_pending(), detail: migrations === 1 ? m.devicesb_selfhosted_view_gitlab_migrations_detail_one() : m.devicesb_selfhosted_view_gitlab_migrations_detail_other({ count: migrations }) });
	}
	if (r.flag('two_factor_required') === false) {
		checks.push({ state: 'advisory', label: m.devicesb_selfhosted_view_two_factor(), detail: m.devicesb_selfhosted_view_two_factor_detail() });
	}
	if (r.flag('signup_enabled')) {
		checks.push({ state: 'advisory', label: m.devicesb_selfhosted_view_signup_open(), detail: m.devicesb_selfhosted_view_signup_open_detail() });
	}
	if (r.flag('license_expired')) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_license_expired(), detail: m.devicesb_selfhosted_view_license_expired_detail() });
	}
	const backlog = r.all('sidekiq_queue_backlog').reduce((sum, p) => sum + p.value, 0);
	const figures: FigureView[] = [
		{ label: m.devicesb_selfhosted_view_fig_projects(), value: count(r.one('projects')) },
		{ label: m.devicesb_selfhosted_view_fig_users(), value: count(r.one('users')) },
		{ label: m.devicesb_selfhosted_view_fig_groups(), value: count(r.one('groups')) },
		{ label: m.devicesb_selfhosted_view_fig_runners_online(), value: count(runnersOnline) },
		{ label: m.devicesb_selfhosted_view_fig_sidekiq_backlog(), value: count(backlog) },
		{ label: m.devicesb_selfhosted_view_fig_open_mrs(), value: count(r.one('merge_requests')) }
	];
	const breakdowns: Breakdown[] = [];
	const queues = r
		.all('sidekiq_queue_backlog')
		.filter((p) => p.value > 0)
		.map((p) => ({ label: p.labels.queue ?? '', value: m.devicesb_selfhosted_view_queue_waiting_row({ count: count(p.value) ?? '0' }) }));
	if (queues.length > 0) breakdowns.push({ title: m.devicesb_selfhosted_view_sidekiq_queues(), rows: queues });
	const offlineRunners = r.all('runner_offline').map((p) => ({ label: p.labels.runner ?? '', state: 'warning' as const }));
	if (offlineRunners.length > 0) breakdowns.push({ title: m.devicesb_selfhosted_view_offline_runners(), rows: offlineRunners });
	const pendingMigrations = r.all('migration_pending').map((p) => ({ label: p.labels.migration ?? '' }));
	if (pendingMigrations.length > 0) breakdowns.push({ title: m.devicesb_selfhosted_view_pending_migrations(), rows: pendingMigrations });
	return { version: r.label('version_info', 'version'), checks, figures, breakdowns };
}

function forgejo(r: Reading): Omit<AppView, 'measured'> {
	const checks: Check[] = [];
	const healthz = r.flag('healthz_ok');
	if (healthz === false) {
		const failing = r.all('healthz_check').filter((p) => p.value >= 1).map((p) => p.labels.check ?? '');
		checks.push({
			state: 'warning',
			label: m.devicesb_selfhosted_view_health_failing(),
			detail: failing.length > 0 ? m.devicesb_selfhosted_view_health_failing_detail({ checks: failing.join(', ') }) : m.devicesb_selfhosted_view_health_unhealthy_detail()
		});
	} else if (healthz) {
		checks.push({ state: 'ok', label: m.devicesb_selfhosted_view_healthy(), detail: m.devicesb_selfhosted_view_healthy_detail() });
	}
	const tasks = r.all('cron_task_overdue');
	const overdue = tasks.filter((p) => p.value >= 1);
	if (overdue.length > 0) {
		checks.push({
			state: 'warning',
			label: m.devicesb_selfhosted_view_task_overdue(),
			detail: m.devicesb_selfhosted_view_task_overdue_detail({ tasks: overdue.map((p) => p.labels.task ?? '').join(', ') })
		});
	} else if (tasks.length > 0) {
		checks.push({ state: 'ok', label: m.devicesb_selfhosted_view_scheduled_tasks(), detail: m.devicesb_selfhosted_view_tasks_on_time_detail() });
	}
	const runnersTotal = r.one('runners_total');
	const runnersOnline = r.one('runners_online');
	const runnersOffline = r.one('runners_offline') ?? 0;
	if (runnersTotal !== null && runnersTotal > 0 && runnersOnline === 0) {
		checks.push({ state: 'warning', label: m.devicesb_selfhosted_view_runners_all_offline(), detail: m.devicesb_selfhosted_view_forgejo_runners_detail() });
	} else if (runnersOffline > 0) {
		checks.push({ state: 'advisory', label: m.devicesb_selfhosted_view_runners_offline(), detail: runnersOffline === 1 ? m.devicesb_selfhosted_view_runners_offline_detail_one() : m.devicesb_selfhosted_view_runners_offline_detail_other({ count: runnersOffline }) });
	}
	if (r.flag('token_is_admin') === false) {
		checks.push({
			state: 'advisory',
			label: m.devicesb_selfhosted_view_ordinary_token(),
			detail: m.devicesb_selfhosted_view_ordinary_token_detail()
		});
	}
	const figures: FigureView[] = [
		{ label: m.devicesb_selfhosted_view_fig_repos(), value: count(r.one('repos')) },
		{ label: m.devicesb_selfhosted_view_fig_users(), value: count(r.one('users')) },
		{ label: m.devicesb_selfhosted_view_fig_orgs(), value: count(r.one('orgs')) },
		{ label: m.devicesb_selfhosted_view_fig_runners_online(), value: count(runnersOnline) }
	];
	const breakdowns: Breakdown[] = [];
	const offlineRunners = r.all('runner_offline').map((p) => ({ label: p.labels.runner ?? '', state: 'warning' as const }));
	if (offlineRunners.length > 0) breakdowns.push({ title: m.devicesb_selfhosted_view_offline_runners(), rows: offlineRunners });
	return { version: r.label('version_info', 'version'), checks, figures, breakdowns };
}

const BUILDERS: Record<string, (r: Reading) => Omit<AppView, 'measured'>> = {
	nextcloud,
	immich,
	paperless,
	jellyfin,
	plex,
	gitlab,
	forgejo
};

/** Problems first, then what needs a look, then what is fine. */
const RANK: Record<CheckState, number> = { warning: 0, advisory: 1, ok: 2 };

export function buildView(kind: string, series: MetricSeries[]): AppView {
	const reading = new Reading(kind, series);
	const build = BUILDERS[kind];
	if (!build || reading.size === 0) return { version: null, checks: [], figures: [], breakdowns: [], measured: false };
	const view = build(reading);
	view.checks.sort((a, b) => RANK[a.state] - RANK[b.state]);
	return { ...view, measured: true };
}

export function overall(view: AppView): CheckState | null {
	if (!view.measured) return null;
	if (view.checks.some((c) => c.state === 'warning')) return 'warning';
	if (view.checks.some((c) => c.state === 'advisory')) return 'advisory';
	return 'ok';
}
