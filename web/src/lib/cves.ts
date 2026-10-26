/** Pure helpers of the CVE views: grouping, ordering, tones, query building. */
import type { CveFilters, CveFinding, CveSeverity, CveState } from './api/types';

export interface DeviceGroup {
	target_id: number;
	target_name: string;
	findings: CveFinding[];
	vulnerable: number;
	exploited: number;
	unknown: number;
	patched: number;
	ignored: number;
}

/** Group by device, keeping the server's order: a device comes where its worst finding is. */
export function groupByDevice(findings: CveFinding[]): DeviceGroup[] {
	const groups = new Map<number, DeviceGroup>();
	for (const f of findings) {
		let g = groups.get(f.target_id);
		if (!g) {
			g = { target_id: f.target_id, target_name: f.target_name, findings: [], vulnerable: 0, exploited: 0, unknown: 0, patched: 0, ignored: 0 };
			groups.set(f.target_id, g);
		}
		g.findings.push(f);
		if (f.ignored) g.ignored++;
		else {
			g[f.state]++;
			if (f.exploited && f.state === 'vulnerable') g.exploited++;
		}
	}
	return [...groups.values()];
}

export type CveTone = 'warning' | 'advisory' | 'info' | 'signal' | 'ghost' | 'muted';

export function severityTone(severity: CveSeverity): CveTone {
	switch (severity) {
		case 'critical':
		case 'high':
			return 'warning';
		case 'medium':
			return 'advisory';
		case 'low':
			return 'info';
		default:
			return 'ghost';
	}
}

export function stateTone(state: CveState, ignored = false): CveTone {
	if (ignored) return 'muted';
	return state === 'vulnerable' ? 'warning' : state === 'unknown' ? 'advisory' : 'signal';
}

/** The few findings worth a glance on a device page: open ones, exploited first, then vulnerable, then to check. */
export function topFindings(findings: CveFinding[], limit = 5): CveFinding[] {
	const rank = (f: CveFinding) => (f.exploited && f.state === 'vulnerable' ? 0 : f.state === 'vulnerable' ? 1 : 2);
	return findings
		.map((f, i) => ({ f, i }))
		.filter(({ f }) => !f.ignored && f.state !== 'patched')
		.sort((a, b) => rank(a.f) - rank(b.f) || a.i - b.i)
		.slice(0, limit)
		.map(({ f }) => f);
}

export interface FilterState {
	target: number | null;
	severity: CveSeverity | '';
	state: CveState | '';
	exploited: boolean;
	showIgnored: boolean;
}

export const EMPTY_FILTERS: FilterState = { target: null, severity: '', state: '', exploited: false, showIgnored: true };

export function filtersToQuery(f: FilterState): CveFilters {
	return {
		target: f.target ?? undefined,
		severity: f.severity || undefined,
		state: f.state || undefined,
		exploited: f.exploited ? true : undefined,
		ignored: f.showIgnored ? undefined : false
	};
}

export function filtersActive(f: FilterState): boolean {
	return f.target !== null || f.severity !== '' || f.state !== '' || f.exploited || !f.showIgnored;
}

/** `?target=7` of the Security page link (a number, or null). */
export function parseTargetParam(value: string | null): number | null {
	if (!value || !/^\d+$/.test(value)) return null;
	return Number(value);
}
