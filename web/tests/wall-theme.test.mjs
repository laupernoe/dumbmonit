// The wall's theme choice: Auto / Day / Night; the retired OLED folds onto Night. Run: npm test
import { test, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { readWallTheme, writeWallTheme, parseForcedWallTheme } from '../src/lib/components/wall/wallTheme.ts';

const store = new Map();
globalThis.localStorage = {
	getItem: (k) => (store.has(k) ? store.get(k) : null),
	setItem: (k, v) => void store.set(k, String(v)),
	removeItem: (k) => void store.delete(k)
};
beforeEach(() => store.clear());

test('a saved OLED choice reads as Night', () => {
	store.set('dumbmonit-wall-theme', 'oled');
	assert.equal(readWallTheme(), 'dark');
});

test('nothing saved, or junk, is Auto', () => {
	assert.equal(readWallTheme(), 'auto');
	store.set('dumbmonit-wall-theme', 'neon');
	assert.equal(readWallTheme(), 'auto');
});

test('Auto is not stored; Day and Night round-trip', () => {
	writeWallTheme('light');
	assert.equal(readWallTheme(), 'light');
	writeWallTheme('auto');
	assert.equal(store.has('dumbmonit-wall-theme'), false);
});

test('a ?theme= link forces Day or Night, OLED lands on Night, Auto forces nothing', () => {
	assert.equal(parseForcedWallTheme('light'), 'light');
	assert.equal(parseForcedWallTheme('oled'), 'dark');
	assert.equal(parseForcedWallTheme('auto'), null);
	assert.equal(parseForcedWallTheme('x'), null);
	assert.equal(parseForcedWallTheme(null), null);
});
