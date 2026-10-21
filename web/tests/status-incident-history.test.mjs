// Past incidents: local date handling and the checks mirrored from the server. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { datedUpdates, dateToLocal, localToIso, validateHistory, updatesPayload } from '../src/lib/components/status/incident-history.ts';

const NOW = new Date('2026-10-21T12:00:00');
const at = (hoursAgo) => dateToLocal(new Date(NOW.getTime() - hoursAgo * 3600_000));
const upd = (status, hoursAgo, body = 'x') => ({ status, at: hoursAgo === null ? '' : at(hoursAgo), body });

test('local input round-trips and goes out as a UTC instant', () => {
	assert.equal(dateToLocal(new Date('2026-03-04T05:06:00')), '2026-03-04T05:06');
	assert.equal(localToIso('2026-03-04T05:06'), new Date('2026-03-04T05:06').toISOString());
	assert.equal(localToIso(''), undefined);
	assert.equal(localToIso('garbage'), undefined);
});

test('an incident that is already over is valid', () => {
	assert.equal(validateHistory(at(30), at(26), [upd('investigating', 30), upd('resolved', 26)], NOW), null);
	assert.equal(validateHistory('', '', [], NOW), null, 'now, still open');
});

test('dates in the future and a negative duration are refused', () => {
	const future = dateToLocal(new Date(NOW.getTime() + 3 * 3600_000));
	assert.equal(validateHistory(future, '', [], NOW), 'future_start');
	assert.equal(validateHistory(at(2), future, [], NOW), 'future_end');
	assert.equal(validateHistory(at(2), at(3), [], NOW), 'end_before_start');
});

test('updates stay between start and end, in the past, and nothing follows the resolution', () => {
	assert.equal(validateHistory(at(5), '', [upd('investigating', 6)], NOW), 'update_outside');
	assert.equal(validateHistory(at(5), at(3), [upd('monitoring', 1)], NOW), 'update_outside');
	assert.equal(validateHistory(at(5), '', [upd('monitoring', 4, '  ')], NOW), 'update_empty');
	assert.equal(validateHistory(at(5), '', [upd('resolved', 4), upd('monitoring', 3)], NOW), 'after_resolution');
	const future = dateToLocal(new Date(NOW.getTime() + 3 * 3600_000));
	assert.equal(validateHistory(at(5), '', [{ status: 'monitoring', at: future, body: 'x' }], NOW), 'update_future');
});

test('updates are sorted in time; an empty date follows the previous one', () => {
	const sorted = datedUpdates(new Date(NOW.getTime() - 10 * 3600_000), [
		upd('resolved', 6),
		upd('investigating', 10),
		upd('identified', 8),
		upd('monitoring', null)
	]);
	assert.deepEqual(
		sorted.map((u) => u.status),
		['investigating', 'identified', 'monitoring', 'resolved']
	);
});

test('the payload keeps the typed order and sends UTC dates', () => {
	const payload = updatesPayload([upd('resolved', 6, ' done '), upd('investigating', null)]);
	assert.equal(payload[0].body, 'done');
	assert.equal(payload[0].at, new Date(NOW.getTime() - 6 * 3600_000).toISOString().replace(/:\d\d\.\d{3}Z$/, ':00.000Z'));
	assert.equal(payload[1].at, undefined);
});
