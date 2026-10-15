/**
 * What counts as a problem in the nav badge and on the wall: warnings and
 * critical alerts. "Info" alerts (a container update is available…) stay in
 * the lists but never inflate a count or turn a badge red.
 */
import type { AlertSeverity } from '#lib/api/types.js';

export function isProblemSeverity(severity: AlertSeverity | string): boolean {
	return severity !== 'info';
}

/** Firing now, not suppressed by a parent, not acknowledged, and worth a count. */
export function countsAsProblem(alert: {
	effective_phase: string;
	acked: boolean;
	severity: AlertSeverity | string;
}): boolean {
	return alert.effective_phase === 'firing' && !alert.acked && isProblemSeverity(alert.severity);
}

/** The wall's "N problems": warnings, advisories, unreachable and misconfigured devices; never info. */
export function attentionCount(parts: {
	warnings: number;
	advisories: number;
	unreachable: number;
	misconfigured: number;
}): number {
	return parts.warnings + parts.advisories + parts.unreachable + parts.misconfigured;
}
