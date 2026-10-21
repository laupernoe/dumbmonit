// Central reload after a write: listeners fire for writes only, never for reads or session routes. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { onMutation, notifyMutation } from '../src/lib/api/mutations.ts';

test('a write notifies subscribers, a read does not', () => {
	const seen = [];
	const off = onMutation((path, method) => seen.push(`${method} ${path}`));
	notifyMutation('/targets/3', 'DELETE');
	notifyMutation('/targets', 'GET');
	off();
	assert.deepEqual(seen, ['DELETE /targets/3']);
});

test('session routes are ignored and unsubscribing stops delivery', () => {
	const seen = [];
	const off = onMutation((path) => seen.push(path));
	notifyMutation('/auth/logout', 'POST');
	off();
	notifyMutation('/targets', 'POST');
	assert.deepEqual(seen, []);
});

test('a throwing listener does not break the others', () => {
	const seen = [];
	const off1 = onMutation(() => {
		throw new Error('boom');
	});
	const off2 = onMutation((path) => seen.push(path));
	notifyMutation('/targets', 'PUT');
	off1();
	off2();
	assert.deepEqual(seen, ['/targets']);
});
