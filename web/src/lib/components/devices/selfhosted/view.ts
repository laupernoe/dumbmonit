/**
 * Self-hosted applications (Nextcloud, Immich, Paperless-ngx, Jellyfin, Plex):
 * what the latest stored measurement says, folded into sentences, figures and
 * short lists. Pure functions over the series of one device, so the panel
 * only lays them out.
 *
 * The collector (`crates/collectors/src/selfhosted`) writes one family per
 * fact, prefixed by the kind: `dumbmonit_nextcloud_maintenance`,
 * `dumbmonit_immich_queue_waiting{queue}`… Every state is a word as well as a
 * colour; a fact that was not measured is left out rather than shown as fine.
 */
import type { MetricSeries } from '$lib/api';
import { formatBytes } from '../docker/api';
import { formatSpan } from '../pbs/format';

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
	plex: 'Plex Media Server'
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

function plural(n: number, one: string, many: string): string {
	return `${n} ${n === 1 ? one : many}`;
}

function updateCheck(r: Reading, product: string, versionLabel = 'latest_version'): Check | null {
	const available = r.flag('update_available');
	if (available === null) return null;
	const latest = r.label('update_available', versionLabel) ?? r.label('update_available', 'available_version');
	return available
		? { state: 'advisory', label: 'Update available', detail: latest ? `${product} ${latest} is out.` : `A newer ${product} is out.` }
		: { state: 'ok', label: 'Up to date', detail: `No newer ${product} release is known.` };
}

function nextcloud(r: Reading): Omit<AppView, 'measured'> {
	const checks: Check[] = [];
	if (r.flag('maintenance')) {
		checks.push({ state: 'warning', label: 'Maintenance mode', detail: 'Nobody can log in or sync until it is turned off (occ maintenance:mode --off).' });
	}
	if (r.flag('needs_db_upgrade')) {
		checks.push({ state: 'warning', label: 'Upgrade pending', detail: 'The code was updated but the database was not: run occ upgrade.' });
	}
	if (r.flag('maintenance') === false && r.flag('needs_db_upgrade') === false) {
		checks.push({ state: 'ok', label: 'Serving', detail: 'Not in maintenance, no upgrade waiting.' });
	}
	const update = updateCheck(r, 'Nextcloud', 'available_version');
	if (update) checks.push(update);
	const apps = r.one('app_updates_available');
	if (apps !== null && apps > 0) {
		checks.push({ state: 'advisory', label: 'App updates', detail: `${plural(apps, 'app has', 'apps have')} a newer version in the app store.` });
	}
	if (r.flag('opcache_enabled') === false) {
		checks.push({ state: 'advisory', label: 'OPcache off', detail: 'PHP compiles every script on every request: enable OPcache.' });
	} else if (r.flag('opcache_full')) {
		checks.push({ state: 'warning', label: 'OPcache full', detail: 'Scripts no longer fit: raise opcache.memory_consumption.' });
	}
	const active = (window: string) => r.all('active_users').find((p) => p.labels.window === window)?.value ?? null;
	const figures: FigureView[] = [
		{ label: 'Active users, 5 min', value: count(active('5m')) },
		{ label: 'Active users, 24 h', value: count(active('24h')) },
		{ label: 'Free space', value: bytes(r.one('free_space_bytes')) },
		{ label: 'Database size', value: bytes(r.one('database_size_bytes')) },
		{ label: 'Users', value: count(r.one('users')) },
		{ label: 'Files', value: count(r.one('files')) },
		{ label: 'OPcache used', value: percent(r.one('opcache_memory_used_percent')) },
		{ label: 'OPcache hit rate', value: percent(r.one('opcache_hit_rate_percent')) }
	];
	const breakdowns: Breakdown[] = [];
	const appRows = r.all('app_update_available').map((p) => ({ label: p.labels.app ?? '', value: p.labels.available_version ?? '' }));
	if (appRows.length > 0) breakdowns.push({ title: 'App updates', rows: appRows });
	const platform = r.all('platform_info')[0]?.labels;
	if (platform) {
		breakdowns.push({
			title: 'Platform',
			rows: [
				{ label: 'Database', value: [platform.database, platform.database_version].filter(Boolean).join(' ') },
				{ label: 'PHP', value: platform.php_version ?? '—' },
				{ label: 'PHP memory limit', value: bytes(r.one('php_memory_limit_bytes')) ?? '—' }
			]
		});
	}
	return { version: r.label('version_info', 'version'), checks, figures, breakdowns };
}

const IMMICH_QUEUES: Record<string, string> = {
	thumbnailGeneration: 'Thumbnails',
	metadataExtraction: 'Metadata',
	videoConversion: 'Video conversion',
	faceDetection: 'Face detection',
	facialRecognition: 'Face recognition',
	smartSearch: 'Smart search',
	duplicateDetection: 'Duplicates',
	backgroundTask: 'Background tasks',
	storageTemplateMigration: 'Storage template',
	migration: 'Migration',
	search: 'Search',
	sidecar: 'Sidecar files',
	library: 'External libraries',
	notifications: 'Notifications',
	backupDatabase: 'Database backup',
	ocr: 'Text recognition',
	workflow: 'Workflows',
	integrityCheck: 'Integrity check',
	editor: 'Editor'
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
		checks.push({ state: 'warning', label: 'Maintenance mode', detail: 'Immich is in maintenance mode: the apps cannot sync.' });
	}
	if (stalled.length > 0) {
		checks.push({ state: 'warning', label: 'Jobs waiting', detail: `${plural(stalled.length, 'queue has', 'queues have')} jobs waiting and none running.` });
	}
	if (paused.length > 0) {
		checks.push({ state: 'advisory', label: 'Queue paused', detail: `${paused.map(([name]) => IMMICH_QUEUES[name] ?? name).join(', ')}: new photos wait until it is resumed.` });
	}
	if (queues.size > 0 && stalled.length === 0 && paused.length === 0) {
		checks.push({ state: 'ok', label: 'Jobs flowing', detail: 'No queue is paused or stuck.' });
	}
	const update = updateCheck(r, 'Immich');
	if (update) checks.push(update);
	if (r.flag('statistics_readable') === false) {
		checks.push({ state: 'advisory', label: 'Counts skipped', detail: 'The key belongs to an ordinary account: photo counts and job queues need one that can open Administration.' });
	}
	const figures: FigureView[] = [
		{ label: 'Photos', value: count(r.one('photos')) },
		{ label: 'Videos', value: count(r.one('videos')) },
		{ label: 'Library size', value: bytes(r.one('usage_bytes')) },
		{ label: 'Disk used', value: percent(r.one('storage_used_percent')) },
		{ label: 'Jobs waiting', value: count(r.one('jobs_waiting')) },
		{ label: 'Jobs running', value: count(r.one('jobs_active')) }
	];
	const busy = [...queues.entries()]
		.filter(([, q]) => q.waiting > 0 || q.active > 0 || q.failed > 0 || q.paused)
		.sort((a, b) => b[1].waiting - a[1].waiting);
	const breakdowns: Breakdown[] = [];
	if (busy.length > 0) {
		breakdowns.push({
			title: 'Job queues',
			note: 'Queues with work waiting, running, failed or paused.',
			rows: busy.map(([name, q]) => ({
				label: IMMICH_QUEUES[name] ?? name,
				value: `${count(q.waiting)} waiting · ${count(q.active)} running${q.failed > 0 ? ` · ${count(q.failed)} failed` : ''}`,
				state: q.paused ? 'advisory' : q.waiting > 0 && q.active === 0 ? 'warning' : undefined
			}))
		});
	}
	return { version: r.label('version_info', 'version'), checks, figures, breakdowns };
}

const PAPERLESS_COMPONENTS: Record<string, string> = {
	database: 'Database',
	redis: 'Redis',
	celery: 'Celery workers',
	index: 'Search index',
	classifier: 'Classifier',
	sanity_check: 'Sanity check',
	llm_index: 'AI index'
};

function paperless(r: Reading): Omit<AppView, 'measured'> {
	const checks: Check[] = [];
	const components = r.all('component_status');
	const broken = components.filter((p) => p.value >= 2).map((p) => PAPERLESS_COMPONENTS[p.labels.component ?? ''] ?? p.labels.component);
	if (broken.length > 0) {
		checks.push({ state: 'warning', label: 'Component error', detail: `${broken.join(', ')} reported an error.` });
	} else if (components.length > 0) {
		checks.push({ state: 'ok', label: 'Components', detail: 'Database, Redis, Celery and the index answer.' });
	}
	const failed = r.one('tasks_failed_recent');
	if (failed !== null && failed > 0) {
		checks.push({ state: 'warning', label: 'Failed tasks', detail: `${plural(failed, 'task', 'tasks')} failed recently and were not dismissed.` });
	}
	const migrations = r.one('unapplied_migrations');
	if (migrations !== null && migrations > 0) {
		checks.push({ state: 'warning', label: 'Migrations pending', detail: `${plural(migrations, 'database migration', 'database migrations')} left to apply.` });
	}
	const update = updateCheck(r, 'Paperless-ngx');
	if (update) checks.push(update);
	if (r.flag('status_readable') === false) {
		checks.push({ state: 'advisory', label: 'Health not readable', detail: 'The account lacks the System Monitoring permission: component health is skipped.' });
	}
	const figures: FigureView[] = [
		{ label: 'Documents', value: count(r.one('documents')) },
		{ label: 'In the inbox', value: count(r.one('documents_inbox')) },
		{ label: 'Tasks pending', value: count(r.one('tasks_pending')) },
		{ label: 'Failed, 30 days', value: count(r.one('tasks_failed_window')) },
		{ label: 'Disk used', value: percent(r.one('storage_used_percent')) },
		{ label: 'Disk free', value: bytes(r.one('storage_available_bytes')) }
	];
	const breakdowns: Breakdown[] = [];
	if (components.length > 0) {
		breakdowns.push({
			title: 'Components',
			rows: components.map((p) => ({
				label: PAPERLESS_COMPONENTS[p.labels.component ?? ''] ?? p.labels.component ?? '',
				state: p.value >= 2 ? 'warning' : p.value >= 1 ? 'advisory' : 'ok'
			}))
		});
	}
	const failedRows = r.all('task_failed').map((p) => ({ label: p.labels.file || p.labels.task || 'task', value: p.labels.task ?? '', state: 'warning' as const }));
	if (failedRows.length > 0) breakdowns.push({ title: 'Failed tasks', rows: failedRows });
	return { version: r.label('version_info', 'version'), checks, figures, breakdowns };
}

function jellyfin(r: Reading): Omit<AppView, 'measured'> {
	const checks: Check[] = [];
	const failed = r.one('scheduled_tasks_failed');
	if (failed !== null && failed > 0) {
		checks.push({ state: 'warning', label: 'Task failed', detail: `${plural(failed, 'scheduled task', 'scheduled tasks')} failed on the last run.` });
	} else if (failed !== null) {
		checks.push({ state: 'ok', label: 'Scheduled tasks', detail: 'None failed on its last run.' });
	}
	const plugins = r.one('plugins_broken');
	if (plugins !== null && plugins > 0) {
		checks.push({ state: 'warning', label: 'Plugin broken', detail: `${plural(plugins, 'plugin does', 'plugins do')} not load.` });
	}
	if (r.flag('pending_restart')) {
		checks.push({ state: 'advisory', label: 'Restart pending', detail: 'A plugin install or update waits for Jellyfin to restart.' });
	}
	const scanAge = r.one('library_scan_age_seconds');
	const figures: FigureView[] = [
		{ label: 'Streams', value: count(r.one('streams')) },
		{ label: 'Transcoding', value: count(r.one('transcodes')) },
		{ label: 'Sessions', value: count(r.one('sessions')) },
		{ label: 'Last library scan', value: scanAge === null ? null : `${formatSpan(scanAge)} ago` }
	];
	const breakdowns: Breakdown[] = [];
	const rows: Row[] = [
		...r.all('scheduled_task_failed').map((p) => ({ label: p.labels.task ?? '', value: p.labels.error ?? '', state: 'warning' as const })),
		...r.all('plugin_broken').map((p) => ({ label: `Plugin ${p.labels.plugin ?? ''}`, value: p.labels.status ?? '', state: 'warning' as const }))
	];
	if (rows.length > 0) breakdowns.push({ title: 'Failures', rows });
	return { version: r.label('version_info', 'version'), checks, figures, breakdowns };
}

function plex(r: Reading): Omit<AppView, 'measured'> {
	const checks: Check[] = [];
	const update = updateCheck(r, 'Plex Media Server');
	if (update) checks.push(update);
	if (r.flag('claimed') === false) {
		checks.push({ state: 'advisory', label: 'Not claimed', detail: 'The server is not linked to a Plex account: anyone on the network can manage it.' });
	}
	const scanning = r.one('libraries_scanning');
	if (scanning !== null && scanning > 0) {
		checks.push({ state: 'ok', label: 'Scanning', detail: `${plural(scanning, 'library is', 'libraries are')} being scanned.` });
	}
	const bandwidth = r.one('stream_bandwidth_bits_per_second');
	const figures: FigureView[] = [
		{ label: 'Streams', value: count(r.one('streams')) },
		{ label: 'Transcoding', value: count(r.one('transcodes')) },
		{ label: 'Hardware transcodes', value: count(r.one('transcodes_hardware')) },
		{ label: 'Remote streams', value: count(r.one('streams_remote')) },
		{ label: 'Bandwidth', value: bandwidth === null ? null : `${(bandwidth / 1e6).toFixed(1)} Mbit/s` },
		{ label: 'Libraries', value: count(r.one('libraries')) }
	];
	return { version: r.label('version_info', 'version'), checks, figures, breakdowns: [] };
}

const BUILDERS: Record<string, (r: Reading) => Omit<AppView, 'measured'>> = { nextcloud, immich, paperless, jellyfin, plex };

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
