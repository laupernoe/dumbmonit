// Sub-headings for a long "More options" list. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { groupOptions } from '../src/lib/components/device-form/option-groups.ts';

function option(key, overrides = {}) {
	return {
		key,
		label: key,
		help: '',
		placeholder: '',
		default: '',
		required: false,
		input: 'boolean',
		choices: [],
		group: '',
		...overrides
	};
}

test('a short option list is never grouped', () => {
	const options = [option('a'), option('b'), option('c')];
	const groups = groupOptions(options);
	assert.deepEqual(groups, [{ label: '', options }]);
});

test('an explicit group on any option takes over, long list', () => {
	const options = [
		option('nodes', { group: 'Inventory' }),
		option('packages', { group: 'Inventory' }),
		option('ha', { group: 'Performance' }),
		option('disks', { group: 'Performance' }),
		option('zfs', { group: 'Performance' }),
		option('misc') // no group declared: falls into "More settings"
	];
	const groups = groupOptions(options);
	assert.deepEqual(groups.map((g) => g.label), ['Inventory', 'Performance', 'More settings']);
	assert.deepEqual(groups[0].options.map((o) => o.key), ['nodes', 'packages']);
	assert.deepEqual(groups[2].options.map((o) => o.key), ['misc']);
});

test('without explicit groups, a shared key prefix buckets options generically', () => {
	const options = [
		option('scan_backup_storage'),
		option('scan_snapshots'),
		option('backup_jobs'),
		option('backup_volumes'),
		option('unique_thing'),
		option('another_unique_thing')
	];
	const groups = groupOptions(options);
	const labels = groups.map((g) => g.label);
	assert.ok(labels.includes('Scan'));
	assert.ok(labels.includes('Backup'));
	assert.ok(labels.includes('More settings'), 'singleton prefixes fall back together');
	const fallback = groups.find((g) => g.label === 'More settings');
	assert.deepEqual(fallback.options.map((o) => o.key), ['unique_thing', 'another_unique_thing']);
});

test('when the generic fallback cannot form any real bucket, it gives up and stays flat', () => {
	const options = [option('alpha'), option('beta'), option('gamma'), option('delta'), option('epsilon'), option('zeta')];
	const groups = groupOptions(options);
	assert.deepEqual(groups, [{ label: '', options }]);
});

test('an empty option list produces no groups', () => {
	assert.deepEqual(groupOptions([]), []);
});
