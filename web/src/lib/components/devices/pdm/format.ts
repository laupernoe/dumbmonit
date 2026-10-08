/**
 * Presentation helpers shared by the PDM panels: Unix seconds in, words out.
 * The API speaks Unix seconds because PDM does; the rest of the interface
 * speaks server date strings, hence these local variants.
 */
import type { PdmTaskKind } from '#lib/api/index.js';
import { formatDateTime } from '#lib/format.js';
import type { Tone } from '#lib/ui/index.js';
import { m } from '#lib/paraglide/messages.js';
export { formatBytes } from '../docker/api';

export function formatUnix(seconds: number | null | undefined): string {
	if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) return m.devicesb_pdm_format_never();
	return formatDateTime(new Date(seconds * 1000));
}

/** "3 h ago", or "never". Negative ages (the future) read as "in 2 h". */
export function formatAgo(seconds: number | null | undefined): string {
	if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) return m.devicesb_pdm_format_never();
	const delta = Math.round(Date.now() / 1000 - seconds);
	if (delta < 0) return m.devicesb_pdm_format_in({ span: formatSpan(-delta) });
	if (delta < 45) return m.devicesb_pdm_format_just_now();
	return m.devicesb_pdm_format_ago({ span: formatSpan(delta) });
}

/** Whole-unit span: "40 s", "12 min", "3 h", "5 d". */
export function formatSpan(seconds: number): string {
	if (seconds < 60) return m.devicesb_pdm_format_seconds({ n: Math.round(seconds) });
	if (seconds < 3600) return m.devicesb_pdm_format_minutes({ n: Math.round(seconds / 60) });
	if (seconds < 86400) {
		const hours = Math.floor(seconds / 3600);
		const minutes = Math.round((seconds % 3600) / 60);
		return minutes > 0 && hours < 10
			? m.devicesb_pdm_format_hours_minutes({ h: hours, m: minutes })
			: m.devicesb_pdm_format_hours({ h: hours });
	}
	return m.devicesb_pdm_format_days({ n: Math.round(seconds / 86400) });
}

export function formatDuration(start: number, end: number | null): string {
	if (end === null) return m.devicesb_pdm_format_running();
	return formatSpan(Math.max(0, end - start));
}

/** A count, or an em dash when the console said nothing — never a zero. */
export function formatCount(value: number | null | undefined): string {
	return value === null || value === undefined ? '—' : `${Math.round(value)}`;
}

export function formatPercent(value: number | null | undefined): string {
	return value === null || value === undefined ? '—' : `${Math.round(value)} %`;
}

/** Days until a Unix date, rounded down; `null` when there is no date. */
export function daysUntil(seconds: number | null | undefined): number | null {
	if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) return null;
	return Math.floor((seconds - Date.now() / 1000) / 86400);
}

export function remoteKindLabel(kind: string | null): string {
	if (!kind) return m.devicesb_pdm_format_remote_instance();
	if (kind === 'pve') return 'Proxmox VE';
	if (kind === 'pbs') return m.devicesb_pdm_format_remote_pbs();
	return kind;
}

/** The label of a task kind, or `null` for a kind this build does not know. */
export function taskKindLabel(kind: PdmTaskKind): string | null {
	switch (kind) {
		case 'backup':
			return m.devicesb_pdm_format_task_backup();
		case 'migrate':
			return m.devicesb_pdm_format_task_migrate();
		case 'sync':
			return m.devicesb_pdm_format_task_sync();
		case 'verify':
			return m.devicesb_pdm_format_task_verify();
		case 'prune':
			return m.devicesb_pdm_format_task_prune();
		case 'gc':
			return m.devicesb_pdm_format_task_gc();
		case 'replication':
			return m.devicesb_pdm_format_task_replication();
		case 'update':
			return m.devicesb_pdm_format_task_update();
		case 'other':
			return m.devicesb_pdm_format_task_other();
		default:
			return null;
	}
}

/** Tone and word for an instance, so status is never colour alone. */
export function remoteState(remote: {
	reachable: boolean;
	tasks_failed: number;
	version_behind: boolean;
}): { tone: Tone; word: string } {
	if (!remote.reachable) return { tone: 'warning', word: m.devicesb_pdm_format_unreachable() };
	if (remote.tasks_failed > 0) {
		return {
			tone: 'advisory',
			word: m.devicesb_pdm_format_failed_tasks({ count: remote.tasks_failed })
		};
	}
	if (remote.version_behind) return { tone: 'info', word: m.devicesb_pdm_format_version_behind() };
	return { tone: 'signal', word: m.devicesb_pdm_format_reachable() };
}

/** Tone and word for a subscription state, or `null` when the console is silent. */
export function subscriptionState(state: string | null): { tone: Tone; word: string } | null {
	switch (state) {
		case 'active':
			return { tone: 'signal', word: m.devicesb_pdm_format_sub_active() };
		case 'mixed':
			return { tone: 'info', word: m.devicesb_pdm_format_sub_mixed() };
		case 'none':
			return { tone: 'muted', word: m.devicesb_pdm_format_sub_none() };
		default:
			return null;
	}
}
