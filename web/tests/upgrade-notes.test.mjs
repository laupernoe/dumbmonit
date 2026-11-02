// "Configuration changes after an update": the rules the window and the reminder follow,
// and a guard that every message they use exists (in en.json, or still in the fragment).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { afterRecheck, detectionTone, groupNotes, shouldAutoOpen, showsReminder } from '../src/lib/upgrade-notes.ts';

const note = (id, status, detection = null) => ({
	id,
	version: '0.14.0',
	title: id,
	summary: '',
	compose: 'services:',
	apply: null,
	docs_url: null,
	status,
	detection,
	closed_at: status === 'closed' ? '2026-10-31 08:30:00' : null
});
const overview = (over = {}) => ({ current: '0.15.0', upgraded_from: '0.14.3', announce: true, pending: 1, notes: [], ...over });

test('notes are grouped by status, in the server order', () => {
	const grouped = groupNotes([note('a', 'clear'), note('b', 'pending'), note('c', 'closed'), note('d', 'pending')]);
	assert.deepEqual(grouped.pending.map((n) => n.id), ['b', 'd']);
	assert.deepEqual(grouped.closed.map((n) => n.id), ['c']);
	assert.deepEqual(grouped.clear.map((n) => n.id), ['a']);
	assert.deepEqual(groupNotes([]), { pending: [], closed: [], clear: [] });
});

test('the window opens by itself only when announced and something is pending', () => {
	assert.equal(shouldAutoOpen(null), false);
	assert.equal(shouldAutoOpen(overview()), true);
	assert.equal(shouldAutoOpen(overview({ announce: false })), false, 'already shown for this version');
	assert.equal(shouldAutoOpen(overview({ pending: 0 })), false, 'nothing left to apply');
});

test('the reminder stays while changes are pending and the window is closed', () => {
	assert.equal(showsReminder(null, false), false);
	assert.equal(showsReminder(overview({ announce: false }), false), true, 'survives "I will do it later" and a reload');
	assert.equal(showsReminder(overview(), true), false, 'the window is open');
	assert.equal(showsReminder(overview({ pending: 0 }), false), false, 'dismissed or done');
});

test('"Done" asks twice only when the check still fails', () => {
	const missing = { state: 'missing', detail: '' };
	assert.equal(afterRecheck(note('a', 'pending', missing)), 'confirm');
	assert.equal(afterRecheck(note('a', 'pending', { state: 'ok', detail: '' })), 'close');
	assert.equal(afterRecheck(note('a', 'pending', { state: 'unknown', detail: '' })), 'close', 'cannot be checked from here');
	assert.equal(afterRecheck(note('a', 'pending')), 'close', 'no check at all');
	assert.equal(afterRecheck(undefined), 'close', 'the change is gone from the list');
});

test('a check verdict always has a tone', () => {
	assert.equal(detectionTone('ok'), 'signal');
	assert.equal(detectionTone('missing'), 'warning');
	assert.equal(detectionTone('unknown'), 'ghost');
});

test('every message of the window exists in the English file or its fragment', () => {
	const known = new Set(Object.keys(JSON.parse(readFileSync(new URL('../messages/en.json', import.meta.url), 'utf8'))));
	const fragment = new URL('../messages/fragments/upgrade.en.json', import.meta.url);
	if (existsSync(fragment)) for (const key of Object.keys(JSON.parse(readFileSync(fragment, 'utf8')))) known.add(key);
	const files = ['components/UpgradeNotes.svelte', 'components/settings/AboutSection.svelte'];
	const used = new Set();
	for (const file of files) {
		const source = readFileSync(new URL(`../src/lib/${file}`, import.meta.url), 'utf8');
		for (const [, key] of source.matchAll(/\bm\.(upgrade_[a-z_]+)\(/g)) used.add(key);
	}
	assert.ok(used.size > 20, `only ${used.size} messages found`);
	for (const key of used) assert.ok(known.has(key), `missing message ${key}`);
});
