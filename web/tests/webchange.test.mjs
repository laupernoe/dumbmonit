// Website-changes panel helpers. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { pagePath, shortenPath, skipLabel, clampPercent, stepPercent, groupByDay, daysBetween, stepIndex } from '../src/lib/components/devices/webchange/format.ts';

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

test('groupByDay buckets items by local calendar day, keeping their order', () => {
	const at = (y, mo, d, h) => new Date(y, mo - 1, d, h);
	const items = [
		{ id: 1, when: at(2026, 10, 13, 18) },
		{ id: 2, when: at(2026, 10, 13, 0) },
		{ id: 3, when: at(2026, 10, 12, 23) },
		{ id: 4, when: null },
		{ id: 5, when: at(2026, 10, 1, 9) }
	];
	const groups = groupByDay(items, (item) => item.when);
	assert.deepEqual(
		groups.map((g) => [g.key, g.items.map((i) => i.id)]),
		[
			['2026-10-13', [1, 2]],
			['2026-10-12', [3]],
			['', [4]],
			['2026-10-01', [5]]
		]
	);
	assert.equal(groups[0].day.getDate(), 13);
	assert.equal(groups[2].day, null);
	assert.deepEqual(groupByDay([], () => null), []);
});

test('daysBetween counts calendar days, not 24-hour spans', () => {
	const now = new Date(2026, 9, 13, 0, 30);
	assert.equal(daysBetween(new Date(2026, 9, 13, 0, 5), now), 0);
	assert.equal(daysBetween(new Date(2026, 9, 12, 23, 55), now), 1);
	assert.equal(daysBetween(new Date(2026, 9, 6, 12), now), 7);
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
