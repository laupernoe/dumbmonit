// Pure geometry of the visual kit. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { delta, extent, rampLevel, semiPoint, sparkGeometry, toMatrix, unit01 } from '../src/lib/viz/geometry.ts';

test('unit01 clamps and survives an empty range', () => {
	assert.equal(unit01(50, 0, 100), 0.5);
	assert.equal(unit01(-5, 0, 100), 0);
	assert.equal(unit01(500, 0, 100), 1);
	assert.equal(unit01(3, 7, 7), 0);
});

test('ramp has five steps', () => {
	assert.equal(rampLevel(0), 0);
	assert.equal(rampLevel(0.5), 2);
	assert.equal(rampLevel(1), 4);
	assert.equal(rampLevel(2), 4);
});

test('extent ignores nulls and non-finite values', () => {
	assert.deepEqual(extent([3, null, NaN, 9, undefined, 1]), [1, 9]);
	assert.equal(extent([null]), null);
});

test('toMatrix places cells and leaves holes as null', () => {
	const m = toMatrix(['a', 'b'], ['1', '2'], [
		{ row: 'a', col: '2', value: 5 },
		{ row: 'zz', col: '1', value: 9 },
		{ row: 'b', col: '1', value: NaN }
	]);
	assert.deepEqual(m.values, [
		[null, 5],
		[null, null]
	]);
	assert.equal(m.merged, 1);
});

test('toMatrix averages columns above the cell budget', () => {
	const cols = Array.from({ length: 10 }, (_, i) => String(i));
	const cells = cols.map((c, i) => ({ row: 'r', col: c, value: i }));
	const m = toMatrix(['r'], cols, cells, 5);
	assert.equal(m.merged, 2);
	assert.deepEqual(m.values[0], [0.5, 2.5, 4.5, 6.5, 8.5]);
	assert.deepEqual(m.cols, ['0', '2', '4', '6', '8']);
});

test('sparkGeometry: bounds, last point, empty and single value', () => {
	const g = sparkGeometry([1, 5, 3], 100, 20, 2);
	assert.equal(g.min, 1);
	assert.equal(g.max, 5);
	assert.equal(g.points.length, 3);
	assert.deepEqual(g.last, g.points[2]);
	assert.equal(g.points[1][1], 2); // the maximum sits on the top padding
	assert.ok(g.path.startsWith('M0.0'));
	assert.equal(sparkGeometry([], 100, 20).path, '');
	const one = sparkGeometry([4], 100, 20);
	assert.equal(one.path, '');
	assert.equal(one.points.length, 1);
});

test('semiPoint: left, top and right of the gauge arc', () => {
	const [lx, ly] = semiPoint(60, 60, 50, 0);
	const [tx, ty] = semiPoint(60, 60, 50, 0.5);
	const [rx, ry] = semiPoint(60, 60, 50, 1);
	assert.ok(Math.abs(lx - 10) < 1e-9 && Math.abs(ly - 60) < 1e-9);
	assert.ok(Math.abs(tx - 60) < 1e-9 && Math.abs(ty - 10) < 1e-9);
	assert.ok(Math.abs(rx - 110) < 1e-9 && Math.abs(ry - 60) < 1e-9);
});

test('delta: ratio, direction, no previous, previous of zero', () => {
	assert.deepEqual(delta(150, 100), { ratio: 0.5, direction: 'up' });
	assert.deepEqual(delta(50, 100), { ratio: -0.5, direction: 'down' });
	assert.deepEqual(delta(5, 5), { ratio: 0, direction: 'flat' });
	assert.deepEqual(delta(5, null), { ratio: null, direction: 'flat' });
	assert.deepEqual(delta(5, 0), { ratio: null, direction: 'up' });
});
