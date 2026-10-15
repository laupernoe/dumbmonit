import test from 'node:test';
import assert from 'node:assert/strict';
import { bumpToastCount, toastText } from '../src/lib/ui/toast.ts';

test('bumpToastCount starts at 1 and increments', () => {
	assert.equal(bumpToastCount(null), 1);
	assert.equal(bumpToastCount({ error: 'x' }), 1);
	assert.equal(bumpToastCount({ count: 1 }), 2);
	assert.equal(bumpToastCount({ count: 4 }), 5);
});

test('toastText appends the counter only when merged', () => {
	assert.equal(toastText('Saved'), 'Saved');
	assert.equal(toastText('Saved', 1), 'Saved');
	assert.equal(toastText('Saved', 3), 'Saved ×3');
});
