// Swiss holidays, forced date and station clock maths. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	parseForcedDate,
	swissDayFor,
	resolveSwissDay,
	zurichClock,
	zurichDay,
	clockAngles
} from '../src/lib/scene/swiss.ts';

test('forced date: valid, invalid, absent', () => {
	assert.deepEqual(parseForcedDate('?date=2026-08-01'), { y: 2026, m: 8, d: 1 });
	assert.equal(parseForcedDate('?date=2026-02-30'), null);
	assert.equal(parseForcedDate('?date=nope'), null);
	assert.equal(parseForcedDate('?city=geneva'), null);
	assert.equal(parseForcedDate(''), null);
});

test('national day: 1 August only', () => {
	assert.equal(swissDayFor({ y: 2026, m: 8, d: 1 }).nationalDay, true);
	assert.equal(swissDayFor({ y: 2026, m: 7, d: 31 }).nationalDay, false);
	assert.equal(swissDayFor({ y: 2026, m: 8, d: 2 }).nationalDay, false);
});

test('Escalade around 12 December, and winter', () => {
	assert.equal(swissDayFor({ y: 2026, m: 12, d: 12 }).escalade, true);
	assert.equal(swissDayFor({ y: 2026, m: 12, d: 9 }).escalade, false);
	assert.equal(swissDayFor({ y: 2026, m: 12, d: 15 }).escalade, false);
	assert.equal(swissDayFor({ y: 2026, m: 11, d: 30 }).winter, false);
	for (const m of [12, 1, 2]) assert.equal(swissDayFor({ y: 2026, m, d: 5 }).winter, true);
	assert.equal(swissDayFor({ y: 2026, m: 3, d: 1 }).winter, false);
	assert.equal(swissDayFor({ y: 2026, m: 8, d: 1 }).winter, false);
});

test('resolve: the URL beats the clock', () => {
	const now = new Date('2026-06-15T12:00:00Z');
	assert.equal(resolveSwissDay(now, '').winter, false);
	assert.equal(resolveSwissDay(now, '?date=2026-12-12').escalade, true);
	assert.equal(resolveSwissDay(now, '?date=garbage').nationalDay, false);
});

test('Zurich time zone: summer UTC+2, winter UTC+1, day rolls over there', () => {
	assert.deepEqual(zurichClock(new Date('2026-07-01T10:00:00Z')).h, 12);
	assert.deepEqual(zurichClock(new Date('2026-01-01T10:00:00Z')).h, 11);
	assert.deepEqual(zurichDay(new Date('2026-07-31T22:30:00Z')), { y: 2026, m: 8, d: 1 });
});

test('seconds hand laps in 58 s then waits at 12', () => {
	const at = (s, ms = 0) => clockAngles(new Date(Date.UTC(2026, 0, 1, 12, 0, s, ms)));
	assert.equal(at(0).second, 0);
	assert.equal(Math.round(at(29).second), 180);
	assert.equal(at(58).second, 360);
	assert.equal(at(59, 500).second, 360);
	assert.equal(clockAngles(new Date(Date.UTC(2026, 0, 1, 12, 0, 59))).minute, 0);
	// 15:30 in Zurich (14:30Z in winter): hour hand halfway between 3 and 4.
	const a = clockAngles(new Date('2026-01-01T14:30:00Z'));
	assert.equal(a.hour, 105);
	assert.equal(a.minute, 180);
});
