// Website-changes panel helpers. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { pagePath, shortenPath, skipLabel, clampPercent, stepPercent, stepIndex, groupSameEdits, wordDiff, pairWords } from '../src/lib/components/devices/webchange/format.ts';

test('pagePath keeps only the path, query and hash', () => {
	assert.equal(pagePath('https://example.com/'), '/');
	assert.equal(pagePath('https://example.com'), '/');
	assert.equal(pagePath('https://example.com/docs/intro?lang=en#top'), '/docs/intro?lang=en#top');
	assert.equal(pagePath('/already/a/path'), '/already/a/path');
	assert.equal(pagePath(''), '/');
});

test('shortenPath leaves short paths untouched', () => {
	assert.equal(shortenPath('/blog/post'), '/blog/post');
	assert.equal(shortenPath('/', 40), '/');
});

test('shortenPath elides the middle of a long path, keeping both ends', () => {
	const long = '/products/category/widgets/super-widget-3000/specifications/technical-sheet';
	const short = shortenPath(long, 40);
	assert.equal(short.length, 40);
	assert.ok(short.includes('…'));
	assert.ok(long.startsWith(short.split('…')[0]));
	assert.ok(long.endsWith(short.split('…')[1]));
});

test('skipLabel pluralises and defaults a null count to zero', () => {
	assert.equal(skipLabel(1), '⋯ 1 unchanged line');
	assert.equal(skipLabel(42), '⋯ 42 unchanged lines');
	assert.equal(skipLabel(null), '⋯ 0 unchanged lines');
});

test('clampPercent keeps the slider within the track', () => {
	assert.equal(clampPercent(-10), 0);
	assert.equal(clampPercent(50), 50);
	assert.equal(clampPercent(150), 100);
	assert.equal(clampPercent(NaN), 50);
});

test('stepPercent moves the slider by keyboard', () => {
	assert.equal(stepPercent(50, 'ArrowLeft'), 48);
	assert.equal(stepPercent(50, 'ArrowRight'), 52);
	assert.equal(stepPercent(1, 'ArrowLeft'), 0);
	assert.equal(stepPercent(99, 'ArrowRight'), 100);
	assert.equal(stepPercent(50, 'Home'), 0);
	assert.equal(stepPercent(50, 'End'), 100);
	assert.equal(stepPercent(50, 'Tab'), 50);
});

test('stepIndex walks a list by keyboard and stays within it', () => {
	assert.equal(stepIndex(0, 'ArrowDown', 3), 1);
	assert.equal(stepIndex(2, 'ArrowDown', 3), 2);
	assert.equal(stepIndex(1, 'ArrowUp', 3), 0);
	assert.equal(stepIndex(0, 'ArrowUp', 3), 0);
	assert.equal(stepIndex(1, 'Home', 3), 0);
	assert.equal(stepIndex(0, 'End', 3), 2);
	assert.equal(stepIndex(-1, 'ArrowDown', 3), 0);
	assert.equal(stepIndex(1, 'Tab', 3), 1);
	assert.equal(stepIndex(0, 'ArrowDown', 0), -1);
});

test('groupSameEdits folds consecutive changes with the same signature', () => {
	const c = (id, signature) => ({ id, signature });
	const groups = groupSameEdits([c(1, 'a'), c(2, 'a'), c(3, 'a'), c(4, null), c(5, null), c(6, 'b'), c(7, 'a')]);
	assert.deepEqual(
		groups.map((g) => [g.lead.id, g.others.map((o) => o.id)]),
		[[1, [2, 3]], [4, []], [5, []], [6, []], [7, []]]
	);
	assert.deepEqual(groupSameEdits([]), []);
});

test('wordDiff marks only the words that differ', () => {
	const { before, after } = wordDiff('Tarif : 10 € par mois', 'Tarif : 12 € par an');
	const changed = (segs) => segs.filter((s) => s.changed).map((s) => s.text);
	assert.deepEqual(changed(before), ['10', 'mois']);
	assert.deepEqual(changed(after), ['12', 'an']);
	assert.equal(before.map((s) => s.text).join(''), 'Tarif : 10 € par mois', 'the text joins back');
	const phrase = wordDiff('a b c d', 'a x y d');
	assert.deepEqual(changed(phrase.after), ['x y'], 'a replaced phrase is one run');
	const same = wordDiff('pareil', 'pareil');
	assert.deepEqual(same.after, [{ text: 'pareil', changed: false }]);
});

test('pairWords pairs deleted and inserted runs line by line', () => {
	const rows = pairWords([
		{ op: 'equal', text: 'Titre', count: null },
		{ op: 'delete', text: 'Prix 10', count: null },
		{ op: 'delete', text: 'Seul', count: null },
		{ op: 'insert', text: 'Prix 12', count: null },
		{ op: 'skip', text: '', count: 4 }
	]);
	assert.deepEqual(rows[0].segments, [{ text: 'Titre', changed: false }]);
	assert.deepEqual(rows[1].segments.filter((s) => s.changed).map((s) => s.text), ['10']);
	assert.deepEqual(rows[2].segments, [{ text: 'Seul', changed: false }], 'unpaired: whole');
	assert.deepEqual(rows[3].segments.filter((s) => s.changed).map((s) => s.text), ['12']);
	assert.equal(rows[4].count, 4);
});

test('wordDiff only compares what lies between the common head and tail', () => {
	const words = Array.from({ length: 3000 }, (_, i) => `w${i}`);
	const edited = [...words.slice(0, 1500), 'X', ...words.slice(1501)];
	const { before, after } = wordDiff(words.join(' '), edited.join(' '));
	assert.deepEqual(before.filter((s) => s.changed).map((s) => s.text), ['w1500']);
	assert.deepEqual(after.filter((s) => s.changed).map((s) => s.text), ['X']);
});

test('pairWords leaves pairs whole once the budget is spent', () => {
	const long = (tag) => Array.from({ length: 700 }, (_, i) => `${tag}${i}`).join(' ');
	const lines = [];
	for (let p = 0; p < 20; p++) lines.push({ op: 'delete', text: long('a'), count: null });
	for (let p = 0; p < 20; p++) lines.push({ op: 'insert', text: long('b'), count: null });
	const started = Date.now();
	const rows = pairWords(lines);
	assert.ok(Date.now() - started < 2000, 'stays fast');
	assert.ok(rows.some((r) => r.segments.length === 1 && !r.segments[0].changed), 'late pairs are whole');
});
