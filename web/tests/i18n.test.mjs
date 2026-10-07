// Every locale has exactly the keys and {variables} of en.json (same check as `npm run i18n:check`).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { check } from '../scripts/merge-messages.mjs';

test('all locales are complete and use the same variables', () => {
	assert.deepEqual(check(), []);
});
