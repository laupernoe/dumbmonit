/**
 * Values of the two list-shaped options a collector can describe, as they are
 * stored in `Target.tags` (always strings): a `checklist` (ticked values) and
 * `rows` (a JSON array of objects). Kept apart from the components so the
 * encoding and the validation are testable without Svelte.
 */
import type { CollectorOption, CollectorOptionColumn } from '#lib/api/types.js';

/** What a checklist tag holds when nothing is ticked (a blank tag would mean "all"). */
export const NONE = '-';

const splitList = (raw: string): string[] =>
	raw
		.split(',')
		.map((part) => part.trim())
		.filter(Boolean);

/** Ticked values, in the order of the items. An absent tag means every item is ticked. */
export function checkedValues(option: CollectorOption, raw: string | undefined): string[] {
	const all = option.items.map((item) => item.value);
	const text = (raw ?? '').trim();
	if (!text) return all;
	if (text === NONE) return [];
	const wanted = new Set(splitList(text));
	return all.filter((value) => wanted.has(value));
}

/** The tag for a selection: blank when everything is ticked (the server then takes them all), `-` when nothing is. */
export function encodeChecked(option: CollectorOption, checked: string[]): string {
	const wanted = new Set(checked);
	const ordered = option.items.map((item) => item.value).filter((value) => wanted.has(value));
	if (ordered.length === option.items.length) return '';
	return ordered.length === 0 ? NONE : ordered.join(',');
}

export type Row = Record<string, string | boolean>;

/** The default row: first choice of each select, an empty text, an unticked box. */
export function emptyRow(columns: CollectorOptionColumn[]): Row {
	return Object.fromEntries(
		columns.map((column) => [column.key, column.input === 'boolean' ? false : column.input === 'select' ? (column.choices[0] ?? '') : ''])
	);
}

/** The rows of a tag; a value that is not a JSON array of objects gives none. */
export function parseRows(option: CollectorOption, raw: string | undefined): Row[] {
	const text = (raw ?? '').trim();
	if (!text) return [];
	try {
		const parsed: unknown = JSON.parse(text);
		if (!Array.isArray(parsed)) return [];
		return parsed
			.filter((entry): entry is Record<string, unknown> => typeof entry === 'object' && entry !== null)
			.map((entry) => {
				const row = emptyRow(option.columns);
				for (const column of option.columns) {
					const value = entry[column.key];
					if (column.input === 'boolean') row[column.key] = value === true || value === 'true';
					else if (typeof value === 'string') row[column.key] = value;
				}
				return row;
			});
	} catch {
		return [];
	}
}

/** A row the user has not started to fill: all its text columns are blank. */
function blank(option: CollectorOption, row: Row): boolean {
	return option.columns.filter((column) => column.input === 'text').every((column) => String(row[column.key] ?? '').trim() === '');
}

/** The tag for the rows: unfilled rows are left out, no row at all gives a blank tag. */
export function serializeRows(option: CollectorOption, rows: Row[]): string {
	const kept = rows
		.filter((row) => !blank(option, row))
		.map((row) =>
			Object.fromEntries(
				option.columns.map((column) => [column.key, typeof row[column.key] === 'string' ? (row[column.key] as string).trim() : row[column.key]])
			)
		);
	return kept.length === 0 ? '' : JSON.stringify(kept);
}

export interface RowMessages {
	/** A text column does not match its pattern and the server gave no message. */
	invalid: string;
	/** Two rows hold the same value in a `unique` column. */
	duplicate: string;
}

const same = (value: string): string => value.trim().toLowerCase().replace(/\.$/, '');

/** Errors by row index, then by column key. Unfilled rows have none. */
export function rowErrors(option: CollectorOption, rows: Row[], messages: RowMessages): Record<number, Record<string, string>> {
	const errors: Record<number, Record<string, string>> = {};
	const seen = new Map<string, Set<string>>();
	rows.forEach((row, index) => {
		if (blank(option, row)) return;
		for (const column of option.columns) {
			if (column.input !== 'text') continue;
			const value = String(row[column.key] ?? '').trim();
			let message = '';
			if (!value) message = messages.invalid;
			else if (column.pattern) {
				try {
					if (!new RegExp(column.pattern, 'i').test(value)) message = column.invalid || messages.invalid;
				} catch {
					// A pattern this browser cannot read is the server's to enforce.
				}
			}
			if (!message && column.unique) {
				const bucket = seen.get(column.key) ?? new Set<string>();
				if (bucket.has(same(value))) message = messages.duplicate;
				bucket.add(same(value));
				seen.set(column.key, bucket);
			}
			if (message) (errors[index] ??= {})[column.key] = message;
		}
	});
	return errors;
}

/** The first error of a set of rows, for the form-level check; `null` when they are all right. */
export function firstRowError(errors: Record<number, Record<string, string>>): string | null {
	for (const index of Object.keys(errors).map(Number).sort((a, b) => a - b)) {
		const message = Object.values(errors[index])[0];
		if (message) return message;
	}
	return null;
}

/** What a new device starts with for the list-shaped options: the described default, written down. */
export function seedChecklists(options: CollectorOption[], tags: Record<string, string>): Record<string, string> {
	const next = { ...tags };
	for (const option of options) {
		if (option.input === 'checklist' && option.default.trim() && next[option.key] === undefined) {
			next[option.key] = option.default.trim();
		}
	}
	return next;
}
