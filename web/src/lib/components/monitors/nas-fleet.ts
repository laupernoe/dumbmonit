/**
 * NAS fleet: the verdict of one Synology NAS and of the whole fleet, from what
 * the collector already reports (`synology_storage_health`,
 * `synology_update_available`, `synology_system_info`) and the probe state.
 * Pure, so every rule is tested without a browser.
 */
import type { TargetState } from '#lib/format.js';

export type NasLevel = 'ok' | 'warn' | 'bad' | 'unknown';
export type NasReason = 'ok' | 'down' | 'config' | 'storage' | 'disk' | 'update' | 'unknown';

export interface NasReadings {
	/** Probe state of the monitor. */
	state: TargetState;
	/** `synology_storage_health`: 0 normal, 1 to watch, 2 critical; `null` when not reported. */
	storageHealth: number | null;
	/** `synology_update_available`: `null` when DSM did not say. */
	updateAvailable: boolean | null;
}

export interface NasVerdict {
	level: NasLevel;
	reason: NasReason;
}

/** The worst thing wrong with a NAS decides: unreachable, then storage, then the rest. */
export function nasVerdict(readings: NasReadings): NasVerdict {
	const { state, storageHealth, updateAvailable } = readings;
	if (state === 'down' || state === 'offline') return { level: 'bad', reason: 'down' };
	if (state === 'misconfigured') return { level: 'warn', reason: 'config' };
	if (state !== 'online') return { level: 'unknown', reason: 'unknown' };
	if (storageHealth !== null && storageHealth >= 2) return { level: 'bad', reason: 'storage' };
	if (storageHealth !== null && storageHealth >= 1) return { level: 'warn', reason: 'disk' };
	if (updateAvailable) return { level: 'warn', reason: 'update' };
	return { level: 'ok', reason: 'ok' };
}

export interface FleetSummary {
	total: number;
	healthy: number;
	/** Worst level of the fleet, for the single light on the banner. */
	level: NasLevel;
}

const SEVERITY: Record<NasLevel, number> = { ok: 0, unknown: 1, warn: 2, bad: 3 };

export function summarizeFleet(levels: readonly NasLevel[]): FleetSummary {
	let worst: NasLevel = 'ok';
	for (const level of levels) if (SEVERITY[level] > SEVERITY[worst]) worst = level;
	return { total: levels.length, healthy: levels.filter((l) => l === 'ok').length, level: worst };
}
