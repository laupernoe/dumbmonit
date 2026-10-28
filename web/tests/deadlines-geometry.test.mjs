// Pure maths of the deadline timeline. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { axisPosition, deadlineTone, placeOnLanes } from '../src/lib/components/deadlines/geometry.ts';

test('tiers follow the 7, 30 and 60 day steps', () => {
	assert.equal(deadlineTone(-3), 'warning');
	assert.equal(deadlineTone(7), 'warning');
	assert.equal(deadlineTone(7.1), 'advisory');
	assert.equal(deadlineTone(30), 'advisory');
	assert.equal(deadlineTone(45), 'info');
	assert.equal(deadlineTone(61), 'signal');
});

test('axis position clamps to the 90 day window', () => {
	assert.equal(axisPosition(45), 0.5);
	assert.equal(axisPosition(-5), 0);
	assert.equal(axisPosition(400), 1);
	assert.equal(axisPosition(Number.NaN), 0);
});

test('close markers go on separate lanes, far ones reuse lane 0', () => {
	const days = [10, 11, 12, 60];
	const placed = placeOnLanes(days, (d) => d);
	assert.deepEqual(
		placed.map((p) => p.lane),
		[0, 1, 2, 0]
	);
});

test('items beyond the horizon are left out and lanes are capped', () => {
	const placed = placeOnLanes([1, 1, 1, 1, 1, 1, 200], (d) => d, { maxLanes: 3 });
	assert.equal(placed.length, 6);
	assert.equal(Math.max(...placed.map((p) => p.lane)), 2);
});
