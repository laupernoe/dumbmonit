/**
 * Types of the visual kit. `MetricDescription` mirrors the optional `metrics`
 * field that `GET /api/collectors` will carry per kind; until the server sends
 * it, every inference falls back to the metric name.
 */

/** Tone of a state or a threshold: always paired with a glyph and a word. */
export type Tone = 'signal' | 'info' | 'advisory' | 'warning' | 'ghost';

export type MetricKindName = 'gauge' | 'counter' | 'info' | 'state';

/** One value of a `state` family: 1 = "up", 2 = "down"… */
export interface StateMap {
	value: number;
	label: string;
	tone?: Tone;
}

/** What the catalogue says about one metric family (name without or with `dumbmonit_`). */
export interface MetricDescription {
	name: string;
	label?: string;
	/** `B`, `B/s`, `%`, `s`, `°C`, `count`, `bool`, `state`, `days`… */
	unit?: string;
	kind?: MetricKindName;
	/** Direction of a good figure, for delta colours and thresholds. */
	better?: 'higher' | 'lower' | '';
	min?: number | null;
	max?: number | null;
	/** Family that gives the maximum of each series (`if_speed_bps`, `fs_size_bytes`). */
	max_from?: string;
	/** Label that identifies the object: `ifname`, `disk`, `vm`, `pool`. */
	entity?: string;
	states?: StateMap[];
}

/** Views a catalogue can ask for. */
export type ViewKind =
	| 'stat'
	| 'gauge'
	| 'timeseries'
	| 'heatmap'
	| 'topn'
	| 'table'
	| 'treemap'
	| 'geo'
	| 'calendar'
	| 'timeline'
	| 'sankey'
	| 'state_grid';

/** What `defaultView` may answer: the catalogue views plus the ones only inferred. */
export type InferredView = ViewKind | 'capacity' | 'countdown' | 'keyvalue';

/** One cell of a heatmap or a state grid. `value` null = no data. */
export interface Cell {
	row: string;
	col: string;
	value: number | null;
}
