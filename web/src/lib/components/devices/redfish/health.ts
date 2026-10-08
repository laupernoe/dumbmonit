/**
 * Words for the Redfish panel. Every verdict here comes from the controller:
 * `Status.Health` (0 OK, 1 Warning, 2 Critical) or a reading compared by the
 * server against the thresholds the controller declares for that very sensor.
 * No temperature, speed or percentage is written in this file.
 */
import type { RedfishFan, RedfishLimit, RedfishOverview, RedfishTemperature } from '#lib/api/index.js';
import type { Tone } from '#lib/ui/index.js';
import { m } from '#lib/paraglide/messages.js';

export interface Verdict {
	tone: Tone;
	label: string;
}

/** A concern that pulls the server's health down, worst first. */
export interface Concern {
	severity: 1 | 2;
	text: string;
}

export function toneOf(health: number | null): Tone {
	if (health === null) return 'ghost';
	if (health >= 2) return 'warning';
	if (health >= 1) return 'advisory';
	return 'signal';
}

export type WordSet = 'component' | 'overall' | 'redundancy' | 'failable';

function wordsOf(set: WordSet): [string, string, string] {
	if (set === 'overall') return [m.devicesb_redfish_health_healthy(), m.devicesb_redfish_health_degraded(), m.devicesb_redfish_health_critical()];
	if (set === 'redundancy') return [m.devicesb_redfish_health_redundant(), m.devicesb_redfish_health_redundancy_degraded(), m.devicesb_redfish_health_redundancy_lost()];
	if (set === 'failable') return [m.devicesb_redfish_health_ok(), m.devicesb_redfish_health_degraded(), m.devicesb_redfish_health_failed()];
	return [m.devicesb_redfish_health_ok(), m.devicesb_redfish_health_degraded(), m.devicesb_redfish_health_critical()];
}

/** A component's own health, in words: "OK", "Degraded", "Critical". */
export function healthPlate(health: number | null, set: WordSet = 'component'): Verdict {
	if (health === null) return { tone: 'ghost', label: m.devicesb_redfish_health_unknown() };
	return { tone: toneOf(health), label: wordsOf(set)[Math.min(2, Math.max(0, Math.round(health)))] };
}

export function limitRank(limit: RedfishLimit | null): number {
	return limit === 'critical' ? 2 : limit === 'caution' ? 1 : 0;
}

export const celsius = (value: number | null) => (value === null ? '—' : `${Math.round(value * 10) / 10} °C`);
export const watts = (value: number | null) => (value === null ? '—' : `${Math.round(value)} W`);

export function fanSpeed(fan: RedfishFan): string {
	if (fan.rpm !== null) return `${Math.round(fan.rpm)} RPM`;
	if (fan.percent !== null) return `${Math.round(fan.percent)}%`;
	return '—';
}

export function fanFloor(fan: RedfishFan): string | null {
	if (fan.rpm !== null && fan.lower_critical_rpm !== null) return `${Math.round(fan.lower_critical_rpm)} RPM`;
	if (fan.rpm === null && fan.lower_critical_percent !== null) return `${Math.round(fan.lower_critical_percent)}%`;
	return null;
}

/**
 * A temperature's plate: its reading against its own thresholds. A sensor
 * that declares none gets no verdict of ours — only the controller's, when it
 * says something is wrong.
 */
export function temperatureVerdict(t: RedfishTemperature): Verdict | null {
	if (t.limit === 'critical') return { tone: 'warning', label: m.devicesb_redfish_health_above_critical() };
	if ((t.health ?? 0) >= 2) return { tone: 'warning', label: m.devicesb_redfish_health_critical() };
	if (t.limit === 'caution') return { tone: 'advisory', label: m.devicesb_redfish_health_near_limit() };
	if ((t.health ?? 0) >= 1) return { tone: 'advisory', label: m.devicesb_redfish_health_degraded() };
	if (t.limit === 'within') return { tone: 'signal', label: m.devicesb_redfish_health_within_limits() };
	return null;
}

export function fanVerdict(fan: RedfishFan): Verdict | null {
	if ((fan.health ?? 0) >= 2) return { tone: 'warning', label: m.devicesb_redfish_health_failed() };
	if (fan.limit === 'critical') return { tone: 'warning', label: m.devicesb_redfish_health_below_minimum() };
	if ((fan.health ?? 0) >= 1) return { tone: 'advisory', label: m.devicesb_redfish_health_degraded() };
	if (fan.limit === 'within') return { tone: 'signal', label: m.devicesb_redfish_health_within_limits() };
	if (fan.health === 0) return { tone: 'signal', label: m.devicesb_redfish_health_ok() };
	return null;
}

/** Several chassis or systems: their names become worth printing. */
export function where(list: { chassis?: string; system?: string }[]): boolean {
	return new Set(list.map((item) => item.chassis ?? item.system ?? '')).size > 1;
}

/**
 * Everything that pulls the server's health down, in words, worst first; and
 * the overall severity, which also honours the controller's own roll-ups.
 */
export function assess(view: RedfishOverview): { severity: number | null; concerns: Concern[] } {
	const concerns: Concern[] = [];
	const add = (severity: number | null, text: string) => {
		if (severity !== null && severity >= 1) concerns.push({ severity: severity >= 2 ? 2 : 1, text });
	};
	const crit = (h: number | null) => (h ?? 0) >= 2;

	for (const t of view.temperatures) {
		if (t.limit === 'critical') {
			add(2, m.devicesb_redfish_concern_temp_critical({ sensor: t.sensor, value: celsius(t.celsius), threshold: celsius(t.upper_critical_celsius) }));
		} else if (t.limit === 'caution') {
			add(1, m.devicesb_redfish_concern_temp_caution({ sensor: t.sensor, value: celsius(t.celsius), threshold: celsius(t.upper_caution_celsius) }));
		} else add(t.health, crit(t.health) ? m.devicesb_redfish_concern_sensor_critical({ sensor: t.sensor }) : m.devicesb_redfish_concern_sensor_degraded({ sensor: t.sensor }));
	}
	for (const f of view.fans) {
		if (crit(f.health)) add(2, m.devicesb_redfish_concern_fan_failed({ fan: f.fan, speed: fanSpeed(f) }));
		else if (f.limit === 'critical') add(2, m.devicesb_redfish_concern_fan_low({ fan: f.fan, speed: fanSpeed(f), floor: fanFloor(f) ?? '—' }));
		else add(f.health, m.devicesb_redfish_concern_fan_degraded({ fan: f.fan }));
	}
	for (const g of view.fan_redundancy) add(g.health, crit(g.health) ? m.devicesb_redfish_concern_fan_red_lost({ group: g.group }) : m.devicesb_redfish_concern_fan_red_degraded({ group: g.group }));
	for (const g of view.power_redundancy) add(g.health, crit(g.health) ? m.devicesb_redfish_concern_power_red_lost({ group: g.group }) : m.devicesb_redfish_concern_power_red_degraded({ group: g.group }));
	for (const p of view.power_supplies) add(p.health, crit(p.health) ? m.devicesb_redfish_concern_psu_failed({ psu: p.psu }) : m.devicesb_redfish_concern_psu_degraded({ psu: p.psu }));
	for (const d of view.drives) {
		if (d.failure_predicted) add(2, m.devicesb_redfish_concern_drive_predicted({ drive: d.drive }));
		else add(d.health, crit(d.health) ? m.devicesb_redfish_concern_drive_critical({ drive: d.drive }) : m.devicesb_redfish_concern_drive_degraded({ drive: d.drive }));
	}
	for (const s of view.storage) add(s.health, crit(s.health) ? m.devicesb_redfish_concern_storage_critical({ storage: s.storage }) : m.devicesb_redfish_concern_storage_degraded({ storage: s.storage }));
	for (const s of view.systems) {
		add(s.memory_health, crit(s.memory_health) ? m.devicesb_redfish_concern_memory_critical() : m.devicesb_redfish_concern_memory_degraded());
		add(s.processor_health, crit(s.processor_health) ? m.devicesb_redfish_concern_cpu_critical() : m.devicesb_redfish_concern_cpu_degraded());
	}
	for (const mgr of view.managers) add(mgr.health, crit(mgr.health) ? m.devicesb_redfish_concern_manager_critical({ id: mgr.id }) : m.devicesb_redfish_concern_manager_degraded({ id: mgr.id }));
	for (const v of view.voltages) add(v.health, crit(v.health) ? m.devicesb_redfish_concern_voltage_critical({ sensor: v.sensor }) : m.devicesb_redfish_concern_voltage_degraded({ sensor: v.sensor }));

	const healths = [
		...view.systems.flatMap((s) => [s.health, s.health_rollup]),
		...view.chassis.flatMap((c) => [c.health, c.health_rollup]),
		...concerns.map((c) => c.severity)
	].filter((h): h is number => h !== null);
	const severity = healths.length === 0 ? null : Math.max(...healths);

	// The controller's roll-up says more than the parts DumbMonit reads: say so
	// instead of leaving a red word unexplained.
	if (severity !== null && severity >= 1 && !concerns.some((c) => c.severity >= severity)) {
		const who = view.systems.find((s) => (s.health_rollup ?? s.health ?? 0) >= severity)?.id ??
			view.chassis.find((c) => (c.health_rollup ?? c.health ?? 0) >= severity)?.id ?? m.devicesb_redfish_concern_the_server();
		add(severity, severity >= 2 ? m.devicesb_redfish_concern_rollup_critical({ who }) : m.devicesb_redfish_concern_rollup_degraded({ who }));
	}
	concerns.sort((a, b) => b.severity - a.severity);
	return { severity, concerns };
}
