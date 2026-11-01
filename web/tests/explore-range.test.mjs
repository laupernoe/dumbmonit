import { test } from 'node:test';
import assert from 'node:assert/strict';
import { RANGE_SECONDS, stepFor } from '../src/lib/explore/range.ts';

test('a range is sampled at about 300 points', () => {
	assert.equal(stepFor(3600), 12);
	assert.equal(stepFor(86400), 288);
	assert.equal(stepFor(60), 10);
});

test('the offered ranges are sorted', () => {
	assert.deepEqual([...RANGE_SECONDS], [...RANGE_SECONDS].sort((a, b) => a - b));
});
