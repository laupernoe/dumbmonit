// Wall layout: zones, order, and the fitting step of a zone. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { DEFAULT_LAYOUT, dropIndex, moveToZone, placeAt, resolveLayout, sameLayout, shift, trimCaps, zoneOf } from '../src/lib/wall/layout.ts';

test('the default layout places every widget once', () => {
	assert.deepEqual(DEFAULT_LAYOUT, {
		top: [],
		bottom: ['crypto', 'stocks', 'fx', 'claude', 'openai'],
		right: ['weather', 'news', 'cve', 'text'],
		hidden: []
	});
});

test('unknown and repeated ids are dropped, missing ones go to their default zone', () => {
	const layout = resolveLayout({ top: ['FX', 'video', 'crypto'], bottom: ['fx'], hidden: ['news'] });
	assert.deepEqual(layout.top, ['fx', 'crypto']);
	assert.deepEqual(layout.hidden, ['news']);
	assert.deepEqual(layout.bottom, ['stocks', 'claude', 'openai']);
	assert.deepEqual(layout.right, ['weather', 'cve', 'text']);
});

test('a widget moves to the end of another zone, and within its zone', () => {
	let layout = moveToZone(DEFAULT_LAYOUT, 'crypto', 'top');
	assert.equal(zoneOf(layout, 'crypto'), 'top');
	assert.ok(!layout.bottom.includes('crypto'));
	layout = shift(layout, 'fx', -1);
	assert.deepEqual(layout.bottom, ['fx', 'stocks', 'claude', 'openai']);
	assert.deepEqual(shift(layout, 'fx', -1).bottom, layout.bottom, 'already first');
	assert.deepEqual(DEFAULT_LAYOUT.bottom, ['crypto', 'stocks', 'fx', 'claude', 'openai'], 'the input is not changed');
});

test('fitting takes one entry from the widget that shows the most', () => {
	assert.deepEqual(trimCaps([3, 5, 5]), [3, 5, 4]);
	assert.deepEqual(trimCaps([1, 2, 1]), [1, 1, 1]);
	assert.equal(trimCaps([1, 1]), null);
	assert.equal(trimCaps([]), null);
});

test('a floor keeps two entries each before the short forms', () => {
	assert.deepEqual(trimCaps([2, 3], 2), [2, 2]);
	assert.equal(trimCaps([2, 2, 1], 2), null);
});

test('a widget is dropped at a given place, in another zone or its own', () => {
	let layout = placeAt(DEFAULT_LAYOUT, 'fx', 'top', 0);
	assert.deepEqual(layout.top, ['fx']);
	assert.ok(!layout.bottom.includes('fx'));
	layout = placeAt(layout, 'crypto', 'top', 5);
	assert.deepEqual(layout.top, ['fx', 'crypto'], 'past the end is the end');
	layout = placeAt(layout, 'crypto', 'top', 0);
	assert.deepEqual(layout.top, ['crypto', 'fx']);
	assert.deepEqual(placeAt(DEFAULT_LAYOUT, 'news', 'right', 1), DEFAULT_LAYOUT, 'same spot, nothing changes');
	assert.deepEqual(placeAt(DEFAULT_LAYOUT, 'text', 'hidden', -3).hidden, ['text']);
	assert.deepEqual(DEFAULT_LAYOUT.right, ['weather', 'news', 'cve', 'text'], 'the input is not changed');
});

test('the drop index counts the widgets whose middle is before the pointer', () => {
	assert.equal(dropIndex([], 40), 0);
	assert.equal(dropIndex([50, 150, 250], 10), 0);
	assert.equal(dropIndex([50, 150, 250], 160), 2);
	assert.equal(dropIndex([50, 150, 250], 900), 3);
});

test('layouts compare by placement', () => {
	assert.ok(sameLayout(DEFAULT_LAYOUT, resolveLayout(null)));
	assert.ok(!sameLayout(DEFAULT_LAYOUT, moveToZone(DEFAULT_LAYOUT, 'text', 'hidden')));
});
