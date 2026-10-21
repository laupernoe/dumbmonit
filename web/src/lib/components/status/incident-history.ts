/**
 * Entering a past incident (history taken over when DumbMonit is set up): the
 * dates are typed in the browser's local time (`datetime-local`) and sent as
 * UTC instants; the checks mirror the server's (`api/status_pages.rs`), so a
 * mistake is shown under the form instead of as a refused request. Pure
 * functions, run by the tests.
 */
import type { IncidentStatus } from '../../api/types';

/** An update typed in the form; `at` is a `datetime-local` value, empty: same as the previous one. */
export interface DraftUpdate {
	status: IncidentStatus;
	at: string;
	body: string;
}

export type HistoryError =
	| 'future_start'
	| 'future_end'
	| 'end_before_start'
	| 'update_empty'
	| 'update_outside'
	| 'update_future'
	| 'after_resolution';

/** The server accepts a browser clock this much ahead. */
const SKEW_MS = 5 * 60_000;

/** `datetime-local` value (local time) → instant; `null` when empty or invalid. */
export function localToDate(local: string): Date | null {
	if (!local) return null;
	const date = new Date(local);
	return Number.isNaN(date.getTime()) ? null : date;
}

/** `datetime-local` value → RFC 3339 in UTC (stored without suffix by the server). */
export function localToIso(local: string): string | undefined {
	return localToDate(local)?.toISOString();
}

/** Instant → `datetime-local` value in local time (for `max` and defaults). */
export function dateToLocal(date: Date): string {
	const pad = (n: number) => String(n).padStart(2, '0');
	return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

const isClosing = (status: IncidentStatus) => status === 'resolved' || status === 'completed';

/** Updates with their effective date (empty: the previous one, the start for the first), sorted in time. */
export function datedUpdates(startsAt: Date, updates: DraftUpdate[]): { status: IncidentStatus; body: string; at: Date }[] {
	let previous = startsAt;
	const dated = updates.map((u) => {
		const at = localToDate(u.at) ?? previous;
		previous = at;
		return { status: u.status, body: u.body.trim(), at };
	});
	return dated.map((u, i) => ({ u, i })).sort((a, b) => a.u.at.getTime() - b.u.at.getTime() || a.i - b.i).map(({ u }) => u);
}

/**
 * First problem of an incident typed in the form, `null` when it can be sent.
 * `startsAt` empty means "now"; `endsAt` empty means "still open".
 */
export function validateHistory(startsAt: string, endsAt: string, updates: DraftUpdate[], now: Date = new Date()): HistoryError | null {
	const limit = now.getTime() + SKEW_MS;
	const start = localToDate(startsAt) ?? now;
	const end = localToDate(endsAt);
	if (start.getTime() > limit) return 'future_start';
	if (end && end.getTime() > limit) return 'future_end';
	if (end && end.getTime() < start.getTime()) return 'end_before_start';
	if (updates.some((u) => !u.body.trim())) return 'update_empty';
	const dated = datedUpdates(start, updates);
	for (const u of dated) {
		if (u.at.getTime() < start.getTime() || (end && u.at.getTime() > end.getTime())) return 'update_outside';
		if (u.at.getTime() > limit) return 'update_future';
	}
	const closing = dated.findIndex((u) => isClosing(u.status));
	if (closing >= 0 && closing < dated.length - 1) return 'after_resolution';
	return null;
}

/** The `updates` array of the API payload, in the order typed (the server sorts). */
export function updatesPayload(updates: DraftUpdate[]) {
	return updates.map((u) => ({ status: u.status, body: u.body.trim(), at: localToIso(u.at) }));
}
