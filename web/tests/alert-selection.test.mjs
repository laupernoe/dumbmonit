// Multi-selection of the active alerts list. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	toggleKey,
	setMany,
	selectionState,
	pruneSelection,
	eligibleItems,
	runBulk,
	afterRun
} from '../src/lib/components/alerts/selection.ts';

const item = (key, over = {}) => ({
	key,
	fingerprints: [key],
	acked: false,
	ruleId: 1,
	targetId: 1,
	...over
});

test('toggleKey flips and forces without mutating', () => {
	const a = new Set(['x']);
	assert.deepEqual([...toggleKey(a, 'y')], ['x', 'y']);
	assert.deepEqual([...toggleKey(a, 'x')], []);
	assert.deepEqual([...toggleKey(a, 'x', true)], ['x']);
	assert.equal(a.size, 1);
});

test('setMany and selectionState', () => {
	const keys = ['a', 'b', 'c'];
	assert.equal(selectionState(new Set(), keys), 'none');
	assert.equal(selectionState(new Set(), []), 'none');
	assert.equal(selectionState(new Set(['a']), keys), 'some');
	const all = setMany(new Set(), keys, true);
	assert.equal(selectionState(all, keys), 'all');
	assert.equal(selectionState(setMany(all, ['b'], false), keys), 'some');
	assert.equal(setMany(new Set(['z']), keys, true).has('z'), true);
});

test('pruneSelection drops vanished rows', () => {
	assert.deepEqual([...pruneSelection(new Set(['a', 'b']), ['b', 'c'])], ['b']);
});

test('eligibleItems per action', () => {
	const items = [
		item('a'),
		item('b', { acked: true, ruleId: 2 }),
		item('c', { ruleId: 1, targetId: 1 }),
		item('d', { ruleId: undefined }),
		item('e', { targetId: null })
	];
	const all = new Set(['a', 'b', 'c', 'd', 'e']);
	assert.deepEqual(eligibleItems('ack', items, all).map((i) => i.key), ['a', 'c', 'd', 'e']);
	assert.deepEqual(eligibleItems('unack', items, all).map((i) => i.key), ['b']);
	// a and c share the same (rule, device): ignored once; d and e are not ignorable
	assert.deepEqual(eligibleItems('ignore', items, all).map((i) => i.key), ['a', 'b']);
	assert.deepEqual(eligibleItems('ack', items, new Set(['c'])).map((i) => i.key), ['c']);
});

test('runBulk continues after a failure and reports it', async () => {
	const items = [item('a'), item('b'), item('c')];
	const calls = [];
	const result = await runBulk(items, async (i) => {
		calls.push(i.key);
		if (i.key === 'b') throw new Error('boom');
	});
	assert.deepEqual(calls, ['a', 'b', 'c']);
	assert.equal(result.done, 2);
	assert.deepEqual(result.failed, [{ key: 'b', message: 'boom' }]);
	assert.deepEqual([...afterRun(result)], ['b']);
});
