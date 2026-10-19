// Wall panels: the round of headlines, pictures and widget cards. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	buildPanels,
	cardMetrics,
	DEFAULT_ROWS,
	interleave,
	MAX_HEADLINES,
	MAX_PICTURES,
	nextPanel,
	rowsThatFit,
	widgetRows
} from '../src/lib/wall/panels.ts';

const section = (data) => ({ data, updated: '2026-10-19T09:00:00', error: null, needs_key: false, source: 'X', source_url: 'https://x' });
const article = (i, image) => ({ title: `Title ${i}`, link: `https://x/${i}`, source: 'S', published: null, image });

test('interleaving takes one from each list in turn', () => {
	assert.deepEqual(interleave([1, 2, 3], ['a']), [1, 'a', 2, 3]);
	assert.deepEqual(interleave([], ['a', 'b']), ['a', 'b']);
	assert.deepEqual(interleave([], []), []);
});

test('rows that fit: from the room and the measured chrome, never below one', () => {
	assert.equal(rowsThatFit(0, 60, 44, 5), 5);
	assert.equal(rowsThatFit(300, 60, 0, 5), 5);
	assert.equal(rowsThatFit(300, 60, 44, 5), 5);
	assert.equal(rowsThatFit(200, 60, 44, 5), 3);
	assert.equal(rowsThatFit(50, 60, 44, 5), 1);
	assert.deepEqual(cardMetrics(104, 148), { chrome: 60, row: 44 });
	assert.equal(cardMetrics(0, 148), null);
	assert.equal(cardMetrics(148, 148), null);
});

test('a widget has rows only when it is on, keyed and has data', () => {
	assert.equal(widgetRows(null, 'cve'), 0);
	const widgets = {
		rotate_seconds: 20,
		crypto: section({ currency: 'USD', coins: new Array(7).fill({}) }),
		stocks: { ...section(null), needs_key: true },
		fx: section({ base: 'EUR', date: '2026-10-16', rates: [] }),
		cve: section(new Array(3).fill({})),
		claude: { ...section(null), error: 'boom' },
		openai: section({ period: 'week', from: '2026-10-12', cost_usd: 4, input_tokens: 10, output_tokens: 5, people: null })
	};
	assert.equal(widgetRows(widgets, 'crypto'), 7);
	assert.equal(widgetRows(widgets, 'stocks'), 0);
	assert.equal(widgetRows(widgets, 'fx'), 0);
	assert.equal(widgetRows(widgets, 'cve'), 3);
	assert.equal(widgetRows(widgets, 'claude'), 0);
	// The usage card always has its total, even with nobody listed.
	assert.equal(widgetRows(widgets, 'openai'), 1);
});

test('widgets alone: one card per widget with data, long lists cut by the rows that fit', () => {
	const widgets = {
		rotate_seconds: 20,
		crypto: section({ currency: 'USD', coins: new Array(7).fill({}) }),
		cve: section(new Array(5).fill({})),
		claude: section({ period: 'week', from: '2026-10-12', cost_usd: 4, input_tokens: 10, output_tokens: 5, people: new Array(7).fill({}) })
	};
	const panels = buildPanels({ news: [], pictures: false, widgets });
	assert.deepEqual(panels.map((p) => p.key), ['crypto:0', 'crypto:1', 'cve:0', 'cve:1', 'claude:0', 'claude:1']);
	assert.deepEqual(panels.filter((p) => p.kind === 'crypto').map((p) => [p.from, p.to, p.page, p.pages]), [
		[0, 6, 0, 2],
		[6, 7, 1, 2]
	]);
	// A shorter room: more pages, never a clipped row.
	const short = buildPanels({ news: [], pictures: false, widgets, rows: { crypto: 3, cve: 5, claude: 2 } });
	assert.deepEqual(short.map((p) => p.key), ['crypto:0', 'crypto:1', 'crypto:2', 'cve:0', 'claude:0', 'claude:1', 'claude:2', 'claude:3']);
	assert.deepEqual(buildPanels({ news: [], pictures: false, widgets: null }), []);
});

test('news without pictures: pages of headlines, capped', () => {
	const news = Array.from({ length: 30 }, (_, i) => article(i, `/api/wall/news/image/${i}`));
	const panels = buildPanels({ news, pictures: false, widgets: null });
	assert.ok(panels.every((p) => p.kind === 'headlines'));
	assert.equal(panels.reduce((n, p) => n + p.items.length, 0), MAX_HEADLINES);
	assert.equal(panels.length, Math.ceil(MAX_HEADLINES / DEFAULT_ROWS.headlines));
	assert.deepEqual(panels.map((p) => [p.page, p.pages]), [[0, 4], [1, 4], [2, 4], [3, 4]]);
});

test('pictures on: pictured articles get a panel, the others stay headlines, failed pictures too', () => {
	const news = [article(0, '/i/0'), article(1, null), article(2, '/i/2'), article(3, null), article(4, '/i/4')];
	const panels = buildPanels({ news, pictures: true, widgets: null });
	assert.deepEqual(panels.map((p) => p.kind), ['picture', 'headlines', 'picture', 'picture']);
	assert.deepEqual(panels[1].items.map((i) => i.link), ['https://x/1', 'https://x/3']);
	// A picture that did not load: its article is read as a headline instead.
	const failed = buildPanels({ news, pictures: true, widgets: null, failed: new Set(['/i/2']) });
	assert.deepEqual(failed.map((p) => p.kind), ['picture', 'headlines', 'picture']);
	assert.deepEqual(failed[1].items.map((i) => i.link), ['https://x/1', 'https://x/2', 'https://x/3']);
	// A feed with no picture at all still shows, as headlines.
	const text = buildPanels({ news: [article(0, null), article(1, null)], pictures: true, widgets: null });
	assert.deepEqual(text.map((p) => p.kind), ['headlines']);
	// At most MAX_PICTURES pictures a round; the rest are headlines.
	const many = Array.from({ length: 20 }, (_, i) => article(i, `/i/${i}`));
	const capped = buildPanels({ news: many, pictures: true, widgets: null });
	assert.equal(capped.filter((p) => p.kind === 'picture').length, MAX_PICTURES);
	assert.equal(capped.filter((p) => p.kind === 'headlines').reduce((n, p) => n + p.items.length, 0), 12);
});

test('news and widgets take turns', () => {
	const news = [article(0, '/i/0'), article(1, null)];
	const widgets = { rotate_seconds: 20, crypto: section({ currency: 'USD', coins: [{}] }), cve: section([{}]) };
	const panels = buildPanels({ news, pictures: true, widgets });
	assert.deepEqual(panels.map((p) => p.key), ['picture:https://x/0', 'crypto:0', 'headlines:0', 'cve:0']);
});

test('the next panel wraps and survives a shrinking round', () => {
	assert.equal(nextPanel(5, 6), 0);
	assert.equal(nextPanel(9, 3), 1);
	assert.equal(nextPanel(0, 0), 0);
});
