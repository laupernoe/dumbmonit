// Map export: the PNG image must come from a data: URL (the page's CSP refuses blob images).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { ExportError, pngSize, svgDataUrl, viewBoxSize, xmlSafe } from '../src/lib/components/topology/export.ts';

test('the SVG travels as a data: URL, never a blob: one', () => {
	const svg = '<svg xmlns="http://www.w3.org/2000/svg"><text>é & "x" #1</text></svg>';
	const url = svgDataUrl(svg);
	assert.ok(url.startsWith('data:image/svg+xml;charset=utf-8,'));
	assert.equal(decodeURIComponent(url.slice(url.indexOf(',') + 1)), svg);
});

test('characters XML forbids are dropped or replaced', () => {
	assert.equal(xmlSafe('a\u0000b\u001Fc'), 'abc');
	assert.equal(xmlSafe('ok \u{1F600}'), 'ok \u{1F600}');
	assert.equal(xmlSafe('cut \uD83D'), 'cut �');
});

test('view box and PNG size', () => {
	assert.deepEqual(viewBoxSize('0 0 1200 800'), { w: 1200, h: 800 });
	assert.deepEqual(viewBoxSize(null), { w: 0, h: 0 });
	assert.deepEqual(pngSize(1000, 500), { width: 2000, height: 1000 });
	const big = pngSize(12000, 6000);
	assert.equal(Math.max(big.width, big.height), 6000);
	assert.deepEqual(pngSize(0, 0), { width: 1, height: 1 });
});

test('a local export failure names its step', () => {
	const error = new ExportError('image');
	assert.equal(error.step, 'image');
	assert.ok(error instanceof Error);
});
