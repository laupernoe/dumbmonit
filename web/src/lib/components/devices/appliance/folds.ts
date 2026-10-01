/**
 * How each appliance's stored series become a panel: a verdict sentence, a
 * few figures and the lists worth reading. One fold per kind, all pure, all
 * reading `dumbmonit_<prefix>_*` series as the collectors write them
 * (`crates/collectors/src/{pfsense,unraid,veeam,tailscale,fortigate,sophos}`,
 * and the Hyper-V preset of the Windows agent).
 */
import type { MetricSeries } from '$lib/api';
import type { Tone } from '$lib/ui';
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

function plural(n: number, one: string, many: string): string {
	return `${n} ${n === 1 ? one : many}`;
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
	if (down > 0) problems.push({ text: `${plural(down, 'gateway', 'gateways')} down`, severe: true });
	if (lossy > 0) problems.push({ text: `${plural(lossy, 'gateway', 'gateways')} losing packets`, severe: false });
	if (interfaces.length > 0) problems.push({ text: `${plural(interfaces.length, 'interface', 'interfaces')} without link`, severe: false });
	if (services.length > 0) problems.push({ text: `${plural(services.length, 'service', 'services')} stopped`, severe: false });
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
	if (bad > 0) problems.push({ text: `${plural(bad, 'disk', 'disks')} disabled or missing`, severe: true });
	const withErrors = disks.filter((d) => d.tone === 'advisory').length;
	if (withErrors > 0) problems.push({ text: `${plural(withErrors, 'disk', 'disks')} with read errors`, severe: false });
	if (parityOk !== null && parityOk < 1) problems.push({ text: 'the last parity check found errors', severe: false });
	if (parityAge !== null && parityAge > 40 * 86400) problems.push({ text: 'no parity check for over 40 days', severe: false });
	if (cacheFull > 0) problems.push({ text: 'cache nearly full', severe: false });
	if (containers.length > 0) problems.push({ text: `${plural(containers.length, 'container', 'containers')} stopped`, severe: false });
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
	if (failedCount > 0) problems.push({ text: `${plural(failedCount, 'job', 'jobs')} failed`, severe: true });
	if (warningCount > 0) problems.push({ text: `${plural(warningCount, 'job', 'jobs')} ended with a warning`, severe: false });
	if (full > 0) problems.push({ text: `${plural(full, 'repository', 'repositories')} nearly full`, severe: false });
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
	if (down > 0) problems.push({ text: `${plural(down, 'watched device', 'watched devices')} offline`, severe: true });
	if (pending > 0) problems.push({ text: `${plural(pending, 'device', 'devices')} awaiting approval`, severe: false });
	if (expiring > 0) problems.push({ text: `${plural(expiring, 'node key', 'node keys')} expiring within 14 days`, severe: false });
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
	if (tunnelsDown > 0) problems.push({ text: `${plural(tunnelsDown, 'IPsec tunnel', 'IPsec tunnels')} down`, severe: true });
	if (interfaces.length > 0) problems.push({ text: `${plural(interfaces.length, 'interface', 'interfaces')} without link`, severe: false });
	if (inSync !== null && inSync < 1) problems.push({ text: 'the HA cluster is out of sync', severe: false });
	if (memory !== null && memory > 85) problems.push({ text: 'memory near conserve mode', severe: true });
	const expired = licenses.filter((l) => l.tone === 'warning').length;
	if (expired > 0) problems.push({ text: `${plural(expired, 'licence', 'licences')} expired`, severe: false });
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
				value: members === null || members === 0 ? 'Standalone' : inSync === null ? plural(members, 'member', 'members') : inSync >= 1 ? 'In sync' : 'Out of sync',
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
		verdict: verdict(down > 0 ? [{ text: `${plural(down, 'interface', 'interfaces')} without link`, severe: false }] : [], 'Every interface in a zone has its link.'),
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
			plate: err > 0 ? plural(err, 'error', 'errors') : `${formatBytes(p.value + (write.get(name)?.value ?? 0))}/s`,
			details: lat !== null ? [`${(lat * 1000).toFixed(1)} ms latency`] : []
		};
	});
	disks.sort((a, b) => Number(a.tone !== 'advisory') - Number(b.tone !== 'advisory') || byName(a, b));
	const critical = single(ps, 'vms_health_critical') ?? 0;
	const cpu = single(ps, 'host_cpu_percent');
	const problems = [];
	if (critical > 0) problems.push({ text: `${plural(critical, 'virtual machine', 'virtual machines')} in critical health`, severe: true });
	if (cpu !== null && cpu > 90) problems.push({ text: 'the host CPU is saturated', severe: false });
	const withErrors = disks.filter((d) => d.tone === 'advisory').length;
	if (withErrors > 0) problems.push({ text: `${plural(withErrors, 'virtual disk', 'virtual disks')} with errors`, severe: false });
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

export const FOLDS: Record<string, Fold> = {
	pfsense: { prefix: 'pfsense', title: 'Firewall', fold: pfsense },
	unraid: { prefix: 'unraid', title: 'Array', fold: unraid },
	veeam: { prefix: 'veeam', title: 'Backups', fold: veeam },
	tailscale: { prefix: 'tailscale', title: 'Tailnet', fold: tailscale },
	fortigate: { prefix: 'fortigate', title: 'Firewall', fold: fortigate },
	sophos: { prefix: 'sophos', title: 'Firewall', fold: sophos },
	hyperv: { prefix: 'hyperv', title: 'Hyper-V', fold: hyperv }
};
