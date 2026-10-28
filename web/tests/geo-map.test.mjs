// Generic world map: choropleth steps, country ranking, cloud regions. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { LEVELS, choroplethLevel, cloudMetrics, cloudPlaces, levelFloors, rankCountries, regionFor, regionKey } from '../src/lib/map/geo.ts';

const table = JSON.parse(readFileSync(new URL('../src/lib/map/regions.json', import.meta.url), 'utf8'));

test('nothing is step 0, the busiest country the last step', () => {
	assert.equal(choroplethLevel(0, 100), 0);
	assert.equal(choroplethLevel(-3, 100), 0);
	assert.equal(choroplethLevel(5, 0), 0);
	assert.equal(choroplethLevel(Number.NaN, 10), 0);
	assert.equal(choroplethLevel(100, 100), LEVELS);
	assert.equal(choroplethLevel(1, 100), 1);
});

test('steps grow with the value and stay within bounds', () => {
	let previous = 0;
	for (const value of [1, 3, 10, 30, 100, 300, 1000]) {
		const level = choroplethLevel(value, 1000);
		assert.ok(level >= previous, `${value} → ${level}`);
		assert.ok(level >= 1 && level <= LEVELS);
		previous = level;
	}
	// A value above the max (stale max) never goes past the last step.
	assert.equal(choroplethLevel(5000, 1000), LEVELS);
});

test('the scale is logarithmic: one huge country leaves room for the others', () => {
	assert.ok(choroplethLevel(30, 10_000) >= 2, 'thirty hits still show above the palest step');
});

test('legend floors start at 1, rise, and never pass the max', () => {
	const floors = levelFloors(1000);
	assert.equal(floors.length, LEVELS);
	assert.equal(floors[0], 1);
	for (let i = 1; i < floors.length; i++) assert.ok(floors[i] >= floors[i - 1]);
	assert.ok(floors.at(-1) <= 1000);
	assert.deepEqual(levelFloors(0), []);
	// Each floor falls in its own step.
	floors.forEach((floor, i) => assert.ok(choroplethLevel(floor, 1000) >= i + 1 || floor === 1, `${floor}`));
});

test('countries are ranked busiest first, codes only, zeros left out', () => {
	assert.deepEqual(rankCountries({ FR: 2, CN: 9, US: 9, XX: 0, de: 4, local: 3, BR: 0 }), [
		{ code: 'CN', value: 9 },
		{ code: 'US', value: 9 },
		{ code: 'FR', value: 2 }
	]);
});

test('region keys: OVHcloud datacenters and OpenStack zones, Azure display names', () => {
	assert.equal(regionKey('ovh', 'gra7'), 'gra');
	assert.equal(regionKey('ovh', 'Region OpenStack: os-sbg5'), 'sbg');
	assert.equal(regionKey('ovh', 'RBX'), 'rbx');
	assert.equal(regionKey('azure', 'West Europe'), 'westeurope');
	assert.equal(regionKey('aws', ' eu-west-3 '), 'eu-west-3');
	assert.equal(regionKey('hetzner', 'fsn1'), 'fsn1');
});

test('the region table answers known regions and nothing else', () => {
	assert.equal(regionFor(table, 'aws', 'eu-west-3')?.name, 'Paris');
	assert.equal(regionFor(table, 'hetzner', 'hel1')?.code, 'hel1');
	assert.equal(regionFor(table, 'ovh', 'bhs5')?.name, 'Beauharnois');
	assert.equal(regionFor(table, 'azure', 'switzerlandnorth')?.name, 'Switzerland North');
	assert.equal(regionFor(table, 'aws', 'mars-north-1'), null);
	assert.equal(regionFor(table, 'aws', ''), null);
	assert.equal(regionFor(table, 'nowhere', 'eu-west-3'), null);
	assert.equal(regionFor(table, 'aws', 'constructor'), null, 'no prototype key leaks through');
});

test('every region of the table has a name and coordinates in range', () => {
	let count = 0;
	for (const [provider, regions] of Object.entries(table)) {
		for (const [code, region] of Object.entries(regions)) {
			count++;
			assert.ok(region.name, `${provider}/${code}`);
			assert.ok(Math.abs(region.lat) <= 90 && Math.abs(region.lon) <= 180, `${provider}/${code}`);
		}
	}
	assert.ok(count > 100, `${count} regions`);
});

test('cloud places: one per known region, problems first, the unknown counted apart', () => {
	const series = [
		{ metric: { __name__: 'dumbmonit_hetzner_server_info', location: 'fsn1', status: 'running' } },
		{ metric: { __name__: 'dumbmonit_hetzner_server_info', location: 'fsn1', status: 'running' } },
		{ metric: { __name__: 'dumbmonit_hetzner_server_info', location: 'hel1', status: 'off' } },
		{ metric: { __name__: 'dumbmonit_hetzner_server_info', location: 'mars1', status: 'running' } },
		{ metric: { __name__: 'dumbmonit_hetzner_servers' } }
	];
	const { places, unknown } = cloudPlaces(table, 'hetzner', series);
	assert.deepEqual(
		places.map((p) => [p.code, p.total, p.down]),
		[
			['hel1', 1, 1],
			['fsn1', 2, 0]
		]
	);
	assert.equal(unknown, 1);
});

test('OVHcloud places read both VPS zones and dedicated datacenters', () => {
	const series = [
		{ metric: { __name__: 'dumbmonit_ovh_vps_info', zone: 'Region OpenStack: os-gra7', state: 'running' } },
		{ metric: { __name__: 'dumbmonit_ovh_dedicated_info', datacenter: 'gra3', state: 'ok', power: 'poweron' } },
		{ metric: { __name__: 'dumbmonit_ovh_dedicated_info', datacenter: 'rbx8', state: 'ok', power: 'poweroff' } }
	];
	const { places } = cloudPlaces(table, 'ovh', series);
	assert.deepEqual(
		places.map((p) => [p.code, p.total, p.down]),
		[
			['rbx', 1, 1],
			['gra', 2, 0]
		]
	);
	assert.deepEqual(cloudMetrics('ovh'), ['dumbmonit_ovh_vps_info', 'dumbmonit_ovh_dedicated_info']);
	assert.deepEqual(cloudMetrics('infomaniak'), []);
});

test("an account's own region is a place even with nothing listed", () => {
	const { places } = cloudPlaces(table, 'aws', [], 'eu-central-2');
	assert.deepEqual(
		places.map((p) => [p.name, p.total]),
		[['Zurich', 0]]
	);
});
