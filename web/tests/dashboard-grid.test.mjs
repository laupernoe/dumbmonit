// Dashboard grid layout. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { compact, flow, moveTo, overlaps, resizeTo, shift, slotFor } from '../src/lib/dashboards/grid.ts';

const cell = (id, x, y, w, h) => ({ id, x, y, w, h });
const noOverlap = (cells) => cells.every((a, i) => cells.every((b, j) => i === j || !overlaps(a, b)));

test('widgets fall to the top and never overlap', () => {
	const out = compact([cell('a', 0, 5, 6, 2), cell('b', 0, 9, 6, 2), cell('c', 6, 3, 6, 2)]);
	assert.deepEqual(out.map((c) => [c.id, c.x, c.y]), [['a', 0, 0], ['b', 0, 2], ['c', 6, 0]]);
	assert.ok(noOverlap(out));
});

test('the dragged widget wins, the others make room below', () => {
	const start = [cell('a', 0, 0, 6, 2), cell('b', 6, 0, 6, 2)];
	const out = moveTo(start, 'b', 0, 0);
	assert.deepEqual(out.find((c) => c.id === 'b'), cell('b', 0, 0, 6, 2));
	assert.equal(out.find((c) => c.id === 'a').y, 2);
	assert.ok(noOverlap(out));
});

test('a widget stays inside the 12 columns', () => {
	const out = moveTo([cell('a', 0, 0, 6, 2)], 'a', 20, -3);
	assert.deepEqual(out[0], cell('a', 6, 0, 6, 2));
	const wide = resizeTo([cell('a', 8, 0, 2, 2)], 'a', 9, 3);
	assert.equal(wide[0].w, 4);
	assert.equal(wide[0].h, 3);
});

test('resizing pushes the neighbours down', () => {
	const out = resizeTo([cell('a', 0, 0, 6, 2), cell('b', 0, 2, 6, 2)], 'a', 6, 4);
	assert.equal(out.find((c) => c.id === 'b').y, 4);
});

test('the single column reorders by swapping reading neighbours', () => {
	const start = [cell('a', 0, 0, 12, 2), cell('b', 0, 2, 12, 2), cell('c', 0, 4, 12, 2)];
	const out = shift(start, 'c', -1);
	const order = [...out].sort((p, q) => p.y - q.y).map((c) => c.id);
	assert.deepEqual(order, ['a', 'c', 'b']);
	assert.deepEqual(shift(start, 'a', -1), start);
	assert.ok(noOverlap(flow(out)));
});

test('a new widget goes under the others', () => {
	assert.deepEqual(slotFor([cell('a', 0, 0, 6, 3), cell('b', 6, 0, 6, 5)]), { x: 0, y: 5 });
	assert.deepEqual(slotFor([]), { x: 0, y: 0 });
});

import { statLevel, formatStat, stepFor } from '../src/lib/dashboards/widgets.ts';

test('thresholds read in both directions', () => {
	assert.equal(statLevel(50, 80, 95, 'above'), 'ok');
	assert.equal(statLevel(85, 80, 95, 'above'), 'warn');
	assert.equal(statLevel(99, 80, 95, 'above'), 'crit');
	assert.equal(statLevel(5, 20, 10, 'below'), 'crit');
	assert.equal(statLevel(15, 20, 10, 'below'), 'warn');
	assert.equal(statLevel(30, undefined, undefined, 'above'), 'none');
	assert.equal(statLevel(null, 1, 2, 'above'), 'none');
});

test('numbers stay short', () => {
	assert.equal(formatStat(42), '42');
	assert.equal(formatStat(1234567), '1.23M');
	assert.equal(formatStat(0.123456), '0.123');
	assert.equal(stepFor(3600), 12);
});
