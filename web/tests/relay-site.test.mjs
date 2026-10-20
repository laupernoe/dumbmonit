// A relay's site and the world-map location that stands for it. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { locationForSite } from '../src/lib/components/devices/relay/relay-site.ts';

const locations = [
	{ id: 1, name: 'Headquarters', site: '' },
	{ id: 2, name: 'Lyon office', site: 'Lyon' },
	{ id: 3, name: 'Annexe', site: '' }
];

test('the site of a location wins, without case or spaces', () => {
	assert.equal(locationForSite(locations, '  lyon ')?.id, 2);
});

test('a location named like the site is the fallback', () => {
	assert.equal(locationForSite(locations, 'annexe')?.id, 3);
});

test('no site, or an unknown one, is no location', () => {
	assert.equal(locationForSite(locations, null), undefined);
	assert.equal(locationForSite(locations, '   '), undefined);
	assert.equal(locationForSite(locations, 'Paris'), undefined);
});
