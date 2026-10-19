/**
 * Pure helpers of the firewall live map (`LiveMap.svelte`), tested in
 * `tests/live-map.test.mjs`.
 */

export type LatLon = [number, number];

/**
 * Points of a gentle arc from `a` to `b`, bulging to the left of the trip.
 * The shorter way round the globe: a trip across the date line goes over it
 * rather than across the whole map (Leaflet draws beyond ±180 happily).
 */
export function arcPoints(a: LatLon, b: LatLon, steps = 24): LatLon[] {
	let [lat1, lon1] = a;
	const [lat2, lon2] = b;
	if (lon2 - lon1 > 180) lon1 += 360;
	else if (lon1 - lon2 > 180) lon1 -= 360;
	const dLat = lat2 - lat1;
	const dLon = lon2 - lon1;
	const length = Math.hypot(dLat, dLon);
	if (length === 0) return [[lat1, lon1]];
	// Control point: the middle, pushed sideways by a fifth of the distance.
	const bend = length / 5;
	const cLat = (lat1 + lat2) / 2 + (dLon / length) * bend;
	const cLon = (lon1 + lon2) / 2 - (dLat / length) * bend;
	const points: LatLon[] = [];
	for (let i = 0; i <= steps; i++) {
		const t = i / steps;
		const u = 1 - t;
		points.push([u * u * lat1 + 2 * u * t * cLat + t * t * lat2, u * u * lon1 + 2 * u * t * cLon + t * t * lon2]);
	}
	return points;
}

/** Disc radius in pixels: grows with the square root of the count, within bounds. */
export function discRadius(count: number): number {
	return Math.min(26, 4 + 2.2 * Math.sqrt(Math.max(0, count)));
}

/** What most of a country's traffic was: `block` wins ties, it is what one looks for. */
export function dominant(c: { blocked: number; passed: number; other: number }): 'block' | 'pass' {
	return c.blocked >= c.passed + c.other && c.blocked > 0 ? 'block' : 'pass';
}

/** A country's name in the reader's language, the code itself when unknown. */
export function countryName(code: string, names: { of(code: string): string | undefined }): string {
	if (!/^[A-Z]{2}$/.test(code)) return code;
	try {
		return names.of(code) ?? code;
	} catch {
		return code;
	}
}
