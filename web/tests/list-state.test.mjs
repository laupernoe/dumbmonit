// URL state, sorting and bulk runner of the /targets list.
// Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	parseListQuery,
	runBulk,
	serializeListQuery,
	sortTargets,
	splitDisabled
} from '../src/lib/components/devices/listState.ts';

const t = (id, name, extra = {}) => ({
	id,
	name,
	kind: 'snmp',
	enabled: true,
	last_probe_at: null,
	...extra
});
const stateOf = (x) => (x.enabled ? 'online' : 'disabled');
const labelOf = (k) => k;

test('a pristine state serializes to nothing', () => {
	assert.equal(serializeListQuery(parseListQuery(new URLSearchParams(''))), '');
});

test('query round-trips and ignores junk values', () => {
	const q = parseListQuery(new URLSearchParams('q=nas&state=attention&kind=snmp&sort=name&dir=desc&view=tiles'));
	assert.deepEqual(q, { search: 'nas', segment: 'attention', kind: 'snmp', sort: 'name', dir: 'desc', view: 'tiles' });
	assert.equal(serializeListQuery(q), '?q=nas&state=attention&kind=snmp&sort=name&dir=desc&view=tiles');
	const junk = parseListQuery(new URLSearchParams('state=x&sort=y&view=z&dir=up'));
	assert.equal(junk.segment, 'all');
	assert.equal(junk.sort, 'default');
	assert.equal(junk.view, null);
	assert.equal(junk.dir, 'asc');
});

test('direction is only written for an explicit sort', () => {
	const q = parseListQuery(new URLSearchParams('dir=desc'));
	assert.equal(serializeListQuery(q), '');
});

test('sorts by name, both directions', () => {
	const list = [t(1, 'b'), t(2, 'A'), t(3, 'c')];
	assert.deepEqual(sortTargets(list, stateOf, labelOf, 'name', 'asc').map((r) => r.target.id), [2, 1, 3]);
	assert.deepEqual(sortTargets(list, stateOf, labelOf, 'name', 'desc').map((r) => r.target.id), [3, 1, 2]);
});

test('last check: most recent first, never-checked always last', () => {
	const list = [
		t(1, 'a'),
		t(2, 'b', { last_probe_at: '2026-01-01T00:00:00Z' }),
		t(3, 'c', { last_probe_at: '2026-02-01T00:00:00Z' })
	];
	assert.deepEqual(sortTargets(list, stateOf, labelOf, 'checked', 'asc').map((r) => r.target.id), [3, 2, 1]);
	assert.deepEqual(sortTargets(list, stateOf, labelOf, 'checked', 'desc').map((r) => r.target.id), [2, 3, 1]);
});

test('status sort puts attention first, kind sorts by label', () => {
	const list = [t(1, 'a', { kind: 'z' }), t(2, 'b', { enabled: false, kind: 'a' })];
	assert.deepEqual(sortTargets(list, stateOf, labelOf, 'status', 'asc').map((r) => r.target.id), [1, 2]);
	assert.deepEqual(sortTargets(list, stateOf, labelOf, 'kind', 'asc').map((r) => r.target.id), [2, 1]);
});

test('splits disabled from active', () => {
	const { active, disabled } = splitDisabled([t(1, 'a'), t(2, 'b', { enabled: false })], stateOf);
	assert.deepEqual([...active], [1]);
	assert.deepEqual([...disabled], [2]);
});

test('bulk runner keeps going after a failure and reports it', async () => {
	const out = await runBulk([t(1, 'a'), t(2, 'b'), t(3, 'c')], async (x) => {
		if (x.id === 2) throw new Error('boom');
	});
	assert.deepEqual(out.ok, [1, 3]);
	assert.equal(out.failed.length, 1);
	assert.equal(out.failed[0].name, 'b');
});
