// Firewall live map: arcs, disc sizes, the dominant action, country names. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { arcPoints, countryName, discRadius, dominant } from '../src/lib/map/live.ts';

test('an arc starts and ends where asked, and bends', () => {
	const points = arcPoints([50, 8], [46.5, 6.6], 10);
	assert.equal(points.length, 11);
	assert.deepEqual(points[0], [50, 8]);
	assert.deepEqual(points.at(-1), [46.5, 6.6]);
	const mid = points[5];
	assert.ok(Math.abs(mid[0] - 48.25) > 0.1 || Math.abs(mid[1] - 7.3) > 0.1, 'the middle is off the straight line');
});

test('an arc takes the short way across the date line', () => {
	const points = arcPoints([35.7, 139.7], [37.8, -122.4]);
	const lons = points.map((p) => p[1]);
	assert.ok(lons[0] < -180, 'Tokyo is moved a world to the west');
	assert.equal(lons.at(-1), -122.4);
	for (let i = 1; i < lons.length; i++) assert.ok(Math.abs(lons[i] - lons[i - 1]) < 30, 'no jump across the map');
});

test('discs grow slowly and stay bounded', () => {
	assert.equal(discRadius(0), 4);
	assert.ok(discRadius(100) > discRadius(10));
	assert.equal(discRadius(1e9), 26);
});

test('blocked wins a tie, nothing at all is not blocked', () => {
	assert.equal(dominant({ blocked: 3, passed: 3, other: 0 }), 'block');
	assert.equal(dominant({ blocked: 1, passed: 5, other: 0 }), 'pass');
	assert.equal(dominant({ blocked: 0, passed: 0, other: 0 }), 'pass');
});

test('country names come from Intl, codes stay for the rest', () => {
	const names = new Intl.DisplayNames(['fr'], { type: 'region' });
	assert.equal(countryName('CH', names), 'Suisse');
	assert.equal(countryName('local', names), 'local');
	assert.equal(countryName('ZZ', { of: () => { throw new RangeError('no'); } }), 'ZZ');
});
