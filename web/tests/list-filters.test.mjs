// URL filters and tag filter of the devices list.
// Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	freeTags,
	hasAllTags,
	parseFilters,
	serializeFilters,
	tagCounts
} from '../src/lib/components/devices/listFilters.ts';

const none = new Set();

test('filters round-trip through the query string', () => {
	const filters = { search: 'nas', segment: 'attention', kind: 'http', tags: ['env=prod', 'team=a b'] };
	const query = serializeFilters(filters);
	assert.equal(query, '?q=nas&state=attention&kind=http&tag=env%3Dprod&tag=team%3Da+b');
	assert.deepEqual(parseFilters(new URLSearchParams(query)), filters);
	assert.equal(serializeFilters({ search: '', segment: 'all', kind: '', tags: [] }), '');
});

test('unknown or malformed parameters fall back to defaults', () => {
	const filters = parseFilters(new URLSearchParams('state=bogus&tag=nokey&tag=a%3Db'));
	assert.equal(filters.segment, 'all');
	assert.deepEqual(filters.tags, ['a=b']);
});

test('options of the type are not tags', () => {
	const tags = { keyword: 'ok', env: 'prod', empty: '' };
	assert.deepEqual(freeTags(tags, new Set(['keyword'])), ['env=prod']);
	assert.equal(hasAllTags(tags, ['env=prod'], new Set(['keyword'])), true);
	assert.equal(hasAllTags(tags, ['keyword=ok'], new Set(['keyword'])), false);
	assert.equal(hasAllTags(tags, ['env=prod', 'team=x'], none), false);
	assert.equal(hasAllTags(tags, [], none), true);
});

test('tag counts are sorted by use, then name', () => {
	const devices = [
		{ kind: 'http', tags: { env: 'prod' } },
		{ kind: 'http', tags: { env: 'prod', team: 'web' } },
		{ kind: 'ping', tags: { env: 'dev' } }
	];
	assert.deepEqual(tagCounts(devices, () => none), [
		['env=prod', 2],
		['env=dev', 1],
		['team=web', 1]
	]);
});
