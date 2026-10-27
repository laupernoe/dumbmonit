// Corrections of the topology map: undo/redo through the server, node search. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EditHistory, canUseForAlerts, searchNodes } from '../src/lib/components/topology/edits.ts';
import { buildGraph, layoutMap } from '../src/lib/components/topology/layout.ts';

/** A fake server with the same replacement rule as the real one: one re-parenting per node. */
function fakeServer() {
	let next = 1;
	let nextRule = 1;
	const rows = new Map();
	const rules = new Map();
	return {
		rows,
		rules,
		async create(input) {
			const replaced = [...rows.values()].filter((c) => c.kind === input.kind && c.to === input.to);
			for (const r of replaced) rows.delete(r.id);
			const correction = { id: next++, author: 'admin', created_at: '2026-10-25 10:00:00', from: null, source: null, value: null, use_for_alerts: false, ...input };
			rows.set(correction.id, correction);
			return { correction, replaced };
		},
		async remove(id) {
			if (!rows.delete(id)) throw Object.assign(new Error('not found'), { status: 404 });
		},
		async saveRule(spec, status, evidence) {
			const rule = { ...spec, id: nextRule++, status, evidence, author: 'admin', created_at: '', matches: 0 };
			rules.set(rule.id, rule);
			return rule;
		},
		async deleteRule(id) {
			rules.delete(id);
		}
	};
}

const parents = (server) => [...server.rows.values()].map((c) => `${c.from}>${c.to}`).sort();

test('undo brings back what a correction replaced, redo replaces it again', async () => {
	const server = fakeServer();
	const history = new EditHistory(server);
	await history.create({ kind: 'parent', from: 't1', to: 't3' });
	await history.create({ kind: 'parent', from: 't2', to: 't3' });
	assert.deepEqual(parents(server), ['t2>t3']);

	await history.undo();
	assert.deepEqual(parents(server), ['t1>t3'], 'the first attachment is back');
	await history.undo();
	assert.deepEqual(parents(server), [], 'and goes too, although the server gave it a new id');
	assert.equal(history.canUndo, false);

	await history.redo();
	await history.redo();
	assert.deepEqual(parents(server), ['t2>t3']);
	assert.equal(history.canRedo, false);
});

test('a removed correction comes back on undo, and a new step clears redo', async () => {
	const server = fakeServer();
	const history = new EditHistory(server);
	const { correction } = await history.create({ kind: 'site', to: 't4', value: 'Annexe' });
	await history.remove(correction);
	assert.equal(server.rows.size, 0);
	await history.undo();
	assert.equal(server.rows.size, 1);
	await history.undo();
	assert.equal(server.rows.size, 0, 'the re-created correction is the one undone');

	await history.redo();
	await history.create({ kind: 'group', to: 't4', value: 'Cameras' });
	assert.equal(history.canRedo, false);
});

test('accepting a rule is undone by deleting it', async () => {
	const server = fakeServer();
	const history = new EditHistory(server);
	const spec = { kind: 'parent', subnet: '10.0.5.0/24', address: null, target_kind: null, source: null, parent: 't2' };
	const rule = await history.rule(spec, 'active', 2);
	assert.equal(server.rules.size, 1);
	await history.undo();
	assert.equal(server.rules.size, 0);
	await history.redo();
	assert.equal(server.rules.size, 1);
	await history.unrule([...server.rules.values()][0]);
	await history.undo();
	assert.equal([...server.rules.values()][0].status, 'active');
	assert.ok(rule.id > 0);
});

test('a failed undo leaves the history as it was', async () => {
	const server = fakeServer();
	const history = new EditHistory(server);
	await history.create({ kind: 'parent', from: 't1', to: 't3' });
	server.remove = async () => {
		throw Object.assign(new Error('offline'), { status: 0 });
	};
	await assert.rejects(history.undo());
	assert.equal(history.canUndo, true);
	assert.equal(history.canRedo, false);
});

test('node search ranks names before types and leaves the node itself out', () => {
	const nodes = [
		{ id: 't1', name: 'core-switch', kind: 'snmp' },
		{ id: 't2', name: 'switch-lab', kind: 'snmp' },
		{ id: 't3', name: 'nas', kind: 'synology' },
		{ id: 't4', name: 'printer', kind: 'snmp' }
	];
	assert.deepEqual(searchNodes(nodes, 'switch', 't9').map((n) => n.id), ['t2', 't1']);
	assert.deepEqual(searchNodes(nodes, 'snmp', 't1').map((n) => n.id), ['t4', 't2']);
	assert.equal(searchNodes(nodes, '', 't3').length, 3);
	assert.equal(canUseForAlerts('t1', 't2'), true);
	assert.equal(canUseForAlerts('g1:web', 't2'), false);
});

test('a node attached by hand hangs from its new parent and the link wears its origin', () => {
	const node = (id, kind = 'snmp') => ({ id, targetId: Number(id.slice(1)), hostId: null, name: id, kind, group: '', site: null, siteSource: null, state: 'online' });
	const graph = buildGraph(
		[node('t1', 'opnsense'), node('t2'), node('t3', 'synology')],
		[
			{ from: 't1', to: 't2', source: 'firewall' },
			{ from: 't1', to: 't3', source: 'parent' },
			{ from: 't2', to: 't3', source: 'manual', origin: 'manual', correctionId: 4 }
		]
	);
	assert.equal(graph.parent.get('t3'), 't2', 'placed by hand before the declared parent');
	assert.equal(graph.treeEdge.get('t3').origin, 'manual');
	const layout = layoutMap(graph);
	const edge = layout.edges.find((e) => e.key === 't2>t3');
	assert.equal(edge.origin, 'manual');
	assert.equal(edge.style, 'dependency');
	assert.ok(Number.isFinite(edge.mid.x) && Number.isFinite(edge.mid.y));
	assert.equal(new Set(layout.nodes.map((p) => p.node.id)).size, 3, 'no node drawn twice');
});
