// World map: the state of a location from its monitors, and typed coordinates. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseCoordinates, pinCount, sortByVerdict, tally, verdict, wrapLongitude } from '../src/lib/map/aggregate.ts';

test('a location is as bad as its worst monitor', () => {
	assert.equal(verdict(tally([])), 'empty');
	assert.equal(verdict(tally(['online', 'online'])), 'ok');
	assert.equal(verdict(tally(['online', 'pending'])), 'warn');
	assert.equal(verdict(tally(['misconfigured', 'offline', 'online'])), 'down');
	assert.equal(verdict(tally(['down'])), 'down');
	assert.equal(verdict(tally(['disabled', 'unknown'])), 'idle', 'paused and never probed say nothing');
	assert.equal(verdict(tally(['disabled', 'online'])), 'ok');
});

test('the pin shows problems out of the total, or the total alone', () => {
	assert.equal(pinCount(tally(['online', 'online', 'online'])), '3');
	assert.equal(pinCount(tally(['online', 'down', 'pending', 'disabled'])), '2/4');
	const t = tally(['down', 'offline', 'misconfigured', 'online', 'disabled']);
	assert.deepEqual(t, { total: 5, down: 2, warn: 1, ok: 1, idle: 1 });
});

test('the list reads worst first, then by name', () => {
	const sorted = sortByVerdict([
		{ name: 'Paris', verdict: 'ok' },
		{ name: 'Annexe', verdict: 'empty' },
		{ name: 'Zurich', verdict: 'down' },
		{ name: 'Berne', verdict: 'ok' },
		{ name: 'Genève', verdict: 'warn' }
	]);
	assert.deepEqual(
		sorted.map((s) => s.name),
		['Zurich', 'Genève', 'Berne', 'Paris', 'Annexe']
	);
});

test('a click on a wrapped world comes back into range', () => {
	assert.equal(wrapLongitude(6.6), 6.6);
	assert.equal(wrapLongitude(366.6).toFixed(1), '6.6');
	assert.equal(wrapLongitude(-190), 170);
	assert.equal(wrapLongitude(180), 180);
	assert.equal(wrapLongitude(-180), -180);
});

test('coordinates are read as typed or refused', () => {
	assert.deepEqual(parseCoordinates('46.52, 6.63'), { latitude: 46.52, longitude: 6.63 });
	assert.deepEqual(parseCoordinates(' -33.86 151.21 '), { latitude: -33.86, longitude: 151.21 });
	assert.equal(parseCoordinates('91, 0'), null);
	assert.equal(parseCoordinates('0, 181'), null);
	assert.equal(parseCoordinates('Lausanne'), null);
	assert.equal(parseCoordinates('1, 2, 3'), null);
});
