import test from 'node:test';
import assert from 'node:assert/strict';
import { rovingIndex, rovingTabindex } from '../src/lib/ui/roving.ts';

test('rovingIndex wraps and handles Home/End', () => {
	assert.equal(rovingIndex('ArrowRight', 2, 3), 0);
	assert.equal(rovingIndex('ArrowLeft', 0, 3), 2);
	assert.equal(rovingIndex('Home', 2, 3), 0);
	assert.equal(rovingIndex('End', 0, 3), 2);
	assert.equal(rovingIndex('a', 0, 3), null);
	assert.equal(rovingIndex('End', 0, 0), null);
});

test('rovingTabindex keeps one tab stop', () => {
	assert.equal(rovingTabindex(1, 1), 0);
	assert.equal(rovingTabindex(0, 1), -1);
	assert.equal(rovingTabindex(0, -1), 0);
});
