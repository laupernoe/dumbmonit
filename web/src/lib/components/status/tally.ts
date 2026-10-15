/**
 * Counts of a public status page, shared by the banner and the tally line.
 * A service in the `unknown` state (disabled, never measured) has nothing to
 * say: it is left out of the "all operational" claim, never counted as up.
 */
import type { PublicItemState } from '#lib/api/index.js';

export interface TallyCounts {
	total: number;
	/** Services with a state worth reporting (everything but `unknown`). */
	known: number;
	up: number;
	down: number;
	degraded: number;
	maintenance: number;
}

export function tallyItems(items: readonly { state: PublicItemState }[]): TallyCounts {
	const count = (state: PublicItemState) => items.filter((item) => item.state === state).length;
	return {
		total: items.length,
		known: items.filter((item) => item.state !== 'unknown').length,
		up: count('up'),
		down: count('down'),
		degraded: count('degraded'),
		maintenance: count('maintenance')
	};
}
