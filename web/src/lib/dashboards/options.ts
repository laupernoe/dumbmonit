/** Labelled choices for the range and refresh selectors (words follow the language). */
import { m } from '#lib/paraglide/messages.js';
import { RANGE_SECONDS, REFRESH_SECONDS } from './widgets.js';

const RANGE_LABEL: Record<number, () => string> = {
	900: () => m.dash_range_15m(),
	3600: () => m.dash_range_1h(),
	21600: () => m.dash_range_6h(),
	86400: () => m.dash_range_24h(),
	604800: () => m.dash_range_7d(),
	2592000: () => m.dash_range_30d()
};

const REFRESH_LABEL: Record<number, () => string> = {
	0: () => m.dash_refresh_off(),
	10: () => m.dash_refresh_10s(),
	30: () => m.dash_refresh_30s(),
	60: () => m.dash_refresh_1m(),
	300: () => m.dash_refresh_5m()
};

/** The offered ranges, plus the current one when the API holds another value. */
export function rangeOptions(current: number): { value: number; label: string }[] {
	const values: number[] = [...RANGE_SECONDS];
	if (!values.includes(current)) values.push(current);
	return values
		.sort((a, b) => a - b)
		.map((value) => ({ value, label: RANGE_LABEL[value]?.() ?? m.dash_range_custom({ hours: Math.round(value / 3600) }) }));
}

export function refreshOptions(current: number): { value: number; label: string }[] {
	const values: number[] = [...REFRESH_SECONDS];
	if (!values.includes(current)) values.push(current);
	return values
		.sort((a, b) => a - b)
		.map((value) => ({ value, label: REFRESH_LABEL[value]?.() ?? m.dash_refresh_custom({ seconds: value }) }));
}
