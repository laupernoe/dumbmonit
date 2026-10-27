// "Where is it?": what the add flow proposes for a site. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	defaultsFor,
	defaultsParams,
	discoveredFor,
	hostOf,
	payloadFor,
	relaysOf,
	suggestedKinds,
	visibleLocations
} from '../src/lib/components/device-form/add-by-site.ts';

const collectors = ['http', 'ping', 'tcp', 'snmp', 'proxmox', 'agent'].map((kind) => ({ kind, label: kind }));
const target = (id, o = {}) => ({ id, name: `t${id}`, address: `10.0.0.${id}`, kind: 'ping', group_name: '', via_agent: null, client_id: 1, ...o });
const location = (id, o = {}) => ({ id, name: `L${id}`, site: '', main_site: false, members: [], ...o });
const relay = (id, o = {}) => ({ id, name: `relay${id}`, site: null, relay: true, relayed: 0, ...o });
const device = (id, ip, o = {}) => ({ id, schedule_id: 1, ip, hostname: null, vendor: null, known: false, ports: [], ...o });

test('relays: declared for the site or already used by its devices, busiest first', () => {
	const paris = location(1, { site: 'Paris', members: [{ target_id: 10, source: 'assigned' }] });
	const targets = [target(10, { via_agent: 7 })];
	const relays = [relay(5, { site: 'paris', relayed: 1 }), relay(6, { site: 'Lyon' }), relay(7, { relayed: 9 }), relay(8, { site: 'Paris', relay: false })];
	assert.deepEqual(relaysOf(paris, relays, targets).map((r) => r.id), [7, 5]);
	assert.deepEqual(relaysOf(null, relays, targets), []);
});

test('defaults: location, busiest relay and the folder most used on the site', () => {
	const loc = location(3, { site: 'Lyon', members: [{ target_id: 1, source: 'site' }, { target_id: 2, source: 'site' }, { target_id: 3, source: 'site' }] });
	const targets = [target(1, { group_name: 'Offices' }), target(2, { group_name: 'Offices' }), target(3, { group_name: 'Lab' }), target(4, { group_name: 'Elsewhere' })];
	const d = defaultsFor(loc, [relay(9, { site: 'Lyon' })], targets);
	assert.deepEqual(d, { location_id: 3, via_agent: 9, folder: 'Offices' });
	assert.deepEqual(defaultsParams(d), { location: '3', via_agent: '9', folder: 'Offices' });
	assert.deepEqual(defaultsParams(defaultsFor(null, [], [])), {});
});

test('MSP: sites of other clients are hidden, empty ones stay', () => {
	const a = location(1, { members: [{ target_id: 1, source: 'assigned' }] });
	const b = location(2, { members: [{ target_id: 2, source: 'assigned' }] });
	const empty = location(3);
	const targets = [target(1, { client_id: 1 }), target(2, { client_id: 2 })];
	assert.deepEqual(visibleLocations([a, b, empty], targets, 1).map((l) => l.id), [1, 3]);
	assert.equal(visibleLocations([a, b, empty], targets, null).length, 3);
});

test('kinds: the site\'s own first, then the common ones, only known kinds', () => {
	const loc = location(1, { members: [{ target_id: 1, source: 'assigned' }, { target_id: 2, source: 'assigned' }] });
	const targets = [target(1, { kind: 'proxmox' }), target(2, { kind: 'agent' }), target(3, { kind: 'snmp' }), target(4, { kind: 'unknown' })];
	assert.deepEqual(suggestedKinds(loc, targets, collectors), ['proxmox', 'http', 'ping', 'tcp', 'snmp']);
	assert.equal(suggestedKinds(null, targets, collectors)[0], 'proxmox');
});

test('hostOf strips scheme, port and path', () => {
	assert.equal(hostOf('https://NAS.lan:5001/x'), 'nas.lan');
	assert.equal(hostOf('10.0.0.5'), '10.0.0.5');
	assert.equal(hostOf('[::1]:80'), '::1');
});

test('discovered: scans of the site\'s relay, unmonitored only, with a sensible kind', () => {
	const loc = location(1, { site: 'Paris' });
	const relays = [relay(7, { site: 'Paris' })];
	const schedules = [{ id: 1, agent_id: 7 }, { id: 2, agent_id: null }];
	const inventory = [
		device(1, '192.168.1.10', { ports: [{ port: 161, protocol: 'udp' }] }),
		device(2, '192.168.1.11', { ports: [{ port: 443, protocol: 'tcp' }] }),
		device(3, '192.168.1.12'),
		device(4, '192.168.1.13', { schedule_id: 2 })
	];
	const targets = [target(1, { address: 'https://192.168.1.12:8443' })];
	const found = discoveredFor(loc, inventory, schedules, relays, new Set([7]), targets, collectors);
	assert.deepEqual(found.map((f) => [f.device.ip, f.kind, f.direct]), [['192.168.1.10', 'snmp', false], ['192.168.1.11', 'http', true]]);
	assert.equal(found[1].address, 'https://192.168.1.11');
	// "No site": only what no site's relay claims (the server's own scans).
	const none = discoveredFor(null, inventory, schedules, [], new Set([7]), [], collectors);
	assert.deepEqual(none.map((f) => f.device.ip), ['192.168.1.13']);
	assert.deepEqual(payloadFor(found[1], { location_id: 1, via_agent: 7, folder: 'Paris' }), {
		name: '192.168.1.11', address: 'https://192.168.1.11', kind: 'http', via_agent: 7, location_id: 1, group_name: 'Paris'
	});
});
