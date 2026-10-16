// Public status page: day tones of the history bar and the bands its colour key states. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dayTone, DAY_GOOD, DAY_POOR } from '../src/lib/components/status/day-tone.ts';

test('a day without data is grey, never green', () => {
	assert.equal(dayTone(null), 'ghost');
});

test('days are toned by the bands the legend states', () => {
	assert.equal(dayTone(100), 'signal');
	assert.equal(dayTone(DAY_GOOD), 'signal');
	assert.equal(dayTone(99.02), 'advisory');
	assert.equal(dayTone(96.62), 'advisory');
	assert.equal(dayTone(DAY_POOR), 'advisory');
	assert.equal(dayTone(92.97), 'warning');
	assert.equal(dayTone(0), 'warning');
});

test('a few minutes of downtime keep a day out of the poor band', () => {
	// 98 % = about 29 minutes down: some downtime, not a poor day.
	assert.equal(dayTone(98), 'advisory');
	assert.ok(DAY_POOR < DAY_GOOD);
});
