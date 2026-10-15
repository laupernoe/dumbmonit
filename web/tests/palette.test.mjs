// Command palette logic: ranking, recents store, shortcut sequences. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	queryWords,
	score,
	rank,
	pushRecent,
	parseRecents,
	loadRecents,
	saveRecents,
	RECENTS_KEY,
	RECENTS_MAX,
	shortcutKeys,
	isTypingTarget,
	createSequenceMatcher
} from '../src/lib/palette/logic.ts';

const item = (n) => ({ id: `device:${n}`, label: `nas-${n}`, href: `/targets/${n}`, kind: 'device' });

test('score: prefix beats word-prefix beats substring, no match is 0', () => {
	const e = { label: 'Add a monitor', keywords: 'new host' };
	assert.equal(score(e, queryWords('add')), 3);
	assert.equal(score(e, queryWords('mon')), 2);
	assert.equal(score(e, queryWords('onit')), 1);
	assert.equal(score(e, queryWords('host')), 1);
	assert.equal(score(e, queryWords('zzz')), 0);
	assert.equal(score(e, []), 1);
});

test('rank filters and keeps order among ties', () => {
	const list = [{ label: 'b host' }, { label: 'host a' }, { label: 'c host' }, { label: 'x' }];
	assert.deepEqual(
		rank(list, queryWords('host')).map((e) => e.label),
		['host a', 'b host', 'c host']
	);
});

test('pushRecent: newest first, deduplicated, capped at 8', () => {
	let list = [];
	for (let i = 1; i <= 10; i++) list = pushRecent(list, item(i));
	assert.equal(list.length, RECENTS_MAX);
	assert.equal(list[0].id, 'device:10');
	list = pushRecent(list, item(5));
	assert.equal(list[0].id, 'device:5');
	assert.equal(list.filter((i) => i.id === 'device:5').length, 1);
	assert.equal(list.length, RECENTS_MAX);
});

test('parseRecents tolerates garbage and refuses external hrefs', () => {
	assert.deepEqual(parseRecents(null), []);
	assert.deepEqual(parseRecents('not json'), []);
	assert.deepEqual(parseRecents('{"a":1}'), []);
	const raw = JSON.stringify([
		item(1),
		item(1),
		{ ...item(2), href: '//evil.example' },
		{ ...item(3), href: 'https://evil.example' },
		{ id: 4 },
		item(5)
	]);
	assert.deepEqual(
		parseRecents(raw).map((i) => i.id),
		['device:1', 'device:5']
	);
});

test('load/save round trip, and a failing store never throws', () => {
	const data = new Map();
	const store = { getItem: (k) => data.get(k) ?? null, setItem: (k, v) => data.set(k, v) };
	saveRecents(store, [item(1), item(2)]);
	assert.ok(data.has(RECENTS_KEY));
	assert.deepEqual(loadRecents(store), [item(1), item(2)]);
	const broken = {
		getItem() {
			throw new Error('denied');
		},
		setItem() {
			throw new Error('full');
		}
	};
	assert.deepEqual(loadRecents(broken), []);
	saveRecents(broken, [item(1)]);
	assert.deepEqual(loadRecents(null), []);
});

test('shortcutKeys uppercases single keys', () => {
	assert.deepEqual(shortcutKeys('g o'), ['G', 'O']);
	assert.deepEqual(shortcutKeys('?'), ['?']);
});

test('isTypingTarget', () => {
	assert.equal(isTypingTarget({ tagName: 'INPUT' }), true);
	assert.equal(isTypingTarget({ tagName: 'textarea' }), true);
	assert.equal(isTypingTarget({ tagName: 'DIV', isContentEditable: true }), true);
	assert.equal(isTypingTarget({ tagName: 'DIV', getAttribute: () => 'combobox' }), true);
	assert.equal(isTypingTarget({ tagName: 'BUTTON', getAttribute: () => null }), false);
	assert.equal(isTypingTarget(null), false);
});

test('sequence matcher: g then o, expiry, chords and restarts', () => {
	const mk = () =>
		createSequenceMatcher([
			{ id: 'overview', keys: 'g o' },
			{ id: 'devices', keys: 'g d' },
			{ id: 'help', keys: '?' }
		]);
	let m = mk();
	assert.equal(m.feed({ key: 'g' }, 0), null);
	assert.equal(m.feed({ key: 'o' }, 100), 'overview');
	assert.equal(m.feed({ key: 'o' }, 200), null);

	m = mk();
	m.feed({ key: 'g' }, 0);
	assert.equal(m.feed({ key: 'd' }, 5000), null, 'expired');

	m = mk();
	m.feed({ key: 'g' }, 0);
	assert.equal(m.feed({ key: 'g' }, 100), null);
	assert.equal(m.feed({ key: 'd' }, 200), 'devices', 'g g d restarts the sequence');

	m = mk();
	m.feed({ key: 'g' }, 0);
	assert.equal(m.feed({ key: 'o', ctrlKey: true }, 100), null, 'chord ignored');
	assert.equal(m.feed({ key: 'o' }, 150), null, 'chord cancelled the sequence');

	m = mk();
	assert.equal(m.feed({ key: '?' }, 0), 'help');
	assert.equal(m.feed({ key: 'Shift' }, 10), null);
});
