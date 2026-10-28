// Restart markers read from stored series. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { MAX_RESTARTS, restartEvents } from '../src/lib/restart-events.ts';

const T0 = 1_790_000_000;
const uptime = (values) => ({ metric: { __name__: 'dumbmonit_uptime_seconds', target: '1' }, values: values.map(([t, v]) => [t, String(v)]) });
const restarts = (container, values) => ({
	metric: { __name__: 'dumbmonit_container_restart_count', target: '1', container },
	values: values.map(([t, v]) => [t, String(v)])
});

test('an uptime that falls is a reboot, dated by its boot instant', () => {
	// Samples every 300 s; the host booted 120 s before the sample at T0 + 600.
	const out = restartEvents([uptime([[T0, 9000], [T0 + 300, 9300], [T0 + 600, 120], [T0 + 900, 420]])]);
	assert.deepEqual(out, [{ at: T0 + 480, kind: 'host', name: '' }]);
});

test('a steady or slightly noisy uptime raises nothing', () => {
	assert.deepEqual(restartEvents([uptime([[T0, 1000], [T0 + 300, 1300], [T0 + 600, 1290], [T0 + 900, 1600]])]), []);
});

test('a boot older than the previous sample falls back to the sample time', () => {
	const out = restartEvents([uptime([[T0, 5000], [T0 + 300, 4000]])]);
	assert.deepEqual(out, [{ at: T0 + 300, kind: 'host', name: '' }]);
});

test('a container restart is each rise of its count; a recreated container (count falls) is not one', () => {
	const out = restartEvents([
		restarts('web', [[T0, 1], [T0 + 300, 1], [T0 + 600, 2], [T0 + 900, 4]]),
		restarts('db', [[T0, 5], [T0 + 300, 0], [T0 + 600, 0]])
	]);
	assert.deepEqual(out, [
		{ at: T0 + 600, kind: 'container', name: 'web' },
		{ at: T0 + 900, kind: 'container', name: 'web' }
	]);
});

test('other series are ignored and the newest markers are kept within the cap', () => {
	const many = Array.from({ length: MAX_RESTARTS + 10 }, (_, i) => [T0 + i * 60, i]);
	const out = restartEvents([{ metric: { __name__: 'dumbmonit_cpu_usage_percent' }, values: [[T0, '1'], [T0 + 60, '0']] }, restarts('x', many)]);
	assert.equal(out.length, MAX_RESTARTS);
	assert.equal(out.at(-1).at, T0 + (MAX_RESTARTS + 9) * 60);
});
