// Folder sections and manual reordering on /targets. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	buildFolders,
	compareFolderSections,
	folderKeyOf,
	knownFolders,
	moveWithinScope,
	reorderByDrop
} from '../src/lib/components/devices/folders.ts';

let nextId = 1;
function target(overrides = {}) {
	const id = overrides.id ?? nextId++;
	return {
		id,
		name: overrides.name ?? `device-${id}`,
		address: '10.0.0.1',
		kind: 'snmp',
		profile_id: null,
		parent_id: overrides.parent_id ?? null,
		via_agent: null,
		interval_secs: 60,
		enabled: true,
		tags: {},
		credential_kind: 'none',
		group_name: overrides.group_name ?? '',
		position: overrides.position ?? 0,
		last_probe_at: null,
		last_error: null,
		error_kind: null,
		...overrides
	};
}

function stateOfMap(map) {
	return (t) => map.get(t.id) ?? 'online';
}

const visibleAll = (targets) => new Set(targets.map((t) => t.id));

test('an ungrouped device sits in the "No folder" section, listed first', () => {
	const loose = target({ name: 'loose' });
	const grouped = target({ name: 'grouped', group_name: 'Rack A' });
	const all = [loose, grouped];
	const sections = buildFolders(all, () => 'online', visibleAll(all));
	assert.deepEqual(sections.map((s) => s.key), ['', 'Rack A']);
	assert.deepEqual(sections[0].rows.map((r) => r.target.id), [loose.id]);
	assert.deepEqual(sections[1].rows.map((r) => r.target.id), [grouped.id]);
});

test('a child follows its parent into the folder, ignoring its own group_name', () => {
	const parent = target({ name: 'parent', group_name: 'Rack A' });
	const child = target({ name: 'child', parent_id: parent.id, group_name: 'Rack B' });
	const all = [parent, child];
	const sections = buildFolders(all, () => 'online', visibleAll(all));
	assert.equal(sections.length, 1);
	assert.equal(sections[0].key, 'Rack A');
	assert.deepEqual(sections[0].rows.map((r) => r.target.id), [parent.id, child.id]);
});

test('a folder with a device needing attention sorts before a quiet one', () => {
	const quiet = target({ name: 'quiet-folder', group_name: 'Z quiet' });
	const trouble = target({ name: 'trouble-folder', group_name: 'A trouble' });
	const all = [quiet, trouble];
	const states = new Map([
		[quiet.id, 'online'],
		[trouble.id, 'down']
	]);
	const sections = buildFolders(all, stateOfMap(states), visibleAll(all));
	assert.deepEqual(sections.map((s) => s.key), ['A trouble', 'Z quiet']);
});

test('knownFolders lists distinct, sorted, non-empty group names', () => {
	const all = [
		target({ group_name: 'Rack B' }),
		target({ group_name: 'Rack A' }),
		target({ group_name: '' }),
		target({ group_name: 'Rack A' })
	];
	assert.deepEqual(knownFolders(all), ['Rack A', 'Rack B']);
});

test('moveWithinScope swaps a device with its same-tier neighbour', () => {
	const a = target({ name: 'a', position: 0 });
	const b = target({ name: 'b', position: 1 });
	const all = [a, b];
	const order = moveWithinScope(all, () => 'online', a.id, 1);
	assert.deepEqual(order, [b.id, a.id]);
});

test('moveWithinScope refuses to cross a state tier', () => {
	const trouble = target({ name: 'trouble', position: 0 });
	const quiet = target({ name: 'quiet', position: 1 });
	const all = [trouble, quiet];
	const states = new Map([
		[trouble.id, 'down'],
		[quiet.id, 'online']
	]);
	assert.equal(moveWithinScope(all, stateOfMap(states), trouble.id, 1), null);
});

test('moveWithinScope only compares devices of the same folder', () => {
	const a = target({ name: 'a', group_name: 'Rack A', position: 0 });
	const b = target({ name: 'b', group_name: 'Rack B', position: 1 });
	const all = [a, b];
	assert.equal(moveWithinScope(all, () => 'online', a.id, 1), null);
});

test('reorderByDrop reinserts the dragged device at the drop target', () => {
	const a = target({ name: 'a', position: 0 });
	const b = target({ name: 'b', position: 1 });
	const c = target({ name: 'c', position: 2 });
	const all = [a, b, c];
	const order = reorderByDrop(all, () => 'online', a.id, c.id);
	assert.deepEqual(order, [b.id, c.id, a.id]);
});

test('reorderByDrop refuses devices outside the same reorder scope', () => {
	const a = target({ name: 'a', group_name: 'Rack A' });
	const b = target({ name: 'b', group_name: 'Rack B' });
	assert.equal(reorderByDrop([a, b], () => 'online', a.id, b.id), null);
});

test('folderKeyOf reads the group of the topmost ancestor, not the device itself', () => {
	const parent = target({ name: 'parent', group_name: 'Rack A' });
	const child = target({ name: 'child', parent_id: parent.id, group_name: 'Rack B' });
	const all = [parent, child];
	assert.equal(folderKeyOf(all, parent), 'Rack A');
	assert.equal(folderKeyOf(all, child), 'Rack A');
});

test('compareFolderSections keeps "no folder" first, then attention, then alphabetical', () => {
	const sections = [
		{ key: 'Z quiet', label: 'Z quiet', rows: [] },
		{ key: '', label: 'No folder', rows: [] },
		{ key: 'A trouble', label: 'A trouble', rows: [{ state: 'down' }] },
		{ key: 'B quiet', label: 'B quiet', rows: [] }
	];
	assert.deepEqual(
		sections.slice().sort(compareFolderSections).map((s) => s.key),
		['', 'A trouble', 'B quiet', 'Z quiet']
	);
});
