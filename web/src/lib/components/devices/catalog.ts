/**
 * Catalogue-driven device page: what a device kind declares it should show.
 *
 * `GET /api/collectors` may give each kind its metric families (unit, counter or
 * gauge, which way is good) and the views worth showing: a row of figures
 * (`stat`), then at most four charts. This module turns the families loaded for
 * a device into those views; everything else stays reachable under "All
 * measurements". A kind without views keeps the generic page.
 *
 * Pure functions, no messages: the component formats what is computed here, so
 * the logic runs under `node --test`.
 */
import type { CollectorInfo, MetricDescription, ViewDescription } from '#lib/api/index.js';
import type { DeviceMetricGroup, DeviceSerie } from './metrics.js';

/** The server stores every family with this prefix; the catalogue omits it. */
export const PREFIX = 'dumbmonit_';

/** At most this many entities in a ranking unless the view says otherwise. */
const DEFAULT_LIMIT = 10;

/** Interfaces Docker and the kernel make up: hidden from charts when a real one exists. */
const VIRTUAL_IF = /^(lo|veth.*|br-.*|docker\d*|virbr\d*|vnet\d*|tap\d*|cni\d*|flannel.*|dummy\d*)$/;

export function isVirtualInterface(name: string): boolean {
	return VIRTUAL_IF.test(name);
}

/** `if_octets_in` → `dumbmonit_if_octets_in`; a name that already has the prefix is kept. */
export function fullName(name: string): string {
	return name.startsWith(PREFIX) ? name : `${PREFIX}${name}`;
}

/** The kind declares views: the page follows them instead of charting everything. */
export function hasViews(collector: CollectorInfo | null | undefined): collector is CollectorInfo {
	return !!collector && collector.views.length > 0;
}

function isChart(view: ViewDescription): boolean {
	return view.view !== 'stat';
}

/**
 * Counters the page must ask as rates: the ones a chart shows. A counter only
 * listed in the row of figures keeps its running total (errors since boot).
 */
export function ratedFamilies(collector: CollectorInfo | null | undefined): string[] {
	if (!hasViews(collector)) return [];
	const counters = new Set(collector.metrics.filter((d) => d.kind === 'counter').map((d) => d.name));
	const names = new Set<string>();
	for (const view of collector.views.filter(isChart)) {
		for (const name of view.metrics) if (counters.has(name)) names.add(fullName(name));
	}
	return [...names].sort();
}

/** Unit of a counter once charted as a rate: `B` → `B/s`, nothing → `/s`. */
export function rateUnit(unit: string): string {
	return unit ? `${unit}/s` : '/s';
}

/** Unit as displayed: a rated counter is per second. */
export function displayUnit(metric: MetricDescription, rated: ReadonlySet<string>): string {
	return metric.kind === 'counter' && rated.has(fullName(metric.name)) ? rateUnit(metric.unit) : metric.unit;
}

/** Last value of a series, or `null` when it holds none. */
function last(serie: DeviceSerie): number | null {
	const point = serie.points.at(-1);
	return point ? point[1] : null;
}

/** Label keys that identify the whole device, never the thing measured. */
const COMMON = new Set(['__name__', 'target', 'host', 'instance', 'job']);
/** When an `_info` family carries one of these, it is the one people want to read. */
const INFO_KEYS = ['version', 'firmware', 'model', 'name', 'status'];

/** The text an `_info` family stands for: its version, its model… */
function infoText(serie: DeviceSerie): string {
	for (const key of INFO_KEYS) if (serie.labels[key]) return serie.labels[key];
	const rest = Object.entries(serie.labels).find(([key, value]) => !COMMON.has(key) && value);
	return rest ? rest[1] : '';
}

// --- Row of figures --------------------------------------------------------------

/**
 * One figure of the row. `mode` says how to read `value`:
 * - `single`: the only series' last value;
 * - `max` / `min`: the worst of `count` series (by `better`);
 * - `bool`: 1 or 0, read as yes / no;
 * - `ratio`: `value` of `count` series are at 1 (links up, units healthy);
 * - `state`: `text` is the declared meaning of the code, `tone` its tone;
 * - `info`: `text` is a label value (a version).
 */
export interface StatFigure {
	name: string;
	label: string;
	unit: string;
	mode: 'single' | 'max' | 'min' | 'bool' | 'ratio' | 'state' | 'info';
	value: number | null;
	count: number;
	text: string;
	tone: string;
}

function statOf(metric: MetricDescription, group: DeviceMetricGroup, rated: ReadonlySet<string>): StatFigure | null {
	const series = group.series.filter((s) => last(s) !== null);
	if (series.length === 0) return null;
	const base = {
		name: group.name,
		label: metric.label || group.title,
		unit: displayUnit(metric, rated),
		value: null,
		count: series.length,
		text: '',
		tone: ''
	};
	if (metric.kind === 'info' || group.name.endsWith('_info')) {
		const text = series.map(infoText).find((t) => t) ?? '';
		return text ? { ...base, mode: 'info', text } : null;
	}
	const values = series.map((s) => last(s) as number);
	if (metric.kind === 'state' && metric.states.length > 0) {
		const code = values[0];
		const state = metric.states.find((s) => s.value === code);
		return { ...base, mode: 'state', value: code, text: state?.label ?? String(code), tone: state?.tone ?? 'neutral' };
	}
	if (metric.unit === 'bool') {
		if (values.length === 1) return { ...base, mode: 'bool', value: values[0] };
		return { ...base, mode: 'ratio', value: values.filter((v) => v >= 1).length };
	}
	if (values.length === 1) return { ...base, mode: 'single', value: values[0] };
	return metric.better === 'higher'
		? { ...base, mode: 'min', value: Math.min(...values) }
		: { ...base, mode: 'max', value: Math.max(...values) };
}

// --- Charts ----------------------------------------------------------------------

/** One entity of a ranking (`topn`, provisional `heatmap`). */
export interface RankRow {
	key: string;
	value: number;
}

export interface CatalogChart {
	key: string;
	title: string;
	view: string;
	unit: string;
	/** Curves for `timeseries` (and any view this page does not know). */
	series: DeviceSerie[];
	/** Entities, largest first, for `topn` and `heatmap`. */
	rows: RankRow[];
}

export interface CatalogLayout {
	stats: StatFigure[];
	charts: CatalogChart[];
}

/** Drops container-made interfaces when the device has a real one. */
function physicalOnly(series: DeviceSerie[]): DeviceSerie[] {
	const physical = (s: DeviceSerie) => !s.labels.ifname || !isVirtualInterface(s.labels.ifname);
	return series.some((s) => s.labels.ifname) && series.some(physical) ? series.filter(physical) : series;
}

function timeseries(members: { metric: MetricDescription; group: DeviceMetricGroup }[]): DeviceSerie[] {
	const several = members.length > 1;
	return members.flatMap(({ metric, group }) => {
		const series = physicalOnly(group.series);
		const name = metric.label || group.title;
		if (!several) return series;
		return series.map((s) => ({ ...s, label: series.length > 1 ? `${name} · ${s.label}` : name }));
	});
}

function ranking(
	view: ViewDescription,
	members: { metric: MetricDescription; group: DeviceMetricGroup }[]
): RankRow[] {
	// A heatmap ranks by the worst moment of the window, a top-N by the latest value.
	const peak = view.view === 'heatmap';
	// Counters of one entity add up (traffic in + out, errors of every kind); any
	// other family keeps its worst value (two pools named alike on two nodes).
	const add = members.every(({ metric }) => metric.kind === 'counter');
	const totals = new Map<string, number>();
	for (const { metric, group } of members) {
		for (const serie of physicalOnly(group.series)) {
			const value = peak ? Math.max(...serie.points.map((p) => p[1])) : last(serie);
			if (value === null || !Number.isFinite(value)) continue;
			const key =
				(view.group_by && serie.labels[view.group_by]) ||
				(metric.entity && serie.labels[metric.entity]) ||
				serie.label;
			const known = totals.get(key);
			totals.set(key, known === undefined ? value : add ? known + value : Math.max(known, value));
		}
	}
	const limit = Number(view.options.limit);
	return [...totals]
		.map(([key, value]) => ({ key, value }))
		.filter((row) => !peak || row.value > 0)
		.sort((a, b) => b.value - a.value || a.key.localeCompare(b.key, 'en'))
		.slice(0, Number.isInteger(limit) && limit > 0 ? limit : DEFAULT_LIMIT);
}

/**
 * The declared views filled with the device's families. `null` when the kind
 * declares none, or when the device has none of the declared families: the page
 * then keeps its generic layout. A view whose families the device does not have
 * is left out: a switch shows no UPS chart.
 */
export function catalogLayout(
	groups: DeviceMetricGroup[],
	collector: CollectorInfo | null | undefined
): CatalogLayout | null {
	if (!hasViews(collector)) return null;
	const rated = new Set(ratedFamilies(collector));
	const byName = new Map(groups.map((g) => [g.name, g]));
	const declared = new Map(collector.metrics.map((d) => [d.name, d]));
	const describe = (name: string): MetricDescription =>
		declared.get(name) ?? {
			name,
			label: '',
			unit: '',
			kind: 'gauge',
			better: '',
			min: null,
			max: null,
			max_from: '',
			entity: '',
			states: []
		};

	const stats: StatFigure[] = [];
	const charts: CatalogChart[] = [];
	const seen = new Set<string>();
	collector.views.forEach((view, index) => {
		const members = view.metrics
			.map((name) => ({ metric: describe(name), group: byName.get(fullName(name)) }))
			.filter((x): x is { metric: MetricDescription; group: DeviceMetricGroup } => x.group !== undefined);
		if (members.length === 0) return;
		if (!isChart(view)) {
			for (const { metric, group } of members) {
				if (seen.has(group.name)) continue;
				seen.add(group.name);
				const figure = statOf(metric, group, rated);
				if (figure) stats.push(figure);
			}
			return;
		}
		const unit = displayUnit(members[0].metric, rated) || members[0].group.unit;
		const known = view.view === 'topn' || view.view === 'heatmap';
		const chart: CatalogChart = {
			key: `${index}-${view.title}`,
			title: view.title || members[0].metric.label || members[0].group.title,
			view: view.view,
			unit,
			series: known ? [] : timeseries(members),
			rows: known ? ranking(view, members) : []
		};
		if (chart.series.length > 0 || known) charts.push(chart);
	});
	return stats.length > 0 || charts.length > 0 ? { stats, charts } : null;
}

/**
 * Display units and titles from the catalogue for every loaded family, so the
 * "All measurements" fold reads the same as the views (a rated counter in `B/s`).
 */
export function withCatalogUnits(
	groups: DeviceMetricGroup[],
	collector: CollectorInfo | null | undefined
): DeviceMetricGroup[] {
	if (!collector || collector.metrics.length === 0) return groups;
	const rated = new Set(ratedFamilies(collector));
	const declared = new Map(collector.metrics.map((d) => [fullName(d.name), d]));
	return groups.map((g) => {
		const metric = declared.get(g.name);
		if (!metric) return g;
		// No declared unit: keep the one guessed from the name (`err/s`), else the rate's `/s`.
		const unit = metric.unit ? displayUnit(metric, rated) : g.unit || displayUnit(metric, rated);
		return { ...g, title: metric.label || g.title, unit: unit === 'bool' ? '' : unit };
	});
}
