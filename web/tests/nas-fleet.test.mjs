// NAS fleet: verdict of a NAS and of the whole fleet. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nasVerdict, summarizeFleet } from '../src/lib/components/monitors/nas-fleet.ts';

const nas = (o = {}) => ({ state: 'online', storageHealth: 0, updateAvailable: false, ...o });

test('a healthy NAS is ok', () => {
	assert.deepEqual(nasVerdict(nas()), { level: 'ok', reason: 'ok' });
});

test('a missing reading is not invented: no storage nor update info stays ok', () => {
	assert.deepEqual(nasVerdict(nas({ storageHealth: null, updateAvailable: null })), { level: 'ok', reason: 'ok' });
});

test('an unreachable NAS is red whatever its last readings said', () => {
	assert.deepEqual(nasVerdict(nas({ state: 'down' })), { level: 'bad', reason: 'down' });
	assert.deepEqual(nasVerdict(nas({ state: 'offline', updateAvailable: true })), { level: 'bad', reason: 'down' });
});

test('a degraded volume is red, a disk alert or an update is orange', () => {
	assert.deepEqual(nasVerdict(nas({ storageHealth: 2, updateAvailable: true })), { level: 'bad', reason: 'storage' });
	assert.deepEqual(nasVerdict(nas({ storageHealth: 1, updateAvailable: true })), { level: 'warn', reason: 'disk' });
	assert.deepEqual(nasVerdict(nas({ updateAvailable: true })), { level: 'warn', reason: 'update' });
});

test('a misconfigured or not yet probed NAS is neither healthy nor red', () => {
	assert.deepEqual(nasVerdict(nas({ state: 'misconfigured' })), { level: 'warn', reason: 'config' });
	assert.deepEqual(nasVerdict(nas({ state: 'pending' })), { level: 'unknown', reason: 'unknown' });
	assert.deepEqual(nasVerdict(nas({ state: 'disabled' })), { level: 'unknown', reason: 'unknown' });
});

test('the fleet counts the healthy ones and shows its worst level', () => {
	assert.deepEqual(summarizeFleet(['ok', 'ok']), { total: 2, healthy: 2, level: 'ok' });
	assert.deepEqual(summarizeFleet(['ok', 'warn', 'unknown']), { total: 3, healthy: 1, level: 'warn' });
	assert.deepEqual(summarizeFleet(['warn', 'bad', 'ok']), { total: 3, healthy: 1, level: 'bad' });
	assert.deepEqual(summarizeFleet([]), { total: 0, healthy: 0, level: 'ok' });
});
