/**
 * Synology DSM read by the agent package (`crates/agent/src/collect/dsm.rs`):
 * folds the `agent_dsm_*` series of one machine, plus the fill of its
 * `/volumeN` mounts, its hottest sensor and its UPS (`nut_*`), into what the
 * NAS panel shows. Pure, so every rule is tested without a browser.
 */
import type { MetricSeries } from '#lib/api/index.js';

export interface DsmArray {
	name: string;
	level: string;
	/** `system` (md0), `swap` (md1) or `data`. */
	role: string;
	/** 0 normal, 1 syncing, 2 degraded (rebuild included), 3 inactive. */
	status: number;
	members: number | null;
	failed: string[];
	sync: { action: string; percent: number } | null;
}

export interface DsmVolume {
	mount: string;
	fstype: string;
	readOnly: boolean;
	usedPercent: number | null;
	usedBytes: number | null;
	totalBytes: number | null;
}

export interface DsmDisk {
	name: string;
	model: string;
	kind: string;
	sizeBytes: number | null;
	temperature: number | null;
	life: number | null;
	unc: number | null;
}

export interface DsmPackage {
	id: string;
	name: string;
	version: string;
	enabled: boolean;
	/** `null`: synopkg did not answer the agent, the state is unknown. */
	running: boolean | null;
	/** Should run (enabled) and is stopped. */
	down: boolean;
}

export interface DsmUps {
	name: string;
	model: string;
	onBattery: boolean;
	lowBattery: boolean;
	charge: number | null;
	runtime: number | null;
	load: number | null;
}

export interface DsmView {
	model: string;
	version: string;
	/** 0 normal, 1 to watch, 2 critical; `null` when not reported. */
	storageHealth: number | null;
	/** Hottest sensor of the machine, °C. */
	temperature: number | null;
	arrays: DsmArray[];
	volumes: DsmVolume[];
	disks: DsmDisk[];
	packages: DsmPackage[];
	services: { service: string; running: boolean }[];
	ups: DsmUps[];
}

/** The packages a NAS owner checks first, listed before the others. */
export const KEY_PACKAGES = [
	'SynologyDrive',
	'SynologyPhotos',
	'HyperBackup',
	'ActiveBackup',
	'ContainerManager',
	'Docker'
];

/** The query that feeds `foldDsm` for one machine. */
export function dsmQuery(target: number): string {
	const names =
		'agent_dsm_.+|agent_sensor_temperature_celsius|filesystem_(used_percent|used_bytes|total_bytes)' +
		'|nut_(ups_info|ups_on_battery|ups_low_battery|battery_charge_percent|battery_runtime_seconds|ups_load_percent)';
	// Series without a mount point match the empty alternative: only `/volumeN` mounts are added.
	return `{__name__=~"dumbmonit_(${names})", target="${target}", mountpoint=~"(/volume[0-9]+)?"}`;
}

const VOLUME = /^\/volume\d+$/;

function numberOf(serie: MetricSeries): number | null {
	const value = Number(serie.values?.at(-1)?.[1]);
	return Number.isFinite(value) ? value : null;
}

function natural(a: string, b: string): number {
	return a.localeCompare(b, 'en', { numeric: true });
}

export function foldDsm(series: MetricSeries[]): DsmView | null {
	let seen = false;
	const view: DsmView = {
		model: '',
		version: '',
		storageHealth: null,
		temperature: null,
		arrays: [],
		volumes: [],
		disks: [],
		packages: [],
		services: [],
		ups: []
	};
	const arrays = new Map<string, DsmArray>();
	const volumes = new Map<string, DsmVolume>();
	const disks = new Map<string, DsmDisk>();
	const packages = new Map<string, DsmPackage>();
	const ups = new Map<string, DsmUps>();

	const array = (name: string) => {
		let a = arrays.get(name);
		if (!a) {
			a = { name, level: '', role: 'data', status: 0, members: null, failed: [], sync: null };
			arrays.set(name, a);
		}
		return a;
	};
	const volume = (mount: string) => {
		let v = volumes.get(mount);
		if (!v) {
			v = { mount, fstype: '', readOnly: false, usedPercent: null, usedBytes: null, totalBytes: null };
			volumes.set(mount, v);
		}
		return v;
	};
	const disk = (name: string) => {
		let d = disks.get(name);
		if (!d) {
			d = { name, model: '', kind: '', sizeBytes: null, temperature: null, life: null, unc: null };
			disks.set(name, d);
		}
		return d;
	};
	const pkg = (id: string) => {
		let p = packages.get(id);
		if (!p) {
			p = { id, name: id, version: '', enabled: false, running: null, down: false };
			packages.set(id, p);
		}
		return p;
	};
	const unit = (name: string) => {
		let u = ups.get(name);
		if (!u) {
			u = { name, model: '', onBattery: false, lowBattery: false, charge: null, runtime: null, load: null };
			ups.set(name, u);
		}
		return u;
	};

	for (const serie of series) {
		const labels = serie.metric ?? {};
		const name = (labels.__name__ ?? '').replace(/^dumbmonit_/, '');
		const value = numberOf(serie);
		if (value === null) continue;
		switch (name) {
			case 'agent_dsm_info':
				seen = true;
				view.model = labels.model ?? '';
				view.version = labels.dsm_version ?? '';
				break;
			case 'agent_dsm_storage_health':
				view.storageHealth = value;
				break;
			case 'agent_dsm_raid_status':
				if (labels.array) {
					const a = array(labels.array);
					a.status = value;
					a.level = labels.level ?? '';
					a.role = labels.role ?? 'data';
				}
				break;
			case 'agent_dsm_raid_members':
				if (labels.array) array(labels.array).members = value;
				break;
			case 'agent_dsm_raid_failed_member_info':
				if (labels.array && labels.member) array(labels.array).failed.push(labels.member);
				break;
			case 'agent_dsm_raid_sync_percent':
				if (labels.array) array(labels.array).sync = { action: labels.action ?? '', percent: value };
				break;
			case 'agent_dsm_volume_read_only':
				if (labels.volume) {
					const v = volume(labels.volume);
					v.readOnly = value >= 1;
					v.fstype = labels.fstype ?? v.fstype;
				}
				break;
			case 'filesystem_used_percent':
			case 'filesystem_used_bytes':
			case 'filesystem_total_bytes':
				if (labels.mountpoint && VOLUME.test(labels.mountpoint)) {
					const v = volume(labels.mountpoint);
					if (!v.fstype) v.fstype = labels.fstype ?? '';
					if (name === 'filesystem_used_percent') v.usedPercent = value;
					else if (name === 'filesystem_used_bytes') v.usedBytes = value;
					else v.totalBytes = value;
				}
				break;
			case 'agent_dsm_disk_info':
				if (labels.disk) {
					const d = disk(labels.disk);
					d.model = labels.model ?? '';
					d.kind = labels.kind ?? '';
				}
				break;
			case 'agent_dsm_disk_size_bytes':
				if (labels.disk) disk(labels.disk).sizeBytes = value;
				break;
			case 'agent_dsm_disk_temperature_celsius':
				if (labels.disk) disk(labels.disk).temperature = value;
				break;
			case 'agent_dsm_disk_remaining_life_percent':
				if (labels.disk) disk(labels.disk).life = value;
				break;
			case 'agent_dsm_disk_unc_count':
				if (labels.disk) disk(labels.disk).unc = value;
				break;
			case 'agent_dsm_package_info':
				if (labels.package) {
					const p = pkg(labels.package);
					p.name = labels.name || labels.package;
					p.version = labels.version ?? '';
				}
				break;
			case 'agent_dsm_package_enabled':
				if (labels.package) pkg(labels.package).enabled = value >= 1;
				break;
			case 'agent_dsm_package_running':
				if (labels.package) pkg(labels.package).running = value >= 1;
				break;
			case 'agent_dsm_package_down':
				if (labels.package) pkg(labels.package).down = value >= 1;
				break;
			case 'agent_dsm_service_running':
				if (labels.service) view.services.push({ service: labels.service, running: value >= 1 });
				break;
			case 'agent_sensor_temperature_celsius':
				view.temperature = view.temperature === null ? value : Math.max(view.temperature, value);
				break;
			case 'nut_ups_info':
				if (labels.ups) unit(labels.ups).model = labels.model ?? '';
				break;
			case 'nut_ups_on_battery':
				if (labels.ups) unit(labels.ups).onBattery = value >= 1;
				break;
			case 'nut_ups_low_battery':
				if (labels.ups) unit(labels.ups).lowBattery = value >= 1;
				break;
			case 'nut_battery_charge_percent':
				if (labels.ups) unit(labels.ups).charge = value;
				break;
			case 'nut_battery_runtime_seconds':
				if (labels.ups) unit(labels.ups).runtime = value;
				break;
			case 'nut_ups_load_percent':
				if (labels.ups) unit(labels.ups).load = value;
				break;
		}
	}
	if (!seen) return null;

	// Data arrays first (where the user's files are), then the system ones.
	const roleRank = (role: string) => (role === 'data' ? 0 : role === 'system' ? 1 : 2);
	view.arrays = [...arrays.values()].sort((a, b) => roleRank(a.role) - roleRank(b.role) || natural(a.name, b.name));
	view.volumes = [...volumes.values()].sort((a, b) => natural(a.mount, b.mount));
	view.disks = [...disks.values()].sort((a, b) => natural(a.name, b.name));
	const keyRank = (id: string) => {
		const index = KEY_PACKAGES.indexOf(id);
		return index === -1 ? KEY_PACKAGES.length : index;
	};
	// A package that should run and does not comes first, then the key ones.
	view.packages = [...packages.values()].sort(
		(a, b) => Number(b.down) - Number(a.down) || keyRank(a.id) - keyRank(b.id) || natural(a.name, b.name)
	);
	view.services.sort((a, b) => a.service.localeCompare(b.service, 'en'));
	view.ups = [...ups.values()].sort((a, b) => natural(a.name, b.name));
	return view;
}
