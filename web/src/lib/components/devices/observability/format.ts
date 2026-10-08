/**
 * Words and numbers for the log and metrics server panel. The server sends
 * the verdicts and raw values; only their presentation is decided here.
 */
import type { ObservabilityState, ObservabilityUnit } from '#lib/api/index.js';
import type { Tone } from '#lib/ui/index.js';
import { m } from '#lib/paraglide/messages.js';
import { formatBytes } from '../docker/api';
import { formatSpan } from '../pbs/format';

/** Product names per kind (proper nouns, not translated); an unknown kind keeps a neutral title. */
export const TITLES: Record<string, string> = {
	victoriametrics: 'VictoriaMetrics',
	victorialogs: 'VictoriaLogs',
	loki: 'Loki',
	graylog: 'Graylog',
	redis: 'Redis',
	mongodb: 'MongoDB',
	rabbitmq: 'RabbitMQ',
	crowdsec: 'CrowdSec',
	traefik: 'Traefik',
	caddy: 'Caddy',
	npm: 'Nginx Proxy Manager'
};

/** The product name of a kind, or a neutral word translated at call time. */
export function kindTitle(kind: string): string {
	if (kind === 'domain') return m.devicesb_observability_format_domain();
	return TITLES[kind] ?? m.devicesb_observability_format_server();
}

export function stateTone(state: ObservabilityState | null): Tone {
	if (state === 'warning') return 'warning';
	if (state === 'advisory') return 'advisory';
	if (state === 'ok') return 'signal';
	return 'ghost';
}

export function stateWord(state: ObservabilityState | null): string {
	if (state === 'warning') return m.devicesb_observability_format_failing();
	if (state === 'advisory') return m.devicesb_observability_format_attention();
	if (state === 'ok') return m.devicesb_observability_format_ok();
	return m.devicesb_observability_format_unknown();
}

/** A count with thin grouping: 12 480, 3.2 k/s stays readable at a glance. */
function compact(value: number): string {
	const abs = Math.abs(value);
	if (abs >= 1e9) return `${(value / 1e9).toFixed(1)} G`;
	if (abs >= 1e6) return `${(value / 1e6).toFixed(1)} M`;
	if (abs >= 1e4) return `${(value / 1e3).toFixed(1)} k`;
	if (abs >= 100 || Number.isInteger(value)) return String(Math.round(value));
	return value.toFixed(abs >= 10 ? 1 : 2);
}

export function formatValue(value: number | null, unit: ObservabilityUnit | string): string | null {
	if (value === null || !Number.isFinite(value)) return null;
	switch (unit) {
		case 'bytes':
			return formatBytes(value);
		case 'bytes_per_second':
			return `${formatBytes(value)}/s`;
		case 'per_second':
			return `${compact(value)}/s`;
		case 'percent':
			return `${value < 10 ? value.toFixed(1) : Math.round(value)}%`;
		case 'seconds':
			return formatSpan(value);
		default:
			return compact(value);
	}
}
