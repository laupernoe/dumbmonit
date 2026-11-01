/**
 * Completion for the query box: metric names anywhere, label names inside
 * `{…}` after a metric, and label values after `label="`. Only what the
 * catalogue knows; no grammar, so it never blocks typing something else.
 */
import type { CatalogMetric } from '#lib/api/types.js';

export interface Suggestions {
	/** Replace `text.slice(from, to)` with the chosen item. */
	from: number;
	to: number;
	items: string[];
	kind: 'metric' | 'label' | 'value';
}

const MAX_ITEMS = 8;
const WORD = /[A-Za-z_:][A-Za-z0-9_:]*$/;

function rank(candidates: Iterable<string>, word: string): string[] {
	const lower = word.toLowerCase();
	const starts: string[] = [];
	const inside: string[] = [];
	for (const candidate of candidates) {
		const name = candidate.toLowerCase();
		if (name.startsWith(lower)) starts.push(candidate);
		else if (lower.length > 1 && name.includes(lower)) inside.push(candidate);
	}
	return [...starts.sort(), ...inside.sort()].slice(0, MAX_ITEMS);
}

export function suggest(text: string, caret: number, catalog: readonly CatalogMetric[], force = false): Suggestions | null {
	const before = text.slice(0, caret);
	const open = before.lastIndexOf('{');
	const inBraces = open >= 0 && open > before.lastIndexOf('}');

	if (inBraces) {
		const metricName = /([A-Za-z_:][A-Za-z0-9_:]*)\s*$/.exec(before.slice(0, open))?.[1];
		const metrics = metricName ? catalog.filter((c) => c.name === metricName) : catalog;
		const segment = before.slice(Math.max(open, before.lastIndexOf(',')) + 1);
		const valuePos = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*(?:=~|!=|!~|=)\s*"([^"]*)$/.exec(segment);
		if (valuePos) {
			const values = new Set<string>();
			for (const metric of metrics) {
				for (const label of metric.labels) if (label.name === valuePos[1]) label.values.forEach((v) => values.add(v));
			}
			const items = rank(values, valuePos[2]).filter((v) => v !== valuePos[2]);
			return items.length ? { from: caret - valuePos[2].length, to: caret, items, kind: 'value' } : null;
		}
		if (/[=~!"]/.test(segment)) return null;
		const word = WORD.exec(before)?.[0] ?? '';
		const names = new Set<string>();
		for (const metric of metrics) for (const label of metric.labels) names.add(label.name);
		const items = rank(names, word).filter((n) => n !== word);
		return items.length && (word || force) ? { from: caret - word.length, to: caret, items, kind: 'label' } : null;
	}

	const word = WORD.exec(before)?.[0] ?? '';
	if (!word && !force) return null;
	const items = rank(
		catalog.map((c) => c.name),
		word
	).filter((n) => n !== word);
	return items.length ? { from: caret - word.length, to: caret, items, kind: 'metric' } : null;
}

/** Applies a chosen item; returns the new text and where the caret goes. */
export function apply(text: string, found: Suggestions, item: string): { text: string; caret: number } {
	return { text: text.slice(0, found.from) + item + text.slice(found.to), caret: found.from + item.length };
}
