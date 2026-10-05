// Per-alert snooze: the payload sent to the server, and which maintenance
// window (if any) explains why an alert is quiet right now. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { SNOOZE_DURATIONS, snoozePayload, coveringSilence } from '../src/lib/components/alerts/snooze.ts';

function alert(overrides) {
	return {
		fingerprint: 'cpu_high@0000000000000001',
		rule_uid: 'cpu_high',
		rule_name: 'High CPU',
		severity: 'warning',
		target_id: 1,
		series_key: 'dumbmonit_cpu_usage_percent{target="1"}',
		labels: { __name__: 'dumbmonit_cpu_usage_percent', target: '1', host: 'nas' },
		phase: 'firing',
		effective_phase: 'firing',
		suppressed: false,
		suppressed_by: null,
		silenced: false,
		learning: false,
		acked: false,
		acked_until: null,
		acked_by: null,
		ack_note: null,
		value: 95,
		score: null,
		condition_since: null,
		firing_since: null,
		last_eval_at: null,
		last_notified_at: null,
		notify_count: 0,
		...overrides
	};
}

function silence(overrides) {
	return {
		id: 1,
		name: 'Snooze · High CPU · nas',
		comment: '',
		target_id: 1,
		matchers: {},
		schedule: { kind: 'once', starts_at: '2026-01-01T00:00:00Z', ends_at: '2026-01-01T01:00:00Z' },
		enabled: true,
		active_now: true,
		active_until: '2026-01-01T01:00:00Z',
		next_start_at: null,
		...overrides
	};
}

test('four presets are offered, the last one reaching the server\'s 30-day ceiling', () => {
	assert.equal(SNOOZE_DURATIONS.length, 4);
	assert.deepEqual(SNOOZE_DURATIONS.map((d) => d.secs), [3600, 8 * 3600, 24 * 3600, 30 * 24 * 3600]);
	assert.equal(SNOOZE_DURATIONS.at(-1).label, 'Until resolved');
});

test('the snooze payload carries the alert\'s own labels, not a device-wide silence', () => {
	const a = alert();
	const target = { id: 1, name: 'nas' };
	const payload = snoozePayload(a, target, 3600);

	assert.equal(payload.target_id, 1);
	assert.deepEqual(payload.matchers, a.labels);
	assert.equal(payload.schedule.kind, 'once');
	assert.match(payload.name, /High CPU/);
	assert.match(payload.name, /nas/);

	const starts = new Date(payload.schedule.starts_at).getTime();
	const ends = new Date(payload.schedule.ends_at).getTime();
	assert.equal(ends - starts, 3600 * 1000);
});

test('the snooze payload survives an unknown device', () => {
	const payload = snoozePayload(alert(), undefined, 60);
	assert.equal(payload.name, 'Snooze · High CPU');
});

test('coveringSilence finds the window matching this alert\'s device and labels', () => {
	const a = alert();
	const matching = silence({ id: 7, matchers: { host: 'nas' } });
	const otherDevice = silence({ id: 8, target_id: 2 });
	const notActive = silence({ id: 9, active_now: false });

	assert.equal(coveringSilence(a, [otherDevice, notActive, matching]).id, 7);
	assert.equal(coveringSilence(a, [otherDevice, notActive]), null);
});

test('coveringSilence requires every matcher to agree, not just one', () => {
	const a = alert();
	const partial = silence({ matchers: { host: 'nas', mountpoint: '/data' } });
	assert.equal(coveringSilence(a, [partial]), null, 'the alert carries no "mountpoint" label');
});

test('a global window (no target) still covers a device-scoped alert', () => {
	const a = alert();
	const global = silence({ target_id: null, matchers: {} });
	assert.equal(coveringSilence(a, [global])?.id, global.id);
});

test('a sibling rule on the same device, with its own labels, is not covered', () => {
	const diskAlert = alert({
		fingerprint: 'disk_almost_full@0000000000000001',
		rule_uid: 'disk_almost_full',
		labels: { __name__: 'dumbmonit_disk_used_percent', target: '1', host: 'nas' }
	});
	// A snooze created for the CPU alert only.
	const cpuSnooze = silence({
		matchers: { __name__: 'dumbmonit_cpu_usage_percent', target: '1', host: 'nas' }
	});
	assert.equal(coveringSilence(diskAlert, [cpuSnooze]), null);
});
