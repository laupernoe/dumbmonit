// The Overview's "Needs attention", week ahead and alerts per day. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { attentionRows, attentionMonitors, weekAhead, alertsPerDay } from '../src/lib/components/overview/attention.ts';

const target = (id) => ({ id, name: `m${id}` });
const device = (id) => ({ kind: 'device', key: `device:${id}`, rank: 0, target: target(id) });
const alert = (fingerprint, over = {}, row = {}) => ({
	kind: 'alert',
	key: `alert:${fingerprint}`,
	rank: 1,
	alert: { fingerprint, effective_phase: 'firing', acked: false, silenced: false, severity: 'warning', ...over },
	...row
});

test('only what is broken now: firing, not acked, not snoozed, not info, not pending', () => {
	const rows = [
		device(1),
		alert('a', {}, { target: target(2) }),
		alert('b', { acked: true }),
		alert('c', { silenced: true }),
		alert('d', { severity: 'info' }),
		alert('e', { effective_phase: 'pending' }),
		alert('f', { effective_phase: 'suppressed' }, { parent: target(1) })
	];
	assert.deepEqual(
		attentionRows(rows).map((r) => r.row.key),
		['device:1', 'alert:a']
	);
});

test('guests held back are counted once, on the parent row', () => {
	const rows = [
		device(1),
		alert('x', { effective_phase: 'suppressed' }, { parent: target(1) }),
		alert('y', { effective_phase: 'suppressed' }, { parent: target(1) }),
		alert('z', { effective_phase: 'suppressed' }, { parent: target(9) })
	];
	const out = attentionRows(rows);
	assert.equal(out.length, 1);
	assert.equal(out[0].held, 2);
});

test('a parent voiced by its own alert carries the guests on that alert row', () => {
	const rows = [
		alert('host-down', {}, { target: target(3) }),
		alert('cpu', {}, { target: target(3) }),
		alert('g', { effective_phase: 'suppressed' }, { parent: target(3) })
	];
	assert.deepEqual(
		attentionRows(rows).map((r) => r.held),
		[1, 0]
	);
});

test('monitors are counted once, alerts without a monitor apart', () => {
	const rows = attentionRows([device(1), alert('a', {}, { target: target(1) }), alert('b'), alert('c', {}, { target: target(2) })]);
	assert.equal(attentionMonitors(rows), 3);
});

test('week ahead: within the horizon, expired first, soonest first', () => {
	const items = [{ days_left: 12 }, { days_left: 3 }, { days_left: -1 }, { days_left: 7 }];
	assert.deepEqual(
		weekAhead(items).map((i) => i.days_left),
		[-1, 3, 7]
	);
});

test('alerts per day: seven local days, today last, flapping counts each firing', () => {
	const now = new Date(2026, 10, 1, 15, 0, 0).getTime();
	const utc = (d) => d.toISOString().replace('Z', '');
	const history = [
		{ at: utc(new Date(2026, 10, 1, 9)), to_phase: 'firing' },
		{ at: utc(new Date(2026, 10, 1, 10)), to_phase: 'resolved' },
		{ at: utc(new Date(2026, 10, 1, 11)), to_phase: 'firing' },
		{ at: utc(new Date(2026, 9, 31, 23, 30)), to_phase: 'firing' },
		{ at: utc(new Date(2026, 9, 26, 0, 10)), to_phase: 'resolved' },
		{ at: utc(new Date(2026, 9, 20)), to_phase: 'firing' },
		{ at: utc(new Date(2026, 10, 1, 12)), to_phase: 'pending' }
	];
	const days = alertsPerDay(history, now);
	assert.equal(days.length, 7);
	assert.equal(days[6].day, Math.round(new Date(2026, 10, 1).getTime() / 1000));
	assert.deepEqual(days[6], { day: days[6].day, opened: 2, resolved: 1 });
	assert.equal(days[5].opened, 1);
	assert.equal(days[0].resolved, 1);
	assert.equal(days.reduce((sum, d) => sum + d.opened + d.resolved, 0), 5);
});
