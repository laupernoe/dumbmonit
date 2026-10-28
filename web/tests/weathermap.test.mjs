// Weathermap of the topology map: load bands, rates, ranking, curves. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildGraph, layoutMap, listRows } from '../src/lib/components/topology/layout.ts';
import { bandOf, formatRate, linkLoad, mostLoaded, sparkPaths, widthFor } from '../src/lib/components/topology/traffic.ts';

const traffic = (extra = {}) => ({
	measured_on: 't1',
	ports: ['Gi24'],
	bps_in: 100e6,
	bps_out: 10e6,
	speed_bps: 1e9,
	utilization_in: 0.1,
	utilization_out: 0.01,
	errors_per_sec: 0,
	...extra
});

test('bands follow the legend: 0-25, 25-50, 50-75, 75-100 %', () => {
	assert.equal(bandOf(null), 'unknown');
	assert.equal(bandOf(0.4), 'idle');
	assert.equal(bandOf(10), 'low');
	assert.equal(bandOf(25), 'mid');
	assert.equal(bandOf(50), 'high');
	assert.equal(bandOf(75), 'full');
	assert.equal(bandOf(100), 'full');
});

test('the busiest direction gives the percentage and the rate', () => {
	const load = linkLoad(traffic(), 't1');
	assert.equal(load.pct, 10);
	assert.equal(load.bps, 100e6);
	assert.equal(load.band, 'low');
	assert.equal(load.errors, false);
});

test('flow direction follows the measured side', () => {
	// Mostly received by the `from` side: the traffic comes back along the path.
	assert.equal(linkLoad(traffic(), 't1').forward, false);
	// The same figures measured on the `to` side: received by `to`, so forward.
	assert.equal(linkLoad(traffic(), 't2').forward, true);
	// Mostly sent by the `from` side: forward.
	assert.equal(linkLoad(traffic({ bps_in: 1e6, bps_out: 50e6 }), 't1').forward, true);
	// Nothing flows: no direction.
	assert.equal(linkLoad(traffic({ bps_in: 10, bps_out: 20 }), 't1').forward, null);
});

test('without a port speed there is a rate but no percentage', () => {
	const load = linkLoad(traffic({ speed_bps: null, utilization_in: null, utilization_out: null }), 't1');
	assert.equal(load.pct, null);
	assert.equal(load.band, 'unknown');
	assert.equal(load.width, widthFor(null));
});

test('a port that loses frames is flagged', () => {
	assert.equal(linkLoad(traffic({ errors_per_sec: 0.2 }), 't1').errors, true);
	assert.equal(linkLoad(traffic({ errors_per_sec: 0.01 }), 't1').errors, false);
});

test('width grows with the load and stays bounded', () => {
	assert.ok(widthFor(0) < widthFor(50));
	assert.ok(widthFor(50) < widthFor(100));
	assert.equal(widthFor(250), widthFor(100));
});

test('rates use decimal units', () => {
	assert.equal(formatRate(0, 'en'), '0 b/s');
	assert.equal(formatRate(412e6, 'en'), '412 Mb/s');
	assert.equal(formatRate(1.5e9, 'en'), '1.5 Gb/s');
	assert.equal(formatRate(2_500, 'en'), '2.5 kb/s');
});

test('the busiest links come first and links without traffic are left out', () => {
	const edges = [
		{ from: 'a', to: 'b', traffic: traffic({ utilization_in: 0.2, utilization_out: 0 }) },
		{ from: 'a', to: 'c' },
		{ from: 'a', to: 'd', traffic: traffic({ utilization_in: 0.9, utilization_out: 0 }) },
		{ from: 'a', to: 'e', traffic: traffic({ speed_bps: null, utilization_in: null, utilization_out: null, bps_in: 5e9 }) }
	];
	assert.deepEqual(mostLoaded(edges).map((r) => r.edge.to), ['d', 'b', 'e']);
	assert.equal(mostLoaded(edges, 1).length, 1);
});

test('the curve shares one scale between both directions', () => {
	const none = sparkPaths([], 100, 20);
	assert.equal(none.inbound, '');
	const { inbound, outbound, max } = sparkPaths(
		[
			{ t: 0, bps_in: 0, bps_out: 50 },
			{ t: 60, bps_in: 100, bps_out: 0 }
		],
		100,
		20
	);
	assert.equal(max, 100);
	assert.equal(inbound, 'M0.0,20.0 L100.0,0.0');
	assert.equal(outbound, 'M0.0,10.0 L100.0,20.0');
});

function node(id, state = 'online') {
	return { id, targetId: Number(id.slice(1)), hostId: null, name: id, kind: 'snmp', group: '', site: null, siteSource: null, state };
}

test('a link behind a down device is greyed out, along with the dependants the cascade already marks', () => {
	const nodes = [node('t1', 'offline'), node('t2'), node('t3')];
	const edges = [
		{ from: 't1', to: 't2', source: 'parent' },
		{ from: 't2', to: 't3', source: 'l2', fromPort: 'Gi1', traffic: traffic({ measured_on: 't2' }) },
		{ from: 't1', to: 't3', source: 'l2', fromPort: 'Gi2', traffic: traffic({ measured_on: 't1' }) }
	];
	const graph = buildGraph(nodes, edges);
	assert.equal(graph.impacted.has('t2'), true);
	const layout = layoutMap(graph, { byGroup: false, bySubnet: false });
	const stale = Object.fromEntries(layout.edges.map((e) => [e.key, e.stale]));
	// t2 is impacted (behind the down t1), t1 is down: both cablings are stale.
	assert.equal(stale['t2>t3'], true);
	assert.equal(stale['t1>t3'], true);
	assert.equal(layout.edges.find((e) => e.key === 't2>t3').traffic.ports[0], 'Gi24');
});

test('a healthy link keeps its figures, and the list carries the load of the link that places a node', () => {
	const nodes = [node('t1'), node('t2')];
	const edges = [{ from: 't1', to: 't2', source: 'l2', fromPort: 'Gi1', traffic: traffic() }];
	const graph = buildGraph(nodes, edges);
	const layout = layoutMap(graph, { byGroup: false, bySubnet: false });
	assert.equal(layout.edges[0].stale, false);
	const rows = listRows(graph)[0].rows;
	assert.equal(rows.find((r) => r.node.id === 't2').traffic.bps_in, 100e6);
	assert.equal(rows.find((r) => r.node.id === 't2').linkFrom, 't1');
	assert.equal(rows.find((r) => r.node.id === 't1').traffic, null);
});
