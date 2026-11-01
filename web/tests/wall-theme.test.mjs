// The wall's theme: Night by default, Day on request; the retired OLED and Auto read as Night. Run: npm test
import { test, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { readWallTheme, writeWallTheme, parseForcedWallTheme, DEFAULT_WALL_THEME } from '../src/lib/components/wall/wallTheme.ts';

const store = new Map();
globalThis.localStorage = {
	getItem: (k) => (store.has(k) ? store.get(k) : null),
	setItem: (k, v) => void store.set(k, String(v)),
	removeItem: (k) => void store.delete(k)
};
beforeEach(() => store.clear());

test('nothing saved, or junk, is Night', () => {
	assert.equal(DEFAULT_WALL_THEME, 'dark');
	assert.equal(readWallTheme(), 'dark');
	store.set('dumbmonit-wall-theme', 'neon');
	assert.equal(readWallTheme(), 'dark');
});

test('a saved OLED or Auto choice reads as Night', () => {
	store.set('dumbmonit-wall-theme', 'oled');
	assert.equal(readWallTheme(), 'dark');
	store.set('dumbmonit-wall-theme', 'auto');
	assert.equal(readWallTheme(), 'dark');
});

test('Day and Night round-trip', () => {
	writeWallTheme('light');
	assert.equal(readWallTheme(), 'light');
	writeWallTheme('dark');
	assert.equal(readWallTheme(), 'dark');
});

test('a ?theme= link forces Day or Night (by either name), OLED lands on Night, Auto forces nothing', () => {
	assert.equal(parseForcedWallTheme('light'), 'light');
	assert.equal(parseForcedWallTheme('day'), 'light');
	assert.equal(parseForcedWallTheme('night'), 'dark');
	assert.equal(parseForcedWallTheme('oled'), 'dark');
	assert.equal(parseForcedWallTheme('auto'), null);
	assert.equal(parseForcedWallTheme('x'), null);
	assert.equal(parseForcedWallTheme(null), null);
});
