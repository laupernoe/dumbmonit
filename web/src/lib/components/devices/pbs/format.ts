/**
 * Presentation helpers shared by the PBS panels: Unix seconds in, words out.
 * The API speaks Unix seconds because PBS does; the rest of the interface
 * speaks server date strings, hence these local variants.
 */
import type { PbsDayState, PbsTaskKind } from '#lib/api/index.js';
import { formatDateTime } from '#lib/format.js';
import type { Tone } from '#lib/ui/index.js';
import { m } from '#lib/paraglide/messages.js';
export { formatAge, formatBytes } from '../docker/api';

export function formatUnix(seconds: number | null | undefined): string {
	if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) return m.devicesb_pbs_format_never();
	return formatDateTime(new Date(seconds * 1000));
}

/** "3 h ago", or "never". Negative ages (the future) read as "in 2 h". */
export function formatAgo(seconds: number | null | undefined): string {
	if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) return m.devicesb_pbs_format_never();
	const delta = Math.round(Date.now() / 1000 - seconds);
	if (delta < 0) return m.devicesb_pbs_format_in_span({ span: formatSpan(-delta) });
	if (delta < 45) return m.devicesb_pbs_format_just_now();
	return m.devicesb_pbs_format_ago({ span: formatSpan(delta) });
}

/** Whole-unit span: "40 s", "12 min", "3 h", "5 d". */
export function formatSpan(seconds: number): string {
	if (seconds < 60) return m.devicesb_pbs_format_span_s({ n: Math.round(seconds) });
	if (seconds < 3600) return m.devicesb_pbs_format_span_min({ n: Math.round(seconds / 60) });
	if (seconds < 86400) {
		const hours = Math.floor(seconds / 3600);
		const minutes = Math.round((seconds % 3600) / 60);
		return minutes > 0 && hours < 10
			? m.devicesb_pbs_format_span_h_min({ h: hours, min: minutes })
			: m.devicesb_pbs_format_span_h({ n: hours });
	}
	return m.devicesb_pbs_format_span_d({ n: Math.round(seconds / 86400) });
}

export function formatDuration(start: number, end: number | null): string {
	if (end === null) return m.devicesb_pbs_format_running();
	return formatSpan(Math.max(0, end - start));
}

export function taskKindLabel(kind: PbsTaskKind): string {
	switch (kind) {
		case 'backup':
			return m.devicesb_pbs_format_task_backup();
		case 'sync':
			return m.devicesb_pbs_format_task_sync();
		case 'verify':
			return m.devicesb_pbs_format_task_verify();
		case 'prune':
			return m.devicesb_pbs_format_task_prune();
		case 'gc':
			return m.devicesb_pbs_format_task_gc();
		default:
			return m.devicesb_pbs_format_task_other();
	}
}

export function jobKindLabel(kind: string): string {
	switch (kind) {
		case 'sync':
			return m.devicesb_pbs_format_task_sync();
		case 'verify':
			return m.devicesb_pbs_format_task_verify();
		case 'prune':
			return m.devicesb_pbs_format_task_prune();
		case 'gc':
			return m.devicesb_pbs_format_task_gc();
		default:
			return kind;
	}
}

export const DAY_TONE: Record<PbsDayState, Tone> = {
	ok: 'signal',
	verify_failed: 'advisory',
	failed: 'warning',
	running: 'info',
	none: 'ghost'
};

export function dayWord(state: PbsDayState): string {
	switch (state) {
		case 'ok':
			return m.devicesb_pbs_format_day_ok();
		case 'verify_failed':
			return m.devicesb_pbs_format_day_verify_failed();
		case 'failed':
			return m.devicesb_pbs_format_day_failed();
		case 'running':
			return m.devicesb_pbs_format_day_running();
		default:
			return m.devicesb_pbs_format_day_none();
	}
}

/** Tailwind background class of a day dot: token colours only, never raw palette. */
export const DAY_BG: Record<PbsDayState, string> = {
	ok: 'bg-signal',
	verify_failed: 'bg-advisory',
	failed: 'bg-warning',
	running: 'bg-info',
	none: 'ghost-cell bg-ghost opacity-70'
};

/** "vm/100" or "nextcloud (vm/100)". */
export function groupTitle(group: { name: string | null; backup_type: string; backup_id: string }): string {
	const id = `${group.backup_type}/${group.backup_id}`;
	return group.name ? `${group.name} (${id})` : id;
}

/** "main" or "main / pve". */
export function groupPlace(group: { datastore: string; namespace: string }): string {
	return group.namespace ? `${group.datastore} / ${group.namespace}` : group.datastore;
}

/** Percent of a total, or `null` when the total is missing or zero. */
export function percentOf(used: number | null, total: number | null): number | null {
	if (used === null || total === null || total <= 0) return null;
	return Math.min(100, Math.max(0, (used / total) * 100));
}
