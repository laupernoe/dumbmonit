// Day labels of the status history bars: one cached formatter per locale.
// Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { formatUtcDay } from '../src/lib/components/status/utc-day.ts';

test('a bucket date reads as the UTC day, whatever the time zone', () => {
	const expected = new Intl.DateTimeFormat('en', { day: 'numeric', month: 'short', year: 'numeric', timeZone: 'UTC' }).format(
		new Date('2026-10-18T00:00:00Z')
	);
	assert.equal(formatUtcDay('2026-10-18', 'en'), expected);
	assert.match(formatUtcDay('2026-10-18', 'en'), /18/);
});

test('each locale keeps its own wording', () => {
	assert.equal(formatUtcDay('2026-03-01', 'fr'), formatUtcDay('2026-03-01', 'fr'));
	assert.notEqual(formatUtcDay('2026-03-01', 'fr'), formatUtcDay('2026-03-01', 'de'));
});

test('an unreadable date is shown as given', () => {
	assert.equal(formatUtcDay('not-a-day', 'en'), 'not-a-day');
});
