/**
 * The UPS of a NUT server, folded from the stored `dumbmonit_nut_*` series.
 * The collector writes one series per UPS (label `ups`); here they become one
 * record each, with a state in words decided from the `ups.status` flags.
 */
import type { MetricSeries } from '#lib/api/index.js';
import type { Tone } from '#lib/ui/index.js';
import { m } from '#lib/paraglide/messages.js';

export interface UpsReading {
	name: string;
	description: string | null;
	manufacturer: string | null;
	model: string | null;
	status: string | null;
	flags: Set<string>;
	stale: boolean;
	driverConnected: boolean;
	charge: number | null;
	runtime: number | null;
	load: number | null;
	inputVoltage: number | null;
	batteryAge: number | null;
	batteryAgeSource: string | null;
	selfTestFailed: boolean | null;
	selfTestResult: string | null;
}

/** Metric (without prefix) → the `ups.status` flag it stands for. */
const FLAG_METRICS: Record<string, string> = {
	ups_on_line: 'OL',
	ups_on_battery: 'OB',
	ups_low_battery: 'LB',
	ups_replace_battery: 'RB',
	ups_charging: 'CHRG',
	ups_discharging: 'DISCHRG',
	ups_bypass: 'BYPASS',
	ups_calibrating: 'CAL',
	ups_off: 'OFF',
	ups_overload: 'OVER',
	ups_forced_shutdown: 'FSD',
	ups_alarm: 'ALARM'
};

export const NUT_QUERY_NAMES = [
	...Object.keys(FLAG_METRICS),
	'ups_info',
	'ups_status_info',
	'ups_data_stale',
	'ups_driver_connected',
	'battery_charge_percent',
	'battery_runtime_seconds',
	'ups_load_percent',
	'input_voltage_volts',
	'battery_age_seconds',
	'ups_self_test_failed'
];

function empty(name: string): UpsReading {
	return {
		name,
		description: null,
		manufacturer: null,
		model: null,
		status: null,
		flags: new Set(),
		stale: false,
		driverConnected: true,
		charge: null,
		runtime: null,
		load: null,
		inputVoltage: null,
		batteryAge: null,
		batteryAgeSource: null,
		selfTestFailed: null,
		selfTestResult: null
	};
}

export function foldUps(series: MetricSeries[]): UpsReading[] {
	const byName = new Map<string, UpsReading>();
	for (const serie of series) {
		const ups = serie.metric.ups;
		if (!ups) continue;
		const name = (serie.metric.__name__ ?? '').replace(/^dumbmonit_nut_/, '');
		const value = Number(serie.values.at(-1)?.[1]);
		if (!Number.isFinite(value)) continue;
		let r = byName.get(ups);
		if (!r) {
			r = empty(ups);
			byName.set(ups, r);
		}
		const flag = FLAG_METRICS[name];
		if (flag) {
			if (value >= 1) r.flags.add(flag);
			continue;
		}
		switch (name) {
			case 'ups_info':
				r.description = serie.metric.description || null;
				r.manufacturer = serie.metric.manufacturer || null;
				r.model = serie.metric.model || null;
				break;
			case 'ups_status_info':
				r.status = serie.metric.status || null;
				break;
			case 'ups_data_stale':
				r.stale = value >= 1;
				break;
			case 'ups_driver_connected':
				r.driverConnected = value >= 1;
				break;
			case 'battery_charge_percent':
				r.charge = value;
				break;
			case 'battery_runtime_seconds':
				r.runtime = value;
				break;
			case 'ups_load_percent':
				r.load = value;
				break;
			case 'input_voltage_volts':
				r.inputVoltage = value;
				break;
			case 'battery_age_seconds':
				r.batteryAge = value;
				r.batteryAgeSource = serie.metric.source || null;
				break;
			case 'ups_self_test_failed':
				r.selfTestFailed = value >= 1;
				r.selfTestResult = serie.metric.result || null;
				break;
		}
	}
	// Trouble first, then by name.
	return [...byName.values()].sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name, 'en'));
}

function rank(r: UpsReading): number {
	const s = verdict(r).tone;
	return s === 'warning' ? 0 : s === 'advisory' ? 1 : 2;
}

export interface Verdict {
	tone: Tone;
	word: string;
}

/** The one-word state of a UPS, the worst first. */
export function verdict(r: UpsReading): Verdict {
	if (r.stale) return { tone: 'advisory', word: r.driverConnected ? m.devicesb_nut_ups_state_no_fresh_data() : m.devicesb_nut_ups_state_driver_stopped() };
	const f = r.flags;
	if (f.has('FSD')) return { tone: 'warning', word: m.devicesb_nut_ups_state_shutting_down() };
	if (f.has('OB') && f.has('LB')) return { tone: 'warning', word: m.devicesb_nut_ups_state_battery_low() };
	if (f.has('OB')) return { tone: 'warning', word: m.devicesb_nut_ups_state_on_battery() };
	if (f.has('LB')) return { tone: 'warning', word: m.devicesb_nut_ups_state_battery_low() };
	if (f.has('OVER')) return { tone: 'warning', word: m.devicesb_nut_ups_state_overloaded() };
	if (f.has('OFF')) return { tone: 'warning', word: m.devicesb_nut_ups_state_output_off() };
	if (f.has('RB')) return { tone: 'advisory', word: m.devicesb_nut_ups_state_replace_battery() };
	if (f.has('BYPASS')) return { tone: 'advisory', word: m.devicesb_nut_ups_state_on_bypass() };
	if (f.has('ALARM')) return { tone: 'advisory', word: m.devicesb_nut_ups_state_alarm() };
	if (f.has('CAL')) return { tone: 'info', word: m.devicesb_nut_ups_state_calibrating() };
	if (f.has('OL')) return { tone: 'signal', word: f.has('CHRG') ? m.devicesb_nut_ups_state_on_mains_charging() : m.devicesb_nut_ups_state_on_mains() };
	return { tone: 'ghost', word: r.status ?? m.devicesb_nut_ups_state_unknown() };
}

/** Other flags worth a plate next to the main state. */
export function extraFlags(r: UpsReading): Verdict[] {
	const main = verdict(r).word;
	const out: Verdict[] = [];
	const add = (flag: string, tone: Tone, word: string) => {
		if (r.flags.has(flag) && word !== main) out.push({ tone, word });
	};
	add('RB', 'advisory', m.devicesb_nut_ups_state_replace_battery());
	add('BYPASS', 'advisory', m.devicesb_nut_ups_state_on_bypass());
	add('OVER', 'warning', m.devicesb_nut_ups_state_overloaded());
	add('ALARM', 'advisory', m.devicesb_nut_ups_state_alarm());
	if (r.selfTestFailed) out.push({ tone: 'advisory', word: m.devicesb_nut_ups_state_self_test_failed() });
	return out;
}

export function formatRuntime(seconds: number | null): string | null {
	if (seconds === null) return null;
	if (seconds < 60) return m.devicesb_nut_ups_format_seconds({ n: Math.round(seconds) });
	if (seconds < 3600) return m.devicesb_nut_ups_format_minutes({ n: Math.round(seconds / 60) });
	const h = Math.floor(seconds / 3600);
	const mins = Math.round((seconds % 3600) / 60);
	return mins === 0
		? m.devicesb_nut_ups_format_hours({ h })
		: m.devicesb_nut_ups_format_hours_minutes({ h, m: mins });
}

export function formatAge(seconds: number | null): string | null {
	if (seconds === null) return null;
	const days = seconds / 86400;
	if (days < 60) return m.devicesb_nut_ups_format_days({ n: Math.round(days) });
	const years = days / 365.25;
	if (years < 2) return m.devicesb_nut_ups_format_months({ n: Math.round(days / 30.44) });
	return m.devicesb_nut_ups_format_years({ n: years.toFixed(1) });
}
