// Runbook Markdown is rendered without ever letting HTML through. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { renderMarkdown, safeHref } from '../src/lib/markdown.ts';

test('raw HTML is shown as text, never as markup', () => {
	const html = renderMarkdown('<script>alert(1)</script>\n<img src=x onerror=alert(1)>');
	assert.ok(!html.includes('<script'), html);
	assert.ok(!html.includes('<img'), html);
	assert.ok(html.includes('&lt;script&gt;'), html);
});

test('only http(s) and same-site links are made', () => {
	assert.ok(renderMarkdown('[docs](https://example.org/a?b=1&c=2)').includes('<a href="https://example.org/a?b=1&amp;c=2"'));
	assert.ok(renderMarkdown('[page](/targets/3)').includes('<a href="/targets/3"'));
	for (const bad of ['javascript:alert(1)', 'data:text/html,x', '//evil.example', 'vbscript:x']) {
		const html = renderMarkdown(`[x](${bad})`);
		assert.ok(!html.includes('<a '), `${bad}: ${html}`);
	}
	assert.equal(safeHref('JaVaScRiPt:alert(1)'), false);
});

test('a link cannot break out of its attribute', () => {
	const html = renderMarkdown('[x](https://a.example/"onmouseover="alert(1))');
	assert.ok(!/<a [^>]*onmouseover/.test(html), html);
	assert.ok(!html.includes('"onmouseover'), html);
});

test('headings, lists, code and bold are rendered', () => {
	const html = renderMarkdown('# Disk full\n\n1. Check `df -h`\n2. **Clean** /var/log\n\n```\nrm -rf <cache>\n```');
	assert.ok(html.includes('<h3>Disk full</h3>'), html);
	assert.ok(html.includes('<ol><li>Check <code>df -h</code></li><li><strong>Clean</strong> /var/log</li></ol>'), html);
	assert.ok(html.includes('<pre><code>rm -rf &lt;cache&gt;</code></pre>'), html);
});

test('code spans are not formatted inside', () => {
	const html = renderMarkdown('`**not bold** [x](https://a.example)`');
	assert.ok(!html.includes('<strong>'), html);
	assert.ok(!html.includes('<a '), html);
});
