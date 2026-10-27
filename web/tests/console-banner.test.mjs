// The Overview banner's one line, read off monitors and alerts. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { bannerLine } from '../src/lib/components/enterprise/banner.ts';

const mon = (id, status, name = `m${id}`) => ({ id, name, status });
const alert = (over) => ({
	target_id: null,
	fingerprint: 'f',
	effective_phase: 'firing',
	acked: false,
	severity: 'critical',
	notify_count: 0,
	last_notified_at: null,
	...over
});

test('no monitor: nothing to watch', () => {
	assert.deepEqual(bannerLine([], []), { kind: 'empty' });
});

test('all up, paused ones counted apart', () => {
	assert.deepEqual(bannerLine([mon(1, 'up'), mon(2, 'up'), mon(3, 'paused')], []), { kind: 'up', count: 2, paused: 1 });
	assert.deepEqual(bannerLine([mon(1, 'paused')], []), { kind: 'up', count: 0, paused: 1 });
});

test('one down: its name, guests held back and whether the alert went out', () => {
	const line = bannerLine(
		[mon(1, 'down', 'pve-02'), mon(2, 'up')],
		[
			alert({ target_id: 1, notify_count: 1 }),
			alert({ target_id: 5, fingerprint: 'a', effective_phase: 'suppressed' }),
			alert({ target_id: 5, fingerprint: 'b', effective_phase: 'suppressed' }),
			alert({ target_id: 6, fingerprint: 'c', effective_phase: 'suppressed' })
		]
	);
	assert.deepEqual(line, { kind: 'down', count: 1, name: 'pve-02', held: 2, sent: true });
});

test('several down: a count, no name; nothing sent yet', () => {
	const line = bannerLine([mon(1, 'down'), mon(2, 'down')], [alert({ target_id: 1 })]);
	assert.deepEqual(line, { kind: 'down', count: 2, name: null, held: 0, sent: false });
});

test('degraded or misconfigured monitors, or a loose alert, need a look', () => {
	assert.deepEqual(bannerLine([mon(1, 'degraded', 'nas'), mon(2, 'up')], [alert({ target_id: 1 })]), {
		kind: 'attention',
		count: 1,
		name: 'nas',
		alerts: 1
	});
	assert.deepEqual(bannerLine([mon(1, 'up')], [alert({ severity: 'warning' })]), { kind: 'attention', count: 0, name: null, alerts: 1 });
	// Info and acknowledged alerts are not news.
	assert.equal(bannerLine([mon(1, 'up')], [alert({ severity: 'info' }), alert({ acked: true })]).kind, 'up');
});

test('first reports still awaited', () => {
	assert.deepEqual(bannerLine([mon(1, 'up'), mon(2, 'waiting'), mon(3, 'paused')], []), { kind: 'waiting', up: 1, total: 2, waiting: 1 });
});
