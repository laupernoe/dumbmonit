// Unsaved-changes comparison of the settings forms. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { snapshot, isDirty } from '../src/lib/settings/dirty.ts';

test('dirty comparison ignores key order and needs a baseline', () => {
	const base = snapshot({ a: 1, b: { y: 2, x: [1, 2] } });
	assert.equal(isDirty(base, { b: { x: [1, 2], y: 2 }, a: 1 }), false);
	assert.equal(isDirty(base, { a: 1, b: { y: 2, x: [2, 1] } }), true);
	assert.equal(isDirty(null, { a: 2 }), false);
});
