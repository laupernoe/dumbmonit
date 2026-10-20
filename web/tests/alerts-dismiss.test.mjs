// Dismissing alerts: the optimistic removal, the Dismissed list order, who did it. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { withoutFingerprints, sortDismissed, dismissedByName } from '../src/lib/alerts-dismiss.ts';

const a = (fingerprint, dismissed_at = null) => ({ fingerprint, dismissed_at });

test('withoutFingerprints drops exactly the dismissed ones and keeps the order', () => {
	const list = [a('x'), a('y'), a('z')];
	assert.deepEqual(withoutFingerprints(list, ['y']).map((e) => e.fingerprint), ['x', 'z']);
	assert.deepEqual(withoutFingerprints(list, new Set(['x', 'z'])).map((e) => e.fingerprint), ['y']);
});

test('withoutFingerprints with nothing to drop returns a copy, never the input', () => {
	const list = [a('x')];
	const out = withoutFingerprints(list, []);
	assert.deepEqual(out, list);
	assert.notEqual(out, list);
	assert.deepEqual(withoutFingerprints(list, ['unknown']), list);
});

test('undo is the inverse: the restored alert is back in the active list after a refresh', () => {
	const active = [a('x'), a('y')];
	const optimistic = withoutFingerprints(active, ['x']);
	assert.equal(optimistic.length, 1);
	// The server answers the next /alerts with both again.
	assert.deepEqual(active.map((e) => e.fingerprint), ['x', 'y']);
});

test('sortDismissed puts the latest dismissal first, unknown dates last', () => {
	const sorted = sortDismissed([
		a('old', '2026-10-20T08:00:00'),
		a('none'),
		a('new', '2026-10-20T09:30:00'),
		a('mid', '2026-10-20T09:00:00Z')
	]);
	assert.deepEqual(sorted.map((e) => e.fingerprint), ['new', 'mid', 'old', 'none']);
});

test('sortDismissed reads suffix-less server dates as UTC', () => {
	const [first] = sortDismissed([a('plain', '2026-10-20T10:00:00'), a('zulu', '2026-10-20T09:59:59Z')]);
	assert.equal(first.fingerprint, 'plain');
});

test('sortDismissed does not touch its input', () => {
	const input = [a('b', '2026-10-20T08:00:00'), a('a', '2026-10-20T09:00:00')];
	sortDismissed(input);
	assert.deepEqual(input.map((e) => e.fingerprint), ['b', 'a']);
});

test('dismissedByName shows a token by its name and a user as is', () => {
	assert.equal(dismissedByName('token:grafana'), 'grafana');
	assert.equal(dismissedByName('noe'), 'noe');
	assert.equal(dismissedByName(null), '');
});
