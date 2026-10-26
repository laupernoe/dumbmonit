// Public status page: the period the history bars cover and how a bar is named. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { PERIOD_CHOICES, barDay, isDayBar, periodParts } from '../src/lib/components/status/period.ts';

test('the period is told in hours up to two days, in days beyond', () => {
	assert.deepEqual(periodParts(3600), { unit: 'hours', count: 1 });
	assert.deepEqual(periodParts(7200), { unit: 'hours', count: 2 });
	assert.deepEqual(periodParts(12 * 3600), { unit: 'hours', count: 12 });
	assert.deepEqual(periodParts(86400), { unit: 'hours', count: 24 });
	assert.deepEqual(periodParts(3 * 86400), { unit: 'days', count: 3 });
	assert.deepEqual(periodParts(90 * 86400), { unit: 'days', count: 90 });
	assert.deepEqual(periodParts(600), { unit: 'hours', count: 1 }, 'never less than an hour');
});

test('a bar of a day or more carries a date; a shorter one a time', () => {
	assert.equal(isDayBar(86400), true);
	assert.equal(isDayBar(21600), false);
	assert.equal(isDayBar(300), false);
	assert.equal(barDay('2026-10-21T08:00:00Z'), '2026-10-21');
	assert.equal(barDay('2026-10-21'), '2026-10-21');
});

test('the page can pick 24 hours, 7, 30, 60 or 90 days', () => {
	assert.deepEqual([...PERIOD_CHOICES], [1, 7, 30, 60, 90]);
});
