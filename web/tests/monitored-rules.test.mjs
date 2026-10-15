// "Monitored alerts" on the edit form: order, summary and override calls. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { sortRules, isWatched, summarise, pendingChanges, overrideActions } from '../src/lib/components/device-form/monitored-rules.ts';

function rule(id, extra = {}) {
	return {
		id,
		uid: `rule_${id}`,
		name: `Rule ${id}`,
		description: '',
		severity: 'warning',
		enabled: true,
		ignored: false,
		custom_threshold: false,
		firing: false,
		...extra
	};
}

test('sortRules puts firing rules first, rules off everywhere last, then severity and name', () => {
	const rules = [
		rule(1, { name: 'b', severity: 'info' }),
		rule(2, { name: 'a', severity: 'info' }),
		rule(3, { name: 'z', severity: 'critical' }),
		rule(4, { name: 'y', enabled: false, severity: 'critical' }),
		rule(5, { name: 'x', firing: true, severity: 'info' })
	];
	assert.deepEqual(sortRules(rules).map((r) => r.id), [5, 3, 2, 1, 4]);
	assert.deepEqual(rules.map((r) => r.id), [1, 2, 3, 4, 5], 'the input is left alone');
});

test('a moved switch wins over the saved state', () => {
	assert.equal(isWatched(rule(1), {}), true);
	assert.equal(isWatched(rule(1, { ignored: true }), {}), false);
	assert.equal(isWatched(rule(1, { ignored: true }), { 1: true }), true);
	assert.equal(isWatched(rule(1), { 1: false }), false);
});

test('summarise counts the rules, those off and those firing', () => {
	const rules = [rule(1, { ignored: true }), rule(2, { firing: true }), rule(3)];
	assert.deepEqual(summarise(rules, { 3: false }), { total: 3, off: 2, firing: 1 });
});

test('pendingChanges keeps only the switches that differ from what is saved', () => {
	const rules = [rule(1), rule(2, { ignored: true }), rule(3), rule(4, { ignored: true })];
	const changes = pendingChanges(rules, { 1: false, 2: true, 3: true, 4: false });
	assert.deepEqual(changes.map((c) => [c.rule.id, c.watch]), [[1, false], [2, true]]);
});

test('overrideActions ignores with a PUT and un-ignores with a DELETE', () => {
	const actions = overrideActions(
		[
			{ rule: rule(1), watch: false },
			{ rule: rule(2, { ignored: true }), watch: true }
		],
		[],
		true
	);
	assert.deepEqual(actions, [
		{ ruleId: 1, method: 'PUT', payload: { enabled: false } },
		{ ruleId: 2, method: 'DELETE' }
	]);
});

test("overrideActions keeps an administrator's threshold for this device", () => {
	const current = [{ rule_uid: 'rule_1', target_id: 7, threshold: 95, clear_threshold: 90, enabled: null }];
	const off = overrideActions([{ rule: rule(1, { custom_threshold: true }), watch: false }], current, true);
	assert.deepEqual(off, [{ ruleId: 1, method: 'PUT', payload: { threshold: 95, clear_threshold: 90, enabled: false } }]);
	const back = overrideActions(
		[{ rule: rule(1, { custom_threshold: true, ignored: true }), watch: true }],
		[{ ...current[0], enabled: false }],
		true
	);
	assert.deepEqual(back, [{ ruleId: 1, method: 'PUT', payload: { threshold: 95, clear_threshold: 90 } }]);
});

test('an operator only sends enabled: false and DELETE, the server keeps the threshold', () => {
	const current = [{ rule_uid: 'rule_1', target_id: 7, threshold: 95, clear_threshold: null, enabled: false }];
	const actions = overrideActions(
		[
			{ rule: rule(1, { custom_threshold: true, ignored: true }), watch: true },
			{ rule: rule(2), watch: false }
		],
		current,
		false
	);
	assert.deepEqual(actions, [
		{ ruleId: 1, method: 'DELETE' },
		{ ruleId: 2, method: 'PUT', payload: { enabled: false } }
	]);
});
