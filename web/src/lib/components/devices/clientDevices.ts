/**
 * Client devices (phones, tablets, desktops) of any product that tracks who
 * connects and backs up through an account. Every such collector writes the
 * same metric family, `dumbmonit_client_device_*` (see
 * `crates/collectors/src/client_devices.rs`), so this one parser serves
 * every one of them: nothing here names a product.
 */
import type { MetricSeries } from '$lib/api';

export interface ClientDevice {
	/** Label `device`: the most descriptive id the collector could give it. */
	device: string;
	/** Label `type`: "Mobile", "Desktop", "Web"… free text from the product. */
	type: string;
	/** Label `os`: "iOS 17", "Android 14"… free text from the product. */
	os: string;
	/** Label `user`: the account the device is signed in as. */
	user: string;
	/** Label `kind`: which collector wrote this row (`immich`…). */
	kind: string;
	/** Unix seconds, or `null` if the product cannot report it for this device. */
	lastSeen: number | null;
	lastBackup: number | null;
	/** True once either signal (`connection` or `backup`) is past the target's `device_stale_days`. */
	stale: boolean;
}

const PREFIX = 'dumbmonit_client_device_';

export function buildClientDevices(series: MetricSeries[]): ClientDevice[] {
	const byDevice = new Map<string, ClientDevice>();
	for (const serie of series) {
		const name = serie.metric.__name__ ?? '';
		if (!name.startsWith(PREFIX)) continue;
		const device = serie.metric.device;
		if (!device) continue;
		const value = Number(serie.values.at(-1)?.[1]);
		if (!Number.isFinite(value)) continue;
		let row = byDevice.get(device);
		if (!row) {
			row = {
				device,
				type: serie.metric.type ?? '',
				os: serie.metric.os ?? '',
				user: serie.metric.user ?? '',
				kind: serie.metric.kind ?? '',
				lastSeen: null,
				lastBackup: null,
				stale: false
			};
			byDevice.set(device, row);
		}
		const family = name.slice(PREFIX.length);
		if (family === 'last_seen_timestamp_seconds') row.lastSeen = value;
		else if (family === 'last_backup_timestamp_seconds') row.lastBackup = value;
		else if (family === 'stale_seconds' && value > 0) row.stale = true;
	}
	return [...byDevice.values()].sort((a, b) => a.device.localeCompare(b.device));
}
