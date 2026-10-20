/**
 * Pure helpers for dismissing alerts ("Dismiss": this occurrence leaves every
 * list and count and is never notified; undoable). No DOM, no network.
 */

interface Dismissable {
	fingerprint: string;
	dismissed_at?: string | null;
}

/** The alerts without the given fingerprints (the optimistic removal). */
export function withoutFingerprints<T extends { fingerprint: string }>(
	alerts: readonly T[],
	fingerprints: Iterable<string>
): T[] {
	const gone = new Set(fingerprints);
	return gone.size === 0 ? [...alerts] : alerts.filter((a) => !gone.has(a.fingerprint));
}

/** Server timestamps are UTC without a suffix. */
function utcMillis(value: string | null | undefined): number {
	return value ? Date.parse(/(Z|[+-]\d\d:?\d\d)$/.test(value) ? value : `${value}Z`) : NaN;
}

/** The dismissed alerts, most recently dismissed first (unknown dates last). */
export function sortDismissed<T extends Dismissable>(alerts: readonly T[]): T[] {
	return [...alerts].sort((a, b) => {
		const x = utcMillis(a.dismissed_at);
		const y = utcMillis(b.dismissed_at);
		if (Number.isNaN(x) && Number.isNaN(y)) return 0;
		if (Number.isNaN(x)) return 1;
		if (Number.isNaN(y)) return -1;
		return y - x;
	});
}

/** Who dismissed, for display: an API token reads as its name. */
export function dismissedByName(by: string | null | undefined): string {
	if (!by) return '';
	return by.startsWith('token:') ? by.slice('token:'.length) : by;
}
