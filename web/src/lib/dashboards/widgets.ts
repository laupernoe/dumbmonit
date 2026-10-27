/**
 * What a widget needs to be shown: defaults per type, threshold levels, the
 * time ranges on offer. Pure, so it can be tested without a browser.
 */
import type { DashboardWidget, WidgetKind } from '#lib/api/types.js';

/** Smallest a widget can be resized to, in grid cells. */
export const MIN_SIZE = { w: 3, h: 2 } as const;

export const WIDGET_KINDS: readonly WidgetKind[] = ['chart', 'stat', 'group', 'alerts', 'text'];

export const DEFAULT_SIZE: Record<WidgetKind, { w: number; h: number }> = {
	chart: { w: 6, h: 5 },
	stat: { w: 3, h: 3 },
	group: { w: 4, h: 5 },
	alerts: { w: 4, h: 5 },
	text: { w: 4, h: 3 }
};

export function defaultConfig(kind: WidgetKind): Record<string, unknown> {
	switch (kind) {
		case 'chart':
			return { query: '', unit: '' };
		case 'stat':
			return { query: '', unit: '', direction: 'above' };
		case 'group':
			return { folder: '' };
		case 'alerts':
			return { limit: 10 };
		default:
			return { markdown: '' };
	}
}

/** A random-enough id that is valid for the server (`[A-Za-z0-9_-]{1,40}`). */
export function newWidgetId(taken: readonly string[]): string {
	for (;;) {
		const id = `w${Math.random().toString(36).slice(2, 10)}`;
		if (!taken.includes(id)) return id;
	}
}

export function cfgString(widget: DashboardWidget, key: string): string {
	const value = widget.config[key];
	return typeof value === 'string' ? value : '';
}

export function cfgNumber(widget: DashboardWidget, key: string): number | undefined {
	const value = widget.config[key];
	return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}

export type StatLevel = 'none' | 'ok' | 'warn' | 'crit';

/** Where a value stands against its thresholds. `below`: low is bad. */
export function statLevel(
	value: number | null,
	warn: number | undefined,
	crit: number | undefined,
	direction: 'above' | 'below'
): StatLevel {
	if (value === null || (warn === undefined && crit === undefined)) return 'none';
	const past = (limit: number | undefined) =>
		limit !== undefined && (direction === 'below' ? value <= limit : value >= limit);
	if (past(crit)) return 'crit';
	if (past(warn)) return 'warn';
	return 'ok';
}

/** Time ranges offered per dashboard, in seconds. */
export const RANGE_SECONDS = [900, 3600, 6 * 3600, 24 * 3600, 7 * 86400, 30 * 86400] as const;

/** About 300 points per chart; see `metrics.ts`. */
export function stepFor(seconds: number): number {
	return Math.max(10, Math.round(seconds / 300));
}

export const REFRESH_SECONDS = [0, 10, 30, 60, 300] as const;

/** Compact number for a big value: 1234567 becomes "1.23M". */
export function formatStat(value: number): string {
	const abs = Math.abs(value);
	if (abs >= 1e9) return `${+(value / 1e9).toPrecision(3)}G`;
	if (abs >= 1e6) return `${+(value / 1e6).toPrecision(3)}M`;
	if (abs >= 1e4) return `${+(value / 1e3).toPrecision(3)}k`;
	if (Number.isInteger(value)) return String(value);
	return String(+value.toPrecision(abs >= 100 ? 4 : 3));
}
