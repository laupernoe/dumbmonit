// How the Alerts "Now" list folds and files its rows. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fold, bucketOf, deviceKey, fingerprintsOf } from '../src/lib/components/alerts/list.ts';

const alert = (fingerprint, over = {}) => ({
	fingerprint,
	rule_uid: 'container_stopped',
	target_id: 7,
	severity: 'warning',
	effective_phase: 'firing',
	acked: false,
	silenced: false,
	...over
});
const row = (a) => ({ kind: 'alert', key: `alert:${a.fingerprint}`, alert: a, tone: 'advisory', plate: 'Advisory', rank: 1 });
const device = (id, tone = 'warning') => ({ kind: 'device', key: `device:${id}`, target: { id }, tone, plate: 'Unreachable', rank: 0 });

test('fold gathers one rule on one device, even when not consecutive', () => {
	const rows = [row(alert('a')), row(alert('x', { target_id: 8 })), row(alert('b')), row(alert('c'))];
	const folded = fold(rows);
	assert.equal(folded.length, 2);
	assert.deepEqual(fingerprintsOf(folded[0]), ['a', 'b', 'c']);
	assert.deepEqual(fingerprintsOf(folded[1]), ['x']);
});

test('fold keeps acknowledged, snoozed and pending apart from the live ones', () => {
	const rows = [
		row(alert('a')),
		row(alert('b', { acked: true })),
		row(alert('c', { silenced: true })),
		row(alert('d', { effective_phase: 'pending' }))
	];
	assert.equal(fold(rows).length, 4);
});

test('fold never folds device rows', () => {
	assert.equal(fold([device(1), device(1)]).length, 2);
});

test('bucketOf files each row under its group', () => {
	assert.equal(bucketOf(device(1)), 'warning');
	assert.equal(bucketOf(device(1, 'advisory')), 'advisory');
	assert.equal(bucketOf(row(alert('a', { severity: 'critical' }))), 'warning');
	assert.equal(bucketOf(row(alert('a'))), 'advisory');
	assert.equal(bucketOf(row(alert('a', { severity: 'info' }))), 'info');
	assert.equal(bucketOf(row(alert('a', { effective_phase: 'pending' }))), 'building');
	assert.equal(bucketOf(row(alert('a', { effective_phase: 'suppressed' }))), 'suppressed');
	assert.equal(bucketOf(row(alert('a', { silenced: true }))), 'snoozed');
	// An acknowledged alert is "known" even if a window also covers it.
	assert.equal(bucketOf(row(alert('a', { acked: true, silenced: true }))), 'acked');
});

test('deviceKey groups instance-wide alerts together', () => {
	assert.equal(deviceKey(row(alert('a', { target_id: null }))), 'none');
	assert.equal(deviceKey(row(alert('a'))), '7');
	assert.equal(deviceKey(device(3)), '3');
});
