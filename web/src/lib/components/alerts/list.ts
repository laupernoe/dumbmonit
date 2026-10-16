/**
 * How the Alerts "Now" list files its rows: pure helpers, no DOM, no network.
 *
 * One rule firing on several series of the same device (eight containers
 * stopped) is one problem, not eight: alert rows sharing the device, rule
 * and phase fold into the first one met, which carries the others.
 * Folded rows are then gathered either by severity (the default: what needs
 * you first) or by device, and the known problems (snoozed, acknowledged)
 * always sit in their own groups at the bottom, collapsed.
 */
import type { Alert } from '#lib/api/index.js';
import type { SkyRow } from '#lib/components/overview/sky.js';
import type { Tone } from '#lib/ui/index.js';

export interface Folded {
	row: SkyRow;
	/** Sibling alerts folded into this row (same device, rule and phase). */
	extra: Alert[];
}

export function fold(input: SkyRow[]): Folded[] {
	const out: Folded[] = [];
	const byKey = new Map<string, Folded>();
	for (const row of input) {
		if (row.kind !== 'alert') {
			out.push({ row, extra: [] });
			continue;
		}
		const a = row.alert;
		const key = [a.target_id, a.rule_uid, a.effective_phase, a.acked, a.silenced].join('|');
		const first = byKey.get(key);
		if (first) {
			first.extra.push(a);
			continue;
		}
		const folded: Folded = { row, extra: [] };
		byKey.set(key, folded);
		out.push(folded);
	}
	return out;
}

/** The severity groups, in reading order. */
export type Bucket = 'warning' | 'advisory' | 'info' | 'building' | 'suppressed' | 'snoozed' | 'acked';

export const BUCKETS: readonly Bucket[] = ['warning', 'advisory', 'info', 'building', 'suppressed', 'snoozed', 'acked'];

/** Groups that are already dealt with, or a consequence of another: folded away by default. */
export const QUIET_BUCKETS: ReadonlySet<Bucket> = new Set(['suppressed', 'snoozed', 'acked']);

/** Which severity group a row belongs to. */
export function bucketOf(row: SkyRow): Bucket {
	if (row.kind === 'device') return row.tone === 'warning' ? 'warning' : 'advisory';
	const alert = row.alert;
	if (alert.acked) return 'acked';
	if (alert.silenced) return 'snoozed';
	if (alert.effective_phase === 'suppressed') return 'suppressed';
	if (alert.effective_phase === 'pending') return 'building';
	// The bulletin's ladder, one rung down: critical reads Warning, warning Advisory.
	return alert.severity === 'critical' ? 'warning' : alert.severity === 'warning' ? 'advisory' : 'info';
}

/** The tone each group heading wears (its icon), never alone: the word goes with it. */
export const BUCKET_TONE: Record<Bucket, Tone> = {
	warning: 'warning',
	advisory: 'advisory',
	info: 'info',
	building: 'ghost',
	suppressed: 'muted',
	snoozed: 'muted',
	acked: 'muted'
};

/** Fingerprints a row stands for: its alert and the folded siblings. */
export function fingerprintsOf(folded: Folded): string[] {
	if (folded.row.kind !== 'alert') return [];
	return [folded.row.alert.fingerprint, ...folded.extra.map((a) => a.fingerprint)];
}

/** The device a row is about, as a group key ('none' for instance-wide alerts). */
export function deviceKey(row: SkyRow): string {
	const id = row.kind === 'device' ? row.target.id : row.alert.target_id;
	return id === null ? 'none' : String(id);
}
