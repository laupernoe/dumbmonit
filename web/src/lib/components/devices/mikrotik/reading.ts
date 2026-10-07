/**
 * Folds the stored MikroTik series (`dumbmonit_mikrotik_*`) into what the
 * panel shows. The router itself is never asked: these are the values the
 * last probe wrote, read back through the metrics API.
 */
import type { MetricSeries } from '#lib/api/index.js';

export interface Sensor {
	name: string;
	/** `temperature`, `voltage`, `fan`, `power`, `current` or `state`. */
	kind: string;
	value: number;
}

export interface Port {
	name: string;
	type: string;
	running: boolean | null;
	rxRate: number | null;
	txRate: number | null;
	/** Receive and transmit errors over the last 24 hours. */
	errors: number | null;
	/** Link losses over the last 24 hours. */
	linkDowns: number | null;
}

export interface Reading {
	version: string | null;
	release: string | null;
	board: string | null;
	identity: string | null;
	uptime: number | null;
	cpu: number | null;
	cpuCount: number | null;
	memoryUsed: number | null;
	memoryTotal: number | null;
	storageUsed: number | null;
	storageTotal: number | null;
	/** `null`: the router never checked (or the check failed). */
	updateChecked: boolean | null;
	update: { available: boolean; latest: string; channel: string | null } | null;
	routerboard: boolean | null;
	firmware: { pending: boolean; current: string; upgrade: string } | null;
	sensors: Sensor[];
	ports: Port[];
	skipped: number;
}

export const EMPTY: Reading = {
	version: null,
	release: null,
	board: null,
	identity: null,
	uptime: null,
	cpu: null,
	cpuCount: null,
	memoryUsed: null,
	memoryTotal: null,
	storageUsed: null,
	storageTotal: null,
	updateChecked: null,
	update: null,
	routerboard: null,
	firmware: null,
	sensors: [],
	ports: [],
	skipped: 0
};

const SENSOR_KIND: Record<string, string> = {
	temperature_celsius: 'temperature',
	voltage_volts: 'voltage',
	fan_rpm: 'fan',
	power_watts: 'power',
	current_amperes: 'current',
	health_ok: 'state'
};

/** Gauges and labels of the latest probe. */
export const GAUGES = (target: number) =>
	`{__name__=~"dumbmonit_mikrotik_(info|uptime_seconds|cpu_load_percent|cpu_count|memory_used_percent|memory_total_bytes|storage_used_percent|storage_total_bytes|update_checked|update_available|routerboard|firmware_upgrade_pending|temperature_celsius|voltage_volts|fan_rpm|power_watts|current_amperes|health_ok|interface_running|interfaces_skipped)", target="${target}"}`;

/** Per-interface traffic, bytes per second over five minutes. */
export const RATES = (target: number) =>
	`label_set(rate(dumbmonit_mikrotik_interface_rx_bytes_total{target="${target}"}[5m]), "dir", "rx") or label_set(rate(dumbmonit_mikrotik_interface_tx_bytes_total{target="${target}"}[5m]), "dir", "tx")`;

/** Errors and link losses over a day. `increase_prometheus`, as the rules do: a fresh series does not count its history. */
export const DAY = (target: number) =>
	`label_set(sum by (interface) (increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total{target="${target}"}[24h]) + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total{target="${target}"}[24h])), "what", "errors") or label_set(sum by (interface) (increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total{target="${target}"}[24h])), "what", "downs")`;

function last(serie: MetricSeries): number | null {
	const value = Number(serie.values.at(-1)?.[1]);
	return Number.isFinite(value) ? value : null;
}

export function fold(gauges: MetricSeries[], rates: MetricSeries[], day: MetricSeries[]): Reading {
	const out: Reading = { ...EMPTY, sensors: [], ports: [] };
	const ports = new Map<string, Port>();
	const port = (name: string, type?: string): Port => {
		let p = ports.get(name);
		if (!p) {
			p = { name, type: type ?? '', running: null, rxRate: null, txRate: null, errors: null, linkDowns: null };
			ports.set(name, p);
		}
		if (type && !p.type) p.type = type;
		return p;
	};
	for (const serie of gauges) {
		const m = serie.metric;
		const name = (m.__name__ ?? '').replace('dumbmonit_mikrotik_', '');
		const value = last(serie);
		if (value === null) continue;
		switch (name) {
			case 'info':
				out.version = m.version || null;
				out.release = m.release || null;
				out.board = m.board || null;
				out.identity = m.identity || null;
				break;
			case 'uptime_seconds':
				out.uptime = value;
				break;
			case 'cpu_load_percent':
				out.cpu = value;
				break;
			case 'cpu_count':
				out.cpuCount = value;
				break;
			case 'memory_used_percent':
				out.memoryUsed = value;
				break;
			case 'memory_total_bytes':
				out.memoryTotal = value;
				break;
			case 'storage_used_percent':
				out.storageUsed = value;
				break;
			case 'storage_total_bytes':
				out.storageTotal = value;
				break;
			case 'update_checked':
				out.updateChecked = value >= 1;
				break;
			case 'update_available':
				out.update = { available: value >= 1, latest: m.latest_version ?? '', channel: m.channel || null };
				break;
			case 'routerboard':
				out.routerboard = value >= 1;
				break;
			case 'firmware_upgrade_pending':
				out.firmware = { pending: value >= 1, current: m.current_firmware ?? '', upgrade: m.upgrade_firmware ?? '' };
				break;
			case 'interface_running':
				if (m.interface) port(m.interface, m.type).running = value >= 1;
				break;
			case 'interfaces_skipped':
				out.skipped = value;
				break;
			default: {
				const kind = SENSOR_KIND[name];
				if (kind && m.sensor) out.sensors.push({ name: m.sensor, kind, value });
			}
		}
	}
	for (const serie of rates) {
		const name = serie.metric.interface;
		const value = last(serie);
		if (!name || value === null) continue;
		if (serie.metric.dir === 'rx') port(name).rxRate = value;
		else if (serie.metric.dir === 'tx') port(name).txRate = value;
	}
	for (const serie of day) {
		const name = serie.metric.interface;
		const value = last(serie);
		if (!name || value === null || !ports.has(name)) continue;
		if (serie.metric.what === 'errors') port(name).errors = Math.round(value);
		else if (serie.metric.what === 'downs') port(name).linkDowns = Math.round(value);
	}
	// Failed states first, then by kind and name: the order an admin reads them in.
	const KIND_ORDER = ['state', 'temperature', 'fan', 'voltage', 'current', 'power'];
	out.sensors.sort(
		(a, b) =>
			Number(a.kind === 'state' ? a.value >= 1 : true) - Number(b.kind === 'state' ? b.value >= 1 : true) ||
			KIND_ORDER.indexOf(a.kind) - KIND_ORDER.indexOf(b.kind) ||
			a.name.localeCompare(b.name, 'en', { numeric: true })
	);
	out.ports = [...ports.values()].sort((a, b) => a.name.localeCompare(b.name, 'en', { numeric: true }));
	return out;
}

/** A sensor's value with its unit; states read as words elsewhere. */
export function sensorValue(sensor: Sensor): string {
	switch (sensor.kind) {
		case 'temperature':
			return `${Math.round(sensor.value)} °C`;
		case 'voltage':
			return `${sensor.value.toFixed(1)} V`;
		case 'fan':
			return `${Math.round(sensor.value).toLocaleString('en')} rpm`;
		case 'power':
			return `${sensor.value.toFixed(1)} W`;
		case 'current':
			return `${sensor.value.toFixed(2)} A`;
		default:
			return sensor.value >= 1 ? 'OK' : 'Failed';
	}
}

/** Bits per second, the way network people read a link. */
export function formatBits(bytesPerSecond: number | null): string {
	if (bytesPerSecond === null || !Number.isFinite(bytesPerSecond)) return '—';
	const bits = bytesPerSecond * 8;
	const units = ['bit/s', 'kbit/s', 'Mbit/s', 'Gbit/s', 'Tbit/s'];
	let i = 0;
	let value = bits;
	while (value >= 1000 && i < units.length - 1) {
		value /= 1000;
		i++;
	}
	return `${value < 10 && i > 0 ? value.toFixed(1) : Math.round(value)} ${units[i]}`;
}
