/**
 * Pure helpers for the website-changes panel: shortening a page's URL down to
 * something that fits a table cell, wording a collapsed run of unchanged
 * diff lines, clamping the before/after slider's position, and grouping the
 * change timeline by day and walking it by keyboard.
 *
 * Pure module, no imports: `npm test` (tests/webchange.test.mjs) runs it as is.
 */

/** The path (+ query/hash) of a URL, without its scheme and host; `"/"` for the root. */
export function pagePath(url: string): string {
	try {
		const parsed = new URL(url);
		return `${parsed.pathname}${parsed.search}${parsed.hash}` || '/';
	} catch {
		// Not a parseable absolute URL (a relative path the server already sent
		// shortened, for instance): show it as is.
		return url || '/';
	}
}

/**
 * Shortens a path to `max` characters by eliding its middle, keeping the
 * start (what section it's under) and the end (the actual page) legible.
 * Short enough paths are returned unchanged.
 */
export function shortenPath(path: string, max = 40): string {
	if (path.length <= max) return path;
	const ellipsis = '…';
	const keep = max - ellipsis.length;
	const head = Math.ceil(keep * 0.6);
	const tail = keep - head;
	return `${path.slice(0, head)}${ellipsis}${path.slice(path.length - tail)}`;
}

/** Wording of a collapsed run, per plural form. */
export interface SkipWords {
	one: (count: number) => string;
	other: (count: number) => string;
}

/**
 * The plain-English fallback, used where no translation is injected (this module
 * is pure, so it cannot import the message catalogue; the UI passes its own).
 */
const PLAIN_SKIP: SkipWords = {
	one: (n) => `⋯ ${n} unchanged line`,
	other: (n) => `⋯ ${n} unchanged lines`
};

/** Wording for a collapsed `skip` diff line: "⋯ 42 unchanged lines". */
export function skipLabel(count: number | null, words: SkipWords = PLAIN_SKIP): string {
	const n = count ?? 0;
	return n === 1 ? words.one(n) : words.other(n);
}

/** Clamps a slider position to the 0–100 range (percent from the left edge). */
export function clampPercent(value: number): number {
	if (Number.isNaN(value)) return 50;
	return Math.min(100, Math.max(0, value));
}

/**
 * Next slider position for an arrow-key press: ±`step` percent, Home/End to
 * the ends. Returns the unchanged value for any other key.
 */
export function stepPercent(current: number, key: string, step = 2): number {
	switch (key) {
		case 'ArrowLeft':
		case 'ArrowDown':
			return clampPercent(current - step);
		case 'ArrowRight':
		case 'ArrowUp':
			return clampPercent(current + step);
		case 'Home':
			return 0;
		case 'End':
			return 100;
		default:
			return current;
	}
}

/** Local calendar day of a date as `YYYY-MM-DD`. */
function dayKey(date: Date): string {
	const month = String(date.getMonth() + 1).padStart(2, '0');
	const day = String(date.getDate()).padStart(2, '0');
	return `${date.getFullYear()}-${month}-${day}`;
}

/** A run of items sharing one local calendar day; `key` is `''` and `day` `null` for undated items. */
export interface DayGroup<T> {
	key: string;
	day: Date | null;
	items: T[];
}

/**
 * Groups items, already sorted newest first, by the local calendar day of
 * `dateOf(item)`. Consecutive items of the same day share a group; the order
 * of groups and of items within them is the input's.
 */
export function groupByDay<T>(items: T[], dateOf: (item: T) => Date | null): DayGroup<T>[] {
	const groups: DayGroup<T>[] = [];
	for (const item of items) {
		const date = dateOf(item);
		const key = date ? dayKey(date) : '';
		const last = groups.at(-1);
		if (last && last.key === key) last.items.push(item);
		else groups.push({ key, day: date, items: [item] });
	}
	return groups;
}

/** Calendar days from `date` to `now` in local time: 0 the same day, 1 the day before. */
export function daysBetween(date: Date, now: Date): number {
	const start = new Date(date.getFullYear(), date.getMonth(), date.getDate());
	const end = new Date(now.getFullYear(), now.getMonth(), now.getDate());
	return Math.round((end.getTime() - start.getTime()) / 86_400_000);
}

/**
 * Next selected index in a list of `length` items for a key press: up/down
 * by one, Home/End to the ends, clamped. Unchanged for any other key; `-1`
 * for an empty list.
 */
export function stepIndex(current: number, key: string, length: number): number {
	if (length === 0) return -1;
	switch (key) {
		case 'ArrowDown':
			return Math.min(length - 1, current + 1);
		case 'ArrowUp':
			return Math.max(0, current - 1);
		case 'Home':
			return 0;
		case 'End':
			return length - 1;
		default:
			return current;
	}
}

/** A change and the ones right after it that made the same edit on other pages. */
export interface SameEdit<T> {
	lead: T;
	others: T[];
}

/**
 * Folds runs of consecutive items sharing a non-null `signature` (the same
 * lines removed and added, found by one check on several pages) into one
 * entry led by the first. Items without a signature stand alone.
 */
export function groupSameEdits<T extends { signature: string | null }>(items: T[]): SameEdit<T>[] {
	const groups: SameEdit<T>[] = [];
	for (const item of items) {
		const last = groups.at(-1);
		if (last && item.signature !== null && last.lead.signature === item.signature) last.others.push(item);
		else groups.push({ lead: item, others: [] });
	}
	return groups;
}

/** A run of a diff line's text, `changed` when it is what differs from the other side. */
export interface Segment {
	text: string;
	changed: boolean;
}

/** Above this many words on either side, a pair of lines is shown whole rather than word by word. */
const MAX_WORDS = 400;

/** Words and the spaces between them, kept as their own tokens so the text joins back unchanged. */
function tokens(line: string): string[] {
	return line.split(/(\s+)/).filter((token) => token !== '');
}

/** Joins consecutive tokens of the same kind into segments. */
function segments(parts: Array<[string, boolean]>): Segment[] {
	const out: Segment[] = [];
	for (const [text, changed] of parts) {
		const last = out.at(-1);
		if (last && last.changed === changed) last.text += text;
		else out.push({ text, changed });
	}
	return out;
}

/**
 * Word-level comparison of a removed line and the line that replaced it: the
 * longest common run of words is plain, the rest is `changed`. Spaces between
 * two changed words count as changed, so a replaced phrase reads as one run.
 * Lines too long to compare word by word come back entirely changed.
 */
export function wordDiff(before: string, after: string): { before: Segment[]; after: Segment[] } {
	const a = tokens(before);
	const b = tokens(after);
	if (a.length > MAX_WORDS * 2 || b.length > MAX_WORDS * 2) {
		return { before: [{ text: before, changed: true }], after: [{ text: after, changed: true }] };
	}
	// Longest common subsequence, by suffix lengths.
	const lcs: number[][] = Array.from({ length: a.length + 1 }, () => new Array(b.length + 1).fill(0));
	for (let i = a.length - 1; i >= 0; i--) {
		for (let j = b.length - 1; j >= 0; j--) {
			lcs[i][j] = a[i] === b[j] ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
		}
	}
	const left: Array<[string, boolean]> = [];
	const right: Array<[string, boolean]> = [];
	let i = 0;
	let j = 0;
	while (i < a.length || j < b.length) {
		if (i < a.length && j < b.length && a[i] === b[j]) {
			left.push([a[i++], false]);
			right.push([b[j++], false]);
		} else if (j < b.length && (i === a.length || lcs[i][j + 1] >= lcs[i + 1][j])) {
			right.push([b[j++], true]);
		} else {
			left.push([a[i++], true]);
		}
	}
	return { before: segments(absorbSpaces(left)), after: segments(absorbSpaces(right)) };
}

/** A space kept between two changed words is part of the change. */
function absorbSpaces(parts: Array<[string, boolean]>): Array<[string, boolean]> {
	return parts.map(([text, changed], k) => {
		const between = !changed && /^\s+$/.test(text) && parts[k - 1]?.[1] && parts[k + 1]?.[1];
		return [text, changed || Boolean(between)];
	});
}

/** One line of a diff, ready to render: its words, split where they differ from the paired line. */
export interface DiffRow<Op extends string> {
	op: Op;
	segments: Segment[];
	count: number | null;
}

/**
 * Pairs each run of deleted lines with the run of inserted lines right after
 * it, line by line, and compares each pair word by word. Unpaired lines (more
 * deleted than inserted, or the reverse) stay whole.
 */
export function pairWords<Op extends string>(
	lines: Array<{ op: Op; text: string; count: number | null }>
): DiffRow<Op>[] {
	const rows: DiffRow<Op>[] = lines.map((line) => ({
		op: line.op,
		segments: [{ text: line.text, changed: false }],
		count: line.count
	}));
	let k = 0;
	while (k < lines.length) {
		if (lines[k].op !== 'delete') {
			k++;
			continue;
		}
		let deletes = k;
		while (deletes < lines.length && lines[deletes].op === 'delete') deletes++;
		let inserts = deletes;
		while (inserts < lines.length && lines[inserts].op === 'insert') inserts++;
		const pairs = Math.min(deletes - k, inserts - deletes);
		for (let p = 0; p < pairs; p++) {
			const words = wordDiff(lines[k + p].text, lines[deletes + p].text);
			rows[k + p].segments = words.before;
			rows[deletes + p].segments = words.after;
		}
		k = inserts;
	}
	return rows;
}
