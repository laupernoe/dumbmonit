// Website-changes panel helpers. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { pagePath, shortenPath, skipLabel, clampPercent, stepPercent } from '../src/lib/components/devices/webchange/format.ts';

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
