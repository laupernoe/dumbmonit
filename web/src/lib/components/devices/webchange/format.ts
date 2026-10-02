/**
 * Pure helpers for the website-changes panel: shortening a page's URL down to
 * something that fits a table cell, wording a collapsed run of unchanged
 * diff lines, and clamping the before/after slider's position.
 *
 * Pure module, no imports: `npm test` (tests/webchange.test.mjs) runs it as is.
 */

/** The path (+ query/hash) of a URL, without its scheme and host; `"/"` for the root. */
export function pagePath(url: string): string {
	try {
		const parsed = new URL(url);
		return `${parsed.pathname}${parsed.search}${parsed.hash}` || '/';
	} catch {
		// Not a parseable absolute URL (a relative path the server already sent
		// shortened, for instance): show it as is.
		return url || '/';
	}
}

/**
 * Shortens a path to `max` characters by eliding its middle, keeping the
 * start (what section it's under) and the end (the actual page) legible.
 * Short enough paths are returned unchanged.
 */
export function shortenPath(path: string, max = 40): string {
	if (path.length <= max) return path;
	const ellipsis = '…';
	const keep = max - ellipsis.length;
	const head = Math.ceil(keep * 0.6);
	const tail = keep - head;
	return `${path.slice(0, head)}${ellipsis}${path.slice(path.length - tail)}`;
}

/** Wording for a collapsed `skip` diff line: "⋯ 42 unchanged lines". */
export function skipLabel(count: number | null): string {
	const n = count ?? 0;
	return `⋯ ${n} unchanged ${n === 1 ? 'line' : 'lines'}`;
}

/** Clamps a slider position to the 0–100 range (percent from the left edge). */
export function clampPercent(value: number): number {
	if (Number.isNaN(value)) return 50;
	return Math.min(100, Math.max(0, value));
}

/**
 * Next slider position for an arrow-key press: ±`step` percent, Home/End to
 * the ends. Returns the unchanged value for any other key.
 */
export function stepPercent(current: number, key: string, step = 2): number {
	switch (key) {
		case 'ArrowLeft':
		case 'ArrowDown':
			return clampPercent(current - step);
		case 'ArrowRight':
		case 'ArrowUp':
			return clampPercent(current + step);
		case 'Home':
			return 0;
		case 'End':
			return 100;
		default:
			return current;
	}
}
