/**
 * Pins that sit too close on screen are drawn as one: a group pin with the
 * count and the worst state of its members. Recomputed at each zoom, by
 * distance in pixels — no library. Pure, tested in `tests/world-map.test.mjs`.
 */

export interface ScreenPoint {
	id: string;
	x: number;
	y: number;
	/** Lower is worse: the worst pin seeds its group. */
	rank: number;
}

/**
 * Groups of ids, worst seed first. Each group gathers the pins within
 * `radius` pixels of its seed (the worst pin not yet taken), so a group never
 * spreads further than its radius from the pin that names its state.
 */
export function clusterByDistance(points: readonly ScreenPoint[], radius: number): string[][] {
	const order = [...points].sort((a, b) => a.rank - b.rank || a.id.localeCompare(b.id));
	const taken = new Set<string>();
	const groups: string[][] = [];
	for (const seed of order) {
		if (taken.has(seed.id)) continue;
		const group = [seed.id];
		taken.add(seed.id);
		for (const other of order) {
			if (taken.has(other.id)) continue;
			if (Math.hypot(other.x - seed.x, other.y - seed.y) <= radius) {
				group.push(other.id);
				taken.add(other.id);
			}
		}
		groups.push(group);
	}
	return groups;
}
