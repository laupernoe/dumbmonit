/**
 * The subnet grid of a scheduled scan: one cell per address, from the
 * inventory the scans built. Pure: no network, no strings (the component
 * translates the states).
 */
import type { NetscanDevice } from '#lib/api/types.js';

/**
 * - `mac_changed`: a new device took the address of a known device under another MAC;
 * - `new`: not in the inventory before the last scan;
 * - `known` / `ignored`: a decision was taken;
 * - `absent`: a known device the last scan did not see;
 * - `free`: no device was ever seen at this address.
 */
export type GridState = 'mac_changed' | 'new' | 'known' | 'ignored' | 'absent' | 'free';

/** Order of the filters and of the legend: what needs a look first. */
export const GRID_STATES: GridState[] = ['mac_changed', 'new', 'known', 'ignored', 'absent', 'free'];

export interface GridCell {
	ip: string;
	/** Last octet: the short name of a tile. */
	label: string;
	state: GridState;
	device: NetscanDevice | null;
}

/** Same limit as the server (`check_network`): 1024 addresses. */
const MAX_ADDRESSES = 1024;

function parseIpv4(text: string): number | null {
	const parts = text.trim().split('.');
	if (parts.length !== 4) return null;
	let value = 0;
	for (const part of parts) {
		if (!/^\d{1,3}$/.test(part)) return null;
		const octet = Number(part);
		if (octet > 255) return null;
		value = value * 256 + octet;
	}
	return value;
}

function formatIpv4(value: number): string {
	return [24, 16, 8, 0].map((shift) => Math.floor(value / 2 ** shift) % 256).join('.');
}

/**
 * The addresses of an IPv4 network, in order. For /30 and wider the network and
 * broadcast addresses are left out. `null` for IPv6, a malformed range, or more
 * than 1024 addresses.
 */
export function subnetAddresses(cidr: string): string[] | null {
	const match = /^\s*([0-9.]+)\/(\d{1,2})\s*$/.exec(cidr);
	if (!match) return null;
	const base = parseIpv4(match[1]);
	const prefix = Number(match[2]);
	if (base === null || prefix < 0 || prefix > 32) return null;
	const size = 2 ** (32 - prefix);
	if (size > MAX_ADDRESSES) return null;
	const start = base - (base % size);
	const edges = prefix <= 30;
	const out: string[] = [];
	for (let offset = edges ? 1 : 0; offset < (edges ? size - 1 : size); offset++) out.push(formatIpv4(start + offset));
	return out;
}

/** Which device speaks for an address: a pending one first, then the latest seen. */
function pickDevice(candidates: NetscanDevice[]): NetscanDevice {
	return [...candidates].sort((a, b) => {
		if (a.known !== b.known) return a.known ? 1 : -1;
		return a.last_seen < b.last_seen ? 1 : a.last_seen > b.last_seen ? -1 : 0;
	})[0];
}

export function stateOf(device: NetscanDevice, lastRunAt: string | null, running: boolean): GridState {
	if (device.ignored) return 'ignored';
	if (!device.known) return device.mac_change_from ? 'mac_changed' : 'new';
	// Dates share one format (UTC, no suffix), so strings compare in order. While a
	// scan runs, last_seen of the devices it has not reached yet is still the old one.
	if (!running && lastRunAt && device.last_seen < lastRunAt) return 'absent';
	return 'known';
}

/**
 * One cell per address of the network. Devices whose address is outside the
 * network (they moved) are not drawn. `null` when no grid can be drawn.
 */
export function buildGrid(
	cidr: string,
	devices: NetscanDevice[],
	scheduleId: number,
	lastRunAt: string | null,
	running: boolean
): GridCell[] | null {
	const addresses = subnetAddresses(cidr);
	if (!addresses) return null;
	const byIp = new Map<string, NetscanDevice[]>();
	for (const device of devices) {
		if (device.schedule_id !== scheduleId) continue;
		const list = byIp.get(device.ip);
		if (list) list.push(device);
		else byIp.set(device.ip, [device]);
	}
	return addresses.map((ip) => {
		const found = byIp.get(ip);
		const device = found ? pickDevice(found) : null;
		return {
			ip,
			label: ip.slice(ip.lastIndexOf('.') + 1),
			state: device ? stateOf(device, lastRunAt, running) : 'free',
			device
		};
	});
}

/** Number of cells per state. */
export function countStates(cells: GridCell[]): Record<GridState, number> {
	const counts: Record<GridState, number> = { mac_changed: 0, new: 0, known: 0, ignored: 0, absent: 0, free: 0 };
	for (const cell of cells) counts[cell.state] += 1;
	return counts;
}
