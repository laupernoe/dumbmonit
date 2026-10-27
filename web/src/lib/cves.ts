/** Pure helpers of the CVE views: grouping, ordering, tones, query building. */
import type { CveFinding, CveSeverity, CveState } from './api/types';

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

export type Pivot = 'cve' | 'machine';
export type StatusFilter = 'open' | 'all' | CveState;
export type IgnoredFilter = 'all' | 'hide' | 'only';

/** Everything the Security page shows is a function of this (and lives in the URL). */
export interface ViewState {
	pivot: Pivot;
	q: string;
	severity: CveSeverity | '';
	/** Known exploited (CISA KEV) only. */
	kev: boolean;
	/** A fixed version exists. */
	fix: boolean;
	ignored: IgnoredFilter;
	status: StatusFilter;
	folder: string | null;
	client: number | null;
	/** Focus on one CVE (the machines that have it) or one machine (its CVEs). */
	cve: string;
	machine: number | null;
}

export const EMPTY_VIEW: ViewState = {
	pivot: 'cve',
	q: '',
	severity: '',
	kev: false,
	fix: false,
	ignored: 'all',
	status: 'open',
	folder: null,
	client: null,
	cve: '',
	machine: null
};

const SEVERITIES: CveSeverity[] = ['critical', 'high', 'medium', 'low', 'none'];
const STATUSES: StatusFilter[] = ['open', 'all', 'vulnerable', 'unknown', 'patched'];

/** `?view=machine&sev=high&kev=1`; unknown values fall back to the default. `target` is the device page's link. */
export function parseViewState(params: URLSearchParams): ViewState {
	const sev = params.get('sev') as CveSeverity | null;
	const status = params.get('status') as StatusFilter | null;
	const ignored = params.get('ignored');
	const client = params.get('client');
	const machine = parseTargetParam(params.get('machine') ?? params.get('target'));
	return {
		pivot: params.get('view') === 'machine' ? 'machine' : 'cve',
		q: params.get('q') ?? '',
		severity: sev && SEVERITIES.includes(sev) ? sev : '',
		kev: params.get('kev') === '1',
		fix: params.get('fix') === '1',
		ignored: ignored === 'hide' || ignored === 'only' ? ignored : 'all',
		status: status && STATUSES.includes(status) ? status : 'open',
		folder: params.get('folder'),
		client: parseTargetParam(client),
		cve: params.get('cve') ?? '',
		machine
	};
}

/** Only what differs from the default goes in the URL; other, unrelated params are kept. */
export function viewStateToParams(v: ViewState, base: URLSearchParams = new URLSearchParams()): URLSearchParams {
	const out = new URLSearchParams(base);
	for (const k of ['view', 'q', 'sev', 'kev', 'fix', 'ignored', 'status', 'folder', 'client', 'cve', 'machine', 'target']) {
		out.delete(k);
	}
	if (v.pivot === 'machine') out.set('view', 'machine');
	if (v.q.trim()) out.set('q', v.q.trim());
	if (v.severity) out.set('sev', v.severity);
	if (v.kev) out.set('kev', '1');
	if (v.fix) out.set('fix', '1');
	if (v.ignored !== 'all') out.set('ignored', v.ignored);
	if (v.status !== 'open') out.set('status', v.status);
	if (v.folder !== null) out.set('folder', v.folder);
	if (v.client !== null) out.set('client', String(v.client));
	if (v.cve) out.set('cve', v.cve);
	if (v.machine !== null) out.set('machine', String(v.machine));
	return out;
}

/** The filters that narrow the list (not the pivot): drives the "reset" button and the empty state. */
export function viewFiltersActive(v: ViewState): boolean {
	return (
		v.q.trim() !== '' || v.severity !== '' || v.kev || v.fix || v.ignored !== 'all' || v.status !== 'open' ||
		v.folder !== null || v.client !== null || v.cve !== '' || v.machine !== null
	);
}

/** What the page knows about a device beyond the finding: its folder and client. */
export interface MachineMeta {
	folder: string;
	client: number | null;
}

export function filterFindings(findings: CveFinding[], v: ViewState, meta: Map<number, MachineMeta>): CveFinding[] {
	const q = v.q.trim().toLowerCase();
	return findings.filter((f) => {
		if (v.severity && f.severity !== v.severity) return false;
		if (v.kev && !f.exploited) return false;
		if (v.fix && (f.fixed_version === null || f.state === 'patched')) return false;
		if (v.ignored === 'hide' && f.ignored) return false;
		if (v.ignored === 'only' && !f.ignored) return false;
		if (v.status === 'open' ? f.state === 'patched' : v.status !== 'all' && f.state !== v.status) return false;
		if (v.cve && f.cve_id !== v.cve) return false;
		if (v.machine !== null && f.target_id !== v.machine) return false;
		if (v.folder !== null || v.client !== null) {
			const m = meta.get(f.target_id);
			if (v.folder !== null && (m?.folder ?? '') !== v.folder) return false;
			if (v.client !== null && (m?.client ?? null) !== v.client) return false;
		}
		if (q) {
			const hay = `${f.cve_id} ${f.component} ${f.target_name} ${f.summary}`.toLowerCase();
			if (!hay.includes(q)) return false;
		}
		return true;
	});
}

const SEVERITY_RANK: Record<CveSeverity, number> = { critical: 0, high: 1, medium: 2, low: 3, none: 4 };
export const severityRank = (s: CveSeverity): number => SEVERITY_RANK[s];

/** Exploited and open first, then open, then the rest. */
const urgency = (f: CveFinding): number => (f.ignored ? 3 : f.exploited && f.state === 'vulnerable' ? 0 : f.state === 'vulnerable' ? 1 : f.state === 'unknown' ? 2 : 3);

/** One CVE, once, with every machine it touches. */
export interface CveGroup {
	cve_id: string;
	link: string;
	summary: string;
	severity: CveSeverity;
	cvss: number | null;
	exploited: boolean;
	ransomware: boolean;
	/** A fixed version exists for at least one open finding. */
	fixAvailable: boolean;
	/** Every finding is ignored. */
	allIgnored: boolean;
	/** At least one finding is still open (vulnerable or to check, not ignored). */
	open: boolean;
	machines: { id: number; name: string }[];
	findings: CveFinding[];
}

export function aggregateByCve(findings: CveFinding[]): CveGroup[] {
	const map = new Map<string, CveGroup>();
	for (const f of findings) {
		let g = map.get(f.cve_id);
		if (!g) {
			g = {
				cve_id: f.cve_id, link: f.link, summary: f.summary, severity: f.severity, cvss: f.cvss,
				exploited: false, ransomware: false, fixAvailable: false, allIgnored: true, open: false,
				machines: [], findings: []
			};
			map.set(f.cve_id, g);
		}
		g.findings.push(f);
		if (SEVERITY_RANK[f.severity] < SEVERITY_RANK[g.severity]) g.severity = f.severity;
		if (f.cvss !== null && (g.cvss === null || f.cvss > g.cvss)) g.cvss = f.cvss;
		if (!g.summary && f.summary) g.summary = f.summary;
		if (f.exploited) g.exploited = true;
		if (f.ransomware) g.ransomware = true;
		if (!f.ignored) g.allIgnored = false;
		if (!f.ignored && f.state !== 'patched') {
			g.open = true;
			if (f.fixed_version !== null) g.fixAvailable = true;
		}
		if (!g.machines.some((m) => m.id === f.target_id)) g.machines.push({ id: f.target_id, name: f.target_name });
	}
	const list = [...map.values()];
	for (const g of list) {
		g.findings.sort((a, b) => urgency(a) - urgency(b) || a.target_name.localeCompare(b.target_name));
		g.machines.sort((a, b) => a.name.localeCompare(b.name));
	}
	return list.sort(
		(a, b) =>
			Number(b.exploited && b.open) - Number(a.exploited && a.open) ||
			Number(a.allIgnored) - Number(b.allIgnored) ||
			SEVERITY_RANK[a.severity] - SEVERITY_RANK[b.severity] ||
			(b.cvss ?? -1) - (a.cvss ?? -1) ||
			b.machines.length - a.machines.length ||
			a.cve_id.localeCompare(b.cve_id)
	);
}

/** One machine, once, with its CVEs. */
export interface MachineGroup extends DeviceGroup {
	folder: string;
	client: number | null;
	worst: CveSeverity;
	/** Distinct CVE ids. */
	cves: number;
	fixable: number;
}

export function aggregateByMachine(findings: CveFinding[], meta: Map<number, MachineMeta>): MachineGroup[] {
	return groupByDevice(findings)
		.map((g): MachineGroup => {
			const open = g.findings.filter((f) => !f.ignored && f.state !== 'patched');
			const shown = open.length > 0 ? open : g.findings;
			let worst: CveSeverity = 'none';
			for (const f of shown) if (SEVERITY_RANK[f.severity] < SEVERITY_RANK[worst]) worst = f.severity;
			const m = meta.get(g.target_id);
			return {
				...g,
				findings: [...g.findings].sort((a, b) => urgency(a) - urgency(b) || SEVERITY_RANK[a.severity] - SEVERITY_RANK[b.severity] || (b.cvss ?? -1) - (a.cvss ?? -1)),
				folder: m?.folder ?? '',
				client: m?.client ?? null,
				worst,
				cves: new Set(g.findings.map((f) => f.cve_id)).size,
				fixable: new Set(open.filter((f) => f.fixed_version !== null).map((f) => f.cve_id)).size
			};
		})
		.sort(
			(a, b) =>
				Number(b.exploited > 0) - Number(a.exploited > 0) ||
				(a.vulnerable + a.unknown === 0 ? 1 : 0) - (b.vulnerable + b.unknown === 0 ? 1 : 0) ||
				SEVERITY_RANK[a.worst] - SEVERITY_RANK[b.worst] ||
				b.vulnerable - a.vulnerable ||
				a.target_name.localeCompare(b.target_name)
		);
}

/** `?target=7` of the Security page link (a number, or null). */
export function parseTargetParam(value: string | null): number | null {
	if (!value || !/^\d+$/.test(value)) return null;
	return Number(value);
}
