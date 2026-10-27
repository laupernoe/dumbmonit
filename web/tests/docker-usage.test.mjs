// What the Docker panel derives from a container's latest readings. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { CRASHLOOP_RESTARTS, isCrashLooping, memoryRatio, ratioTone, topBy } from '../src/lib/components/devices/docker/usage.ts';

const c = (name, extra = {}) => ({ name, up: true, ...extra });

test('three restarts in fifteen minutes make a restart loop, fewer or unknown do not', () => {
	assert.equal(CRASHLOOP_RESTARTS, 3);
	assert.equal(isCrashLooping({ restarts_15m: 3 }), true);
	assert.equal(isCrashLooping({ restarts_15m: 2 }), false);
	assert.equal(isCrashLooping({ restarts_15m: null }), false);
	assert.equal(isCrashLooping({}), false);
});

test('memory ratio needs both a reading and a limit', () => {
	assert.equal(memoryRatio(c('a', { memory_bytes: 250, memory_limit_bytes: 1000 })), 0.25);
	assert.equal(memoryRatio(c('a', { memory_bytes: 250, memory_limit_bytes: null })), null);
	assert.equal(memoryRatio(c('a', { memory_bytes: 250, memory_limit_bytes: 0 })), null);
	assert.equal(memoryRatio(c('a', { memory_limit_bytes: 1000 })), null);
});

test('tones step at 80 and 90 percent', () => {
	assert.equal(ratioTone(0.79), 'signal');
	assert.equal(ratioTone(0.8), 'advisory');
	assert.equal(ratioTone(0.9), 'warning');
	assert.equal(ratioTone(2), 'warning');
});

test('top lists keep running containers with a reading, largest first, capped', () => {
	const list = [
		c('a', { cpu_percent: 5 }),
		c('b', { cpu_percent: 250 }),
		c('c', { cpu_percent: null }),
		c('d', { cpu_percent: 90, up: false }),
		c('e', { cpu_percent: 40 }),
		c('f')
	];
	assert.deepEqual(
		topBy(list, 'cpu_percent', 2).map((i) => [i.key, i.value]),
		[['b', 250], ['e', 40]]
	);
	assert.deepEqual(topBy(list, 'memory_bytes'), []);
});
