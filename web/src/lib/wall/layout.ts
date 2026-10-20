/**
 * Where the wall's widgets go: a zone each (top bar, bottom bar, right column,
 * or hidden) and an order within it. Mirrors
 * `crates/server/src/api/wall_widgets/layout.rs`: the server always sends a
 * complete layout, and these helpers keep one complete while it is edited.
 *
 * Pure (no Svelte, no Paraglide): tested by `tests/wall-layout.test.mjs`.
 */

export type WallWidgetId = 'weather' | 'news' | 'cve' | 'crypto' | 'stocks' | 'fx' | 'claude' | 'openai' | 'text';
export type WallZone = 'top' | 'bottom' | 'right' | 'hidden';

export interface WallLayout {
	top: WallWidgetId[];
	bottom: WallWidgetId[];
	right: WallWidgetId[];
	hidden: WallWidgetId[];
}

export const WALL_WIDGETS: readonly WallWidgetId[] = ['weather', 'news', 'cve', 'crypto', 'stocks', 'fx', 'claude', 'openai', 'text'];
export const WALL_ZONES: readonly WallZone[] = ['top', 'bottom', 'right', 'hidden'];

function defaultZone(id: WallWidgetId): WallZone {
	return id === 'weather' || id === 'news' || id === 'cve' || id === 'text' ? 'right' : 'bottom';
}

/**
 * A complete layout from whatever was stored or sent: unknown and repeated ids
 * dropped, every widget once, the missing ones at the end of their default zone.
 */
export function resolveLayout(raw: Partial<Record<WallZone, readonly string[]>> | null | undefined): WallLayout {
	const seen = new Set<WallWidgetId>();
	const keep = (list: readonly string[] | undefined): WallWidgetId[] => {
		const out: WallWidgetId[] = [];
		for (const value of list ?? []) {
			const id = String(value).trim().toLowerCase() as WallWidgetId;
			if (!WALL_WIDGETS.includes(id) || seen.has(id)) continue;
			seen.add(id);
			out.push(id);
		}
		return out;
	};
	const layout: WallLayout = { top: keep(raw?.top), bottom: keep(raw?.bottom), right: keep(raw?.right), hidden: keep(raw?.hidden) };
	for (const id of WALL_WIDGETS) if (!seen.has(id)) layout[defaultZone(id)].push(id);
	return layout;
}

export const DEFAULT_LAYOUT: WallLayout = resolveLayout(null);

export function zoneOf(layout: WallLayout, id: WallWidgetId): WallZone {
	return WALL_ZONES.find((zone) => layout[zone].includes(id)) ?? defaultZone(id);
}

/** The widget moved to the end of another zone. */
export function moveToZone(layout: WallLayout, id: WallWidgetId, zone: WallZone): WallLayout {
	const next = resolveLayout(layout);
	for (const z of WALL_ZONES) next[z] = next[z].filter((other) => other !== id);
	next[zone] = [...next[zone], id];
	return next;
}

/**
 * The widget dropped into a zone at a given place (0 = first; past the end =
 * last). The index counts the zone's widgets without the dragged one, so
 * dropping on the same spot changes nothing.
 */
export function placeAt(layout: WallLayout, id: WallWidgetId, zone: WallZone, index: number): WallLayout {
	const next = resolveLayout(layout);
	for (const z of WALL_ZONES) next[z] = next[z].filter((other) => other !== id);
	const list = [...next[zone]];
	list.splice(Math.max(0, Math.min(index, list.length)), 0, id);
	next[zone] = list;
	return next;
}

/**
 * Where a pointer dropped among the widgets of a zone: the number of widgets
 * whose middle (`centers`, along the zone's axis, dragged one excluded) lies
 * before the pointer.
 */
export function dropIndex(centers: readonly number[], pointer: number): number {
	let index = 0;
	for (const center of centers) if (pointer > center) index += 1;
	return index;
}

/** True when two layouts place every widget the same way. */
export function sameLayout(a: WallLayout, b: WallLayout): boolean {
	return WALL_ZONES.every((zone) => a[zone].join(',') === b[zone].join(','));
}

/** The widget one place earlier (`-1`) or later (`+1`) within its zone; unchanged at an end. */
export function shift(layout: WallLayout, id: WallWidgetId, delta: -1 | 1): WallLayout {
	const next = resolveLayout(layout);
	const zone = zoneOf(next, id);
	const list = [...next[zone]];
	const from = list.indexOf(id);
	const to = from + delta;
	if (from < 0 || to < 0 || to >= list.length) return next;
	[list[from], list[to]] = [list[to], list[from]];
	next[zone] = list;
	return next;
}

/**
 * One step of fitting a zone that does not fit: the widget showing the most
 * entries gives one up (the last such widget first, so the first ones keep
 * theirs longest), never below `floor`. Null when none can give one up.
 */
export function trimCaps(shown: readonly number[], floor = 1): number[] | null {
	let index = -1;
	for (let i = 0; i < shown.length; i += 1) if (shown[i] > floor && (index === -1 || shown[i] >= shown[index])) index = i;
	if (index === -1) return null;
	const next = [...shown];
	next[index] -= 1;
	return next;
}
