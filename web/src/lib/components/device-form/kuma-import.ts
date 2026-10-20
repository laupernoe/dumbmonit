/**
 * Pure helpers of the "Import from Uptime Kuma" dialog: what kind of file was
 * chosen, and which rows of the preview are ticked.
 */
import type { KumaImportRow } from '#lib/api/types.js';

/** First bytes of every SQLite file: `SQLite format 3\0`. */
const SQLITE_MAGIC = [0x53, 0x51, 0x4c, 0x69, 0x74, 0x65, 0x20, 0x66, 0x6f, 0x72, 0x6d, 0x61, 0x74, 0x20, 0x33, 0x00];

/** True when the bytes start like an SQLite database (the `kuma.db` of Uptime Kuma 1.x and 2.x). */
export function isSqliteHead(head: Uint8Array): boolean {
	return head.length >= SQLITE_MAGIC.length && SQLITE_MAGIC.every((byte, i) => head[i] === byte);
}

/** Rows that can be ticked: the ones the preview would create. */
export function importableRows(rows: KumaImportRow[]): number[] {
	return rows.filter((row) => row.status === 'ok').map((row) => row.row);
}

/** Ticks or unticks one row; returns a new set. */
export function toggleRow(selected: ReadonlySet<number>, row: number): Set<number> {
	const next = new Set(selected);
	if (next.has(row)) next.delete(row);
	else next.add(row);
	return next;
}

/** `all` when every importable row is ticked, `none` when none is, `some` between. */
export function selectionState(selected: ReadonlySet<number>, rows: KumaImportRow[]): 'all' | 'some' | 'none' {
	const importable = importableRows(rows);
	const ticked = importable.filter((row) => selected.has(row)).length;
	if (ticked === 0) return 'none';
	return ticked === importable.length ? 'all' : 'some';
}

/** The ticked rows, in preview order, ready for `?rows=`. */
export function tickedRows(selected: ReadonlySet<number>, rows: KumaImportRow[]): number[] {
	return importableRows(rows).filter((row) => selected.has(row));
}
