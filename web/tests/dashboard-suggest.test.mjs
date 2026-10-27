// Query box completion. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { suggest, apply } from '../src/lib/dashboards/suggest.ts';

const catalog = [
	{ name: 'dumbmonit_cpu_usage_percent', series: 2, labels: [{ name: 'target', values: ['1', '2'], more: false }, { name: 'core', values: ['0'], more: false }] },
	{ name: 'dumbmonit_memory_used_bytes', series: 1, labels: [{ name: 'target', values: ['1'], more: false }] }
];

test('completes a metric name by prefix, then by fragment', () => {
	assert.deepEqual(suggest('avg(dumbmonit_c', 15, catalog)?.items, ['dumbmonit_cpu_usage_percent']);
	assert.deepEqual(suggest('rate(memory', 11, catalog)?.items, ['dumbmonit_memory_used_bytes']);
	assert.equal(suggest('', 0, catalog), null);
	assert.equal(suggest('dumbmonit_cpu_usage_percent', 27, catalog), null);
});

test('completes label names and values inside braces', () => {
	const text = 'dumbmonit_cpu_usage_percent{co';
	assert.deepEqual(suggest(text, text.length, catalog)?.items, ['core']);
	const values = 'dumbmonit_cpu_usage_percent{target="';
	const found = suggest(values, values.length, catalog);
	assert.deepEqual(found?.items, ['1', '2']);
	assert.equal(apply(values, found, '2').text, 'dumbmonit_cpu_usage_percent{target="2');
});

test('applying a name puts the caret after it', () => {
	const found = suggest('avg(dumbmonit_c', 15, catalog);
	assert.deepEqual(apply('avg(dumbmonit_c', found, 'dumbmonit_cpu_usage_percent'), { text: 'avg(dumbmonit_cpu_usage_percent', caret: 31 });
});
