// One port model for the switch panel, whatever collector stored the series. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	SOURCES,
	blocks,
	foldMikrotik,
	foldSnmp,
	foldUnifi,
	formatBitRate,
	heatmapCells,
	linkCount,
	portTone,
	quote,
	shortLabels,
	shortSpeed
} from '../src/lib/components/devices/switch/ports.ts';

const serie = (name, labels, value) => ({
	metric: { __name__: name, target: '7', ...labels },
	values: [[1_790_000_000, String(value)]]
});

// What the SNMP collector stores for the synthetic PoE switch of
// crates/collectors/src/snmp/fixtures/poe-switch-synthetic.snmprec.
const SNMP_STATE = [
	serie('dumbmonit_switch_port_status', { index: '1', ifname: 'ge1', ifalias: 'Hall AP', admin: '1' }, 1),
	serie('dumbmonit_switch_port_status', { index: '2', ifname: 'ge2', admin: '1' }, 1),
	serie('dumbmonit_switch_port_status', { index: '3', ifname: 'ge3', admin: '1' }, 2),
	serie('dumbmonit_switch_port_status', { index: '4', ifname: 'ge4', admin: '2' }, 2),
	serie('dumbmonit_if_speed_bps', { index: '1', ifname: 'ge1' }, 1e9),
	serie('dumbmonit_if_speed_bps', { index: '2', ifname: 'ge2' }, 1e8),
	serie('dumbmonit_ether_duplex_status', { index: '2', ifname: 'ge2' }, 2),
	serie('dumbmonit_bridge_port_info', { index: '1' }, 1),
	serie('dumbmonit_bridge_port_info', { index: '2' }, 2),
	serie('dumbmonit_bridge_port_info', { index: '3' }, 3),
	serie('dumbmonit_bridge_port_info', { index: '4' }, 4),
	serie('dumbmonit_port_pvid', { index: '1' }, 10),
	serie('dumbmonit_port_pvid', { index: '3' }, 20),
	serie('dumbmonit_lldp_local_port_info', { index: '1', port_id: 'ge1' }, 5),
	serie(
		'dumbmonit_lldp_neighbor_info',
		{ index: '0.1.1', remote_sysname: 'ap-hall', remote_port: 'eth0', remote_chassis: '02:00:00:00:0a:01' },
		4
	),
	serie('dumbmonit_poe_port_status', { index: '1.1', admin: '1', class: '3' }, 3),
	serie('dumbmonit_poe_port_status', { index: '1.2', admin: '1', class: '1' }, 2),
	serie('dumbmonit_poe_port_status', { index: '1.3', admin: '1', class: '1' }, 4),
	serie('dumbmonit_poe_port_status', { index: '1.4', admin: '2', class: '1' }, 1),
	serie('dumbmonit_poe_power_budget_watts', { index: '1' }, 60),
	serie('dumbmonit_poe_power_consumed_watts', { index: '1' }, 7)
];

test('SNMP: every front-panel port, joined on the ifIndex', () => {
	const day = [serie('', { index: '2' }, 41.6)];
	const [unit] = foldSnmp(SNMP_STATE, day);
	assert.equal(unit.ports.length, 4);
	const [ge1, ge2, ge3, ge4] = unit.ports;
	assert.deepEqual(
		unit.ports.map((p) => p.label),
		['1', '2', '3', '4']
	);
	assert.equal(ge1.link, 'up');
	assert.equal(ge1.speedBps, 1e9);
	assert.equal(ge1.alias, 'Hall AP');
	assert.equal(ge1.vlan, 10);
	assert.deepEqual(ge1.neighbor, { name: 'ap-hall', port: 'eth0', protocol: 'lldp' });
	assert.equal(ge1.poe, 'delivering');
	assert.equal(ge1.poeClass, 2);
	assert.equal(ge2.halfDuplex, true);
	assert.equal(ge2.errors, 42);
	assert.equal(ge2.poe, 'searching');
	assert.equal(ge3.link, 'down');
	assert.equal(ge3.speedBps, null);
	assert.equal(ge3.vlan, 20);
	assert.equal(ge3.poe, 'fault');
	assert.equal(ge4.link, 'disabled');
	assert.equal(ge4.poe, 'off');
	assert.deepEqual(unit.poeBudget, { used: 7, total: 60 });
});

test('SNMP: without a bridge table, unnamed interfaces are not ports', () => {
	const state = [
		serie('dumbmonit_switch_port_status', { index: '1', ifname: 'swp00', admin: '1' }, 1),
		serie('dumbmonit_switch_port_status', { index: '20000', admin: '1' }, 1)
	];
	const [unit] = foldSnmp(state, []);
	assert.deepEqual(
		unit.ports.map((p) => p.name),
		['swp00']
	);
	assert.deepEqual(foldSnmp([], []), [], 'not a switch: no unit, the panel stays hidden');
});

test('SNMP: CDP neighbours sit on the first arc of their index', () => {
	const state = [
		serie('dumbmonit_switch_port_status', { index: '10101', ifname: 'Gi1/0/1', admin: '1' }, 1),
		serie('dumbmonit_cdp_neighbor_info', { index: '10101.3', remote_sysname: 'core', remote_port: 'Te1/1/1' }, 1)
	];
	const [unit] = foldSnmp(state, []);
	assert.deepEqual(unit.ports[0].neighbor, { name: 'core', port: 'Te1/1/1', protocol: 'cdp' });
});

test('UniFi: one unit per switch, PoE and LLDP from port_table', () => {
	const sw = { device: 'Office switch', mac: 'f4:e2:c6:00:00:02' };
	const state = [
		serie('dumbmonit_unifi_port_poe_watts', { ...sw, port: '2', port_name: 'Hall AP' }, 6.42),
		serie('dumbmonit_unifi_port_enabled', { ...sw, port: '2', port_name: 'Hall AP' }, 1),
		serie('dumbmonit_unifi_port_up', { ...sw, port: '2', port_name: 'Hall AP' }, 1),
		serie('dumbmonit_unifi_port_speed_bps', { ...sw, port: '2', port_name: 'Hall AP' }, 1e9),
		serie('dumbmonit_unifi_port_poe_enabled', { ...sw, port: '2', port_name: 'Hall AP' }, 1),
		serie('dumbmonit_unifi_port_neighbor_info', { ...sw, port: '2', remote_chassis: '74:83:c2:00:00:03', remote_port: 'eth0' }, 1),
		serie('dumbmonit_unifi_port_enabled', { ...sw, port: '10', port_name: 'Port 10' }, 1),
		serie('dumbmonit_unifi_port_up', { ...sw, port: '10', port_name: 'Port 10' }, 0),
		serie('dumbmonit_unifi_port_poe_enabled', { ...sw, port: '10', port_name: 'Port 10' }, 1),
		serie('dumbmonit_unifi_port_enabled', { ...sw, port: '4', port_name: 'Port 4' }, 0),
		serie('dumbmonit_unifi_port_up', { device: 'Other', mac: 'aa', port: '1' }, 1)
	];
	const day = [serie('', { mac: sw.mac, port: '2' }, 3)];
	const units = foldUnifi(state, day);
	assert.deepEqual(
		units.map((u) => u.name),
		['Office switch', 'Other']
	);
	const office = units[0];
	assert.deepEqual(
		office.ports.map((p) => p.key),
		['2', '4', '10'],
		'numeric order, not text order'
	);
	const [hall, four, ten] = office.ports;
	assert.equal(hall.name, 'Hall AP');
	assert.equal(hall.link, 'up');
	assert.equal(hall.poe, 'delivering');
	assert.equal(hall.poeWatts, 6.42);
	assert.equal(hall.errors, 3);
	assert.equal(hall.neighbor.name, '74:83:c2:00:00:03');
	assert.equal(four.link, 'disabled');
	assert.equal(ten.link, 'down');
	assert.equal(ten.poe, 'searching');
});

test('MikroTik: the ether interfaces, with their errors of the day', () => {
	const state = [
		serie('dumbmonit_mikrotik_interface_running', { interface: 'ether2', type: 'ether' }, 0),
		serie('dumbmonit_mikrotik_interface_running', { interface: 'ether10', type: 'ether' }, 1),
		serie('dumbmonit_mikrotik_interface_running', { interface: 'ether1', type: 'ether' }, 1)
	];
	const [unit] = foldMikrotik(state, [serie('', { interface: 'ether1' }, 5)]);
	assert.deepEqual(
		unit.ports.map((p) => p.label),
		['1', '2', '10']
	);
	assert.equal(unit.ports[0].errors, 5);
	assert.equal(unit.ports[1].link, 'down');
	assert.deepEqual(foldMikrotik([], []), []);
});

test('tones carry the state; never colour alone, the word comes from the panel', () => {
	const base = { link: 'up', errors: 0, halfDuplex: false, poe: null };
	assert.equal(portTone(base), 'signal');
	assert.equal(portTone({ ...base, errors: 3 }), 'advisory');
	assert.equal(portTone({ ...base, halfDuplex: true }), 'advisory');
	assert.equal(portTone({ ...base, link: 'down' }), 'ghost');
	assert.equal(portTone({ ...base, link: 'down', poe: 'fault' }), 'advisory');
	assert.equal(portTone({ ...base, link: 'disabled' }), 'info');
});

test('labels, speeds and blocks', () => {
	const ports = [{ name: 'Gi1/0/1' }, { name: 'Te1/1/1' }];
	shortLabels(ports);
	assert.deepEqual(
		ports.map((p) => p.label),
		['Gi1/0/1', 'Te1/1/1'],
		'two ports would read "1": full names'
	);
	assert.equal(shortSpeed(1e9), '1G');
	assert.equal(shortSpeed(2.5e9), '2.5G');
	assert.equal(shortSpeed(1e8), '100M');
	assert.equal(shortSpeed(0), null);
	assert.equal(formatBitRate(1_234_000), '1.2 Mbit/s');
	assert.equal(formatBitRate(null), '—');
	assert.deepEqual(
		blocks([1, 2, 3, 4, 5], 2),
		[[1, 2], [3, 4], [5]]
	);
	assert.deepEqual(linkCount([{ link: 'up' }, { link: 'down' }, { link: 'disabled' }]), { up: 1, total: 2 });
});

test('heatmap: one row per known port, one column per sample time', () => {
	const ports = [{ key: '1', name: 'ge1' }, { key: '2', name: 'ge2' }];
	const series = [
		{ metric: { index: '2' }, values: [[100, '5'], [200, '7']] },
		{ metric: { index: '1' }, values: [[200, '3']] },
		{ metric: { index: '99' }, values: [[100, '1']] }
	];
	const heat = heatmapCells(series, ports, (m) => m.index);
	assert.deepEqual(
		heat.rows.map((r) => r.key),
		['1', '2']
	);
	assert.deepEqual(heat.cols, ['100', '200']);
	assert.equal(heat.cells.find((c) => c.row === '1' && c.col === '100').value, null);
	assert.equal(heat.cells.find((c) => c.row === '2' && c.col === '200').value, 7);
});

test('queries stay on the target and escape label values', () => {
	assert.equal(quote('a"b\\c'), '"a\\"b\\\\c"');
	for (const kind of ['snmp', 'unifi', 'mikrotik']) {
		const source = SOURCES[kind];
		for (const q of source.instant(7)) assert.match(q, /target="7"/);
		const { in: rx, out: tx } = source.traffic(7, 'aa:bb', 'p"1');
		assert.match(rx, /target="7"/);
		assert.match(tx, /\* 8$/);
		assert.ok(rx.includes('"p\\"1"'));
		assert.match(source.heatmap(7, 'aa:bb'), /target="7"/);
	}
	assert.equal(SOURCES.omada, undefined, 'no port data collected for Omada yet');
});
