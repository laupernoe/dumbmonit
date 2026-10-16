/**
 * Unsaved-changes tracking for forms with an explicit Save button. Pure
 * logic: a form is "dirty" when a canonical snapshot of its values differs
 * from the snapshot taken when it was loaded or last saved.
 */

/** Canonical JSON: object keys sorted, so key order never makes a form dirty. */
export function snapshot(value: unknown): string {
	return JSON.stringify(value, (_key, v) => {
		if (v && typeof v === 'object' && !Array.isArray(v)) {
			return Object.fromEntries(Object.entries(v as Record<string, unknown>).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
		}
		return v;
	});
}

/** True when `current` differs from the `baseline` snapshot. A missing baseline is never dirty. */
export function isDirty(baseline: string | null, current: unknown): boolean {
	return baseline !== null && snapshot(current) !== baseline;
}
