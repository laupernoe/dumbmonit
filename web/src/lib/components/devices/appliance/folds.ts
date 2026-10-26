/**
 * How each appliance's stored series become a panel: a verdict sentence, a
 * few figures and the lists worth reading. One fold per kind, all pure, all
 * reading `dumbmonit_<prefix>_*` series as the collectors write them
 * (`crates/collectors/src/{pfsense,unraid,veeam,tailscale,fortigate,sophos,meraki,omada,nginx,apache}`,
 * and the Hyper-V preset of the Windows agent).
 */
import type { MetricSeries } from '#lib/api/index.js';
import type { Tone } from '#lib/ui/index.js';
import { m } from '#lib/paraglide/messages.js';
import { formatAge, formatBytes } from '../docker/api';

export interface Figure {
	label: string;
	value: string | null;
	tone?: 'ink' | 'signal' | 'advisory' | 'warning';
	hint?: string;
}

export interface Row {
	key: string;
	tone: Tone;
	plate: string;
	name: string;
	details?: string[];
}

export interface Section {
	title: string;
	rows: Row[];
	/** A closing line, for instance what was left out. */
	more?: string;
}

export interface ApplianceView {
	title: string;
	description: string;
	verdict: { tone: 'signal' | 'advisory' | 'warning'; text: string };
	figures: Figure[];
	sections: Section[];
}

export interface Fold {
	/** The metric prefix after `dumbmonit_`, without the trailing underscore. */
	prefix: string;
	/** Panel title while nothing is loaded. */
	title: string;
	fold: (series: MetricSeries[], nowSeconds: number) => ApplianceView;
}

/** One reading: metric name without its prefix, labels, last value. */
interface Point {
	name: string;
	labels: Record<string, string>;
	value: number;
}

function points(series: MetricSeries[], prefix: string): Point[] {
	const out: Point[] = [];
	for (const s of series) {
		const value = Number(s.values.at(-1)?.[1]);
		if (!Number.isFinite(value)) continue;
		out.push({ name: (s.metric.__name__ ?? '').replace(`dumbmonit_${prefix}_`, ''), labels: s.metric, value });
	}
	return out;
}

/** The value of an unlabelled series, or null. */
function single(ps: Point[], name: string): number | null {
	return ps.find((p) => p.name === name)?.value ?? null;
}

/** Every series of that name, keyed by one label. */
function byLabel(ps: Point[], name: string, label: string): Map<string, Point> {
	const out = new Map<string, Point>();
	for (const p of ps) if (p.name === name && p.labels[label] !== undefined) out.set(p.labels[label], p);
	return out;
}

function percent(value: number | null): string | null {
	return value === null ? null : `${Math.round(value)}%`;
}

function days(seconds: number): string {
	if (seconds < 0) return `expired ${formatAge(-seconds)} ago`;
	return `in ${formatAge(seconds)}`;
}

function verdict(problems: { text: string; severe: boolean }[], healthy: string): ApplianceView['verdict'] {
	if (problems.length === 0) return { tone: 'signal', text: healthy };
	const text = problems.map((p) => p.text).join(', ');
	return {
		tone: problems.some((p) => p.severe) ? 'warning' : 'advisory',
		text: `${text.charAt(0).toUpperCase()}${text.slice(1)}.`
	};
}

function byName<T extends { name: string }>(a: T, b: T): number {
	return a.name.localeCompare(b.name, 'en');
}

// --------------------------------------------------------------------- pfSense

function pfsense(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'pfsense');
	const version = ps.find((p) => p.name === 'version_info')?.labels.version ?? null;
	const loss = byLabel(ps, 'gateway_loss_percent', 'gateway');
	const delay = byLabel(ps, 'gateway_delay_milliseconds', 'gateway');
	const gateways: Row[] = [...byLabel(ps, 'gateway_up', 'gateway').values()].map((p) => {
		const name = p.labels.gateway;
		const l = loss.get(name)?.value ?? null;
		const d = delay.get(name)?.value ?? null;
		const up = p.value >= 1;
		const lossy = up && l !== null && l > 10;
		return {
			key: name,
			name,
			tone: !up ? 'warning' : lossy ? 'advisory' : 'signal',
			plate: !up ? 'Down' : lossy ? 'Losing packets' : 'Online',
			details: [d !== null ? `${d.toFixed(1)} ms` : '', l !== null ? `${Math.round(l)}% loss` : ''].filter(Boolean)
		};
	});
	gateways.sort(byName);
	const interfaces: Row[] = [...byLabel(ps, 'interface_down', 'interface').values()]
		.filter((p) => p.value >= 1)
		.map((p) => ({ key: p.labels.interface, name: `${p.labels.descr ?? p.labels.interface} (${p.labels.interface})`, tone: 'warning', plate: 'No link' }));
	const services: Row[] = [...byLabel(ps, 'service_stopped', 'service').values()]
		.filter((p) => p.value >= 1)
		.map((p) => ({ key: p.labels.service, name: p.labels.description ?? p.labels.service, tone: 'warning', plate: 'Stopped', details: [p.labels.service] }));
	const down = gateways.filter((g) => g.plate === 'Down').length;
	const lossy = gateways.filter((g) => g.plate === 'Losing packets').length;
	const problems = [];
	if (down > 0) problems.push({ text: m.misc2_folds_gateway_down({ count: down }), severe: true });
	if (lossy > 0) problems.push({ text: m.misc2_folds_gateway_losing_packets({ count: lossy }), severe: false });
	if (interfaces.length > 0) problems.push({ text: m.misc2_folds_interface_without_link({ count: interfaces.length }), severe: false });
	if (services.length > 0) problems.push({ text: m.misc2_folds_service_stopped({ count: services.length }), severe: false });
	const disk = single(ps, 'disk_used_percent');
	if (disk !== null && disk > 90) problems.push({ text: 'disk nearly full', severe: false });
	const update = single(ps, 'restapi_update_available');
	return {
		title: 'Firewall',
		description: version ? `pfSense ${version}, read through the REST API package.` : 'Read through the REST API package.',
		verdict: verdict(problems, 'Every gateway is online and every enabled service runs.'),
		figures: [
			{ label: 'CPU', value: percent(single(ps, 'cpu_usage_percent')) },
			{ label: 'Memory', value: percent(single(ps, 'memory_used_percent')) },
			{ label: 'Disk', value: percent(disk), tone: disk !== null && disk > 90 ? 'advisory' : 'ink' },
			{ label: 'Temperature', value: single(ps, 'temperature_celsius') !== null ? `${Math.round(single(ps, 'temperature_celsius') ?? 0)} °C` : null }
		],
		sections: [
			{ title: 'Gateways', rows: gateways },
			{ title: 'Interfaces without link', rows: interfaces },
			{ title: 'Stopped services', rows: services },
			{
				title: 'REST API package',
				rows:
					update !== null && update >= 1
						? [{ key: 'restapi', name: `Version ${ps.find((p) => p.name === 'restapi_version_info')?.labels.latest ?? ''} is available`, tone: 'info', plate: 'Update' }]
						: []
			}
		]
	};
}

// ---------------------------------------------------------------------- Unraid

function unraid(series: MetricSeries[], now: number): ApplianceView {
	const ps = points(series, 'unraid');
	void now;
	const version = ps.find((p) => p.name === 'version_info')?.labels.version ?? null;
	const state = ps.find((p) => p.name === 'array_state_info')?.labels.state ?? null;
	const started = single(ps, 'array_started');
	const temps = byLabel(ps, 'disk_temperature_celsius', 'disk');
	const errors = byLabel(ps, 'disk_errors', 'disk');
	const used = byLabel(ps, 'disk_used_percent', 'disk');
	const statuses = byLabel(ps, 'disk_status_info', 'disk');
	const ROLE_ORDER: Record<string, number> = { parity: 0, data: 1, cache: 2 };
	const disks: (Row & { role: string })[] = [...byLabel(ps, 'disk_ok', 'disk').values()].map((p) => {
		const name = p.labels.disk;
		const ok = p.value >= 1;
		const err = errors.get(name)?.value ?? 0;
		const t = temps.get(name)?.value ?? null;
		const u = used.get(name)?.value ?? null;
		const status = statuses.get(name)?.labels.status ?? '';
		return {
			key: name,
			name,
			role: p.labels.role ?? '',
			tone: !ok ? 'warning' : err > 0 ? 'advisory' : 'signal',
			plate: !ok ? status.replace(/^disk_/, '').toUpperCase() || 'Problem' : err > 0 ? `${err} errors` : 'OK',
			details: [p.labels.role ?? '', t !== null ? `${Math.round(t)} °C` : 'spun down', u !== null ? `${Math.round(u)}% used` : ''].filter(Boolean)
		};
	});
	disks.sort((a, b) => (ROLE_ORDER[a.role] ?? 9) - (ROLE_ORDER[b.role] ?? 9) || a.name.localeCompare(b.name, 'en', { numeric: true }));
	const containers: Row[] = [...byLabel(ps, 'container_autostart_stopped', 'container').values()]
		.filter((p) => p.value >= 1)
		.map((p) => ({ key: p.labels.container, name: p.labels.container, tone: 'advisory', plate: 'Stopped' }));
	const parityOk = single(ps, 'parity_check_ok');
	const parityAge = single(ps, 'parity_check_age_seconds');
	const parityRunning = single(ps, 'parity_check_running');
	const progress = single(ps, 'parity_check_progress_percent');
	const bad = disks.filter((d) => d.tone === 'warning').length;
	const cacheFull = disks.filter((d) => d.role === 'cache' && (used.get(d.name)?.value ?? 0) > 90).length;
	const problems = [];
	if (started !== null && started < 1) problems.push({ text: `the array is ${state ?? 'stopped'}`, severe: true });
	if (bad > 0) problems.push({ text: m.misc2_folds_disk_disabled_or_missing({ count: bad }), severe: true });
	const withErrors = disks.filter((d) => d.tone === 'advisory').length;
	if (withErrors > 0) problems.push({ text: m.misc2_folds_disk_with_read_errors({ count: withErrors }), severe: false });
	if (parityOk !== null && parityOk < 1) problems.push({ text: 'the last parity check found errors', severe: false });
	if (parityAge !== null && parityAge > 40 * 86400) problems.push({ text: 'no parity check for over 40 days', severe: false });
	if (cacheFull > 0) problems.push({ text: 'cache nearly full', severe: false });
	if (containers.length > 0) problems.push({ text: m.misc2_folds_container_stopped({ count: containers.length }), severe: false });
	const parityText =
		parityRunning !== null && parityRunning >= 1
			? `Running${progress !== null ? `, ${Math.round(progress)}%` : ''}`
			: parityAge !== null
				? `${formatAge(parityAge)} ago`
				: null;
	return {
		title: 'Array',
		description: version ? `Unraid ${version}, read through its API.` : 'Read through the Unraid API.',
		verdict: verdict(problems, 'The array is started and every disk is healthy.'),
		figures: [
			{ label: 'Array used', value: percent(single(ps, 'array_used_percent')), hint: formatBytes(single(ps, 'array_size_bytes')) },
			{ label: 'Parity check', value: parityText, tone: parityOk !== null && parityOk < 1 ? 'warning' : 'ink' },
			{ label: 'Containers running', value: single(ps, 'containers') !== null ? `${single(ps, 'containers_running') ?? 0}/${single(ps, 'containers')}` : null },
			{ label: 'VMs running', value: single(ps, 'vms') !== null ? `${single(ps, 'vms_running') ?? 0}/${single(ps, 'vms')}` : null }
		],
		sections: [
			{ title: 'Disks', rows: disks },
			{ title: 'Containers that should be running', rows: containers }
		]
	};
}

// ----------------------------------------------------------------------- Veeam

function veeam(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'veeam');
	const version = ps.find((p) => p.name === 'version_info')?.labels.version ?? null;
	const ages = byLabel(ps, 'job_last_run_age_seconds', 'job');
	const failed = byLabel(ps, 'job_failed', 'job');
	const warning = byLabel(ps, 'job_warning', 'job');
	const running = byLabel(ps, 'job_running', 'job');
	const jobs: Row[] = [...byLabel(ps, 'job_enabled', 'job').values()]
		.filter((p) => p.value >= 1)
		.map((p) => {
			const name = p.labels.job;
			const isFailed = (failed.get(name)?.value ?? 0) >= 1;
			const isWarning = (warning.get(name)?.value ?? 0) >= 1;
			const isRunning = (running.get(name)?.value ?? 0) >= 1;
			const age = ages.get(name)?.value ?? null;
			return {
				key: name,
				name,
				tone: isFailed ? 'warning' : isWarning ? 'advisory' : isRunning ? 'info' : age === null ? 'ghost' : 'signal',
				plate: isFailed ? 'Failed' : isWarning ? 'Warning' : isRunning ? 'Running' : age === null ? 'Never run' : 'Success',
				details: [p.labels.type ?? '', age !== null ? `last run ${formatAge(age)} ago` : ''].filter(Boolean)
			};
		});
	const RANK: Record<string, number> = { warning: 0, advisory: 1, info: 2, signal: 3, ghost: 4 };
	jobs.sort((a, b) => (RANK[a.tone] ?? 9) - (RANK[b.tone] ?? 9) || byName(a, b));
	const free = byLabel(ps, 'repository_free_bytes', 'repository');
	const repositories: Row[] = [...byLabel(ps, 'repository_used_percent', 'repository').values()].map((p) => ({
		key: p.labels.repository,
		name: p.labels.repository,
		tone: p.value > 90 ? 'warning' : p.value > 80 ? 'advisory' : 'signal',
		plate: `${Math.round(p.value)}% used`,
		details: [`${formatBytes(free.get(p.labels.repository)?.value)} free`, p.labels.type ?? '']
	}));
	repositories.sort(byName);
	const licenseReadable = byLabel(ps, 'section_readable', 'section').get('license')?.value;
	const expiry = single(ps, 'license_expiry_seconds');
	const failedCount = jobs.filter((j) => j.plate === 'Failed').length;
	const warningCount = jobs.filter((j) => j.plate === 'Warning').length;
	const full = repositories.filter((r) => r.tone === 'warning').length;
	const problems = [];
	if (failedCount > 0) problems.push({ text: m.misc2_folds_job_failed({ count: failedCount }), severe: true });
	if (warningCount > 0) problems.push({ text: m.misc2_folds_job_ended_with_a_warning({ count: warningCount }), severe: false });
	if (full > 0) problems.push({ text: m.misc2_folds_repository_nearly_full({ count: full }), severe: false });
	if (expiry !== null && expiry < 30 * 86400) problems.push({ text: `the license expires ${days(expiry)}`, severe: expiry < 0 });
	return {
		title: 'Backups',
		description: version ? `Veeam Backup & Replication ${version}, read through its REST API.` : 'Read through the Veeam REST API.',
		verdict: verdict(problems, 'Every enabled job succeeded on its last run.'),
		figures: [
			{ label: 'Jobs failed', value: single(ps, 'jobs_failed') !== null ? String(single(ps, 'jobs_failed')) : null, tone: failedCount > 0 ? 'warning' : 'ink' },
			{ label: 'Failed sessions, 24 h', value: single(ps, 'sessions_failed_24h') !== null ? String(single(ps, 'sessions_failed_24h')) : null },
			{ label: 'Jobs running', value: single(ps, 'jobs_running') !== null ? String(single(ps, 'jobs_running')) : null },
			{
				label: 'License',
				value: expiry !== null ? days(expiry) : licenseReadable === 0 ? 'Not readable' : null,
				tone: expiry !== null && expiry < 30 * 86400 ? 'advisory' : 'ink',
				hint: licenseReadable === 0 ? 'Veeam keeps it for the Backup Administrator role.' : undefined
			}
		],
		sections: [
			{ title: 'Jobs', rows: jobs },
			{ title: 'Repositories', rows: repositories }
		]
	};
}

// ----------------------------------------------------------------------- Kopia

function kopia(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'kopia');
	const info = ps.find((p) => p.name === 'repo_info')?.labels ?? null;
	const connected = single(ps, 'repo_connected');
	const ages = byLabel(ps, 'source_last_snapshot_age_seconds', 'source');
	const errors = byLabel(ps, 'source_last_snapshot_errors', 'source');
	const scheduled = byLabel(ps, 'source_scheduled', 'source');
	const uploading = byLabel(ps, 'source_uploading', 'source');
	const bytes = byLabel(ps, 'source_last_snapshot_bytes', 'source');
	const sources: Row[] = [...byLabel(ps, 'source_has_snapshot', 'source').values()].map((p) => {
		const name = p.labels.source;
		const age = ages.get(name)?.value ?? null;
		const failed = errors.get(name)?.value ?? 0;
		const running = (uploading.get(name)?.value ?? 0) >= 1;
		const behind = (scheduled.get(name)?.value ?? 0) >= 1 && age !== null && age > 2 * 86400;
		const withErrors = failed > 0;
		return {
			key: name,
			name,
			tone: behind || withErrors ? 'advisory' : running ? 'info' : age === null ? 'ghost' : 'signal',
			plate: behind ? 'Behind schedule' : withErrors ? 'With errors' : running ? 'Running' : age === null ? 'No snapshot' : 'Healthy',
			details: [
				age !== null ? `last snapshot ${formatAge(age)} ago` : '',
				withErrors ? `${Math.round(failed)} unreadable` : '',
				bytes.has(name) ? formatBytes(bytes.get(name)?.value) : ''
			].filter(Boolean)
		};
	});
	const RANK: Record<string, number> = { warning: 0, advisory: 1, info: 2, signal: 3, ghost: 4 };
	sources.sort((a, b) => (RANK[a.tone] ?? 9) - (RANK[b.tone] ?? 9) || byName(a, b));
	const behindCount = sources.filter((r) => r.plate === 'Behind schedule').length;
	const errorCount = sources.filter((r) => r.plate === 'With errors').length;
	const problems = [];
	if (connected !== null && connected < 1) problems.push({ text: 'the server is not connected to its repository', severe: true });
	if (behindCount > 0) problems.push({ text: `${behindCount} scheduled ${behindCount === 1 ? 'source has' : 'sources have'} no recent snapshot`, severe: false });
	if (errorCount > 0) problems.push({ text: `${errorCount} ${errorCount === 1 ? 'source' : 'sources'} with unreadable files in the last snapshot`, severe: false });
	return {
		title: 'Backups',
		description: info?.storage ? `Kopia server, repository on ${info.storage} storage.` : 'Read through the Kopia server API (beta).',
		verdict: verdict(problems, 'Every source has a recent snapshot without errors.'),
		figures: [
			{ label: 'Repository', value: connected === null ? null : connected >= 1 ? 'Connected' : 'Disconnected', tone: connected !== null && connected < 1 ? 'warning' : 'ink' },
			{ label: 'Sources', value: single(ps, 'sources') !== null ? String(single(ps, 'sources')) : null },
			{ label: 'With errors', value: single(ps, 'sources_with_errors') !== null ? String(single(ps, 'sources_with_errors')) : null, tone: errorCount > 0 ? 'advisory' : 'ink' },
			{ label: 'Running now', value: single(ps, 'sources_uploading') !== null ? String(single(ps, 'sources_uploading')) : null }
		],
		sections: [{ title: 'Sources', rows: sources }]
	};
}

// -------------------------------------------------------------------- Duplicati

function duplicati(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'duplicati');
	const failed = byLabel(ps, 'backup_failed', 'backup');
	const running = byLabel(ps, 'backup_running', 'backup');
	const ages = byLabel(ps, 'backup_last_success_age_seconds', 'backup');
	const scheduled = byLabel(ps, 'backup_scheduled', 'backup');
	const warnings = byLabel(ps, 'backup_last_warnings', 'backup');
	const sizes = byLabel(ps, 'backup_source_bytes', 'backup');
	const messages = new Map<string, string>();
	for (const p of ps) if (p.name === 'backup_last_error_info' && p.labels.message) messages.set(p.labels.backup, p.labels.message);
	const backups: Row[] = [...failed.values()].map((p) => {
		const name = p.labels.backup;
		const isFailed = p.value >= 1;
		const isRunning = (running.get(name)?.value ?? 0) >= 1;
		const age = ages.get(name)?.value ?? null;
		const old = (scheduled.get(name)?.value ?? 0) >= 1 && age !== null && age > 2 * 86400;
		const warned = (warnings.get(name)?.value ?? 0) > 0;
		return {
			key: name,
			name,
			tone: isFailed ? 'warning' : old || warned ? 'advisory' : isRunning ? 'info' : age === null ? 'ghost' : 'signal',
			plate: isFailed ? 'Failed' : old ? 'Too old' : warned ? 'Warnings' : isRunning ? 'Running' : age === null ? 'Never succeeded' : 'Success',
			details: [
				age !== null ? `last success ${formatAge(age)} ago` : '',
				sizes.has(name) ? formatBytes(sizes.get(name)?.value) : '',
				isFailed ? (messages.get(name) ?? '') : ''
			].filter(Boolean)
		};
	});
	const RANK: Record<string, number> = { warning: 0, advisory: 1, info: 2, signal: 3, ghost: 4 };
	backups.sort((a, b) => (RANK[a.tone] ?? 9) - (RANK[b.tone] ?? 9) || byName(a, b));
	const failedCount = backups.filter((b) => b.plate === 'Failed').length;
	const oldCount = backups.filter((b) => b.plate === 'Too old').length;
	const paused = single(ps, 'server_paused');
	const problems = [];
	if (failedCount > 0) problems.push({ text: `${failedCount} ${failedCount === 1 ? 'backup' : 'backups'} failed on the last run`, severe: true });
	if (oldCount > 0) problems.push({ text: `${oldCount} scheduled ${oldCount === 1 ? 'backup has' : 'backups have'} not succeeded for two days`, severe: false });
	if (paused !== null && paused >= 1) problems.push({ text: 'the server is paused', severe: false });
	return {
		title: 'Backups',
		description: 'Read through the Duplicati server API (beta).',
		verdict: verdict(problems, 'Every backup succeeded on its last run.'),
		figures: [
			{ label: 'Backups failed', value: single(ps, 'backups_failed') !== null ? String(single(ps, 'backups_failed')) : null, tone: failedCount > 0 ? 'warning' : 'ink' },
			{ label: 'Backups', value: single(ps, 'backups') !== null ? String(single(ps, 'backups')) : null },
			{ label: 'Running now', value: single(ps, 'backups_running') !== null ? String(single(ps, 'backups_running')) : null },
			{ label: 'Server', value: paused === null ? null : paused >= 1 ? 'Paused' : 'Active', tone: paused !== null && paused >= 1 ? 'advisory' : 'ink' }
		],
		sections: [{ title: 'Backups', rows: backups }]
	};
}

// ------------------------------------------------------------------- Tailscale

function tailscale(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'tailscale');
	const watched = byLabel(ps, 'device_watched', 'device');
	const offline = byLabel(ps, 'device_offline_seconds', 'device');
	const keys = byLabel(ps, 'device_key_expiry_seconds', 'device');
	const updates = byLabel(ps, 'device_update_available', 'device');
	const authorized = byLabel(ps, 'device_authorized', 'device');
	const info = byLabel(ps, 'device_info', 'device');
	const devices: Row[] = [];
	for (const p of byLabel(ps, 'device_online', 'device').values()) {
		const name = p.labels.device;
		const online = p.value >= 1;
		const isWatched = (watched.get(name)?.value ?? 0) >= 1;
		const silent = offline.get(name)?.value ?? null;
		const key = keys.get(name)?.value ?? null;
		const pending = (authorized.get(name)?.value ?? 1) < 1;
		const keySoon = key !== null && key < 14 * 86400;
		const update = (updates.get(name)?.value ?? 0) >= 1;
		// Only what needs a look: watched devices, and anything pending, expiring or outdated.
		if (!isWatched && !pending && !keySoon) continue;
		const details = [info.get(name)?.labels.os ?? ''];
		if (keySoon && key !== null) details.push(`key expires ${days(key)}`);
		if (update) details.push('update available');
		devices.push({
			key: name,
			name,
			tone: pending ? 'advisory' : !online && isWatched ? 'warning' : keySoon ? 'advisory' : online ? 'signal' : 'muted',
			plate: pending ? 'Awaiting approval' : online ? 'Online' : silent !== null ? `Offline ${formatAge(silent)}` : 'Offline',
			details: details.filter(Boolean)
		});
	}
	const RANK: Record<string, number> = { warning: 0, advisory: 1, muted: 2, signal: 3 };
	devices.sort((a, b) => (RANK[a.tone] ?? 9) - (RANK[b.tone] ?? 9) || byName(a, b));
	const down = single(ps, 'devices_watched_offline') ?? 0;
	const pending = single(ps, 'devices_unauthorized') ?? 0;
	const expiring = [...keys.values()].filter((k) => k.value < 14 * 86400).length;
	const problems = [];
	if (down > 0) problems.push({ text: m.misc2_folds_watched_device_offline({ count: down }), severe: true });
	if (pending > 0) problems.push({ text: m.misc2_folds_device_awaiting_approval({ count: pending }), severe: false });
	if (expiring > 0) problems.push({ text: m.misc2_folds_node_key_expiring_within_14_days({ count: expiring }), severe: false });
	return {
		title: 'Tailnet',
		description: 'Read from the Tailscale API.',
		verdict: verdict(problems, 'Every watched device is online.'),
		figures: [
			{ label: 'Devices online', value: single(ps, 'devices') !== null ? `${single(ps, 'devices_online') ?? 0}/${single(ps, 'devices')}` : null },
			{ label: 'Watched offline', value: String(down), tone: down > 0 ? 'warning' : 'ink' },
			{ label: 'Updates available', value: single(ps, 'devices_update_available') !== null ? String(single(ps, 'devices_update_available')) : null },
			{ label: 'Shared in', value: single(ps, 'devices_external') !== null ? String(single(ps, 'devices_external')) : null }
		],
		sections: [{ title: 'Watched devices and devices needing a look', rows: devices }]
	};
}

// ------------------------------------------------------------------- FortiGate

function fortigate(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'fortigate');
	const info = ps.find((p) => p.name === 'version_info')?.labels;
	const tunnels: Row[] = [...byLabel(ps, 'ipsec_tunnel_up', 'tunnel').values()].map((p) => ({
		key: p.labels.tunnel,
		name: p.labels.tunnel,
		tone: p.value >= 1 ? 'signal' : 'warning',
		plate: p.value >= 1 ? 'Up' : 'Down'
	}));
	tunnels.sort((a, b) => Number(a.tone === 'signal') - Number(b.tone === 'signal') || byName(a, b));
	const interfaces: Row[] = [...byLabel(ps, 'interface_down', 'interface').values()]
		.filter((p) => p.value >= 1)
		.map((p) => ({ key: p.labels.interface, name: p.labels.alias ? `${p.labels.alias} (${p.labels.interface})` : p.labels.interface, tone: 'warning', plate: 'No link' }));
	const licenses: Row[] = [...byLabel(ps, 'license_expiry_seconds', 'license').values()]
		.filter((p) => p.value < 60 * 86400)
		.map((p) => ({ key: p.labels.license, name: p.labels.license.replace(/_/g, ' '), tone: p.value < 0 ? 'warning' : 'advisory', plate: p.value < 0 ? 'Expired' : `Expires ${days(p.value)}` }));
	const members = single(ps, 'ha_members');
	const inSync = single(ps, 'ha_in_sync');
	const memory = single(ps, 'memory_used_percent');
	const update = single(ps, 'firmware_update_available');
	const latest = ps.find((p) => p.name === 'firmware_latest_info')?.labels.version ?? null;
	const tunnelsDown = tunnels.filter((t) => t.tone === 'warning').length;
	const problems = [];
	if (tunnelsDown > 0) problems.push({ text: m.misc2_folds_ipsec_tunnel_down({ count: tunnelsDown }), severe: true });
	if (interfaces.length > 0) problems.push({ text: m.misc2_folds_interface_without_link({ count: interfaces.length }), severe: false });
	if (inSync !== null && inSync < 1) problems.push({ text: 'the HA cluster is out of sync', severe: false });
	if (memory !== null && memory > 85) problems.push({ text: 'memory near conserve mode', severe: true });
	const expired = licenses.filter((l) => l.tone === 'warning').length;
	if (expired > 0) problems.push({ text: m.misc2_folds_licence_expired({ count: expired }), severe: false });
	return {
		title: 'Firewall',
		description: info ? `${info.model || 'FortiGate'} ${info.hostname ? `"${info.hostname}" ` : ''}running FortiOS ${info.version}.` : 'Read through the FortiOS REST API.',
		verdict: verdict(problems, 'Every tunnel is up and every addressed interface has its link.'),
		figures: [
			{ label: 'CPU', value: percent(single(ps, 'cpu_usage_percent')) },
			{ label: 'Memory', value: percent(memory), tone: memory !== null && memory > 85 ? 'warning' : 'ink' },
			{ label: 'Sessions', value: single(ps, 'sessions') !== null ? String(single(ps, 'sessions')) : null },
			{
				label: 'HA',
				value: members === null || members === 0 ? 'Standalone' : inSync === null ? m.misc2_folds_member_count({ count: members }) : inSync >= 1 ? 'In sync' : 'Out of sync',
				tone: inSync !== null && inSync < 1 ? 'advisory' : 'ink'
			}
		],
		sections: [
			{ title: 'IPsec tunnels', rows: tunnels },
			{ title: 'Interfaces without link', rows: interfaces },
			{ title: 'Licences expiring or expired', rows: licenses },
			{ title: 'Firmware', rows: update !== null && update >= 1 && latest ? [{ key: 'fw', name: `FortiOS ${latest} is available`, tone: 'info', plate: 'Update' }] : [] }
		]
	};
}

// ---------------------------------------------------------------------- Sophos

function sophos(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'sophos');
	const links = byLabel(ps, 'interface_link_up', 'interface');
	const enabled = byLabel(ps, 'interface_enabled', 'interface');
	const interfaces: Row[] = [...byLabel(ps, 'interface_down', 'interface').values()]
		.filter((p) => p.labels.zone && p.labels.zone !== 'None')
		.map((p) => {
			const name = p.labels.interface;
			const on = (enabled.get(name)?.value ?? 1) >= 1;
			const link = links.get(name)?.value ?? null;
			return {
				key: name,
				name,
				tone: p.value >= 1 ? 'warning' : !on ? 'muted' : link === null ? 'ghost' : 'signal',
				plate: p.value >= 1 ? 'No link' : !on ? 'Switched off' : link === null ? 'Unknown' : 'Connected',
				details: [p.labels.zone]
			};
		});
	interfaces.sort((a, b) => Number(a.tone !== 'warning') - Number(b.tone !== 'warning') || a.name.localeCompare(b.name, 'en', { numeric: true }));
	const connections: Row[] = [...byLabel(ps, 'ipsec_connection_activated', 'connection').values()].map((p) => ({
		key: p.labels.connection,
		name: p.labels.connection,
		tone: p.value >= 1 ? 'muted' : 'ghost',
		plate: p.value >= 1 ? 'Activated' : 'Deactivated'
	}));
	connections.sort(byName);
	const down = single(ps, 'interfaces_down') ?? 0;
	const api = ps.find((p) => p.name === 'api_version_info')?.labels.api_version ?? null;
	return {
		title: 'Firewall',
		description: api ? `Sophos Firewall, XML API ${api}. Tunnel, HA and licence states are not in this API.` : 'Sophos Firewall, read through its XML API.',
		verdict: verdict(down > 0 ? [{ text: m.misc2_folds_interface_without_link({ count: down }), severe: false }] : [], 'Every interface in a zone has its link.'),
		figures: [
			{ label: 'Interfaces', value: single(ps, 'interfaces') !== null ? String(single(ps, 'interfaces')) : null },
			{ label: 'Without link', value: String(down), tone: down > 0 ? 'advisory' : 'ink' },
			{ label: 'IPsec connections', value: single(ps, 'ipsec_connections') !== null ? String(single(ps, 'ipsec_connections')) : null }
		],
		sections: [
			{ title: 'Interfaces in a zone', rows: interfaces },
			{ title: 'IPsec connections, as configured', rows: connections, more: connections.length > 0 ? 'Activation as set by an administrator, not whether the tunnel is up.' : undefined }
		]
	};
}

// ---------------------------------------------------------------------- Meraki

function plural(count: number, noun: string): string {
	return `${count} ${noun}${count === 1 ? '' : 's'}`;
}

function meraki(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'meraki');
	const org = ps.find((p) => p.name === 'organization_info')?.labels.organization ?? null;
	const status = (name: string): number | null => ps.find((p) => p.name === 'devices' && p.labels.status === name)?.value ?? null;
	const online = status('online');
	const alerting = status('alerting');
	const offline = status('offline') ?? 0;
	const dormant = status('dormant');
	const age = byLabel(ps, 'device_last_report_age_seconds', 'serial');
	const devices: Row[] = [];
	for (const p of byLabel(ps, 'device_offline', 'serial').values()) {
		const alert = (byLabel(ps, 'device_alerting', 'serial').get(p.labels.serial)?.value ?? 0) >= 1;
		if (p.value < 1 && !alert) continue;
		const reported = age.get(p.labels.serial)?.value ?? null;
		devices.push({
			key: p.labels.serial,
			name: p.labels.device,
			tone: p.value >= 1 ? 'warning' : 'advisory',
			plate: p.value >= 1 ? 'Offline' : 'Alerting',
			details: [p.labels.model, p.labels.network, reported !== null ? `last seen ${formatAge(reported)} ago` : ''].filter(Boolean)
		});
	}
	devices.sort((a, b) => Number(a.tone !== 'warning') - Number(b.tone !== 'warning') || byName(a, b));
	const failover = new Set(ps.filter((p) => p.name === 'uplink_failover' && p.value >= 1).map((p) => p.labels.serial));
	const uplinks: Row[] = ps
		.filter((p) => p.name === 'uplink_failed' && p.value >= 1)
		.map((p) => ({
			key: `${p.labels.serial}/${p.labels.interface}`,
			name: `${p.labels.device} ${p.labels.interface}`,
			tone: 'warning',
			plate: 'Failed',
			details: [p.labels.network, failover.has(p.labels.serial) ? 'running on its backup uplink' : ''].filter(Boolean)
		}));
	uplinks.sort(byName);
	const licenseOk = single(ps, 'license_ok');
	const expiry = single(ps, 'license_expiry_seconds');
	const problems = [];
	if (offline > 0) problems.push({ text: `${plural(offline, 'device')} offline`, severe: true });
	if (uplinks.length > 0) problems.push({ text: `${plural(uplinks.length, 'uplink')} failed`, severe: false });
	if (licenseOk !== null && licenseOk < 1) problems.push({ text: 'the license is not in compliance', severe: false });
	if (expiry !== null && expiry < 30 * 86400) problems.push({ text: expiry < 0 ? 'the license has expired' : `the license expires ${days(expiry)}`, severe: false });
	const total = (online ?? 0) + (alerting ?? 0) + offline;
	return {
		title: 'Cloud network',
		description: org ? `Organization "${org}", read through the Meraki Dashboard API.` : 'Read through the Meraki Dashboard API.',
		verdict: verdict(problems, 'Every device reaches the dashboard and every uplink works.'),
		figures: [
			{ label: 'Devices online', value: online !== null ? `${online + (alerting ?? 0)}/${total}` : null, tone: offline > 0 ? 'warning' : 'ink' },
			{ label: 'Alerting', value: alerting !== null ? String(alerting) : null, tone: alerting !== null && alerting > 0 ? 'advisory' : 'ink' },
			{ label: 'Dormant', value: dormant !== null ? String(dormant) : null, hint: 'Not seen for a long time, never counted as down' },
			{ label: 'Clients', value: single(ps, 'clients') !== null ? String(single(ps, 'clients')) : null, hint: 'Seen in the last hour' },
			{ label: 'License', value: expiry !== null ? (expiry < 0 ? 'Expired' : days(expiry)) : null, tone: licenseOk !== null && licenseOk < 1 ? 'advisory' : 'ink' }
		],
		sections: [
			{ title: 'Devices offline or alerting', rows: devices },
			{ title: 'Uplinks that failed', rows: uplinks }
		]
	};
}

// ----------------------------------------------------------------------- Omada

function omada(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'omada');
	const info = ps.find((p) => p.name === 'controller_info')?.labels;
	const count = (name: string): number => ps.find((p) => p.name === 'devices' && p.labels.status === name)?.value ?? 0;
	const offlineFlags = byLabel(ps, 'device_offline', 'mac');
	const pendingFlags = byLabel(ps, 'device_pending', 'mac');
	const cpu = byLabel(ps, 'device_cpu_usage_percent', 'mac');
	const memory = byLabel(ps, 'device_memory_used_percent', 'mac');
	const upgrade = byLabel(ps, 'device_upgrade_available', 'mac');
	const devices: Row[] = [];
	let hot = 0;
	for (const p of byLabel(ps, 'device_online', 'mac').values()) {
		const mac = p.labels.mac;
		const isOffline = (offlineFlags.get(mac)?.value ?? 0) >= 1;
		const isPending = (pendingFlags.get(mac)?.value ?? 0) >= 1;
		const c = cpu.get(mac)?.value ?? null;
		const mem = memory.get(mac)?.value ?? null;
		const saturated = (c !== null && c > 90) || (mem !== null && mem > 90);
		const update = (upgrade.get(mac)?.value ?? 0) >= 1;
		if (!isOffline && !isPending && !saturated && !update) continue;
		if (saturated) hot += 1;
		devices.push({
			key: mac,
			name: p.labels.device,
			tone: isOffline ? 'warning' : isPending || saturated ? 'advisory' : 'info',
			plate: isOffline ? 'Disconnected' : isPending ? 'Pending adoption' : saturated ? 'Saturated' : 'Update',
			details: [p.labels.model, p.labels.site, c !== null ? `CPU ${Math.round(c)}%` : '', mem !== null ? `memory ${Math.round(mem)}%` : '', update ? 'firmware update available' : ''].filter(Boolean)
		});
	}
	const RANK: Record<string, number> = { warning: 0, advisory: 1, info: 2 };
	devices.sort((a, b) => (RANK[a.tone] ?? 9) - (RANK[b.tone] ?? 9) || byName(a, b));
	const offline = count('offline');
	const pending = count('pending');
	const problems = [];
	if (offline > 0) problems.push({ text: `${plural(offline, 'device')} disconnected`, severe: true });
	if (pending > 0) problems.push({ text: `${plural(pending, 'device')} awaiting adoption`, severe: false });
	if (hot > 0) problems.push({ text: `${plural(hot, 'device')} with CPU or memory above 90%`, severe: false });
	const online = count('online');
	return {
		title: 'Wi-Fi and switching',
		description: info?.version ? `Omada controller ${info.version}, read through its OpenAPI.` : 'Read through the Omada OpenAPI.',
		verdict: verdict(problems, 'Every device is connected to the controller.'),
		figures: [
			{ label: 'Devices online', value: `${online}/${online + offline + pending}`, tone: offline > 0 ? 'warning' : 'ink' },
			{ label: 'Sites', value: single(ps, 'sites') !== null ? String(single(ps, 'sites')) : null },
			{ label: 'Clients', value: single(ps, 'clients') !== null ? String(single(ps, 'clients')) : null }
		],
		sections: [{ title: 'Devices needing a look', rows: devices }]
	};
}

// ------------------------------------------------------------ Palo Alto PAN-OS

function paloalto(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'paloalto');
	const info = ps.find((p) => p.name === 'version_info')?.labels;
	const interfaces: Row[] = [...byLabel(ps, 'interface_down', 'interface').values()]
		.filter((p) => p.value >= 1)
		.map((p) => ({ key: p.labels.interface, name: p.labels.interface, tone: 'warning', plate: 'No link', details: p.labels.zone ? [p.labels.zone] : undefined }));
	interfaces.sort(byName);
	const licenses: Row[] = [...byLabel(ps, 'license_expiry_seconds', 'license').values()]
		.filter((p) => p.value < 60 * 86400)
		.map((p) => ({ key: p.labels.license, name: p.labels.license, tone: p.value < 0 ? 'warning' : 'advisory', plate: p.value < 0 ? 'Expired' : `Expires ${days(p.value)}` }));
	const haEnabled = single(ps, 'ha_enabled');
	const peerUp = single(ps, 'ha_peer_up');
	const inSync = single(ps, 'ha_in_sync');
	const functional = single(ps, 'ha_local_functional');
	const usage = single(ps, 'session_usage_percent');
	const commit = single(ps, 'last_commit_ok');
	const haState = ps.find((p) => p.name === 'ha_state_info')?.labels;
	const problems = [];
	if (functional !== null && functional < 1) problems.push({ text: 'this firewall is not functional in its HA pair', severe: true });
	if (peerUp !== null && peerUp < 1) problems.push({ text: 'the HA peer is lost', severe: true });
	if (inSync !== null && inSync < 1) problems.push({ text: 'the HA pair is out of sync', severe: false });
	if (interfaces.length > 0) problems.push({ text: m.misc2_folds_interface_without_link({ count: interfaces.length }), severe: false });
	if (usage !== null && usage > 90) problems.push({ text: 'the session table is nearly full', severe: false });
	if (commit !== null && commit < 1) problems.push({ text: 'the last commit failed', severe: false });
	const expired = licenses.filter((l) => l.tone === 'warning').length;
	if (expired > 0) problems.push({ text: m.misc2_folds_licence_expired({ count: expired }), severe: false });
	return {
		title: 'Firewall',
		description: info ? `${info.model || 'Palo Alto'} ${info.hostname ? `"${info.hostname}" ` : ''}running PAN-OS ${info.version}. Beta.` : 'Read through the PAN-OS XML API. Beta.',
		verdict: verdict(problems, 'Every zoned interface has its link and the HA pair, if any, is healthy.'),
		figures: [
			{ label: 'CPU', value: percent(single(ps, 'cpu_usage_percent')), hint: 'Management plane' },
			{ label: 'Memory', value: percent(single(ps, 'memory_used_percent')), hint: 'Management plane' },
			{ label: 'Sessions', value: single(ps, 'sessions') !== null ? String(single(ps, 'sessions')) : null, tone: usage !== null && usage > 90 ? 'advisory' : 'ink', hint: usage !== null ? `${Math.round(usage)}% of the table` : undefined },
			{
				label: 'HA',
				value: haEnabled === null || haEnabled < 1 ? 'Standalone' : haState ? haState.state : inSync !== null && inSync >= 1 ? 'In sync' : 'Enabled',
				tone: (peerUp !== null && peerUp < 1) || (inSync !== null && inSync < 1) ? 'advisory' : 'ink'
			}
		],
		sections: [
			{ title: 'Interfaces without link', rows: interfaces },
			{ title: 'Licences expiring or expired', rows: licenses }
		]
	};
}

// ------------------------------------------------------------ SonicWall SonicOS

function sonicwall(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'sonicwall');
	const info = ps.find((p) => p.name === 'version_info')?.labels;
	const links = byLabel(ps, 'interface_link_up', 'interface');
	const interfaces: Row[] = [...byLabel(ps, 'interface_down', 'interface').values()]
		.filter((p) => p.labels.zone && p.labels.zone.toLowerCase() !== 'unassigned')
		.map((p) => {
			const name = p.labels.interface;
			const link = links.get(name)?.value ?? null;
			return {
				key: name,
				name,
				tone: p.value >= 1 ? 'warning' : link === null ? 'ghost' : 'signal',
				plate: p.value >= 1 ? 'No link' : link === null ? 'Unknown' : 'Connected',
				details: [p.labels.zone]
			};
		});
	interfaces.sort((a, b) => Number(a.tone !== 'warning') - Number(b.tone !== 'warning') || a.name.localeCompare(b.name, 'en', { numeric: true }));
	const down = single(ps, 'interfaces_down') ?? 0;
	return {
		title: 'Firewall',
		description: info ? `${info.model ? `SonicWall ${info.model}, ` : ''}${info.version}. Beta; tunnel, HA and licence states are not read.` : 'SonicWall, read through the SonicOS API. Beta; tunnel, HA and licence states are not read.',
		verdict: verdict(down > 0 ? [{ text: m.misc2_folds_interface_without_link({ count: down }), severe: false }] : [], 'Every interface in a zone has its link.'),
		figures: [
			{ label: 'Interfaces', value: single(ps, 'interfaces') !== null ? String(single(ps, 'interfaces')) : null },
			{ label: 'Without link', value: String(down), tone: down > 0 ? 'advisory' : 'ink' }
		],
		sections: [{ title: 'Interfaces in a zone', rows: interfaces }]
	};
}

// --------------------------------------------------------------------- Nginx

function nginx(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'nginx');
	const active = single(ps, 'connections_active');
	const reading = single(ps, 'connections_reading');
	const writing = single(ps, 'connections_writing');
	const waiting = single(ps, 'connections_waiting');
	const versionInfo = ps.find((p) => p.name === 'version_info');
	const upstreams: Row[] = [...byLabel(ps, 'plus_upstream_server_up', 'server').values()].map((p) => ({
		key: `${p.labels.upstream}/${p.labels.server}`,
		name: p.labels.server,
		tone: p.value >= 1 ? 'signal' : 'warning',
		plate: p.value >= 1 ? 'Up' : 'Down',
		details: [p.labels.upstream]
	}));
	upstreams.sort((a, b) => Number(a.tone !== 'warning') - Number(b.tone !== 'warning') || byName(a, b));
	const down = upstreams.filter((u) => u.tone === 'warning').length;
	return {
		title: 'Web server',
		description: versionInfo ? `Nginx, ${versionInfo.labels.server}.` : 'Nginx, read through stub_status.',
		verdict:
			upstreams.length > 0
				? verdict(down > 0 ? [{ text: m.misc2_folds_upstream_server_down({ count: down }), severe: false }] : [], 'Every upstream server answers its health check.')
				: { tone: 'signal', text: 'Reporting connections and requests.' },
		figures: [
			{ label: 'Active connections', value: active !== null ? String(active) : null },
			{ label: 'Reading', value: reading !== null ? String(reading) : null },
			{ label: 'Writing', value: writing !== null ? String(writing) : null },
			{ label: 'Waiting', value: waiting !== null ? String(waiting) : null }
		],
		sections: upstreams.length > 0 ? [{ title: 'NGINX Plus upstream servers', rows: upstreams }] : []
	};
}

// --------------------------------------------------------------------- Apache

function apache(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'apache');
	const busy = single(ps, 'workers_busy');
	const idle = single(ps, 'workers_idle');
	const reqPerSec = single(ps, 'requests_per_second');
	const cpu = single(ps, 'cpu_load_ratio');
	const versionInfo = ps.find((p) => p.name === 'version_info');
	const total = busy !== null && idle !== null ? busy + idle : null;
	const saturated = busy !== null && total !== null && total > 0 && busy / total >= 0.9;
	return {
		title: 'Web server',
		description: versionInfo ? `Apache httpd, ${versionInfo.labels.version}.` : 'Apache httpd, read through mod_status.',
		verdict: verdict(saturated ? [{ text: 'almost every worker is busy', severe: false }] : [], 'Worker capacity is not under pressure.'),
		figures: [
			{ label: 'Busy workers', value: busy !== null ? String(busy) : null, tone: saturated ? 'advisory' : 'ink' },
			{ label: 'Idle workers', value: idle !== null ? String(idle) : null },
			{ label: 'Requests/s', value: reqPerSec !== null ? reqPerSec.toFixed(2) : null },
			{ label: 'CPU load', value: cpu !== null ? cpu.toFixed(2) : null, hint: "Apache's own CPULoad" }
		],
		sections: []
	};
}

// --------------------------------------------------------------------- OpenVPN

function openvpn(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'openvpn');
	const version = ps.find((p) => p.name === 'server_info')?.labels.version ?? null;
	const up = single(ps, 'server_up');
	const clients = single(ps, 'clients_connected') ?? 0;
	const received = byLabel(ps, 'client_bytes_received', 'common_name');
	const sent = byLabel(ps, 'client_bytes_sent', 'common_name');
	const rows: Row[] = [...byLabel(ps, 'client_connected_seconds', 'common_name').values()].map((p) => {
		const name = p.labels.common_name;
		const rx = received.get(name)?.value ?? 0;
		const tx = sent.get(name)?.value ?? 0;
		return {
			key: name,
			name,
			tone: 'signal',
			plate: m.devices_openvpn_connected_ago({ age: formatAge(p.value) }),
			details: [p.labels.real_address, `↓ ${formatBytes(rx)} · ↑ ${formatBytes(tx)}`].filter((d): d is string => !!d)
		};
	});
	rows.sort(byName);
	const down = up !== null && up < 1;
	return {
		title: 'OpenVPN',
		description: version ? m.devices_openvpn_description_version({ version }) : m.devices_openvpn_description(),
		verdict: verdict(down ? [{ text: m.devices_openvpn_verdict_down(), severe: true }] : [], m.devices_openvpn_verdict_ok()),
		figures: [
			{ label: m.devices_openvpn_clients_connected(), value: String(clients) },
			{ label: m.devices_openvpn_version(), value: version }
		],
		sections: [{ title: m.devices_openvpn_section_clients(), rows, more: m.devices_openvpn_clients_note() }]
	};
}

// --------------------------------------------------------------------- Hyper-V

function hyperv(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'hyperv');
	const pressure = byLabel(ps, 'vm_memory_pressure_percent', 'vm');
	const vms: Row[] = [...byLabel(ps, 'vm_physical_memory_megabytes', 'vm').values()].map((p) => {
		const name = p.labels.vm;
		const pr = pressure.get(name)?.value ?? null;
		return {
			key: name,
			name,
			tone: pr !== null && pr > 100 ? 'advisory' : 'signal',
			plate: formatBytes(p.value * 1024 * 1024),
			details: pr !== null ? [`memory pressure ${Math.round(pr)}%`] : []
		};
	});
	vms.sort(byName);
	const errors = byLabel(ps, 'vhd_errors_total', 'disk');
	const latency = byLabel(ps, 'vhd_latency_seconds', 'disk');
	const write = byLabel(ps, 'vhd_write_bytes_per_second', 'disk');
	const disks: Row[] = [...byLabel(ps, 'vhd_read_bytes_per_second', 'disk').values()].map((p) => {
		const name = p.labels.disk;
		const err = errors.get(name)?.value ?? 0;
		const lat = latency.get(name)?.value ?? null;
		return {
			key: name,
			name,
			tone: err > 0 ? 'advisory' : 'muted',
			plate: err > 0 ? m.misc2_folds_error_count({ count: err }) : `${formatBytes(p.value + (write.get(name)?.value ?? 0))}/s`,
			details: lat !== null ? [`${(lat * 1000).toFixed(1)} ms latency`] : []
		};
	});
	disks.sort((a, b) => Number(a.tone !== 'advisory') - Number(b.tone !== 'advisory') || byName(a, b));
	const critical = single(ps, 'vms_health_critical') ?? 0;
	const cpu = single(ps, 'host_cpu_percent');
	const problems = [];
	if (critical > 0) problems.push({ text: m.misc2_folds_virtual_machine_in_critical_health({ count: critical }), severe: true });
	if (cpu !== null && cpu > 90) problems.push({ text: 'the host CPU is saturated', severe: false });
	const withErrors = disks.filter((d) => d.tone === 'advisory').length;
	if (withErrors > 0) problems.push({ text: m.misc2_folds_virtual_disk_with_errors({ count: withErrors }), severe: false });
	const ok = single(ps, 'vms_health_ok');
	return {
		title: 'Hyper-V',
		description: "Read from Hyper-V's performance counters by this agent.",
		verdict: verdict(problems, 'Every virtual machine reports healthy.'),
		figures: [
			{ label: 'VMs healthy', value: ok !== null ? `${ok}/${ok + critical}` : null, tone: critical > 0 ? 'warning' : 'ink' },
			{ label: 'Host CPU', value: percent(cpu), hint: 'All guests included' },
			{ label: 'Guest CPU', value: percent(single(ps, 'host_cpu_guest_percent')) },
			{ label: 'Partitions', value: single(ps, 'partitions') !== null ? String(single(ps, 'partitions')) : null, hint: 'Running VMs plus the host' }
		],
		sections: [
			{ title: 'Virtual machines, memory assigned', rows: vms },
			{ title: 'Virtual disks', rows: disks }
		]
	};
}

// ----------------------------------------------------------- Cloud services

/** A key figure of a cloud service, in the unit its collector declared. */
function figureValue(unit: string, value: number): string {
	if (unit === 'percent') return `${Math.round(value)}%`;
	if (unit === 'bytes') return formatBytes(value);
	if (unit === 'days') return `${Math.round(value)} d`;
	return String(Math.round(value));
}

/**
 * Cloud services (Microsoft 365, Entra ID, Azure, AWS, Cloudflare, Hetzner,
 * OVHcloud, Infomaniak) all publish the same two generic families
 * (`crates/collectors/src/cloud.rs`): `<kind>_item_ok{group,item,detail}` for
 * the lists and `<kind>_figure{name,unit}` for the figures.
 */
function cloud(series: MetricSeries[], prefix: string, title: string, description: string): ApplianceView {
	const ps = points(series, prefix);
	const groups = new Map<string, Row[]>();
	let bad = 0;
	for (const p of ps) {
		if (p.name !== 'item_ok' || p.labels.item === undefined) continue;
		const ok = p.value >= 1;
		if (!ok) bad += 1;
		const group = p.labels.group || 'Items';
		const rows = groups.get(group) ?? [];
		rows.push({
			key: `${group}/${p.labels.item}`,
			name: p.labels.item,
			tone: ok ? 'signal' : 'warning',
			plate: p.labels.detail || (ok ? 'OK' : 'Problem')
		});
		groups.set(group, rows);
	}
	const LIMIT = 30;
	const sections: Section[] = [...groups.entries()]
		.sort(([a], [b]) => a.localeCompare(b, 'en'))
		.map(([name, rows]) => {
			rows.sort((a, b) => Number(a.tone === 'signal') - Number(b.tone === 'signal') || byName(a, b));
			const hidden = rows.length - LIMIT;
			return { title: name, rows: rows.slice(0, LIMIT), more: hidden > 0 ? `${hidden} more, all healthy or lower priority.` : undefined };
		});
	const figures: Figure[] = ps
		.filter((p) => p.name === 'figure' && p.labels.name !== undefined)
		.sort((a, b) => a.labels.name.localeCompare(b.labels.name, 'en'))
		.slice(0, 4)
		.map((p) => ({ label: p.labels.name, value: figureValue(p.labels.unit ?? '', p.value) }));
	const problems = bad > 0 ? [{ text: `${bad} item${bad === 1 ? '' : 's'} need${bad === 1 ? 's' : ''} attention`, severe: true }] : [];
	return {
		title,
		description,
		verdict: verdict(problems, 'Everything looks healthy.'),
		figures,
		sections
	};
}

function cloudFold(prefix: string, title: string, description: string): Fold {
	return { prefix, title, fold: (series) => cloud(series, prefix, title, description) };
}

// --------------------------------------------------------------- Aruba Central

const ARUBA_TYPES: Record<string, string> = { access_point: 'Access point', switch: 'Switch', gateway: 'Gateway' };

function arubacentral(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'arubacentral');
	const info = byLabel(ps, 'device_info', 'serial');
	const uptime = byLabel(ps, 'device_uptime_seconds', 'serial');
	const devices: Row[] = [...byLabel(ps, 'device_online', 'serial').values()].map((p) => {
		const serial = p.labels.serial;
		const up = uptime.get(serial)?.value ?? null;
		const meta = info.get(serial)?.labels;
		return {
			key: serial,
			name: p.labels.device,
			tone: p.value >= 1 ? 'signal' : 'warning',
			plate: p.value >= 1 ? 'Online' : 'Offline',
			details: [ARUBA_TYPES[p.labels.type] ?? p.labels.type, meta?.model ?? '', p.labels.site ?? '', up !== null ? `up ${formatAge(up)}` : ''].filter(Boolean)
		};
	});
	devices.sort((a, b) => Number(a.tone === 'signal') - Number(b.tone === 'signal') || byName(a, b));
	const offline = single(ps, 'devices_offline') ?? 0;
	const critical = byLabel(ps, 'alerts_active_by_severity', 'severity').get('critical')?.value ?? 0;
	const poor = single(ps, 'devices_poor_health') ?? 0;
	const problems = [];
	if (offline > 0) problems.push({ text: `${offline} ${offline === 1 ? 'device' : 'devices'} offline`, severe: true });
	if (critical > 0) problems.push({ text: `${critical} critical ${critical === 1 ? 'alert' : 'alerts'} active`, severe: true });
	if (poor > 0) problems.push({ text: `${poor} ${poor === 1 ? 'device' : 'devices'} in poor health`, severe: false });
	const clients = [...byLabel(ps, 'clients', 'type').values()].reduce((sum, p) => sum + p.value, 0);
	return {
		title: 'Aruba Central',
		description: 'Read from the Aruba Central API (beta).',
		verdict: verdict(problems, 'Every device is online.'),
		figures: [
			{ label: 'Devices online', value: single(ps, 'devices') !== null ? `${(single(ps, 'devices') ?? 0) - offline}/${single(ps, 'devices')}` : null, tone: offline > 0 ? 'warning' : 'ink' },
			{ label: 'Active alerts', value: single(ps, 'alerts_active') !== null ? String(single(ps, 'alerts_active')) : null, tone: critical > 0 ? 'warning' : 'ink' },
			{ label: 'Poor health', value: single(ps, 'devices_poor_health') !== null ? String(poor) : null },
			{ label: 'Clients', value: byLabel(ps, 'clients', 'type').size > 0 ? String(clients) : null }
		],
		sections: [{ title: 'Devices, offline first', rows: devices }]
	};
}

// ------------------------------------------------------------------------ QNAP

function qnap(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'qnap');
	const info = ps.find((p) => p.name === 'version_info')?.labels;
	const temps = byLabel(ps, 'disk_temperature_celsius', 'disk');
	const warnings = byLabel(ps, 'disk_warning', 'disk');
	const disks: Row[] = [...byLabel(ps, 'disk_ok', 'disk').values()].map((p) => {
		const name = p.labels.disk;
		const t = temps.get(name)?.value ?? null;
		const ok = p.value >= 1;
		const warn = (warnings.get(name)?.value ?? 0) >= 1;
		return {
			key: name,
			name,
			tone: ok ? 'signal' : warn ? 'advisory' : 'warning',
			plate: ok ? 'OK' : warn ? 'SMART warning' : 'SMART abnormal',
			details: [p.labels.model ?? '', t !== null ? `${Math.round(t)} °C` : ''].filter(Boolean)
		};
	});
	disks.sort((a, b) => a.name.localeCompare(b.name, 'en', { numeric: true }));
	const volumes: Row[] = [...byLabel(ps, 'volume_used_percent', 'volume').values()].map((p) => ({
		key: p.labels.volume,
		name: p.labels.volume,
		tone: p.value > 90 ? 'warning' : p.value > 80 ? 'advisory' : 'signal',
		plate: `${Math.round(p.value)}% used`
	}));
	volumes.sort((a, b) => Number(b.tone !== 'signal') - Number(a.tone !== 'signal') || byName(a, b));
	const bad = disks.filter((d) => d.tone !== 'signal').length;
	const hot = [...temps.values()].filter((p) => p.value > 55).length;
	const full = volumes.filter((v) => v.tone === 'warning').length;
	const cpuTemp = single(ps, 'cpu_temperature_celsius');
	const update = single(ps, 'firmware_update_available');
	const latest = ps.find((p) => p.name === 'firmware_latest_info')?.labels.version ?? null;
	const problems = [];
	if (bad > 0) problems.push({ text: `${bad} ${bad === 1 ? 'disk' : 'disks'} without a good SMART health`, severe: true });
	if (hot > 0) problems.push({ text: `${hot} ${hot === 1 ? 'disk' : 'disks'} above 55 °C`, severe: false });
	if (full > 0) problems.push({ text: `${full} ${full === 1 ? 'volume' : 'volumes'} over 90% full`, severe: false });
	if (cpuTemp !== null && cpuTemp > 90) problems.push({ text: 'the processor is very hot', severe: false });
	if (update !== null && update >= 1) problems.push({ text: `a firmware update is available${latest ? ` (${latest})` : ''}`, severe: false });
	const uptime = single(ps, 'uptime_seconds');
	return {
		title: 'NAS',
		description: info ? `${info.model || 'QNAP'} ${info.hostname ? `"${info.hostname}" ` : ''}running QTS ${info.version}.` : 'Read through the QTS web interface.',
		verdict: verdict(problems, 'Every volume has room and every disk reports a good SMART health.'),
		figures: [
			{ label: 'CPU', value: percent(single(ps, 'cpu_usage_percent')) },
			{ label: 'Memory', value: percent(single(ps, 'memory_used_percent')) },
			{ label: 'Temperature', value: cpuTemp !== null ? `${Math.round(cpuTemp)} °C` : null, hint: 'processor' },
			{ label: 'Uptime', value: uptime !== null ? formatAge(uptime) : null }
		],
		sections: [
			{ title: 'Volumes', rows: volumes },
			{ title: 'Disks', rows: disks }
		]
	};
}

// ------------------------------------------------------------------------ Ceph

function ceph(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'ceph');
	const version = ps.find((p) => p.name === 'version_info')?.labels.version ?? null;
	const status = single(ps, 'health_status');
	const checks: Row[] = ps
		.filter((p) => p.name === 'health_check_info')
		.map((p) => ({
			key: p.labels.check,
			name: p.labels.check,
			tone: p.labels.severity === 'HEALTH_ERR' ? 'warning' : 'advisory',
			plate: p.labels.severity === 'HEALTH_ERR' ? 'Error' : 'Warning'
		}));
	checks.sort((a, b) => Number(b.tone === 'warning') - Number(a.tone === 'warning') || byName(a, b));
	const ups = byLabel(ps, 'osd_up', 'osd');
	const inside = byLabel(ps, 'osd_in', 'osd');
	const used = byLabel(ps, 'osd_used_percent', 'osd');
	const osds: Row[] = [...ups.values()]
		.map((p) => {
			const name = p.labels.osd;
			const up = p.value >= 1;
			const isIn = (inside.get(name)?.value ?? 1) >= 1;
			const u = used.get(name)?.value ?? null;
			const nearFull = u !== null && u > 85;
			return {
				key: name,
				name,
				tone: !up ? 'warning' : !isIn || nearFull ? 'advisory' : 'signal',
				plate: !up ? 'Down' : !isIn ? 'Out' : nearFull ? 'Nearly full' : 'Up',
				details: [p.labels.host ?? '', u !== null ? `${Math.round(u)}% used` : ''].filter(Boolean)
			} satisfies Row;
		})
		.filter((r) => r.tone !== 'signal');
	osds.sort((a, b) => a.name.localeCompare(b.name, 'en', { numeric: true }));
	const pools: Row[] = [...byLabel(ps, 'pool_used_percent', 'pool').values()]
		.sort((a, b) => b.value - a.value)
		.map((p) => ({
			key: p.labels.pool,
			name: p.labels.pool,
			tone: p.value > 90 ? 'warning' : p.value > 80 ? 'advisory' : 'signal',
			plate: `${Math.round(p.value)}% used`
		}));
	const osdsTotal = single(ps, 'osds');
	const osdsUp = single(ps, 'osds_up');
	const pgs = single(ps, 'pgs');
	const pgsClean = single(ps, 'pgs_clean');
	const unclean = single(ps, 'pgs_unclean') ?? 0;
	const capacity = single(ps, 'capacity_used_percent');
	const outOfQuorum = single(ps, 'mons_out_of_quorum') ?? 0;
	const unfound = single(ps, 'objects_unfound') ?? 0;
	const down = osds.filter((o) => o.tone === 'warning').length;
	const problems = [];
	if (status !== null && status >= 2) problems.push({ text: 'the cluster reports HEALTH_ERR', severe: true });
	else if (status !== null && status >= 1) problems.push({ text: `the cluster reports HEALTH_WARN (${checks.length} ${checks.length === 1 ? 'check' : 'checks'})`, severe: false });
	if (down > 0) problems.push({ text: `${down} ${down === 1 ? 'OSD' : 'OSDs'} down`, severe: false });
	if (unclean > 0) problems.push({ text: `${unclean} placement ${unclean === 1 ? 'group' : 'groups'} not clean`, severe: false });
	if (unfound > 0) problems.push({ text: 'unfound objects', severe: true });
	if (outOfQuorum > 0) problems.push({ text: `${outOfQuorum} ${outOfQuorum === 1 ? 'monitor' : 'monitors'} out of the quorum`, severe: true });
	if (capacity !== null && capacity > 80) problems.push({ text: 'capacity above 80%', severe: false });
	return {
		title: 'Cluster',
		description: version ? `Ceph ${version}, read through the manager dashboard API.` : 'Read through the Ceph manager dashboard API.',
		verdict: verdict(problems, 'The cluster is healthy: every OSD is up and every placement group is clean.'),
		figures: [
			{ label: 'OSDs up', value: osdsTotal !== null ? `${osdsUp ?? 0}/${osdsTotal}` : null, tone: osdsTotal !== null && (osdsUp ?? 0) < osdsTotal ? 'warning' : 'ink' },
			{ label: 'PGs clean', value: pgs !== null ? `${pgsClean ?? 0}/${pgs}` : null, tone: unclean > 0 ? 'advisory' : 'ink' },
			{ label: 'Capacity used', value: percent(capacity), hint: formatBytes(single(ps, 'capacity_bytes')), tone: capacity !== null && capacity > 80 ? 'advisory' : 'ink' },
			{ label: 'Monitors', value: single(ps, 'mons') !== null ? `${(single(ps, 'mons') ?? 0) - outOfQuorum}/${single(ps, 'mons')}` : null, tone: outOfQuorum > 0 ? 'warning' : 'ink' }
		],
		sections: [
			{ title: 'Health checks', rows: checks },
			{ title: 'OSDs needing attention', rows: osds },
			{ title: 'Pools', rows: pools }
		]
	};
}

// ----------------------------------------------------------------------- MinIO

function minio(series: MetricSeries[]): ApplianceView {
	const ps = points(series, 'minio');
	const version = ps.find((p) => p.name === 'version_info')?.labels.version ?? null;
	const nodesOnline = single(ps, 'nodes_online');
	const nodesOffline = single(ps, 'nodes_offline') ?? 0;
	const drivesOnline = single(ps, 'drives_online');
	const drivesOffline = single(ps, 'drives_offline') ?? 0;
	const used = single(ps, 'capacity_usable_used_percent') ?? single(ps, 'capacity_raw_used_percent');
	const healFailed = single(ps, 'heal_objects_failed') ?? 0;
	const replication = single(ps, 'replication_failed_objects') ?? 0;
	const objects = single(ps, 'objects');
	const problems = [];
	if (nodesOffline > 0) problems.push({ text: `${nodesOffline} ${nodesOffline === 1 ? 'node' : 'nodes'} offline`, severe: true });
	if (drivesOffline > 0) problems.push({ text: `${drivesOffline} ${drivesOffline === 1 ? 'drive' : 'drives'} offline`, severe: false });
	if (used !== null && used > 85) problems.push({ text: 'capacity above 85%', severe: false });
	if (healFailed > 0) problems.push({ text: 'objects the heal could not repair', severe: false });
	if (replication > 0) problems.push({ text: 'objects that failed to replicate', severe: false });
	const total = (online: number | null, offline: number) => (online !== null ? `${online}/${online + offline}` : null);
	return {
		title: 'Object store',
		description: version ? `MinIO ${version.replace(/^RELEASE\./, '')}, read through its Prometheus metrics.` : 'Read through the MinIO Prometheus metrics.',
		verdict: verdict(problems, 'Every node and drive is online and nothing is failing to heal or replicate.'),
		figures: [
			{ label: 'Nodes online', value: total(nodesOnline, nodesOffline), tone: nodesOffline > 0 ? 'warning' : 'ink' },
			{ label: 'Drives online', value: total(drivesOnline, drivesOffline), tone: drivesOffline > 0 ? 'advisory' : 'ink' },
			{ label: 'Capacity used', value: percent(used), hint: formatBytes(single(ps, 'capacity_usable_bytes') ?? single(ps, 'capacity_raw_bytes')), tone: used !== null && used > 85 ? 'advisory' : 'ink' },
			{ label: 'Objects', value: objects !== null ? objects.toLocaleString('en') : null, hint: single(ps, 'buckets') !== null ? `${single(ps, 'buckets')} buckets` : undefined }
		],
		sections: []
	};
}

export const FOLDS: Record<string, Fold> = {
	pfsense: { prefix: 'pfsense', title: 'Firewall', fold: pfsense },
	unraid: { prefix: 'unraid', title: 'Array', fold: unraid },
	veeam: { prefix: 'veeam', title: 'Backups', fold: veeam },
	kopia: { prefix: 'kopia', title: 'Backups', fold: kopia },
	duplicati: { prefix: 'duplicati', title: 'Backups', fold: duplicati },
	tailscale: { prefix: 'tailscale', title: 'Tailnet', fold: tailscale },
	m365: cloudFold('m365', 'Microsoft 365', 'Read through the Microsoft Graph API.'),
	entra: cloudFold('entra', 'Entra ID', 'Read through the Microsoft Graph API.'),
	azure: cloudFold('azure', 'Azure', 'Read through the Azure Resource Manager API.'),
	aws: cloudFold('aws', 'AWS', 'Read through the Amazon CloudWatch API.'),
	cloudflare: cloudFold('cloudflare', 'Cloudflare', 'Read through the Cloudflare API.'),
	hetzner: cloudFold('hetzner', 'Hetzner Cloud', 'Read through the Hetzner Cloud API.'),
	ovh: cloudFold('ovh', 'OVHcloud', 'Read through the OVHcloud API.'),
	infomaniak: cloudFold('infomaniak', 'Infomaniak', 'Read through the Infomaniak API.'),
	fortigate: { prefix: 'fortigate', title: 'Firewall', fold: fortigate },
	qnap: { prefix: 'qnap', title: 'NAS', fold: qnap },
	ceph: { prefix: 'ceph', title: 'Cluster', fold: ceph },
	minio: { prefix: 'minio', title: 'Object store', fold: minio },
	sophos: { prefix: 'sophos', title: 'Firewall', fold: sophos },
	meraki: { prefix: 'meraki', title: 'Cloud network', fold: meraki },
	omada: { prefix: 'omada', title: 'Wi-Fi and switching', fold: omada },
	arubacentral: { prefix: 'arubacentral', title: 'Aruba Central', fold: arubacentral },
	paloalto: { prefix: 'paloalto', title: 'Firewall', fold: paloalto },
	sonicwall: { prefix: 'sonicwall', title: 'Firewall', fold: sonicwall },
	nginx: { prefix: 'nginx', title: 'Web server', fold: nginx },
	apache: { prefix: 'apache', title: 'Web server', fold: apache },
	openvpn: { prefix: 'openvpn', title: 'VPN', fold: openvpn },
	hyperv: { prefix: 'hyperv', title: 'Hyper-V', fold: hyperv }
};
