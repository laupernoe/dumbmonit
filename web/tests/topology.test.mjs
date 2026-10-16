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
