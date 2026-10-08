/**
 * Presentation helpers for the NAS panels: Unix seconds, raw byte counts and
 * ZFS state words in, plain words out. The API speaks Unix seconds because
 * TrueNAS does, while the rest of the interface speaks server date strings —
 * hence these local variants. A missing measurement prints nothing rather
 * than a zero.
 */
import { formatDateTime } from '#lib/format.js';
import { m } from '#lib/paraglide/messages.js';
import type { Tone } from '#lib/ui/index.js';
import type { TruenasPoolRow, TruenasScan, TruenasTaskRow, TruenasVdev } from '#lib/api/index.js';

export type Plating = { tone: Tone; label: string };

export function formatUnix(seconds: number | null | undefined): string {
	if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) {
		return m.devicesb_truenas_format_never();
	}
	return formatDateTime(new Date(seconds * 1000));
}

/** Whole-unit span: "40 s", "12 min", "3 h", "5 d". */
export function formatSpan(seconds: number): string {
	if (seconds < 60) return m.devicesb_truenas_format_span_s({ count: Math.round(seconds) });
	if (seconds < 3600) {
		return m.devicesb_truenas_format_span_min({ count: Math.round(seconds / 60) });
	}
	if (seconds < 86400) {
		const hours = Math.floor(seconds / 3600);
		const minutes = Math.round((seconds % 3600) / 60);
		return minutes > 0 && hours < 10
			? m.devicesb_truenas_format_span_h_min({ hours, minutes })
			: m.devicesb_truenas_format_span_h({ count: hours });
	}
	const days = Math.round(seconds / 86400);
	return days === 1
		? m.devicesb_truenas_format_span_day_one({ count: days })
		: m.devicesb_truenas_format_span_day_other({ count: days });
}

/** "3 h ago", or "never". Negative ages (the future) read as "in 2 h". */
export function formatAgo(seconds: number | null | undefined): string {
	if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) {
		return m.devicesb_truenas_format_never();
	}
	const delta = Math.round(Date.now() / 1000 - seconds);
	if (delta < 0) return m.devicesb_truenas_format_in_span({ span: formatSpan(-delta) });
	if (delta < 45) return m.devicesb_truenas_format_just_now();
	return m.devicesb_truenas_format_span_ago({ span: formatSpan(delta) });
}

/** A plain count, grouped: "1 204". */
export function formatCount(value: number | null | undefined): string {
	if (value === null || value === undefined || !Number.isFinite(value)) return '—';
	return Math.round(value).toLocaleString('en-US').replace(/,/g, ' ');
}

/** Binary sizes, as ZFS counts them. */
export function formatBytes(bytes: number | null | undefined): string {
	if (bytes === null || bytes === undefined || !Number.isFinite(bytes)) return '—';
	const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];
	let value = bytes;
	let index = 0;
	while (value >= 1024 && index < units.length - 1) {
		value /= 1024;
		index += 1;
	}
	const digits = index === 0 || value >= 100 ? 0 : 1;
	return `${value.toFixed(digits)} ${units[index]}`;
}

/** Percent of a total, or `null` when the total is missing or zero. */
export function percentOf(used: number | null, total: number | null): number | null {
	if (used === null || total === null || !Number.isFinite(used) || !Number.isFinite(total)) {
		return null;
	}
	if (total <= 0) return null;
	return Math.min(100, Math.max(0, (used / total) * 100));
}

/** A finite number, or `null`: the API sends `null` for what it could not read. */
export function reading(value: number | null | undefined): number | null {
	return value === null || value === undefined || !Number.isFinite(value) ? null : value;
}

/** `FAULTED` → "Faulted". */
export function titleCase(word: string): string {
	const lower = word.toLowerCase().replace(/_/g, ' ');
	return lower.charAt(0).toUpperCase() + lower.slice(1);
}

export function plural(count: number, one: string, many = `${one}s`): string {
	return `${formatCount(count)} ${count === 1 ? one : many}`;
}

/**
 * Pool plate. ZFS's own verdict decides: a `DEGRADED` pool still serves its
 * data, which is exactly why it must read as a warning. `warning` on a
 * healthy pool means "nothing broken, but look" — a resilver, features not
 * enabled — and reads as an advisory.
 */
export function poolPlate(pool: TruenasPoolRow): Plating {
	const status = pool.status.toUpperCase();
	if (status === 'ONLINE') {
		if (!pool.healthy) return { tone: 'warning', label: m.devicesb_truenas_format_pool_unhealthy() };
		if (pool.warning) return { tone: 'advisory', label: m.devicesb_truenas_format_pool_attention() };
		return { tone: 'signal', label: m.devicesb_truenas_format_pool_healthy() };
	}
	if (status === 'DEGRADED') return { tone: 'warning', label: m.devicesb_truenas_format_pool_degraded() };
	if (!status) return { tone: 'ghost', label: m.devicesb_truenas_format_unknown() };
	return { tone: 'warning', label: titleCase(status) };
}

/** "Scrub 42 %", "Resilver 42 %, ~12 min left". */
export function scanLabel(
	fn: string | null,
	percent: number | null,
	secondsLeft: number | null
): string {
	const upper = fn ? fn.toUpperCase() : '';
	const name =
		upper === 'SCRUB'
			? m.devicesb_truenas_format_scan_scrub()
			: upper === 'RESILVER'
				? m.devicesb_truenas_format_scan_resilver()
				: fn
					? titleCase(fn)
					: m.devicesb_truenas_format_scan_generic();
	const pct = reading(percent);
	const left = reading(secondsLeft);
	const span = left !== null && left > 0 ? formatSpan(left) : null;
	if (pct !== null) {
		const percentText = pct.toFixed(0);
		return span !== null
			? m.devicesb_truenas_format_scan_percent_left({ name, percent: percentText, span })
			: m.devicesb_truenas_format_scan_percent({ name, percent: percentText });
	}
	return span !== null
		? m.devicesb_truenas_format_scan_running_left({ name, span })
		: m.devicesb_truenas_format_scan_running({ name });
}

export function runningScan(scan: TruenasScan | null): TruenasScan | null {
	return scan && scan.state.toUpperCase() === 'SCANNING' ? scan : null;
}

function roleLabel(role: string): string {
	switch (role) {
		case 'data':
			return m.devicesb_truenas_format_role_data();
		case 'log':
			return m.devicesb_truenas_format_role_log();
		case 'cache':
			return m.devicesb_truenas_format_role_cache();
		case 'spare':
			return m.devicesb_truenas_format_role_spare();
		case 'special':
			return m.devicesb_truenas_format_role_special();
		case 'dedup':
			return m.devicesb_truenas_format_role_dedup();
		default:
			return role;
	}
}

function disksWords(count: number): string {
	return count === 1
		? m.devicesb_truenas_format_disks_one({ count: formatCount(count) })
		: m.devicesb_truenas_format_disks_other({ count: formatCount(count) });
}

function vdevWords(kind: string, disks: number): string {
	const upper = kind.toUpperCase();
	if (upper === 'DISK' || upper === 'STRIPE') {
		return disks > 1
			? m.devicesb_truenas_format_disks_other({ count: formatCount(disks) })
			: m.devicesb_truenas_format_single_disk();
	}
	return disks > 0
		? m.devicesb_truenas_format_kind_disks({ kind: upper, disks: disksWords(disks) })
		: upper;
}

/**
 * The shape of a pool, in words: "2 × MIRROR (2 disks)", "RAIDZ1 (3 disks)",
 * then the other roles — "log: MIRROR (2 disks)", "cache: single disk".
 * Identical vdevs of one role are grouped.
 */
export function vdevLayout(vdevs: TruenasVdev[]): string[] {
	const groups = new Map<string, { role: string; kind: string; disks: number; count: number }>();
	for (const vdev of vdevs) {
		const role = vdev.role || 'data';
		const key = `${role}/${vdev.kind}/${vdev.disks}`;
		const group = groups.get(key);
		if (group) group.count += 1;
		else groups.set(key, { role, kind: vdev.kind, disks: vdev.disks, count: 1 });
	}
	const ordered = [...groups.values()].sort(
		(a, b) => Number(a.role !== 'data') - Number(b.role !== 'data')
	);
	return ordered.map((group) => {
		const words = vdevWords(group.kind, group.disks);
		const counted =
			group.count > 1
				? m.devicesb_truenas_format_vdev_counted({ count: group.count, words })
				: words;
		if (group.role === 'data') return counted;
		return m.devicesb_truenas_format_vdev_role({ role: roleLabel(group.role), words: counted });
	});
}

/** TrueNAS's alert levels, on the product's ladder. */
export function alertPlate(level: string): Plating {
	const upper = level.toUpperCase();
	switch (upper) {
		case 'INFO':
			return { tone: 'info', label: m.devicesb_truenas_format_level_info() };
		case 'NOTICE':
			return { tone: 'info', label: m.devicesb_truenas_format_level_notice() };
		case 'WARNING':
			return { tone: 'advisory', label: m.devicesb_truenas_format_level_warning() };
		case 'ERROR':
			return { tone: 'warning', label: m.devicesb_truenas_format_level_error() };
		case 'CRITICAL':
			return { tone: 'warning', label: m.devicesb_truenas_format_level_critical() };
		case 'ALERT':
			return { tone: 'warning', label: m.devicesb_truenas_format_level_alert() };
		case 'EMERGENCY':
			return { tone: 'warning', label: m.devicesb_truenas_format_level_emergency() };
		default:
			return {
				tone: 'ghost',
				label: upper ? titleCase(upper) : m.devicesb_truenas_format_unknown()
			};
	}
}

/** Levels that the product reads as a warning, not an advisory. */
export const SERIOUS_LEVELS = ['ERROR', 'CRITICAL', 'ALERT', 'EMERGENCY'];

/**
 * Task plate. A task in error only counts as failed while enabled — a
 * disabled one that failed long ago is history, said in a quieter voice.
 */
export function taskPlate(task: TruenasTaskRow): Plating {
	switch (task.state.toUpperCase()) {
		case 'FINISHED':
			return { tone: 'signal', label: m.devicesb_truenas_format_task_ok() };
		case 'ERROR':
			return task.failed
				? { tone: 'warning', label: m.devicesb_truenas_format_task_failed() }
				: { tone: 'ghost', label: m.devicesb_truenas_format_task_failed() };
		case 'RUNNING':
			return { tone: 'info', label: m.devicesb_truenas_format_task_running() };
		case 'WAITING':
			return { tone: 'info', label: m.devicesb_truenas_format_task_waiting() };
		case 'PENDING':
			return { tone: 'ghost', label: m.devicesb_truenas_format_task_never_run() };
		case 'HOLD':
			return { tone: 'advisory', label: m.devicesb_truenas_format_task_on_hold() };
		default:
			return {
				tone: 'ghost',
				label: task.state ? titleCase(task.state) : m.devicesb_truenas_format_unknown()
			};
	}
}

export function taskKindLabel(kind: string): string {
	if (kind === 'replication') return m.devicesb_truenas_format_kind_replication();
	if (kind === 'snapshot') return m.devicesb_truenas_format_kind_snapshots();
	return kind;
}

/** Fill colours of the usage bars. */
export const FILL: Record<'signal' | 'advisory' | 'warning', string> = {
	signal: 'bg-signal',
	advisory: 'bg-advisory',
	warning: 'bg-warning'
};
