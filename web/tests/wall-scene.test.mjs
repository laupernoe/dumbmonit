// The wall's optional night backdrop: none by default, four cities, old choices folded. Run: npm test
import { test, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import {
	readWallScene,
	writeWallScene,
	parseForcedWallScene,
	resolveWallScene,
	cityForTimeZone
} from '../src/lib/components/wall/wallScene.ts';

const store = new Map();
globalThis.localStorage = {
	getItem: (k) => (store.has(k) ? store.get(k) : null),
	setItem: (k, v) => void store.set(k, String(v)),
	removeItem: (k) => void store.delete(k)
};
beforeEach(() => store.clear());

test('nothing saved, or junk, is no backdrop', () => {
	assert.equal(readWallScene(), 'none');
	store.set('dumbmonit-wall-city', 'atlantis');
	assert.equal(readWallScene(), 'none');
});

test('a saved Zermatt or Matterhorn lands on Dubai', () => {
	store.set('dumbmonit-wall-city', 'zermatt');
	assert.equal(readWallScene(), 'dubai');
	assert.equal(parseForcedWallScene('matterhorn'), 'dubai');
});

test('choices round-trip; none is not stored', () => {
	writeWallScene('tokyo');
	assert.equal(readWallScene(), 'tokyo');
	writeWallScene('auto');
	assert.equal(readWallScene(), 'auto');
	writeWallScene('none');
	assert.equal(store.has('dumbmonit-wall-city'), false);
});

test('auto follows the time zone, none draws nothing', () => {
	assert.equal(resolveWallScene('none'), null);
	assert.equal(resolveWallScene('paris', 'Asia/Tokyo'), 'paris');
	assert.equal(resolveWallScene('auto', 'Asia/Dubai'), 'dubai');
	assert.equal(cityForTimeZone('Asia/Tokyo'), 'tokyo');
	assert.equal(cityForTimeZone('America/Chicago'), 'newyork');
	assert.equal(cityForTimeZone('Europe/Zurich'), 'paris');
	assert.equal(cityForTimeZone(undefined), 'paris');
});
