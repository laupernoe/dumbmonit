/**
 * Per-alert snooze: hide one alert for a while, without touching its device's
 * other rules.
 *
 * Deliberately dependency-free (only `import type`, erased at build time) so
 * it can be unit tested directly with `node --test`, the way
 * `rules-filter.ts` already is — see `web/tests/snooze.test.mjs`.
 */
import type { Alert, Silence, Target } from '#lib/api/index.js';

/** Snooze presets offered by `SnoozeControl`: a label and a duration in seconds. */
export const SNOOZE_DURATIONS: { label: string; secs: number }[] = [
	{ label: '1 h', secs: 3600 },
	{ label: '8 h', secs: 8 * 3600 },
	{ label: '1 day', secs: 24 * 3600 },
	{ label: 'Until resolved', secs: 30 * 24 * 3600 }
];

/**
 * Payload to snooze one specific alert, not its whole device: the window
 * carries the alert's own labels as matchers, so a sibling rule on the same
 * device keeps talking. Mirrors `Silence::matches` on the server (minus the
 * schedule check, which the server alone evaluates).
 */
export function snoozePayload(alert: Alert, target: Target | undefined, secs: number) {
	const now = new Date();
	const end = new Date(now.getTime() + secs * 1000);
	return {
		name: `Snooze · ${alert.rule_name || alert.rule_uid}${target ? ` · ${target.name}` : ''}`,
		target_id: alert.target_id,
		matchers: alert.labels,
		schedule: {
			kind: 'once' as const,
			starts_at: now.toISOString(),
			ends_at: end.toISOString()
		}
	};
}

/**
 * The maintenance window currently explaining why this alert is silenced, if
 * any — so the interface can show "time left" without re-implementing the
 * server's schedule arithmetic. Only windows the server already reports as
 * `active_now` are considered; this just narrows down to the one that
 * actually matches this alert's device and labels.
 */
export function coveringSilence(alert: Alert, silences: Silence[]): Silence | null {
	return (
		silences.find(
			(s) =>
				s.active_now &&
				(s.target_id === null || s.target_id === alert.target_id) &&
				Object.entries(s.matchers).every(([key, value]) => alert.labels[key] === value)
		) ?? null
	);
}
