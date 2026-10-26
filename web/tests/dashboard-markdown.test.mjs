// Text widget Markdown. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseMarkdown, parseInline, safeHref } from '../src/lib/dashboards/markdown.ts';

test('headings, paragraphs and lists', () => {
	const blocks = parseMarkdown('# Rack\n\nAll **quiet** today.\n\n- one\n- two\n\n1. first');
	assert.deepEqual(blocks.map((b) => b.t), ['h', 'p', 'ul', 'ol']);
	assert.equal(blocks[2].items.length, 2);
	assert.deepEqual(blocks[1].inline[1], { t: 'b', v: 'quiet' });
});

test('only safe links become links, markup stays text', () => {
	assert.equal(safeHref('javascript:alert(1)'), null);
	assert.equal(safeHref('//evil.example'), null);
	assert.equal(safeHref('/targets/3'), '/targets/3');
	const inline = parseInline('[x](javascript:alert(1)) <img src=x onerror=1>');
	assert.ok(inline.every((i) => i.t === 'text'));
});

test('fenced code is kept verbatim', () => {
	const blocks = parseMarkdown('```\na *b*\n```');
	assert.deepEqual(blocks, [{ t: 'code', text: 'a *b*' }]);
});
