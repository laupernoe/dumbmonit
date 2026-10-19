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

/** Active alerts of one monitor, by how much they ask of the reader. */
export interface TargetAlertCounts {
	/** Firing and nobody has taken it on yet. */
	firing: number;
	/** Firing but acknowledged: known, so shown quieter. */
	acked: number;
	/** Masked because the parent device is down: neutral. */
	suppressed: number;
	total: number;
}

/**
 * One pass over the shared alert list: per monitor, how many alerts are
 * active (firing, acknowledged, or masked by a down parent). Pending and
 * resolved ones are not active; info alerts never count, as in the nav badge.
 * Monitors without any active alert are absent from the map.
 */
export function countAlertsByTarget(
	alerts: readonly {
		target_id: number | null;
		effective_phase: string;
		acked: boolean;
		severity: AlertSeverity | string;
	}[]
): Map<number, TargetAlertCounts> {
	const out = new Map<number, TargetAlertCounts>();
	for (const alert of alerts) {
		if (alert.target_id === null || !isProblemSeverity(alert.severity)) continue;
		let kind: 'firing' | 'acked' | 'suppressed';
		if (alert.effective_phase === 'suppressed') kind = 'suppressed';
		else if (alert.effective_phase === 'firing') kind = alert.acked ? 'acked' : 'firing';
		else continue;
		const counts = out.get(alert.target_id) ?? { firing: 0, acked: 0, suppressed: 0, total: 0 };
		counts[kind] += 1;
		counts.total += 1;
		out.set(alert.target_id, counts);
	}
	return out;
}
