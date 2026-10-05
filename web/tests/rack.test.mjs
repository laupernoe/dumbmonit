// Ordering of the device rack: state tier, then manual position, then name.
// Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildRack, compareSiblings, needsAttention, RANK } from '../src/lib/components/devices/rack.ts';

let nextId = 1;
function target(overrides = {}) {
	const id = overrides.id ?? nextId++;
	return {
		id,
		name: overrides.name ?? `device-${id}`,
		address: overrides.address ?? '10.0.0.1',
		kind: overrides.kind ?? 'snmp',
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

function statesOf(map) {
	return (t) => map.get(t.id) ?? 'online';
}

const visibleAll = (targets) => new Set(targets.map((t) => t.id));

test('needsAttention is true only for the tiers that float to the top', () => {
	assert.equal(needsAttention('offline'), true);
	assert.equal(needsAttention('down'), true);
	assert.equal(needsAttention('misconfigured'), true);
	assert.equal(needsAttention('pending'), false);
	assert.equal(needsAttention('online'), false);
	assert.equal(needsAttention('disabled'), false);
});

test('a device needing attention floats above a quiet one regardless of name', () => {
	const trouble = target({ name: 'zzz-trouble' });
	const quiet = target({ name: 'aaa-quiet' });
	const states = new Map([
		[trouble.id, 'down'],
		[quiet.id, 'online']
	]);
	const rows = buildRack([trouble, quiet], statesOf(states), visibleAll([trouble, quiet]));
	assert.deepEqual(rows.map((r) => r.target.id), [trouble.id, quiet.id]);
});

test('within the same tier, manual position wins over alphabetical order', () => {
	const z = target({ name: 'zzz', position: 0 });
	const a = target({ name: 'aaa', position: 1 });
	const states = new Map([
		[z.id, 'online'],
		[a.id, 'online']
	]);
	const rows = buildRack([z, a], statesOf(states), visibleAll([z, a]));
	assert.deepEqual(rows.map((r) => r.target.id), [z.id, a.id], 'z was dragged above a');
});

test('equal position falls back to the name', () => {
	const b = target({ name: 'bbb', position: 5 });
	const a = target({ name: 'aaa', position: 5 });
	const states = new Map([
		[b.id, 'online'],
		[a.id, 'online']
	]);
	const rows = buildRack([b, a], statesOf(states), visibleAll([b, a]));
	assert.deepEqual(rows.map((r) => r.target.id), [a.id, b.id]);
});

test('children stack right under their parent, unaffected by position', () => {
	const parent = target({ name: 'parent', position: 9 });
	const child = target({ name: 'child', parent_id: parent.id, position: 0 });
	const states = new Map([
		[parent.id, 'online'],
		[child.id, 'online']
	]);
	const all = [child, parent];
	const rows = buildRack(all, statesOf(states), visibleAll(all));
	assert.deepEqual(rows.map((r) => [r.target.id, r.depth]), [
		[parent.id, 0],
		[child.id, 1]
	]);
});

test('an offline parent shadows its children', () => {
	const parent = target({ name: 'parent' });
	const child = target({ name: 'child', parent_id: parent.id });
	const states = new Map([
		[parent.id, 'offline'],
		[child.id, 'online']
	]);
	const all = [parent, child];
	const rows = buildRack(all, statesOf(states), visibleAll(all));
	const childRow = rows.find((r) => r.target.id === child.id);
	assert.equal(childRow.shadowed, true);
});

test('compareSiblings orders by tier, then position, then name', () => {
	const states = new Map();
	const down = target({ name: 'b' });
	const online = target({ name: 'a' });
	states.set(down.id, 'down');
	states.set(online.id, 'online');
	assert.ok(compareSiblings(down, online, states) < 0);
	assert.ok(compareSiblings(online, down, states) > 0);
});

test('RANK ties pending and unknown below trouble, above online', () => {
	assert.ok(RANK.down < RANK.pending);
	assert.ok(RANK.pending < RANK.online);
	assert.ok(RANK.online < RANK.disabled);
});
