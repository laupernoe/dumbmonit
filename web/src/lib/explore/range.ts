/** Time ranges offered by the metrics explorer, in seconds. */
export const RANGE_SECONDS = [900, 3600, 6 * 3600, 24 * 3600, 7 * 86400, 30 * 86400] as const;

/** About 300 points per chart, like the device pages (see `metrics.ts`). */
export function stepFor(seconds: number): number {
	return Math.max(10, Math.round(seconds / 300));
}
