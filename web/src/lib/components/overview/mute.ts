/**
 * Muting a monitor from the Overview: the durations on offer, the check of a
 * picked end, and which maintenance window counts as "this monitor is muted".
 *
 * Pure on purpose (types only), so it is tested with `node --test`.
 */
import type { Silence } from '#lib/api/index.js';

/** Durations of the menu, in hours (the fourth choice, "Until…", is a date). */
export const MUTE_HOURS = [1, 8, 24] as const;

/** A mute that is shorter than this is a slip of the finger, not a mute. */
const MIN_AHEAD_MS = 60_000;

export function endAfterHours(hours: number, now: number = Date.now()): Date {
	return new Date(now + hours * 3600_000);
}

export type UntilCheck = { ok: true; end: Date } | { ok: false; problem: 'pick' | 'future' };

/** The value of a `datetime-local` input (local time), or why it cannot be an end. */
export function checkUntil(value: string, now: number = Date.now()): UntilCheck {
	const end = new Date(value);
	if (!value || Number.isNaN(end.getTime())) return { ok: false, problem: 'pick' };
	if (end.getTime() <= now + MIN_AHEAD_MS) return { ok: false, problem: 'future' };
	return { ok: true, end };
}

/**
 * The window muting this monitor right now: a one-off window on it alone (no
 * alert labels: that is a snooze of one alert) that the server reports as
 * active, or that this page just created (the server's clock may be a few
 * seconds behind the browser's). A weekly maintenance window is somebody's
 * schedule, not a mute: it is never offered to lift.
 */
export function mutedBy(silences: readonly Silence[], targetId: number, justCreated: readonly number[] = []): Silence | null {
	return (
		silences.find(
			(s) =>
				s.target_id === targetId &&
				s.enabled &&
				s.schedule.kind === 'once' &&
				Object.keys(s.matchers ?? {}).length === 0 &&
				(s.active_now || justCreated.includes(s.id))
		) ?? null
	);
}

/** When the window ends, as the server wrote it (parsed by the formatters). */
export function muteEnd(silence: Silence): string | null {
	if (silence.active_until) return silence.active_until;
	return silence.schedule.kind === 'once' ? silence.schedule.ends_at : null;
}
