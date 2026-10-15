// Topology map: tree links, outage propagation and layout. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildForest, layoutMap, listRows, NODE_WIDTH } from '../src/lib/components/topology/layout.ts';

function node(id, overrides = {}) {
	return {
		id,
		targetId: Number(id.slice(1)) || null,
		hostId: null,
		name: overrides.name ?? id,
		kind: 'snmp',
		group: overrides.group ?? '',
		site: null,
		state: overrides.state ?? 'online'
	};
}

test('the declared parent wins over a deduced link for the tree', () => {
	const forest = buildForest(
		[node('t1'), node('t2'), node('t3')],
		[
			{ from: 't2', to: 't3', source: 'lldp' },
			{ from: 't1', to: 't3', source: 'parent' }
		]
	);
	assert.equal(forest.parent.get('t3'), 't1');
	assert.equal(forest.treeSource.get('t3'), 'parent');
	assert.deepEqual(forest.roots.sort(), ['t1', 't2']);
});

test('a cycle never hangs a node under its own descendant', () => {
	const forest = buildForest(
		[node('t1'), node('t2')],
		[
			{ from: 't1', to: 't2', source: 'parent' },
			{ from: 't2', to: 't1', source: 'relay' }
		]
	);
	assert.equal(forest.parent.get('t2'), 't1');
	assert.equal(forest.parent.has('t1'), false);
	assert.deepEqual(forest.roots, ['t1']);
});

test('an outage marks everything behind it, but not across LLDP', () => {
	const forest = buildForest(
		[node('t1', { state: 'offline' }), node('t2'), node('t3'), node('t4'), node('t5', { state: 'offline' })],
		[
			{ from: 't1', to: 't2', source: 'parent' },
			{ from: 't2', to: 't3', source: 'hypervisor' },
			{ from: 't1', to: 't4', source: 'lldp' },
			{ from: 't1', to: 't5', source: 'relay' }
		]
	);
	assert.deepEqual([...forest.down].sort(), ['t1', 't5']);
	assert.deepEqual([...forest.impacted].sort(), ['t2', 't3']);
});

test('edges to unknown nodes are dropped', () => {
	const forest = buildForest([node('t1')], [{ from: 't9', to: 't1', source: 'parent' }]);
	assert.equal(forest.edges.length, 0);
	assert.deepEqual(forest.roots, ['t1']);
});

test('the list fallback walks folders, then each tree depth first', () => {
	const forest = buildForest(
		[node('t1', { group: 'Rack' }), node('t2', { group: 'Rack' }), node('t3'), node('t4', { group: 'Annex' })],
		[{ from: 't1', to: 't2', source: 'parent' }]
	);
	const groups = listRows(forest);
	assert.deepEqual(
		groups.map((g) => g.group),
		['', 'Annex', 'Rack']
	);
	const rack = groups[2].rows.map((r) => [r.node.id, r.depth, r.via]);
	assert.deepEqual(rack, [
		['t1', 0, null],
		['t2', 1, 'parent']
	]);
});

test('the layout puts children to the right and never overlaps nodes', () => {
	const nodes = [node('t1')];
	const edges = [];
	for (let i = 2; i <= 12; i++) {
		nodes.push(node(`t${i}`, { group: i % 2 ? 'A' : 'B' }));
		edges.push({ from: 't1', to: `t${i}`, source: 'parent' });
	}
	nodes.push(node('t20', { group: 'C' }), node('t21', { group: 'D' }));
	const layout = layoutMap(buildForest(nodes, edges));
	const at = new Map(layout.nodes.map((p) => [p.node.id, p]));
	assert.equal(layout.nodes.length, nodes.length);
	for (let i = 2; i <= 12; i++) assert.ok(at.get(`t${i}`).x >= at.get('t1').x + NODE_WIDTH);
	for (const a of layout.nodes) {
		for (const b of layout.nodes) {
			if (a === b) continue;
			const apart = Math.abs(a.x - b.x) >= NODE_WIDTH || Math.abs(a.y - b.y) >= 44;
			assert.ok(apart, `${a.node.id} overlaps ${b.node.id}`);
		}
	}
	assert.ok(layout.edges.every((e) => e.tree && e.path.startsWith('M')));
	assert.ok(layout.width > 0 && layout.height > 0);
});

test('an empty map has no size', () => {
	const layout = layoutMap(buildForest([], []));
	assert.equal(layout.width, 0);
	assert.equal(layout.nodes.length, 0);
});
