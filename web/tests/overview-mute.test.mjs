// The Overview's mute menu: durations, the "until…" check, which window counts as a mute. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { MUTE_HOURS, checkUntil, endAfterHours, muteEnd, mutedBy } from '../src/lib/components/overview/mute.ts';

const once = { kind: 'once', starts_at: '2026-11-02T10:00:00Z', ends_at: '2026-11-02T11:00:00Z' };
const weekly = { kind: 'weekly', days: [0], start_minute: 60, end_minute: 120, utc_offset_minutes: 0 };
const silence = (id, over = {}) => ({
	id,
	name: `s${id}`,
	comment: '',
	target_id: 7,
	matchers: {},
	schedule: once,
	enabled: true,
	active_now: true,
	active_until: null,
	next_start_at: null,
	...over
});

test('the menu offers 1 h, 8 h and 24 h', () => {
	assert.deepEqual([...MUTE_HOURS], [1, 8, 24]);
	const now = Date.parse('2026-11-02T10:00:00Z');
	assert.equal(endAfterHours(8, now).toISOString(), '2026-11-02T18:00:00.000Z');
});

test('an "until" picked less than a minute ahead, empty or garbled is refused', () => {
	const now = new Date(2026, 10, 2, 10, 0, 0).getTime();
	assert.deepEqual(checkUntil('', now), { ok: false, problem: 'pick' });
	assert.deepEqual(checkUntil('not a date', now), { ok: false, problem: 'pick' });
	assert.deepEqual(checkUntil('2026-11-02T10:00', now), { ok: false, problem: 'future' });
	assert.deepEqual(checkUntil('2026-11-02T10:01', now), { ok: false, problem: 'future' });
	const ok = checkUntil('2026-11-02T10:30', now);
	assert.equal(ok.ok, true);
	assert.equal(ok.end.getTime(), new Date(2026, 10, 2, 10, 30).getTime());
});

test('a monitor is muted by an active one-off window on it alone', () => {
	assert.equal(mutedBy([silence(1)], 7)?.id, 1);
	assert.equal(mutedBy([silence(1)], 8), null);
	assert.equal(mutedBy([silence(1, { target_id: null })], 7), null, 'a global window is not this monitor muted');
	assert.equal(mutedBy([silence(1, { matchers: { rule: 'cpu' } })], 7), null, 'a snooze of one alert is not a mute');
	assert.equal(mutedBy([silence(1, { schedule: weekly })], 7), null, 'a weekly window is a schedule, never lifted from here');
	assert.equal(mutedBy([silence(1, { enabled: false })], 7), null);
	assert.equal(mutedBy([silence(1, { active_now: false })], 7), null, 'not started or already over');
});

test('a window created a moment ago counts even if the server clock is behind', () => {
	const late = silence(2, { active_now: false });
	assert.equal(mutedBy([late], 7), null);
	assert.equal(mutedBy([late], 7, [2])?.id, 2);
});

test('the end is the occurrence in progress, else the one-off end', () => {
	assert.equal(muteEnd(silence(1, { active_until: '2026-11-02T12:00:00' })), '2026-11-02T12:00:00');
	assert.equal(muteEnd(silence(1)), '2026-11-02T11:00:00Z');
	assert.equal(muteEnd(silence(1, { schedule: weekly })), null);
});
