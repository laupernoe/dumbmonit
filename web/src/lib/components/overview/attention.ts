/**
 * What the Overview keeps from the sky and the history: the "Needs
 * attention" rows, the week's deadlines and the alerts of the last days.
 *
 * Pure on purpose (types only), so it is tested with `node --test`; the
 * component picks the words.
 */
import type { SkyRow } from './sky.js';

export interface AttentionRow {
	row: SkyRow;
	/** Alerts held back behind this monitor while it is down (its guests). */
	held: number;
}

/**
 * Only what is broken now: monitors down, unreachable or misconfigured, and
 * firing alerts nobody has acknowledged or snoozed (info alerts stay on the
 * Alerts page). An alert held back behind a down parent is not a row of its
 * own: it is counted on the parent's row. Keeps the sky's reading order.
 */
export function attentionRows(rows: readonly SkyRow[]): AttentionRow[] {
	const held = new Map<number, number>();
	for (const row of rows) {
		if (row.kind !== 'alert' || row.alert.effective_phase !== 'suppressed' || !row.parent) continue;
		held.set(row.parent.id, (held.get(row.parent.id) ?? 0) + 1);
	}
	const out: AttentionRow[] = [];
	const counted = new Set<number>();
	for (const row of rows) {
		if (row.kind === 'alert') {
			const alert = row.alert;
			if (alert.effective_phase !== 'firing' || alert.acked || alert.silenced || alert.severity === 'info') continue;
		}
		const id = row.kind === 'device' ? row.target.id : (row.target?.id ?? null);
		let guests = 0;
		if (id !== null && !counted.has(id) && held.has(id)) {
			guests = held.get(id) ?? 0;
			counted.add(id);
		}
		out.push({ row, held: guests });
	}
	return out;
}

/** Monitors the rows are about, each once: "N monitors need attention". */
export function attentionMonitors(rows: readonly AttentionRow[]): number {
	const ids = new Set<string>();
	for (const { row } of rows) {
		if (row.kind === 'device') ids.add(`t${row.target.id}`);
		else ids.add(row.target ? `t${row.target.id}` : `a${row.alert.fingerprint}`);
	}
	return ids.size;
}

/** What expires within `days` (already expired first), soonest first. */
export function weekAhead<T extends { days_left: number }>(items: readonly T[], days = 7): T[] {
	return items.filter((item) => item.days_left <= days).sort((a, b) => a.days_left - b.days_left);
}

export interface AlertDay {
	/** Local midnight that starts the day, in seconds (the chart's x). */
	day: number;
	opened: number;
	resolved: number;
}

/** Server timestamps are UTC without a suffix (`parseServerDate` does the same). */
function serverTime(value: string): number {
	const normalised = /[zZ]|[+-]\d{2}:?\d{2}$/.test(value) ? value.replace(' ', 'T') : `${value.replace(' ', 'T')}Z`;
	return new Date(normalised).getTime();
}

/**
 * Alerts that fired and alerts that resolved, per local day, over the last
 * `days` days including today, oldest first. Every firing counts: an alert
 * that flaps three times opened three times.
 */
export function alertsPerDay(
	history: readonly { at: string; to_phase: string }[],
	now: number,
	days = 7
): AlertDay[] {
	const today = new Date(now);
	today.setHours(0, 0, 0, 0);
	const starts: number[] = [];
	for (let i = days - 1; i >= 0; i -= 1) {
		const start = new Date(today);
		start.setDate(today.getDate() - i);
		starts.push(start.getTime());
	}
	const out: AlertDay[] = starts.map((start) => ({ day: Math.round(start / 1000), opened: 0, resolved: 0 }));
	const first = starts[0];
	for (const entry of history) {
		if (entry.to_phase !== 'firing' && entry.to_phase !== 'resolved') continue;
		const at = serverTime(entry.at);
		if (!Number.isFinite(at) || at < first || at > now) continue;
		let index = starts.length - 1;
		while (index > 0 && at < starts[index]) index -= 1;
		if (entry.to_phase === 'firing') out[index].opened += 1;
		else out[index].resolved += 1;
	}
	return out;
}
