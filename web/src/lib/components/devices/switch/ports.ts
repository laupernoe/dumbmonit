/**
 * One model of switch ports, whatever collector measured them.
 *
 * The switch panel draws a faceplate from `SwitchUnit`s. Each collector kind
 * that knows about ports has an adapter here (`SOURCES[kind]`): the queries
 * that read its stored series back, and the fold that turns them into units.
 * The device is never contacted when the page opens.
 *
 * - `snmp`: `switch_port_status` (every physical port, profile
 *   `ethernet-switch`), `if_speed_bps`, `ether_duplex_status`, `if_errors_*`,
 *   `poe_*` (POWER-ETHERNET-MIB), `port_pvid` + `bridge_port_info`
 *   (Q-BRIDGE-MIB), `lldp_*` and `cdp_neighbor_info`. Ports join on the
 *   `index` label, the ifIndex.
 * - `unifi`: `unifi_port_*` from `port_table` / `lldp_table`, one unit per
 *   switch or gateway (`mac`).
 * - `omada`, `arubacentral`: the same `<kind>_port_*` series as UniFi, read
 *   from the controller's REST API (the unit is the switch MAC, or its serial
 *   for Aruba Central). Field names of those two APIs are partly unverified.
 * - `mikrotik`: `mikrotik_interface_*` of the `ether` interfaces; RouterOS
 *   reports neither speed, PoE nor neighbours through the readings stored.
 *
 * Pure module (no Svelte, no network): tested with `node --test`.
 */
import type { MetricSeries } from '#lib/api/types.js';

export type Link = 'up' | 'down' | 'disabled';
export type PoeState = 'delivering' | 'searching' | 'off' | 'fault';
export type PortTone = 'signal' | 'advisory' | 'ghost' | 'info';

export interface Neighbor {
	name: string;
	port: string | null;
	protocol: 'lldp' | 'cdp';
}

export interface SwitchPort {
	/** Unique within its unit: the ifIndex, the UniFi port number, the interface name. */
	key: string;
	/** Short faceplate label ("1", "24", "ether3"). */
	label: string;
	/** Full name (ifName, port name). */
	name: string;
	/** Description set by the admin (ifAlias), when any. */
	alias: string | null;
	link: Link;
	speedBps: number | null;
	halfDuplex: boolean;
	poe: PoeState | null;
	poeWatts: number | null;
	/** IEEE class 0 to 4, when the switch says. */
	poeClass: number | null;
	/** Receive + transmit errors over the last 24 hours. */
	errors: number | null;
	neighbor: Neighbor | null;
	vlan: number | null;
	uplink: boolean;
}

export interface SwitchUnit {
	key: string;
	name: string;
	ports: SwitchPort[];
	/** Watts drawn and available, when the switch publishes its PoE budget. */
	poeBudget: { used: number; total: number } | null;
}

/** The queries of one kind, and how to read them back. */
export interface PortSource {
	/** Instant queries, answered in this order to `fold`. */
	instant(target: number): string[];
	fold(results: MetricSeries[][]): SwitchUnit[];
	/** Bits per second received and sent by one port. */
	traffic(target: number, unit: string, port: string): { in: string; out: string };
	/** One series per port of the unit (bits per second, the busier direction). */
	heatmap(target: number, unit: string): string;
	/** The port key a heatmap series belongs to. */
	heatKey(metric: Record<string, string>): string | null;
}

// --- helpers ------------------------------------------------------------------

/** A label value inside a PromQL string literal. */
export function quote(value: string): string {
	return `"${value.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
}

function last(serie: MetricSeries): number | null {
	const value = Number(serie.values.at(-1)?.[1]);
	return Number.isFinite(value) ? value : null;
}

function nameOf(serie: MetricSeries, prefix: string): string {
	const raw = serie.metric.__name__ ?? '';
	return raw.startsWith(prefix) ? raw.slice(prefix.length) : '';
}

function blank(key: string, name: string): SwitchPort {
	return {
		key,
		label: name,
		name,
		alias: null,
		link: 'down',
		speedBps: null,
		halfDuplex: false,
		poe: null,
		poeWatts: null,
		poeClass: null,
		errors: null,
		neighbor: null,
		vlan: null,
		uplink: false
	};
}

const byName = (a: SwitchPort, b: SwitchPort) => a.name.localeCompare(b.name, 'en', { numeric: true });

/**
 * Short faceplate labels: the trailing number of each name ("ge1" → "1",
 * "GigabitEthernet1/0/24" → "24"), unless two ports would then read the same,
 * in which case every port keeps its full name.
 */
export function shortLabels(ports: SwitchPort[]): void {
	const short = ports.map((p) => /(\d+)$/.exec(p.name)?.[1] ?? p.name);
	const unique = new Set(short).size === short.length;
	ports.forEach((p, i) => {
		p.label = unique ? short[i] : p.name;
	});
}

/** "1G", "100M", "2.5G", "10G": what is printed on the tile. */
export function shortSpeed(bps: number | null): string | null {
	if (bps === null || !Number.isFinite(bps) || bps <= 0) return null;
	if (bps >= 1e9) {
		const g = bps / 1e9;
		return `${Number.isInteger(g) ? g : Math.round(g * 10) / 10}G`;
	}
	if (bps >= 1e6) return `${Math.round(bps / 1e6)}M`;
	return `${Math.round(bps / 1e3)}k`;
}

/** Bits per second the way network people read a link. */
export function formatBitRate(bps: number | null): string {
	if (bps === null || !Number.isFinite(bps)) return '—';
	const units = ['bit/s', 'kbit/s', 'Mbit/s', 'Gbit/s', 'Tbit/s'];
	let value = bps;
	let i = 0;
	while (value >= 1000 && i < units.length - 1) {
		value /= 1000;
		i += 1;
	}
	return `${i === 0 || value >= 100 ? Math.round(value) : value.toFixed(1)} ${units[i]}`;
}

/**
 * The tone of a tile. Up and healthy reads as signal; up with errors, half
 * duplex or a PoE fault as advisory (hatched); no link as ghost; disabled by
 * the admin as info. The word and the glyph always come with it.
 */
export function portTone(port: SwitchPort): PortTone {
	if (port.link === 'disabled') return 'info';
	if (port.poe === 'fault') return 'advisory';
	if (port.link === 'down') return 'ghost';
	if ((port.errors ?? 0) > 0 || port.halfDuplex) return 'advisory';
	return 'signal';
}

/** Consecutive blocks of at most `size` ports, the way real switches group them. */
export function blocks<T>(ports: T[], size = 12): T[][] {
	const out: T[][] = [];
	for (let i = 0; i < ports.length; i += size) out.push(ports.slice(i, i + size));
	return out;
}

/** Up ports out of all the enabled ones. */
export function linkCount(ports: SwitchPort[]): { up: number; total: number } {
	const enabled = ports.filter((p) => p.link !== 'disabled');
	return { up: enabled.filter((p) => p.link === 'up').length, total: enabled.length };
}

/**
 * Heatmap rows, columns and cells from range series: one row per port (in
 * faceplate order), one column per sample time.
 */
export function heatmapCells(
	series: MetricSeries[],
	ports: SwitchPort[],
	keyOf: (metric: Record<string, string>) => string | null
): {
	rows: { key: string; label: string }[];
	cols: string[];
	cells: { row: string; col: string; value: number | null }[];
} {
	const known = new Set(ports.map((p) => p.key));
	const times = new Set<number>();
	const byKey = new Map<string, Map<number, number>>();
	for (const serie of series) {
		const key = keyOf(serie.metric);
		if (key === null || !known.has(key)) continue;
		const points = byKey.get(key) ?? new Map<number, number>();
		for (const [ts, raw] of serie.values) {
			const value = Number(raw);
			if (!Number.isFinite(value)) continue;
			times.add(ts);
			points.set(ts, Math.max(points.get(ts) ?? 0, value));
		}
		byKey.set(key, points);
	}
	const cols = [...times].sort((a, b) => a - b).map(String);
	const rows = ports.filter((p) => byKey.has(p.key)).map((p) => ({ key: p.key, label: p.name }));
	const cells: { row: string; col: string; value: number | null }[] = [];
	for (const row of rows) {
		const points = byKey.get(row.key);
		for (const col of cols) cells.push({ row: row.key, col, value: points?.get(Number(col)) ?? null });
	}
	return { rows, cols, cells };
}

// --- SNMP ---------------------------------------------------------------------

const SNMP = 'dumbmonit_';

const POE_DETECTION: Record<number, PoeState> = {
	1: 'off',
	2: 'searching',
	3: 'delivering',
	4: 'fault',
	5: 'searching',
	6: 'fault'
};

export function foldSnmp(state: MetricSeries[], day: MetricSeries[]): SwitchUnit[] {
	const all = new Map<string, SwitchPort>();
	const bridge = new Map<string, string>();
	const pvid = new Map<string, number>();
	const localPorts = new Map<string, string[]>();
	const poeRows: { group: string; port: string; state: PoeState; klass: number | null }[] = [];
	const lldp: { local: string; neighbor: Neighbor }[] = [];
	const cdp: { ifIndex: string; neighbor: Neighbor }[] = [];
	let budget = 0;
	let consumed = 0;
	let hasBudget = false;
	const speed = new Map<string, number>();
	const half = new Set<string>();

	for (const serie of state) {
		const name = nameOf(serie, SNMP);
		const m = serie.metric;
		const index = m.index;
		const value = last(serie);
		if (!index || value === null) continue;
		switch (name) {
			case 'switch_port_status': {
				const port = blank(index, m.ifname || '');
				port.alias = m.ifalias?.trim() || null;
				port.link = m.admin === '2' ? 'disabled' : value === 1 ? 'up' : 'down';
				all.set(index, port);
				break;
			}
			case 'if_speed_bps':
				speed.set(index, value);
				break;
			case 'ether_duplex_status':
				if (value === 2) half.add(index);
				break;
			case 'bridge_port_info':
				bridge.set(index, String(Math.round(value)));
				break;
			case 'port_pvid':
				pvid.set(index, value);
				break;
			case 'lldp_local_port_info':
				localPorts.set(index, [m.port_id, m.port_desc].filter((v): v is string => Boolean(v)));
				break;
			case 'lldp_neighbor_info': {
				const local = index.split('.')[1];
				const remote = m.remote_sysname || m.remote_chassis;
				if (local && remote) {
					lldp.push({
						local,
						neighbor: { name: remote, port: m.remote_port || m.remote_port_desc || null, protocol: 'lldp' }
					});
				}
				break;
			}
			case 'cdp_neighbor_info': {
				const ifIndex = index.split('.')[0];
				if (ifIndex && m.remote_sysname) {
					cdp.push({ ifIndex, neighbor: { name: m.remote_sysname, port: m.remote_port || null, protocol: 'cdp' } });
				}
				break;
			}
			case 'poe_port_status': {
				const [group, port] = index.split('.');
				if (!group || !port) break;
				const detected = m.admin === '2' ? 'off' : (POE_DETECTION[value] ?? 'off');
				const klass = Number(m.class);
				poeRows.push({ group, port, state: detected, klass: Number.isFinite(klass) && klass >= 1 ? klass - 1 : null });
				break;
			}
			case 'poe_power_budget_watts':
				budget += value;
				hasBudget = true;
				break;
			case 'poe_power_consumed_watts':
				consumed += value;
				break;
		}
	}

	// Front-panel ports: the bridge ports when the switch has a bridge table
	// (it leaves out CPU and management interfaces), else every named port.
	const bridged = new Set(bridge.values());
	const ports = [...all.values()]
		.filter((p) => (bridged.size > 0 ? bridged.has(p.key) : p.name !== ''))
		.map((p) => (p.name ? p : { ...p, name: `#${p.key}` }))
		.sort((a, b) => Number(a.key) - Number(b.key) || byName(a, b));
	if (ports.length === 0) return [];
	const byIndex = new Map(ports.map((p) => [p.key, p]));

	for (const port of ports) {
		if (port.link === 'up') port.speedBps = speed.get(port.key) ?? null;
		port.halfDuplex = port.link === 'up' && half.has(port.key);
	}
	for (const serie of day) {
		const value = last(serie);
		const port = serie.metric.index ? byIndex.get(serie.metric.index) : undefined;
		if (port && value !== null) port.errors = Math.round(value);
	}
	for (const [base, vlan] of pvid) {
		const port = byIndex.get(bridge.get(base) ?? base);
		if (port) port.vlan = vlan;
	}
	// PoE: the RFC leaves the mapping to the vendor. With a single group, the
	// port number is the ifIndex on most small switches, else the n-th port.
	const groups = new Set(poeRows.map((r) => r.group));
	if (groups.size === 1) {
		for (const row of poeRows) {
			const port = byIndex.get(row.port) ?? ports[Number(row.port) - 1];
			if (!port) continue;
			port.poe = row.state;
			port.poeClass = row.state === 'delivering' ? row.klass : null;
		}
	}
	for (const { local, neighbor } of lldp) {
		const names = localPorts.get(local) ?? [];
		const port = ports.find((p) => names.includes(p.name)) ?? byIndex.get(local);
		if (port && !port.neighbor) port.neighbor = neighbor;
	}
	for (const { ifIndex, neighbor } of cdp) {
		const port = byIndex.get(ifIndex);
		if (port && !port.neighbor) port.neighbor = neighbor;
	}
	shortLabels(ports);
	return [
		{
			key: 'snmp',
			name: '',
			ports,
			poeBudget: hasBudget && budget > 0 ? { used: consumed, total: budget } : null
		}
	];
}

const snmp: PortSource = {
	instant: (target) => [
		`{__name__=~"dumbmonit_(switch_port_status|if_speed_bps|ether_duplex_status|bridge_port_info|port_pvid|lldp_local_port_info|lldp_neighbor_info|cdp_neighbor_info|poe_port_status|poe_power_budget_watts|poe_power_consumed_watts)", target="${target}"}`,
		`sum by (index) (increase_prometheus({__name__=~"dumbmonit_if_errors_(in|out)", target="${target}"}[24h]))`
	],
	fold: ([state = [], day = []]) => foldSnmp(state, day),
	traffic: (target, _unit, port) => ({
		in: `rate(dumbmonit_if_octets_in{target="${target}", index=${quote(port)}}[5m]) * 8`,
		out: `rate(dumbmonit_if_octets_out{target="${target}", index=${quote(port)}}[5m]) * 8`
	}),
	heatmap: (target) =>
		`max by (index) (rate({__name__=~"dumbmonit_if_octets_(in|out)", target="${target}"}[5m])) * 8`,
	heatKey: (metric) => metric.index ?? null
};

// --- UniFi --------------------------------------------------------------------

const UNIFI = 'dumbmonit_unifi_port_';

export function foldUnifi(
	state: MetricSeries[],
	day: MetricSeries[],
	prefix: string = UNIFI
): SwitchUnit[] {
	const units = new Map<string, SwitchUnit>();
	const readings = new Map<SwitchPort, Record<string, number>>();
	const port = (m: Record<string, string>): SwitchPort | null => {
		if (!m.mac || !m.port) return null;
		let unit = units.get(m.mac);
		if (!unit) {
			unit = { key: m.mac, name: m.device || m.mac, ports: [], poeBudget: null };
			units.set(m.mac, unit);
		}
		let p = unit.ports.find((x) => x.key === m.port);
		if (!p) {
			p = blank(m.port, m.port_name || m.port);
			p.label = m.port;
			unit.ports.push(p);
			readings.set(p, {});
		}
		return p;
	};
	for (const serie of state) {
		const name = nameOf(serie, prefix);
		const value = last(serie);
		const p = name && value !== null ? port(serie.metric) : null;
		if (!p || value === null) continue;
		readings.get(p)![name] = value;
		if (name === 'neighbor_info' && serie.metric.remote_chassis) {
			p.neighbor = {
				name: serie.metric.remote_chassis,
				port: serie.metric.remote_port || null,
				protocol: 'lldp'
			};
		}
	}
	// Derived once every reading of a port is known, whatever the series order.
	for (const [p, r] of readings) {
		p.link = r.enabled !== undefined && r.enabled < 1 ? 'disabled' : (r.up ?? 0) >= 1 ? 'up' : 'down';
		p.speedBps = p.link === 'up' ? (r.speed_bps ?? null) : null;
		p.halfDuplex = p.link === 'up' && r.full_duplex !== undefined && r.full_duplex < 1;
		p.uplink = (r.uplink ?? 0) >= 1;
		if (r.poe_enabled !== undefined) {
			p.poeWatts = r.poe_watts ?? null;
			p.poe = r.poe_enabled < 1 ? 'off' : (r.poe_watts ?? 0) > 0 ? 'delivering' : 'searching';
		}
	}
	for (const serie of day) {
		const value = last(serie);
		const unit = serie.metric.mac ? units.get(serie.metric.mac) : undefined;
		const p = unit?.ports.find((x) => x.key === serie.metric.port);
		if (p && value !== null) p.errors = Math.round(value);
	}
	const out = [...units.values()];
	for (const unit of out) unit.ports.sort((a, b) => Number(a.key) - Number(b.key));
	return out.sort((a, b) => a.name.localeCompare(b.name, 'en', { numeric: true }));
}

/** Sources that store `<kind>_port_*` series with the UniFi layout and labels. */
function cloudSource(kind: string): PortSource {
	const prefix = `dumbmonit_${kind}_port_`;
	return {
		instant: (target) => [
			`{__name__=~"dumbmonit_${kind}_port_(enabled|up|speed_bps|full_duplex|uplink|poe_enabled|poe_watts|neighbor_info)", target="${target}"}`,
			`sum by (mac, port) (increase_prometheus({__name__=~"dumbmonit_${kind}_port_(rx|tx)_errors_total", target="${target}"}[24h]))`
		],
		fold: ([state = [], day = []]) => foldUnifi(state, day, prefix),
		traffic: (target, unit, port) => ({
			in: `rate(dumbmonit_${kind}_port_rx_bytes_total{target="${target}", mac=${quote(unit)}, port=${quote(port)}}[5m]) * 8`,
			out: `rate(dumbmonit_${kind}_port_tx_bytes_total{target="${target}", mac=${quote(unit)}, port=${quote(port)}}[5m]) * 8`
		}),
		heatmap: (target, unit) =>
			`max by (port) (rate({__name__=~"dumbmonit_${kind}_port_(rx|tx)_bytes_total", target="${target}", mac=${quote(unit)}}[5m])) * 8`,
		heatKey: (metric) => metric.port ?? null
	};
}

const unifi = cloudSource('unifi');
const omada = cloudSource('omada');
const arubacentral = cloudSource('arubacentral');

// --- MikroTik -----------------------------------------------------------------

export function foldMikrotik(state: MetricSeries[], day: MetricSeries[]): SwitchUnit[] {
	const ports = new Map<string, SwitchPort>();
	for (const serie of state) {
		const m = serie.metric;
		const value = last(serie);
		if (m.__name__ !== 'dumbmonit_mikrotik_interface_running' || !m.interface || value === null) continue;
		const p = blank(m.interface, m.interface);
		p.link = value >= 1 ? 'up' : 'down';
		ports.set(m.interface, p);
	}
	for (const serie of day) {
		const value = last(serie);
		const p = serie.metric.interface ? ports.get(serie.metric.interface) : undefined;
		if (p && value !== null) p.errors = Math.round(value);
	}
	const list = [...ports.values()].sort(byName);
	if (list.length === 0) return [];
	shortLabels(list);
	return [{ key: 'mikrotik', name: '', ports: list, poeBudget: null }];
}

const mikrotik: PortSource = {
	instant: (target) => [
		`dumbmonit_mikrotik_interface_running{target="${target}", type="ether"}`,
		`sum by (interface) (increase_prometheus({__name__=~"dumbmonit_mikrotik_interface_(rx|tx)_errors_total", target="${target}", type="ether"}[24h]))`
	],
	fold: ([state = [], day = []]) => foldMikrotik(state, day),
	traffic: (target, _unit, port) => ({
		in: `rate(dumbmonit_mikrotik_interface_rx_bytes_total{target="${target}", interface=${quote(port)}}[5m]) * 8`,
		out: `rate(dumbmonit_mikrotik_interface_tx_bytes_total{target="${target}", interface=${quote(port)}}[5m]) * 8`
	}),
	heatmap: (target) =>
		`max by (interface) (rate({__name__=~"dumbmonit_mikrotik_interface_(rx|tx)_bytes_total", target="${target}", type="ether"}[5m])) * 8`,
	heatKey: (metric) => metric.interface ?? null
};

/** Kinds whose stored readings describe switch ports. */
export const SOURCES: Record<string, PortSource> = { snmp, unifi, omada, arubacentral, mikrotik };
