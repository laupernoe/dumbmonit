/**
 * Pure helpers of the status page editor: the order of the chosen services,
 * and whether the form differs from what is saved (to say "Unsaved changes").
 */

/** Moves `id` one step up (-1) or down (+1); out-of-range moves leave the order as is. */
export function moveId(order: number[], id: number, delta: -1 | 1): number[] {
	const index = order.indexOf(id);
	const target = index + delta;
	if (index < 0 || target < 0 || target >= order.length) return order;
	const next = [...order];
	[next[index], next[target]] = [next[target], next[index]];
	return next;
}

export interface ServicePick {
	label: string;
	group: string;
}

/** The services as they would be saved: in order, with trimmed label and group. */
export function savedItems(order: number[], picks: Map<number, ServicePick>) {
	return order
		.filter((id) => picks.has(id))
		.map((id) => {
			const pick = picks.get(id)!;
			return { target_id: id, label: pick.label.trim(), group_name: pick.group.trim() };
		});
}

/**
 * A comparable fingerprint of the form. Strings are trimmed the way they are
 * saved, so a trailing space does not count as a change.
 */
export function formFingerprint(fields: Record<string, unknown>): string {
	return JSON.stringify(fields, (_key, value) => (typeof value === 'string' ? value.trim() : value));
}
