// Public status page: a service without data never counts as operational. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { tallyItems } from '../src/lib/components/status/tally.ts';

test('services without data are not known', () => {
	const t = tallyItems([{ state: 'unknown' }, { state: 'unknown' }]);
	assert.equal(t.total, 2);
	assert.equal(t.known, 0);
	assert.equal(t.up, 0);
});

test('known services are counted by state', () => {
	const t = tallyItems([{ state: 'up' }, { state: 'down' }, { state: 'unknown' }]);
	assert.deepEqual([t.known, t.up, t.down], [2, 1, 1]);
});
