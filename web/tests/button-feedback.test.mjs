import test from 'node:test';
import assert from 'node:assert/strict';
import { outcome, FEEDBACK_MS } from '../src/lib/ui/feedback.ts';

test('a resolved action is a success', async () => {
	assert.equal(await outcome(undefined), true);
	assert.equal(await outcome(Promise.resolve(true)), true);
});

test('false or a rejection stays silent', async () => {
	assert.equal(await outcome(false), false);
	assert.equal(await outcome(Promise.resolve(false)), false);
	assert.equal(await outcome(Promise.reject(new Error('boom'))), false);
});

test('the confirmation lasts about 1.2 s', () => {
	assert.equal(FEEDBACK_MS, 1200);
});
