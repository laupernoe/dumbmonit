/**
 * Pure logic of Settings → Data & retention and of the log search page:
 * sizes, disk shares, retention presets, what a save restarts, search periods.
 * No message or UI import, so it can be tested on its own.
 */
import type { StorageUpdate, StorageView } from '#lib/api/types';

/** Quick choices for a retention, in days. */
export const RETENTION_PRESETS: readonly number[] = [7, 14, 30, 90, 180, 365, 730];

/** "1.5 GB", "320 kB"; binary units, one decimal under 10. */
export function formatBytes(value: number | null | undefined): string {
	if (value === null || value === undefined || !Number.isFinite(value)) return '—';
	if (value <= 0) return '0 B';
	const units = ['B', 'kB', 'MB', 'GB', 'TB'];
	let n = value;
	let unit = 0;
	while (n >= 1024 && unit < units.length - 1) {
		n /= 1024;
		unit += 1;
	}
	return `${n < 10 && unit > 0 ? n.toFixed(1) : Math.round(n)} ${units[unit]}`;
}

/** Share of the disk (0..100) taken by `bytes`; 0 when the total is unknown. */
export function diskShare(bytes: number | null, total: number | null): number {
	if (bytes === null || !total || total <= 0) return 0;
	return Math.max(0, Math.min(100, (100 * bytes) / total));
}

/** A whole number of days within [min, max]; null when unusable. */
export function parseDays(value: unknown, min: number, max: number): number | null {
	const text = String(value ?? '').trim();
	if (text === '') return null;
	const n = Number(text);
	if (!Number.isInteger(n) || n < min || n > max) return null;
	return n;
}

export interface StorageForm {
	metricsDays: number;
	rollupEnabled: boolean;
	rollupDays: number;
	logsEnabled: boolean;
	logsDays: number;
}

export function formFrom(view: StorageView): StorageForm {
	return {
		metricsDays: view.metrics.retention_days,
		rollupEnabled: view.rollup.enabled,
		rollupDays: view.rollup.retention_days,
		logsEnabled: view.logs_store.enabled,
		logsDays: view.logs_store.retention_days
	};
}

/** The fields that differ from what the server has; empty = nothing to save. */
export function diffUpdate(view: StorageView, form: StorageForm): StorageUpdate {
	const update: StorageUpdate = {};
	if (view.metrics.embedded && form.metricsDays !== view.metrics.retention_days) {
		update.metrics_retention_days = form.metricsDays;
	}
	if (form.rollupEnabled !== view.rollup.enabled) update.rollup_enabled = form.rollupEnabled;
	if (form.rollupDays !== view.rollup.retention_days) update.rollup_retention_days = form.rollupDays;
	if (form.logsEnabled !== view.logs_store.enabled) update.logs_store_enabled = form.logsEnabled;
	if (form.logsDays !== view.logs_store.retention_days) update.logs_store_retention_days = form.logsDays;
	return update;
}

/** True when saving this update restarts (or starts/stops) an embedded process. */
export function needsRestart(update: StorageUpdate): boolean {
	return Object.keys(update).length > 0;
}

/** True when the update lowers the raw retention (older raw data gets deleted). */
export function lowersRawRetention(view: StorageView, update: StorageUpdate): boolean {
	return update.metrics_retention_days !== undefined && update.metrics_retention_days < view.metrics.retention_days;
}

export type SearchPeriod = '15m' | '1h' | '6h' | '24h' | '7d' | 'custom';

const PERIOD_MS: Record<Exclude<SearchPeriod, 'custom'>, number> = {
	'15m': 15 * 60_000,
	'1h': 3_600_000,
	'6h': 6 * 3_600_000,
	'24h': 24 * 3_600_000,
	'7d': 7 * 24 * 3_600_000
};

/** Start/end in ms for a preset period ending at `now`; for custom, the two typed instants (null when invalid or reversed). */
export function periodRange(
	period: SearchPeriod,
	now: number,
	custom?: { start: string; end: string }
): { start: number; end: number } | null {
	if (period !== 'custom') return { start: now - PERIOD_MS[period], end: now };
	if (!custom) return null;
	const start = Date.parse(custom.start);
	const end = custom.end ? Date.parse(custom.end) : now;
	if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start) return null;
	return { start, end };
}
