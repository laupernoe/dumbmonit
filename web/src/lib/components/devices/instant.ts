/**
 * Helpers shared by the panels that read the latest stored measurement of a
 * device straight from `queryInstant` (UniFi, Home Assistant, vSphere): the
 * device itself is never contacted when the page opens.
 */
import type { MetricSeries } from '#lib/api/index.js';

/** One series reduced to its name (prefix removed), labels and last value. */
export interface Reading {
	name: string;
	labels: Record<string, string>;
	value: number;
}

/** Keeps the series of `dumbmonit_<prefix>_*` with a finite last value. */
export function readings(series: MetricSeries[], prefix: string): Reading[] {
	const base = `dumbmonit_${prefix}_`;
	const out: Reading[] = [];
	for (const serie of series) {
		const raw = serie.metric.__name__ ?? '';
		if (!raw.startsWith(base)) continue;
		const value = Number(serie.values.at(-1)?.[1]);
		if (!Number.isFinite(value)) continue;
		out.push({ name: raw.slice(base.length), labels: serie.metric, value });
	}
	return out;
}

/** The selector for a set of families of one target. */
export function selector(prefix: string, names: string[], target: number): string {
	return `{__name__=~"dumbmonit_${prefix}_(${names.join('|')})", target="${target}"}`;
}

/** "42%" with no decimal above 10, one below. */
export function formatPercent(value: number | null | undefined): string {
	if (value === null || value === undefined || !Number.isFinite(value)) return '—';
	return `${value >= 10 ? Math.round(value) : value.toFixed(1)}%`;
}

/** `invalid_server_version` → "Invalid server version". */
export function humanize(id: string): string {
	const words = id.replace(/[_.-]+/g, ' ').trim();
	return words.charAt(0).toUpperCase() + words.slice(1);
}
