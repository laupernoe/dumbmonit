// Subnet grid of the network watch. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildGrid, countStates, stateOf, subnetAddresses } from '../src/lib/components/device-form/netscan-grid.ts';

const device = (ip, o = {}) => ({
	id: 1,
	schedule_id: 1,
	ip,
	mac: null,
	hostname: null,
	vendor: null,
	known: true,
	ignored: false,
	mac_change_from: null,
	first_seen: '2026-01-01T00:00:00',
	last_seen: '2026-01-02T00:00:00',
	ports: [],
	...o
});

test('a /24 has 254 host addresses, without network and broadcast', () => {
	const list = subnetAddresses('192.168.1.0/24');
	assert.equal(list.length, 254);
	assert.equal(list[0], '192.168.1.1');
	assert.equal(list.at(-1), '192.168.1.254');
});

test('a host address is trimmed to its network, and a /31 keeps both addresses', () => {
	assert.deepEqual(subnetAddresses('10.0.0.77/30'), ['10.0.0.77', '10.0.0.78']);
	assert.deepEqual(subnetAddresses('10.0.0.4/31'), ['10.0.0.4', '10.0.0.5']);
});

test('over 1024 addresses, IPv6 and garbage draw no grid', () => {
	assert.equal(subnetAddresses('10.0.0.0/21'), null);
	assert.equal(subnetAddresses('fd00::/64'), null);
	assert.equal(subnetAddresses('nope'), null);
	assert.equal(subnetAddresses('10.0.0.300/24'), null);
	assert.equal(subnetAddresses('10.0.0.0/22').length, 1022);
});

test('states: new, MAC changed, ignored, known, absent', () => {
	const seen = '2026-01-03T00:00:00';
	assert.equal(stateOf(device('a', { known: false }), seen, false), 'new');
	assert.equal(stateOf(device('a', { known: false, mac_change_from: 'aa' }), seen, false), 'mac_changed');
	assert.equal(stateOf(device('a', { ignored: true }), seen, false), 'ignored');
	assert.equal(stateOf(device('a'), seen, false), 'absent');
	assert.equal(stateOf(device('a', { last_seen: seen }), seen, false), 'known');
	// Never scanned, or scanning right now: nobody is absent yet.
	assert.equal(stateOf(device('a'), null, false), 'known');
	assert.equal(stateOf(device('a'), seen, true), 'known');
});

test('one cell per address, free where nothing was seen, other networks left out', () => {
	const cells = buildGrid(
		'10.0.0.0/29',
		[
			device('10.0.0.2', { id: 1, last_seen: '2026-01-03T00:00:00' }),
			device('10.0.0.3', { id: 2, known: false }),
			device('10.0.0.9', { id: 3 }),
			device('10.0.0.4', { id: 4, schedule_id: 2 })
		],
		1,
		'2026-01-03T00:00:00',
		false
	);
	assert.equal(cells.length, 6);
	const counts = countStates(cells);
	assert.deepEqual(counts, { mac_changed: 0, new: 1, known: 1, ignored: 0, absent: 0, free: 4 });
	assert.equal(cells.find((c) => c.ip === '10.0.0.2').label, '2');
	assert.equal(cells.find((c) => c.ip === '10.0.0.3').device.id, 2);
});

test('a pending device speaks for an address shared with an old one', () => {
	const cells = buildGrid(
		'10.0.0.0/29',
		[
			device('10.0.0.2', { id: 1, mac: 'aa', last_seen: '2026-01-05T00:00:00' }),
			device('10.0.0.2', { id: 2, mac: 'bb', known: false, mac_change_from: 'aa', last_seen: '2026-01-04T00:00:00' })
		],
		1,
		null,
		false
	);
	const cell = cells.find((c) => c.ip === '10.0.0.2');
	assert.equal(cell.device.id, 2);
	assert.equal(cell.state, 'mac_changed');
});

test('no grid for an IPv6 network', () => {
	assert.equal(buildGrid('fd00::/64', [], 1, null, false), null);
});
