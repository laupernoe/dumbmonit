// "Import from Uptime Kuma": file sniffing and preview selection. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { rowsParam } from '../src/lib/api/rows-param.ts';
import {
	importableRows,
	isSqliteHead,
	selectionState,
	tickedRows,
	toggleRow
} from '../src/lib/components/device-form/kuma-import.ts';

const row = (n, status) => ({ row: n, name: `m${n}`, status });
const rows = [row(1, 'ok'), row(2, 'skipped'), row(3, 'ok'), row(4, 'error'), row(5, 'ok')];

test('an SQLite file is recognised by its header, a JSON file is not', () => {
	const sqlite = new TextEncoder().encode('SQLite format 3\0rest of the page');
	assert.equal(isSqliteHead(sqlite), true);
	assert.equal(isSqliteHead(new TextEncoder().encode('{"monitorList": []}')), false);
	assert.equal(isSqliteHead(new Uint8Array(4)), false, 'too short');
});

test('only rows the preview would create can be ticked', () => {
	assert.deepEqual(importableRows(rows), [1, 3, 5]);
});

test('toggling does not mutate the previous selection', () => {
	const start = new Set([1, 3]);
	const next = toggleRow(start, 5);
	assert.deepEqual([...start], [1, 3]);
	assert.deepEqual([...next].sort(), [1, 3, 5]);
	assert.deepEqual([...toggleRow(next, 1)].sort(), [3, 5]);
});

test('selection state and ticked rows follow the importable rows', () => {
	assert.equal(selectionState(new Set(), rows), 'none');
	assert.equal(selectionState(new Set([1]), rows), 'some');
	assert.equal(selectionState(new Set([1, 3, 5]), rows), 'all');
	// A stale tick on a row that cannot be imported counts for nothing.
	assert.equal(selectionState(new Set([2]), rows), 'none');
	assert.deepEqual(tickedRows(new Set([5, 2, 1]), rows), [1, 5]);
});

test('an empty selection never asks for every row', () => {
	assert.equal(rowsParam(undefined), undefined);
	assert.equal(rowsParam([]), '0');
	assert.equal(rowsParam([1, 4]), '1,4');
});
