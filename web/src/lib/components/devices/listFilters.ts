/**
 * Filters of the devices list that live in the URL (`?q=nas&state=attention&kind=snmp&tag=env=prod`),
 * so a filtered view can be bookmarked or shared.
 *
 * A device's `tags` hold both the options of its type (`keyword`, `port`…) and
 * the free tags the user added: only the latter are offered as filters.
 */

export type StateFilter = 'all' | 'attention' | 'reporting' | 'disabled';

export interface ListFilters {
	search: string;
	segment: StateFilter;
	kind: string;
	/** `key=value`, all of them must match. */
	tags: string[];
}

const SEGMENTS: StateFilter[] = ['all', 'attention', 'reporting', 'disabled'];

export const EMPTY_FILTERS: ListFilters = { search: '', segment: 'all', kind: '', tags: [] };

export function parseFilters(params: {
	get(name: string): string | null;
	getAll(name: string): string[];
}): ListFilters {
	const segment = params.get('state');
	return {
		search: params.get('q') ?? '',
		segment: SEGMENTS.includes(segment as StateFilter) ? (segment as StateFilter) : 'all',
		kind: params.get('kind') ?? '',
		tags: params.getAll('tag').filter((t) => t.includes('='))
	};
}

/** The query string (with its `?`, or empty) that restores `filters`. */
export function serializeFilters(filters: ListFilters): string {
	const params = new URLSearchParams();
	if (filters.search.trim()) params.set('q', filters.search.trim());
	if (filters.segment !== 'all') params.set('state', filters.segment);
	if (filters.kind) params.set('kind', filters.kind);
	for (const tag of filters.tags) params.append('tag', tag);
	const text = params.toString();
	return text ? `?${text}` : '';
}

/** `key=value` entries of a device that are free tags, not options of its type. */
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
