/**
 * The /targets list state that lives in the URL (search, state segment, kind,
 * tags, sort, layout) so a filtered view can be linked and survives a reload, plus
 * the pure helpers behind sorting and the bulk actions. Defaults are never
 * written, so a pristine list keeps a clean `/targets`.
 */
import type { Target, TargetId } from '#lib/api/index.js';
import type { TargetState } from '#lib/format.js';
// Explicit extensions: also loaded straight by Node (`npm test`).
import { RANK, type RackRow } from './rack.ts';
import { asRackView, type RackView } from './view.ts';

export type Segment = 'all' | 'attention' | 'reporting' | 'disabled';
export const SEGMENTS: readonly Segment[] = ['all', 'attention', 'reporting', 'disabled'];

/** `default` is the rack order (folders, state tier, manual position, name). */
export type SortKey = 'default' | 'name' | 'status' | 'kind' | 'checked';
export const SORT_KEYS: readonly SortKey[] = ['default', 'name', 'status', 'kind', 'checked'];
export type SortDir = 'asc' | 'desc';

export interface ListQuery {
	search: string;
	segment: Segment;
	kind: string;
	/** `key=value` free tags, all of which a device must carry. */
	tags: string[];
	sort: SortKey;
	dir: SortDir;
	/** `null`: not in the URL, the viewer's remembered layout applies. */
	view: RackView | null;
}

export function parseListQuery(params: {
	get(name: string): string | null;
	getAll(name: string): string[];
}): ListQuery {
	const segment = params.get('state');
	const sort = params.get('sort');
	const view = params.get('view');
	return {
		search: params.get('q') ?? '',
		segment: (SEGMENTS as readonly (string | null)[]).includes(segment) ? (segment as Segment) : 'all',
		kind: params.get('kind') ?? '',
		tags: params.getAll('tag').filter((t) => t.includes('=')),
		sort: (SORT_KEYS as readonly (string | null)[]).includes(sort) ? (sort as SortKey) : 'default',
		dir: params.get('dir') === 'desc' ? 'desc' : 'asc',
		// `?view=rows` (retired layout) opens compact, the default.
		view: asRackView(view)
	};
}

/** Query string (with the leading `?`, or empty) for a state; defaults are left out. */
export function serializeListQuery(query: ListQuery): string {
	const params = new URLSearchParams();
	if (query.search.trim()) params.set('q', query.search.trim());
	if (query.segment !== 'all') params.set('state', query.segment);
	if (query.kind) params.set('kind', query.kind);
	for (const tag of query.tags) params.append('tag', tag);
	if (query.sort !== 'default') {
		params.set('sort', query.sort);
		if (query.dir === 'desc') params.set('dir', 'desc');
	}
	if (query.view) params.set('view', query.view);
	const text = params.toString();
	return text ? `?${text}` : '';
}

/**
 * A device's `tags` hold both the options of its type (`keyword`, `port`…) and
 * the free tags the user added; only the latter are tags for filtering.
 * Returned as `key=value`.
 */
export function freeTags(tags: Record<string, string>, optionKeys: ReadonlySet<string>): string[] {
	return Object.entries(tags)
		.filter(([key, value]) => !optionKeys.has(key) && value !== '')
		.map(([key, value]) => `${key}=${value}`);
}

export function hasAllTags(
	tags: Record<string, string>,
	wanted: readonly string[],
	optionKeys: ReadonlySet<string>
): boolean {
	if (wanted.length === 0) return true;
	const own = new Set(freeTags(tags, optionKeys));
	return wanted.every((tag) => own.has(tag));
}

/** Every free tag in use with its device count, most used first. */
export function tagCounts(
	devices: readonly { kind: string; tags: Record<string, string> }[],
	optionKeysOf: (kind: string) => ReadonlySet<string>
): [string, number][] {
	const counts = new Map<string, number>();
	for (const device of devices) {
		for (const tag of freeTags(device.tags, optionKeysOf(device.kind))) {
			counts.set(tag, (counts.get(tag) ?? 0) + 1);
		}
	}
	return [...counts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0], 'en'));
}

const byName = (a: Target, b: Target) => a.name.localeCompare(b.name, 'en', { sensitivity: 'base' });

/**
 * Flat, sorted rows for any explicit sort (no folders, no tree: a sorted
 * list is a flat one). `kindLabel` gives the label the kind column shows.
 * A missing last check always sorts last, whatever the direction.
 */
export function sortTargets(
	targets: Target[],
	stateOf: (target: Target) => TargetState,
	kindLabel: (kind: string) => string,
	sort: Exclude<SortKey, 'default'>,
	dir: SortDir
): RackRow[] {
	const sign = dir === 'desc' ? -1 : 1;
	const states = new Map(targets.map((t) => [t.id, stateOf(t)]));
	const when = (t: Target) => (t.last_probe_at ? Date.parse(t.last_probe_at) : NaN);
	const compare = (a: Target, b: Target): number => {
		switch (sort) {
			case 'name':
				return sign * byName(a, b);
			case 'kind': {
				const c = kindLabel(a.kind).localeCompare(kindLabel(b.kind), 'en', { sensitivity: 'base' });
				return c !== 0 ? sign * c : byName(a, b);
			}
			case 'status': {
				const c = RANK[states.get(a.id)!] - RANK[states.get(b.id)!];
				return c !== 0 ? sign * c : byName(a, b);
			}
			case 'checked': {
				const wa = when(a);
				const wb = when(b);
				if (Number.isNaN(wa) || Number.isNaN(wb)) {
					if (Number.isNaN(wa) && Number.isNaN(wb)) return byName(a, b);
					return Number.isNaN(wa) ? 1 : -1;
				}
				// Ascending: most recently checked first.
				return wa === wb ? byName(a, b) : sign * (wb - wa);
			}
		}
	};
	return targets
		.slice()
		.sort(compare)
		.map((target) => ({ target, state: states.get(target.id)!, depth: 0, shadowed: false }));
}

/** Splits ids into active and disabled, by the displayed state. */
export function splitDisabled(
	targets: Target[],
	stateOf: (target: Target) => TargetState
): { active: Set<TargetId>; disabled: Set<TargetId> } {
	const active = new Set<TargetId>();
	const disabled = new Set<TargetId>();
	for (const target of targets) (stateOf(target) === 'disabled' ? disabled : active).add(target.id);
	return { active, disabled };
}

export interface BulkOutcome {
	ok: TargetId[];
	failed: { id: TargetId; name: string; error: unknown }[];
}

/** Runs `action` over `targets` one after the other, collecting failures instead of stopping at the first. */
export async function runBulk(
	targets: Target[],
	action: (target: Target) => Promise<unknown>
): Promise<BulkOutcome> {
	const outcome: BulkOutcome = { ok: [], failed: [] };
	for (const target of targets) {
		try {
			await action(target);
			outcome.ok.push(target.id);
		} catch (error) {
			outcome.failed.push({ id: target.id, name: target.name, error });
		}
	}
	return outcome;
}
