/**
 * The console's six states, their words and their marks. A state is always
 * shown as a word next to a shape — never by colour alone: up is a circle,
 * degraded a diamond, down a square, a configuration error a square outline,
 * waiting and paused a ring.
 */
import type { Target } from '#lib/api/index.js';
import type { TargetAlertCounts } from '#lib/alerts-attention.js';
import { displayState, type ProbeStatus } from '#lib/format.js';
import { m } from '#lib/paraglide/messages.js';

export type ConsoleStatus = 'down' | 'degraded' | 'config' | 'waiting' | 'up' | 'paused';

/** Severity order of the monitor table: worst first. */
export const STATUS_RANK: Record<ConsoleStatus, number> = {
	down: 0,
	degraded: 1,
	config: 2,
	waiting: 3,
	up: 4,
	paused: 5
};

export function consoleStatus(target: Target, probe: ProbeStatus | undefined, alerts?: TargetAlertCounts): ConsoleStatus {
	const state = displayState(target, probe);
	switch (state) {
		case 'down':
		case 'offline':
			return 'down';
		case 'misconfigured':
			return 'config';
		case 'disabled':
			return 'paused';
		case 'pending':
		case 'unknown':
			return 'waiting';
		case 'online':
			return alerts && alerts.firing > 0 ? 'degraded' : 'up';
	}
}

export function statusWord(status: ConsoleStatus): string {
	switch (status) {
		case 'down':
			return m.console_status_down();
		case 'degraded':
			return m.console_status_degraded();
		case 'config':
			return m.console_status_config();
		case 'waiting':
			return m.console_status_waiting();
		case 'up':
			return m.console_status_up();
		case 'paused':
			return m.console_status_paused();
	}
}

/** Text colour of the word. */
export const STATUS_TEXT: Record<ConsoleStatus, string> = {
	down: 'text-warning-ink',
	degraded: 'text-advisory-ink',
	config: 'text-advisory-ink',
	waiting: 'text-ink-2',
	up: 'text-signal-ink',
	paused: 'text-ink-3'
};

/** Shape and fill of the mark. */
export const STATUS_MARK: Record<ConsoleStatus, string> = {
	down: 'rounded-[2px] bg-warning',
	degraded: 'rotate-45 bg-advisory',
	config: 'rounded-[1px] border-[1.5px] border-advisory',
	waiting: 'rounded-full border-[1.5px] border-ink-3',
	up: 'rounded-full bg-signal',
	paused: 'rounded-full border-[1.5px] border-ink-3 opacity-60'
};

/** Stroke of the sparkline: quiet grey when all is well, the state's hue otherwise. */
export const STATUS_STROKE: Record<ConsoleStatus, string> = {
	down: 'stroke-warning',
	degraded: 'stroke-advisory',
	config: 'stroke-advisory',
	waiting: 'stroke-ink-3',
	up: 'stroke-ink-3',
	paused: 'stroke-ink-3'
};
