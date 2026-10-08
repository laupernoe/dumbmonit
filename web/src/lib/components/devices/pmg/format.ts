/**
 * Presentation helpers for the mail gateway panels: Unix seconds in, words out.
 * The API speaks Unix seconds because PMG does; the rest of the interface
 * speaks server date strings, hence these local variants.
 */
import { formatDateTime } from '#lib/format.js';
import { m } from '#lib/paraglide/messages.js';
import type { Tone } from '#lib/ui/index.js';
export { formatBytes } from '../docker/api';

export function formatUnix(seconds: number | null | undefined): string {
	if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) return m.devicesb_pmg_format_never();
	return formatDateTime(new Date(seconds * 1000));
}

/** Whole-unit span: "40 s", "12 min", "3 h", "5 d". */
export function formatSpan(seconds: number): string {
	if (seconds < 60) return m.devicesb_pmg_format_span_s({ count: Math.round(seconds) });
	if (seconds < 3600) return m.devicesb_pmg_format_span_min({ count: Math.round(seconds / 60) });
	if (seconds < 86400) {
		const hours = Math.floor(seconds / 3600);
		const minutes = Math.round((seconds % 3600) / 60);
		return minutes > 0 && hours < 10
			? m.devicesb_pmg_format_span_h_min({ hours, minutes })
			: m.devicesb_pmg_format_span_h({ count: hours });
	}
	return m.devicesb_pmg_format_span_d({ count: Math.round(seconds / 86400) });
}

/** "3 h ago", or "never". Negative ages (the future) read as "in 2 h". */
export function formatAgo(seconds: number | null | undefined): string {
	if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) {
		return m.devicesb_pmg_format_never();
	}
	const delta = Math.round(Date.now() / 1000 - seconds);
	if (delta < 0) return m.devicesb_pmg_format_in_span({ span: formatSpan(-delta) });
	if (delta < 45) return m.devicesb_pmg_format_just_now();
	return m.devicesb_pmg_format_ago({ span: formatSpan(delta) });
}

/** A plain count, grouped: "1 204". */
export function formatCount(value: number | null | undefined): string {
	if (value === null || value === undefined || !Number.isFinite(value)) return '—';
	return Math.round(value).toLocaleString('en-US').replace(/,/g, ' ');
}

/** The Postfix queue's name, or the raw name for one we do not know. */
export function queueLabel(queue: string): string {
	switch (queue) {
		case 'incoming':
			return m.devicesb_pmg_format_queue_incoming();
		case 'active':
			return m.devicesb_pmg_format_queue_active();
		case 'deferred':
			return m.devicesb_pmg_format_queue_deferred();
		case 'hold':
			return m.devicesb_pmg_format_queue_hold();
		default:
			return queue;
	}
}

/** What each Postfix queue is for, in one line. */
export function queueHelp(queue: string): string {
	switch (queue) {
		case 'incoming':
			return m.devicesb_pmg_format_queue_incoming_help();
		case 'active':
			return m.devicesb_pmg_format_queue_active_help();
		case 'deferred':
			return m.devicesb_pmg_format_queue_deferred_help();
		case 'hold':
			return m.devicesb_pmg_format_queue_hold_help();
		default:
			return '';
	}
}

/**
 * Queue plate. An empty queue is good news; a queue that merely has mail in it
 * is normal traffic, not a warning — only a stuck one earns a colour.
 */
export function queueTone(queue: { messages: number; stuck: boolean }): { tone: Tone; label: string } {
	if (queue.stuck) return { tone: 'warning', label: m.devicesb_pmg_format_queue_stuck() };
	if (queue.messages === 0) return { tone: 'signal', label: m.devicesb_pmg_format_queue_empty() };
	return { tone: 'info', label: m.devicesb_pmg_format_queue_flowing() };
}

/** Signature database plate: fresh, out of date, or never updated. */
export function signatureTone(signature: { stale: boolean; age_seconds: number | null }): {
	tone: Tone;
	label: string;
} {
	if (signature.age_seconds === null) return { tone: 'ghost', label: m.devicesb_pmg_format_sig_never() };
	if (signature.stale) return { tone: 'warning', label: m.devicesb_pmg_format_sig_stale() };
	return { tone: 'signal', label: m.devicesb_pmg_format_sig_fresh() };
}

/** Percent of a total, or `null` when the total is missing or zero. */
export function percentOf(used: number | null, total: number | null): number | null {
	if (used === null || total === null || total <= 0) return null;
	return Math.min(100, Math.max(0, (used / total) * 100));
}

/** `virus` / `spam` written out, for the signature table. */
export function familyLabel(family: string): string {
	return family === 'virus' ? m.devicesb_pmg_format_family_virus() : m.devicesb_pmg_format_family_spam();
}
