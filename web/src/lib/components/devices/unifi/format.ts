/**
 * UniFi words and ordering. The codes are the collector's (`collectors/unifi/metrics.rs`):
 * `unifi_device_state` 0 online, 1 offline, 2 pending, 3 updating, 4 adopting,
 * 5 adoption failed, 6 isolated, 7 other.
 */
import type { Tone } from '$lib/ui';
import type { Reading } from '../instant';

export interface UnifiDevice {
	key: string;
	name: string;
	model: string;
	type: string;
	state: number | null;
	up: boolean | null;
	upgradable: boolean;
	firmware: string | null;
	uptime: number | null;
	cpu: number | null;
	memory: number | null;
	clients: number | null;
}

export interface UnifiView {
	version: string | null;
	api: string | null;
	internetUp: boolean | null;
	wanUp: boolean | null;
	latency: number | null;
	wanLinks: { device: string; wan: string; up: boolean }[];
	availability: { wan: string; percent: number }[];
	clients: number | null;
	clientsByType: { type: string; count: number }[];
	alarms: number | null;
	subsystems: { name: string; status: number }[];
	devices: UnifiDevice[];
	counts: Record<string, number>;
}

const STATES: [string, Tone][] = [
	['Online', 'signal'],
	['Offline', 'warning'],
	['Waiting for adoption', 'info'],
	['Updating', 'info'],
	['Adopting', 'info'],
	['Adoption failed', 'warning'],
	['Isolated', 'warning'],
	['Unknown', 'ghost']
];

export function stateWord(code: number | null): string {
	return code === null ? 'Unknown' : (STATES[code]?.[0] ?? 'Unknown');
}

export function stateTone(code: number | null): Tone {
	return code === null ? 'ghost' : (STATES[code]?.[1] ?? 'ghost');
}

export const TYPE_WORD: Record<string, string> = {
	gateway: 'Gateway',
	switch: 'Switch',
	access_point: 'Access point',
	other: 'Device'
};

const TYPE_RANK: Record<string, number> = { gateway: 0, switch: 1, access_point: 2, other: 3 };

/** Problems first (offline, isolated, failed), then waiting, then by type and name. */
function problemRank(device: UnifiDevice): number {
	if (device.state === 1 || device.state === 5 || device.state === 6) return 0;
	if (device.state === 2 || device.state === 4) return 1;
	return 2;
}

export function sortDevices(devices: UnifiDevice[]): UnifiDevice[] {
	return [...devices].sort(
		(a, b) =>
			problemRank(a) - problemRank(b) ||
			(TYPE_RANK[a.type] ?? 3) - (TYPE_RANK[b.type] ?? 3) ||
			a.name.localeCompare(b.name, 'en')
	);
}

export const SUBSYSTEM_WORD: Record<string, string> = {
	wlan: 'Wi-Fi',
	wan: 'WAN',
	www: 'Internet',
	lan: 'LAN',
	vpn: 'VPN'
};

export function emptyView(): UnifiView {
	return {
		version: null,
		api: null,
		internetUp: null,
		wanUp: null,
		latency: null,
		wanLinks: [],
		availability: [],
		clients: null,
		clientsByType: [],
		alarms: null,
		subsystems: [],
		devices: [],
		counts: {}
	};
}

export function fold(rows: Reading[]): UnifiView {
	const view = emptyView();
	const devices = new Map<string, UnifiDevice>();
	const device = (labels: Record<string, string>): UnifiDevice => {
		const key = labels.mac || labels.device || '';
		let found = devices.get(key);
		if (!found) {
			found = {
				key,
				name: labels.device || key,
				model: labels.model ?? '',
				type: labels.type ?? 'other',
				state: null,
				up: null,
				upgradable: false,
				firmware: null,
				uptime: null,
				cpu: null,
				memory: null,
				clients: null
			};
			devices.set(key, found);
		}
		return found;
	};
	for (const { name, labels, value } of rows) {
		switch (name) {
			case 'info':
				view.version = labels.version || null;
				view.api = labels.api || null;
				break;
			case 'internet_up':
				view.internetUp = value >= 1;
				break;
			case 'wan_up':
				view.wanUp = value >= 1;
				break;
			case 'internet_latency_seconds':
				view.latency = value;
				break;
			case 'wan_link_up':
				view.wanLinks.push({ device: labels.device ?? '', wan: labels.wan ?? '', up: value >= 1 });
				break;
			case 'wan_availability_percent':
				view.availability.push({ wan: labels.wan ?? '', percent: value });
				break;
			case 'clients':
				view.clients = value;
				break;
			case 'clients_by_type':
				view.clientsByType.push({ type: labels.type ?? '', count: value });
				break;
			case 'alarms':
				view.alarms = value;
				break;
			case 'subsystem_status':
				view.subsystems.push({ name: labels.subsystem ?? '', status: value });
				break;
			case 'devices':
				view.counts[labels.state ?? ''] = value;
				break;
			case 'device_state':
				device(labels).state = value;
				break;
			case 'device_up':
				device(labels).up = value >= 1;
				break;
			case 'device_upgradable': {
				const d = device(labels);
				d.upgradable = value >= 1;
				d.firmware = labels.firmware || null;
				break;
			}
			case 'device_uptime_seconds':
				device(labels).uptime = value;
				break;
			case 'device_cpu_percent':
				device(labels).cpu = value;
				break;
			case 'device_memory_percent':
				device(labels).memory = value;
				break;
			case 'device_clients':
				device(labels).clients = value;
				break;
		}
	}
	view.devices = sortDevices([...devices.values()]);
	view.wanLinks.sort((a, b) => a.wan.localeCompare(b.wan, 'en'));
	view.subsystems.sort((a, b) => a.name.localeCompare(b.name, 'en'));
	return view;
}
