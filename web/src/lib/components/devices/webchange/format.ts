/**
 * Pure helpers for the website-changes panel: shortening a page's URL down to
 * something that fits a table cell, wording a collapsed run of unchanged
 * diff lines, clamping the before/after slider's position, and grouping the
 * same edits found on several pages and walking the list by keyboard.
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

/** Matrix cells all the word comparisons of one diff may fill; past it, lines are shown whole. */
const TOTAL_BUDGET = 4_000_000;

/**
 * Marks which tokens of `a` and `b` are outside their longest common
 * subsequence, after stripping the common head and tail (most edits are small,
 * so the matrix stays tiny). `null` when the middle is too long to compare or
 * would overspend `budget.cells`.
 */
function compare(
	a: string[],
	b: string[],
	budget: { cells: number }
): { left: Array<[string, boolean]>; right: Array<[string, boolean]> } | null {
	let head = 0;
	while (head < a.length && head < b.length && a[head] === b[head]) head++;
	let tail = 0;
	while (tail < a.length - head && tail < b.length - head && a[a.length - 1 - tail] === b[b.length - 1 - tail]) tail++;
	const midA = a.slice(head, a.length - tail);
	const midB = b.slice(head, b.length - tail);
	const n = midA.length;
	const m = midB.length;
	// A word is a token and the space after it.
	if (n > MAX_WORDS * 2 || m > MAX_WORDS * 2) return null;
	const width = m + 1;
	const cells = (n + 1) * width;
	if (cells > budget.cells) return null;
	budget.cells -= cells;
	// Longest common subsequence, by suffix lengths, in one flat typed array.
	const lcs = new Int32Array(cells);
	for (let i = n - 1; i >= 0; i--) {
		for (let j = m - 1; j >= 0; j--) {
			lcs[i * width + j] =
				midA[i] === midB[j] ? lcs[(i + 1) * width + j + 1] + 1 : Math.max(lcs[(i + 1) * width + j], lcs[i * width + j + 1]);
		}
	}
	const left: Array<[string, boolean]> = a.slice(0, head).map((t) => [t, false]);
	const right: Array<[string, boolean]> = b.slice(0, head).map((t) => [t, false]);
	let i = 0;
	let j = 0;
	while (i < n || j < m) {
		if (i < n && j < m && midA[i] === midB[j]) {
			left.push([midA[i++], false]);
			right.push([midB[j++], false]);
		} else if (j < m && (i === n || lcs[i * width + j + 1] >= lcs[(i + 1) * width + j])) {
			right.push([midB[j++], true]);
		} else {
			left.push([midA[i++], true]);
		}
	}
	for (let k = a.length - tail; k < a.length; k++) left.push([a[k], false]);
	for (let k = b.length - tail; k < b.length; k++) right.push([b[k], false]);
	return { left, right };
}

function wordDiffWithin(before: string, after: string, budget: { cells: number }): { before: Segment[]; after: Segment[] } | null {
	const result = compare(tokens(before), tokens(after), budget);
	return result && { before: segments(absorbSpaces(result.left)), after: segments(absorbSpaces(result.right)) };
}

/**
 * Word-level comparison of a removed line and the line that replaced it: the
 * longest common run of words is plain, the rest is `changed`. Spaces between
 * two changed words count as changed, so a replaced phrase reads as one run.
 * Lines too long to compare word by word come back entirely changed.
 */
export function wordDiff(before: string, after: string): { before: Segment[]; after: Segment[] } {
	return (
		wordDiffWithin(before, after, { cells: TOTAL_BUDGET }) ?? {
			before: [{ text: before, changed: true }],
			after: [{ text: after, changed: true }]
		}
	);
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
	const budget = { cells: TOTAL_BUDGET };
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
			// Past the budget (or a pair too long), the pair stays whole, unmarked.
			const words = wordDiffWithin(lines[k + p].text, lines[deletes + p].text, budget);
			if (!words) continue;
			rows[k + p].segments = words.before;
			rows[deletes + p].segments = words.after;
		}
		k = inserts;
	}
	return rows;
}
