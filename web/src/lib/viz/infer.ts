/**
 * The only place that guesses what a metric family is: unit, counter or gauge,
 * entity label, default view. Order of authority: catalogue, then name suffix,
 * then default.
 *
 * Names are emitted by the collectors WITHOUT the `dumbmonit_` prefix (the
 * server adds it), and the API returns them with it: every function here takes
 * both spellings.
 *
 * Pure module, no `#lib` import, so `node --test` runs it as is.
 */
import type { InferredView, MetricDescription, MetricKindName } from './types.ts';

export type CatalogIndex = Map<string, MetricDescription>;

const PREFIX = 'dumbmonit_';

/** `dumbmonit_if_octets_in` and `if_octets_in` are the same family. */
export function stripPrefix(name: string): string {
	return name.startsWith(PREFIX) ? name.slice(PREFIX.length) : name;
}

/** Indexes the catalogue's `metrics` by unprefixed name. Absent catalogue = empty index. */
export function indexCatalog(metrics: readonly MetricDescription[] | null | undefined): CatalogIndex {
	const index: CatalogIndex = new Map();
	for (const metric of metrics ?? []) index.set(stripPrefix(metric.name), metric);
	return index;
}

/**
 * Families that only ever grow, charted as a per-second rate. RE2-compatible
 * (VictoriaMetrics matches names with it): no lookahead, no backreference.
 *
 * A `_total` suffix is NOT a hint on its own: Proxmox, PBS, Ceph, AD and
 * Apache (`apache_connections_total`) totals are plain gauges. Each family
 * with real counters is therefore listed by prefix or by name, from what the
 * collectors emit as `MetricKind::Counter`.
 */
export const COUNTER_PATTERNS: readonly string[] = [
	// Interfaces (agent, SNMP profiles, OPNsense, agent perf counters).
	'.+_(octets|packets|errors|discards)_(in|out)',
	'disk_(read|written)_bytes|printer_pages_printed|ups_input_line_bads|agent_dropped_samples',
	// Observability stack.
	'(victoriametrics|victorialogs|loki|graylog)_.+_total',
	// Web servers, proxies, databases, brokers, security.
	'apache_(requests|sent_bytes)_total',
	'(caddy|crowdsec|influxdb|minio|mongodb|nginx|redis|rabbitmq|traefik)_.+_total',
	'mdaemon_(sessions|messages|filtered_messages)_total|push_received_total',
	// Network gear.
	'(mikrotik_interface|fortigate_interface|fortigate_ipsec_tunnel|pfsense_interface)_.+_total',
	'opnsense_(interface_(bytes|packets|errors)_(in|out)|interface_(drops|collisions)|vpn_bytes_(in|out)|mbuf_failures|unbound_queries)',
	'openvpn_client_bytes_(sent|received)',
	'agent_wireguard_peer_(rx|tx)_bytes|agent_ipsec_tunnel_bytes_(in|out)',
	// Orchestrators and hypervisors.
	'k8s_pod_restarts',
	'proxmox_guest_(network_(in|out)_bytes|disk_(read|write)_bytes|netdev_(in|out)_bytes)'
];

const COUNTER_RE = new RegExp(`^(?:${COUNTER_PATTERNS.join('|')})$`);

/** Body of the PromQL `__name__=~"…"` selector that matches every counter family. */
export const COUNTER_SELECTOR = `${PREFIX}(${COUNTER_PATTERNS.join('|')})`;

const STATE_SUFFIX = /_(up|running|ready|ok|healthy|connected|enabled|active|crashlooping|pending|blocked|reachable)$/;

/** Counter, gauge, state (0/1 or enumerated) or info (value 1, labels carry the data). */
export function inferKind(name: string, catalog?: CatalogIndex): MetricKindName {
	const bare = stripPrefix(name);
	const declared = catalog?.get(bare)?.kind;
	if (declared) return declared;
	if (COUNTER_RE.test(bare)) return 'counter';
	if (bare.endsWith('_info')) return 'info';
	if (STATE_SUFFIX.test(bare)) return 'state';
	return 'gauge';
}

export function isCounter(name: string, catalog?: CatalogIndex): boolean {
	return inferKind(name, catalog) === 'counter';
}

/** Unit of a raw (gauge) family, by Prometheus naming convention. */
function suffixUnit(bare: string): string {
	if (bare.endsWith('_percent')) return '%';
	if (bare.endsWith('_bytes')) return 'B';
	if (bare.endsWith('_bytes_per_second')) return 'B/s';
	if (bare.endsWith('_seconds')) return 's';
	if (bare.endsWith('_celsius')) return '°C';
	if (bare.endsWith('_volts')) return 'V';
	if (bare.endsWith('_watts')) return 'W';
	return '';
}

/** Unit of a counter once turned into a per-second rate. */
function rateUnit(bare: string): string {
	if (/_octets_|_bytes(_|$)/.test(bare)) return 'B/s';
	if (/_packets_/.test(bare)) return 'pkt/s';
	if (/_errors_/.test(bare)) return 'err/s';
	if (/_(discards|drops)(_|$)/.test(bare)) return 'pkt/s';
	if (/_pages_/.test(bare)) return 'pages/s';
	if (/_requests_/.test(bare)) return 'req/s';
	return '/s';
}

/** `B` + counter → `B/s`; `count`, `bool` and `state` carry no display unit. */
function fromCatalog(unit: string, counter: boolean): string {
	if (unit === 'count' || unit === 'bool' || unit === 'state') return counter ? '/s' : '';
	if (!counter || unit.includes('/')) return unit;
	return unit ? `${unit}/s` : '/s';
}

/**
 * Display unit of a family.
 *
 * `rated` (default true) says the family is charted as `rate()` when it is a
 * counter. A caller that charts counters raw passes `rated: false` and gets
 * the suffix unit, as before.
 */
export function unitFor(name: string, options: { catalog?: CatalogIndex; rated?: boolean } = {}): string {
	const { catalog, rated = true } = options;
	const bare = stripPrefix(name);
	const declared = catalog?.get(bare);
	const counter = inferKind(name, catalog) === 'counter';
	if (declared?.unit !== undefined && declared.unit !== '') {
		return fromCatalog(declared.unit, counter && rated);
	}
	if (counter && rated) return rateUnit(bare);
	return suffixUnit(bare);
}

/** Labels that identify the whole device: never an entity. */
const COMMON_LABELS = new Set(['__name__', 'target', 'host', 'instance', 'job', 'index']);
/** Label keys people know the series by, most telling first. */
const ENTITY_LABELS = [
	'mountpoint',
	'ifname',
	'interface',
	'container',
	'pod',
	'vm',
	'disk',
	'pool',
	'core',
	'unit',
	'device',
	'name'
];

/** Label that tells the series of a family apart; null when the family has one series. */
export function entityLabel(name: string, labelKeys: readonly string[], catalog?: CatalogIndex): string | null {
	const declared = catalog?.get(stripPrefix(name))?.entity;
	if (declared && labelKeys.includes(declared)) return declared;
	for (const key of ENTITY_LABELS) if (labelKeys.includes(key)) return key;
	const rest = labelKeys.filter((k) => !COMMON_LABELS.has(k) && !k.startsWith('tag_') && !k.endsWith('_id'));
	return rest[0] ?? null;
}

/** `foo_in` ↔ `foo_out` (also `…_bytes_in_total`): the other half of a mirrored chart, or null. */
export function mirrorPair(name: string): string | null {
	const swap = (s: string) => (s === 'in' ? 'out' : 'in');
	const m = /^(.*_)(in|out)(_total)?$/.exec(name);
	return m ? `${m[1]}${swap(m[2])}${m[3] ?? ''}` : null;
}

export interface ViewHints {
	unit?: string;
	/** Series (entities) present for the family. */
	entityCount: number;
	/** A maximum is known per series (catalogue `max_from`, or both bounds). */
	hasMax?: boolean;
}

/**
 * Default view of a family when the catalogue declares none. Mirrors the table
 * of the UI deep-dive: states as a grid, bounded values as gauges or bars,
 * many entities as a heatmap instead of a wall of curves.
 */
export function defaultView(name: string, hints: ViewHints, catalog?: CatalogIndex): InferredView {
	const bare = stripPrefix(name);
	const declared = catalog?.get(bare);
	const kind = inferKind(name, catalog);
	const unit = hints.unit ?? unitFor(name, { catalog });
	const many = hints.entityCount;
	if (kind === 'info') return 'keyvalue';
	if (kind === 'state') return many > 1 ? 'state_grid' : 'stat';
	if (/_expiry_days$|_days_left$|_days_remaining$/.test(bare)) return 'countdown';
	if ((hints.hasMax || declared?.max_from) && unit === 'B') return 'capacity';
	if (many > 8) return 'heatmap';
	if (unit === '%') return many <= 1 ? 'gauge' : 'topn';
	if (many > 1 && kind === 'counter') return 'topn';
	return 'timeseries';
}
