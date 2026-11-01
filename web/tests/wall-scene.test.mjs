// The wall's world: Paris by default, four cities, `none` as an explicit choice, old choices folded. Run: npm test
import { test, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import {
	readWallScene,
	writeWallScene,
	parseForcedWallScene,
	resolveWallScene,
	cityForTimeZone,
	cityHasDay,
	DEFAULT_WALL_SCENE
} from '../src/lib/components/wall/wallScene.ts';

const store = new Map();
globalThis.localStorage = {
	getItem: (k) => (store.has(k) ? store.get(k) : null),
	setItem: (k, v) => void store.set(k, String(v)),
	removeItem: (k) => void store.delete(k)
};
beforeEach(() => store.clear());

test('nothing saved, or junk, is Paris', () => {
	assert.equal(DEFAULT_WALL_SCENE, 'paris');
	assert.equal(readWallScene(), 'paris');
	store.set('dumbmonit-wall-city', 'atlantis');
	assert.equal(readWallScene(), 'paris');
	assert.equal(resolveWallScene(readWallScene()), 'paris');
});

test('a saved Zermatt or Matterhorn lands on Dubai', () => {
	store.set('dumbmonit-wall-city', 'zermatt');
	assert.equal(readWallScene(), 'dubai');
	assert.equal(parseForcedWallScene('matterhorn'), 'dubai');
});

test('choices round-trip, and a plain board is a stored choice', () => {
	writeWallScene('tokyo');
	assert.equal(readWallScene(), 'tokyo');
	writeWallScene('auto');
	assert.equal(readWallScene(), 'auto');
	writeWallScene('none');
	assert.equal(store.get('dumbmonit-wall-city'), 'none');
	assert.equal(readWallScene(), 'none');
	assert.equal(resolveWallScene(readWallScene()), null);
});

test('a ?city= value names a choice, junk names nothing', () => {
	assert.equal(parseForcedWallScene('none'), 'none');
	assert.equal(parseForcedWallScene('newyork'), 'newyork');
	assert.equal(parseForcedWallScene('atlantis'), null);
	assert.equal(parseForcedWallScene(null), null);
});

test('only Paris is drawn by day', () => {
	assert.equal(cityHasDay('paris'), true);
	assert.equal(cityHasDay('tokyo'), false);
	assert.equal(cityHasDay(null), false);
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
