/**
 * AdGuard Home as the probe last stored it: the `dumbmonit_adguard_*` series
 * folded into one reading for the panel. Pure, so the panel only decides the
 * words and the layout.
 */

export interface UpstreamRow {
	name: string;
	/** `null` when the upstream test is turned off. */
	up: boolean | null;
	responses: number | null;
	avgSeconds: number | null;
}

export interface FilterRow {
	name: string;
	rules: number | null;
	ageSeconds: number | null;
}

export interface AdguardReading {
	running: boolean | null;
	protection: boolean | null;
	pausedSeconds: number;
	version: string | null;
	queries: number | null;
	blocked: number | null;
	blockedPercent: number | null;
	safebrowsing: number | null;
	parental: number | null;
	safesearch: number | null;
	avgSeconds: number | null;
	windowSeconds: number | null;
	filteringEnabled: boolean | null;
	filtersEnabled: number | null;
	filterRules: number | null;
	oldestFilterAge: number | null;
	neverUpdated: number;
	updateCheck: boolean | null;
	updateAvailable: boolean | null;
	latest: string | null;
	upstreams: UpstreamRow[];
	filters: FilterRow[];
}

export const EMPTY_READING: AdguardReading = {
	running: null,
	protection: null,
	pausedSeconds: 0,
	version: null,
	queries: null,
	blocked: null,
	blockedPercent: null,
	safebrowsing: null,
	parental: null,
	safesearch: null,
	avgSeconds: null,
	windowSeconds: null,
	filteringEnabled: null,
	filtersEnabled: null,
	filterRules: null,
	oldestFilterAge: null,
	neverUpdated: 0,
	updateCheck: null,
	updateAvailable: null,
	latest: null,
	upstreams: [],
	filters: []
};

export const ADGUARD_QUERY = (target: number) =>
	`{__name__=~"dumbmonit_adguard_.+", target="${target}"}`;

type Series = { metric: Record<string, string>; values: [number, string][] };

export function foldAdguard(series: Series[]): AdguardReading {
	const out: AdguardReading = { ...EMPTY_READING, upstreams: [], filters: [] };
	const upstreams = new Map<string, UpstreamRow>();
	const filters = new Map<string, FilterRow>();
	const upstream = (name: string) => {
		let row = upstreams.get(name);
		if (!row) {
			row = { name, up: null, responses: null, avgSeconds: null };
			upstreams.set(name, row);
		}
		return row;
	};
	const filter = (name: string) => {
		let row = filters.get(name);
		if (!row) {
			row = { name, rules: null, ageSeconds: null };
			filters.set(name, row);
		}
		return row;
	};
	for (const serie of series) {
		const name = (serie.metric.__name__ ?? '').replace('dumbmonit_adguard_', '');
		const value = Number(serie.values.at(-1)?.[1]);
		if (!Number.isFinite(value)) continue;
		const m = serie.metric;
		switch (name) {
			case 'running':
				out.running = value >= 1;
				break;
			case 'protection_enabled':
				out.protection = value >= 1;
				break;
			case 'protection_paused_seconds':
				out.pausedSeconds = value;
				break;
			case 'version_info':
				out.version = m.version || null;
				break;
			case 'dns_queries':
				out.queries = value;
				break;
			case 'blocked_filtering':
				out.blocked = value;
				break;
			case 'blocked_percent':
				out.blockedPercent = value;
				break;
			case 'replaced_safebrowsing':
				out.safebrowsing = value;
				break;
			case 'replaced_parental':
				out.parental = value;
				break;
			case 'replaced_safesearch':
				out.safesearch = value;
				break;
			case 'avg_processing_seconds':
				out.avgSeconds = value;
				break;
			case 'stats_window_seconds':
				out.windowSeconds = value;
				break;
			case 'filtering_enabled':
				out.filteringEnabled = value >= 1;
				break;
			case 'filters_enabled':
				out.filtersEnabled = value;
				break;
			case 'filter_rules':
				out.filterRules = value;
				break;
			case 'filter_oldest_update_age_seconds':
				out.oldestFilterAge = value;
				break;
			case 'filters_never_updated':
				out.neverUpdated = value;
				break;
			case 'update_check_enabled':
				out.updateCheck = value >= 1;
				break;
			case 'update_available':
				out.updateAvailable = value >= 1;
				out.latest = m.latest || null;
				break;
			case 'upstream_up':
				if (m.upstream) upstream(m.upstream).up = value >= 1;
				break;
			case 'upstream_responses':
				if (m.upstream) upstream(m.upstream).responses = value;
				break;
			case 'upstream_avg_seconds':
				if (m.upstream) upstream(m.upstream).avgSeconds = value;
				break;
			case 'filter_rules_count':
				if (m.filter) filter(m.filter).rules = value;
				break;
			case 'filter_update_age_seconds':
				if (m.filter) filter(m.filter).ageSeconds = value;
				break;
		}
	}
	// Failing upstreams first, then the busiest; stalest lists first.
	const rank = (u: UpstreamRow) => (u.up === false ? 0 : 1);
	out.upstreams = [...upstreams.values()].sort(
		(a, b) => rank(a) - rank(b) || (b.responses ?? -1) - (a.responses ?? -1) || a.name.localeCompare(b.name, 'en')
	);
	out.filters = [...filters.values()].sort(
		(a, b) => (b.ageSeconds ?? Number.MAX_VALUE) - (a.ageSeconds ?? Number.MAX_VALUE) || a.name.localeCompare(b.name, 'en')
	);
	return out;
}

/** Three days, as the built-in rule: an enabled list older than this is stale. */
export const STALE_FILTER_SECONDS = 3 * 86_400;
