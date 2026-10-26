// Topology map: tree links, outage propagation, sites and layout. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { blockers, buildGraph, chainOf, layoutMap, listRows, splitSites } from '../src/lib/components/topology/layout.ts';

function node(id, overrides = {}) {
	return {
		id,
		targetId: Number(id.slice(1)) || null,
		hostId: null,
		name: overrides.name ?? id,
		kind: overrides.kind ?? 'snmp',
		group: overrides.group ?? '',
		site: overrides.site ?? null,
		siteSource: overrides.site ? 'relay' : null,
		state: overrides.state ?? 'online'
	};
}

function overlaps(layout) {
	const out = [];
	for (const a of layout.nodes) {
		for (const b of layout.nodes) {
			if (a.node.id >= b.node.id) continue;
			// Each node owns a cell of 120 x 84 (circle plus its label).
			if (Math.abs(a.x - b.x) < 110 && Math.abs(a.y - b.y) < 80) out.push(`${a.node.id}/${b.node.id}`);
		}
	}
	return out;
}

test('the declared parent wins over a deduced link for the tree', () => {
	const graph = buildGraph(
		[node('t1'), node('t2'), node('t3')],
		[
			{ from: 't2', to: 't3', source: 'lldp' },
			{ from: 't1', to: 't3', source: 'parent' }
		]
	);
	assert.equal(graph.parent.get('t3'), 't1');
	assert.equal(graph.treeSource.get('t3'), 'parent');
	assert.deepEqual(graph.cabled.get('t3'), ['t2']);
});

test('a cycle never hangs a node under its own descendant', () => {
	const graph = buildGraph(
		[node('t1'), node('t2')],
		[
			{ from: 't1', to: 't2', source: 'parent' },
			{ from: 't2', to: 't1', source: 'relay' }
		]
	);
	assert.equal(graph.parent.get('t2'), 't1');
	assert.equal(graph.parent.has('t1'), false);
});

test('an outage marks everything behind it, but not across LLDP', () => {
	const graph = buildGraph(
		[node('t1', { state: 'offline' }), node('t2'), node('t3'), node('t4'), node('t5', { state: 'offline' }), node('t6')],
		[
			{ from: 't1', to: 't2', source: 'parent' },
			{ from: 't2', to: 't3', source: 'hypervisor' },
			{ from: 't1', to: 't4', source: 'lldp' },
			{ from: 't1', to: 't5', source: 'relay' },
			{ from: 't3', to: 't6', source: 'service' }
		]
	);
	assert.deepEqual([...graph.down].sort(), ['t1', 't5']);
	assert.deepEqual([...graph.impacted].sort(), ['t2', 't3', 't6']);
	assert.deepEqual(blockers(graph, 't6'), ['t1']);
});

test('the chain of a node is its ancestors, its descendants and its LLDP neighbours', () => {
	const graph = buildGraph(
		[node('t1'), node('t2'), node('t3'), node('t4'), node('t5')],
		[
			{ from: 't1', to: 't2', source: 'parent' },
			{ from: 't2', to: 't3', source: 'service' },
			{ from: 't2', to: 't4', source: 'lldp' },
			{ from: 't1', to: 't5', source: 'parent' }
		]
	);
	assert.deepEqual([...chainOf(graph, 't2')].sort(), ['t1', 't2', 't3', 't4']);
	assert.deepEqual([...chainOf(graph, 't3')].sort(), ['t1', 't2', 't3']);
});

test('edges to unknown nodes are dropped', () => {
	const graph = buildGraph([node('t1')], [{ from: 't9', to: 't1', source: 'parent' }]);
	assert.equal(graph.edges.length, 0);
});

test('sites: the main site first, then islands by size; all remote, the biggest is main', () => {
	const graph = buildGraph([node('t1'), node('t2', { site: 'Cave' }), node('t3', { site: 'Annex' }), node('t4', { site: 'Annex' })], []);
	assert.deepEqual(
		splitSites(graph).map((s) => s.key),
		['', 'Annex', 'Cave']
	);
	const remote = buildGraph([node('t2', { site: 'Cave' }), node('t3', { site: 'Annex' }), node('t4', { site: 'Annex' })], []);
	assert.deepEqual(
		splitSites(remote).map((s) => s.key),
		['Annex', 'Cave']
	);
});

test('the list walks sites, then each tree depth first', () => {
	const graph = buildGraph([node('t1'), node('t2'), node('t3', { site: 'Annex' })], [{ from: 't1', to: 't2', source: 'parent' }]);
	const sections = listRows(graph);
	assert.deepEqual(
		sections.map((s) => s.site.key),
		['', 'Annex']
	);
	assert.deepEqual(
		sections[0].rows.map((r) => [r.node.id, r.depth, r.via]),
		[
			['t1', 0, null],
			['t2', 1, 'parent']
		]
	);
});

test('the layout puts children under their parent and never overlaps nodes', () => {
	const nodes = [node('t1')];
	const edges = [];
	for (let i = 2; i <= 12; i++) {
		nodes.push(node(`t${i}`, { group: i % 2 ? 'A' : 'B' }));
		edges.push({ from: 't1', to: `t${i}`, source: i < 5 ? 'parent' : 'service' });
	}
	nodes.push(node('t20', { group: 'C' }), node('t21', { group: 'D' }));
	for (const byGroup of [false, true]) {
		const layout = layoutMap(buildGraph(nodes, edges), { byGroup });
		const at = new Map(layout.nodes.map((p) => [p.node.id, p]));
		assert.equal(layout.nodes.length, nodes.length);
		if (!byGroup) for (let i = 2; i <= 12; i++) assert.ok(at.get(`t${i}`).y > at.get('t1').y);
		assert.deepEqual(overlaps(layout), []);
		assert.ok(layout.edges.every((e) => e.path.startsWith('M')));
		assert.ok(layout.width > 0 && layout.height > 0);
		assert.equal(layout.frames.length, byGroup ? 5 : 0, "one frame per folder, the unfiled ones included");
	}
	const layout = layoutMap(buildGraph(nodes, edges));
	assert.ok(layout.edges.filter((e) => e.source === 'parent').every((e) => e.style === 'dependency' && e.arrow));
	assert.ok(layout.edges.filter((e) => e.source === 'service').every((e) => e.style === 'mesh' && !e.arrow));
});

test('a hub with many leaves lays them out as a grid, not one endless row', () => {
	const nodes = [node('t1', { kind: 'agent' })];
	const edges = [];
	for (let i = 2; i <= 41; i++) {
		nodes.push(node(`t${i}`, { kind: 'agent_service' }));
		edges.push({ from: 't1', to: `t${i}`, source: 'service' });
	}
	const layout = layoutMap(buildGraph(nodes, edges));
	assert.ok(layout.width < 1400, `${layout.width} wide`);
	assert.deepEqual(overlaps(layout), []);
});

test('remote sites become framed islands joined to the main site by a WAN link', () => {
	const nodes = [
		node('t1'),
		node('t2'),
		node('t3', { site: 'Cave', kind: 'agent' }),
		node('t4', { site: 'Cave', state: 'offline' }),
		node('t5', { site: 'Attic' })
	];
	const layout = layoutMap(buildGraph(nodes, [{ from: 't1', to: 't2', source: 'parent' }, { from: 't3', to: 't4', source: 'relay' }]));
	assert.deepEqual(
		layout.frames.map((f) => [f.kind, f.label, f.down]),
		[
			['main', null, 0],
			['site', 'Cave', 1],
			['site', 'Attic', 0]
		]
	);
	const wan = layout.edges.filter((e) => e.source === 'wan');
	assert.equal(wan.length, 2);
	assert.equal(layout.ports.length, 4);
	const frames = new Map(layout.frames.map((f) => [f.label, f]));
	for (const p of layout.nodes) {
		const frame = frames.get(p.node.site);
		assert.ok(p.x > frame.x && p.x < frame.x + frame.width && p.y > frame.y && p.y < frame.y + frame.height, `${p.node.id} outside its site`);
	}
});

test('a declared link across sites stands for the WAN link', () => {
	const nodes = [node('t1'), node('t2', { site: 'Cave' })];
	const layout = layoutMap(buildGraph(nodes, [{ from: 't1', to: 't2', source: 'parent' }]));
	assert.equal(layout.edges.filter((e) => e.source === 'wan').length, 0);
	assert.ok(layout.edges[0].wan);
});

test('300 nodes lay out quickly without overlaps', () => {
	const nodes = [];
	const edges = [];
	for (let i = 1; i <= 300; i++) {
		nodes.push(node(`t${i}`, { kind: i % 10 === 1 ? 'agent' : 'http', site: i > 260 ? `Site ${i % 3}` : null }));
		if (i % 10 !== 1) edges.push({ from: `t${i - (i % 10) + 1}`, to: `t${i}`, source: i % 2 ? 'parent' : 'service' });
	}
	const start = performance.now();
	const layout = layoutMap(buildGraph(nodes, edges), { byGroup: false });
	assert.ok(performance.now() - start < 200);
	assert.equal(layout.nodes.length, 300);
	assert.deepEqual(overlaps(layout), []);
});

test('an empty map has no size', () => {
	const layout = layoutMap(buildGraph([], []));
	assert.equal(layout.width, 0);
	assert.equal(layout.nodes.length, 0);
});

test('a remote site frame holds its relay and the devices behind it, under a roomy header', () => {
	const nodes = [
		node('t1'),
		node('t2', { kind: 'agent', name: 'dm01', site: 'Crissier' }),
		node('t3', { site: 'Crissier' }),
		node('t4', { site: 'Crissier' }),
		node('t5', { site: 'Crissier' })
	];
	const graph = buildGraph(nodes, [
		{ from: 't2', to: 't3', source: 'relay' },
		{ from: 't2', to: 't4', source: 'relay' }
	]);
	const layout = layoutMap(graph);
	const frame = layout.frames.find((f) => f.key === 's:Crissier');
	assert.ok(frame && frame.width >= 270, 'a narrow site still fits its title and subtitle');
	for (const id of ['t2', 't3', 't4', 't5']) {
		const p = layout.nodes.find((n) => n.node.id === id);
		assert.ok(p.x > frame.x && p.x < frame.x + frame.width && p.y > frame.y + 58 && p.y < frame.y + frame.height, `${id} inside the frame`);
	}
	assert.deepEqual(overlaps(layout), []);
	assert.equal(layout.nodes.length, 5);
});

test('a node listed twice is drawn once', () => {
	const graph = buildGraph([node('t1'), node('t1'), node('t2')], []);
	assert.equal(layoutMap(graph).nodes.length, 2);
});

test('a Proxmox cluster is a tree: nodes side by side, each with its guests underneath', () => {
	const nodes = [
		node('t1', { kind: 'proxmox' }),
		node('n1:a', { kind: 'pve_node' }),
		node('n1:b', { kind: 'pve_node' }),
		node('n1:c', { kind: 'pve_node', state: 'offline' }),
		node('g1:web', { kind: 'vm' }),
		node('g1:db', { kind: 'vm', state: 'stopped' }),
		node('g1:ct', { kind: 'container' })
	];
	const edges = [
		{ from: 't1', to: 'n1:a', source: 'hypervisor' },
		{ from: 't1', to: 'n1:b', source: 'hypervisor' },
		{ from: 't1', to: 'n1:c', source: 'hypervisor' },
		{ from: 'n1:a', to: 'g1:web', source: 'hypervisor' },
		{ from: 'n1:b', to: 'g1:db', source: 'hypervisor' },
		{ from: 'n1:b', to: 'g1:ct', source: 'hypervisor' }
	];
	const layout = layoutMap(buildGraph(nodes, edges));
	const at = new Map(layout.nodes.map((p) => [p.node.id, p]));
	assert.equal(at.get('n1:a').y, at.get('n1:b').y, 'the nodes share a row');
	assert.ok(at.get('n1:c').x < at.get('n1:a').x && at.get('n1:a').x < at.get('n1:b').x, 'an offline node comes first');
	assert.ok(at.get('n1:a').y > at.get('t1').y);
	assert.ok(at.get('g1:web').y > at.get('n1:a').y);
	assert.ok(Math.abs(at.get('g1:web').x - at.get('n1:a').x) < 120, 'a guest sits under its node');
	assert.ok(at.get('g1:db').x > at.get('g1:web').x, 'the second node holds its own guests');
	assert.deepEqual(overlaps(layout), []);
	const down = buildGraph(nodes, edges);
	assert.ok(down.down.has('n1:c'));
});

test('backup, same-host and firewall links do not carry an outage', () => {
	const graph = buildGraph(
		[node('t1', { state: 'offline', kind: 'opnsense' }), node('t2'), node('t3', { kind: 'proxmox', state: 'offline' }), node('t4', { kind: 'pbs' })],
		[
			{ from: 't1', to: 't2', source: 'firewall' },
			{ from: 't3', to: 't4', source: 'backup' }
		]
	);
	assert.equal(graph.impacted.size, 0);
	assert.equal(graph.parent.get('t2'), 't1', 'still placed under the firewall');
	assert.equal(graph.parent.get('t4'), 't3', 'the PBS sits beside what it backs up');
	assert.equal(graph.cabled.get('t2'), undefined);
});

test('a declared parent wins over the firewall link', () => {
	const graph = buildGraph(
		[node('t1', { kind: 'opnsense' }), node('t2', { kind: 'snmp' }), node('t3')],
		[
			{ from: 't1', to: 't3', source: 'firewall' },
			{ from: 't2', to: 't3', source: 'parent' }
		]
	);
	assert.equal(graph.parent.get('t3'), 't2');
});

test('by subnet: firewalls on top, one zone per subnet, guests in their hypervisor zone, unknown network last', () => {
	const net = (id, subnet, extra = {}) => ({ ...node(id, extra), subnet, vlan: extra.vlan ?? null, vlanName: extra.vlanName ?? null });
	const nodes = [
		net('t1', '192.168.1.0/24', { kind: 'opnsense' }),
		net('t2', '192.168.1.0/24', { kind: 'opnsense' }),
		net('t3', '192.168.10.0/24', { kind: 'proxmox', vlan: 10, vlanName: 'Servers' }),
		net('t4', '192.168.10.0/24', { vlan: 10, vlanName: 'Servers' }),
		net('t5', '192.168.2.0/24'),
		net('t6', null),
		{ ...node('g3:vm', { kind: 'vm' }), subnet: null }
	];
	const edges = [
		{ from: 't1', to: 't3', source: 'firewall' },
		{ from: 't1', to: 't4', source: 'firewall' },
		{ from: 't1', to: 't5', source: 'firewall' },
		{ from: 't3', to: 'g3:vm', source: 'hypervisor' }
	];
	const layout = layoutMap(buildGraph(nodes, edges), { bySubnet: true });
	const zones = layout.frames.filter((f) => f.kind === 'zone');
	assert.deepEqual(
		zones.map((z) => [z.label, z.vlan, z.vlanName]),
		[
			['192.168.2.0/24', null, null],
			['192.168.10.0/24', 10, 'Servers'],
			[null, null, null]
		]
	);
	const at = new Map(layout.nodes.map((p) => [p.node.id, p]));
	const inside = (id, zone) => {
		const p = at.get(id);
		return p.x > zone.x && p.x < zone.x + zone.width && p.y > zone.y && p.y < zone.y + zone.height;
	};
	assert.ok(inside('t3', zones[1]) && inside('t4', zones[1]) && inside('g3:vm', zones[1]));
	assert.ok(inside('t5', zones[0]) && inside('t6', zones[2]));
	assert.equal(at.get('t1').y, at.get('t2').y, 'twin firewalls side by side');
	for (const zone of zones) assert.ok(at.get('t1').y < zone.y, 'firewalls above every zone');
	assert.deepEqual(overlaps(layout), []);
});

test('by subnet and by folder are separate groupings', () => {
	const nodes = [{ ...node('t1', { group: 'A' }), subnet: '10.0.0.0/24' }, { ...node('t2', { group: 'B' }), subnet: '10.0.0.0/24' }];
	assert.equal(layoutMap(buildGraph(nodes, []), { byGroup: true }).frames.length, 2);
	assert.equal(layoutMap(buildGraph(nodes, []), { bySubnet: true }).frames.length, 1);
});

test('a link between two sites runs through the frames edges, not across the nodes', () => {
	const nodes = [node('t1'), node('t2'), node('t3', { site: 'Cave', kind: 'agent' }), node('t4', { site: 'Cave' })];
	const layout = layoutMap(buildGraph(nodes, [{ from: 't3', to: 't4', source: 'relay' }, { from: 't1', to: 't4', source: 'backup' }]));
	const edge = layout.edges.find((e) => e.key === 't1>t4');
	assert.ok(edge.wan);
	assert.equal((edge.path.match(/C/g) ?? []).length, 3, 'three smooth curves, through two ports');
	const main = layout.frames.find((f) => f.kind === 'main');
	const cave = layout.frames.find((f) => f.label === 'Cave');
	const near = (port, x) => Math.abs(port.x - x) < 0.01 || Math.abs(port.y - x) < 0.01;
	assert.ok(layout.ports.some((p) => near(p, main.x + main.width) || near(p, main.y + main.height)));
	assert.ok(layout.ports.some((p) => near(p, cave.x) || near(p, cave.y)));
});
