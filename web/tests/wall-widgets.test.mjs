// Wall widgets: trends, prices, rates, weather codes, the rotating deck. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	formatPercent,
	formatPrice,
	formatTokens,
	formatUsd,
	fitScale,
	fxQuote,
	hueOf,
	initialsOf,
	marketItems,
	packColumn,
	packRow,
	isStale,
	nextCard,
	trend,
	weatherKind,
	widgetCards
} from '../src/lib/wall/widgets.ts';

const section = (data) => ({ data, updated: '2026-10-16T12:00:00', error: null, needs_key: false, source: 'X', source_url: 'https://x' });

test('a rise or a fall is named, a tiny change is flat', () => {
	assert.equal(trend(1.03), 'up');
	assert.equal(trend(-0.57), 'down');
	assert.equal(trend(0.001), 'flat');
	assert.equal(trend(null), null);
	assert.equal(trend(Number.NaN), null);
});

test('percentages always carry their sign', () => {
	assert.equal(formatPercent(1.0376, 'en'), '+1.04%');
	assert.equal(formatPercent(-0.57, 'en'), '-0.57%');
	assert.equal(formatPercent(0, 'en'), '0.00%');
});

test('prices take the digits they need', () => {
	assert.equal(formatPrice(73119.4, 'usd', 'en'), '$73,119');
	assert.equal(formatPrice(3.5, 'EUR', 'en'), '€3.50');
	assert.equal(formatPrice(0.18342, 'USD', 'en'), '$0.1834');
	assert.match(formatPrice(0.5, 'BTC', 'en'), /BTC/);
});

test('rates read as the price of one foreign unit, per hundred when tiny', () => {
	const usd = fxQuote(1.1216, 1.1198);
	assert.equal(usd.per, 1);
	assert.ok(Math.abs(usd.value - 0.8916) < 0.0001);
	// The euro buys more dollars than yesterday: the dollar fell against it.
	assert.ok(usd.change < 0);
	const jpy = fxQuote(170.5, null);
	assert.equal(jpy.per, 100);
	assert.ok(Math.abs(jpy.value - 0.5865) < 0.001);
	assert.equal(jpy.change, null);
});

test('WMO codes map to a picture', () => {
	assert.equal(weatherKind(0), 'clear');
	assert.equal(weatherKind(2), 'partly');
	assert.equal(weatherKind(3), 'cloudy');
	assert.equal(weatherKind(48), 'fog');
	assert.equal(weatherKind(53), 'drizzle');
	assert.equal(weatherKind(81), 'rain');
	assert.equal(weatherKind(75), 'snow');
	assert.equal(weatherKind(99), 'storm');
});

test('the rail: only widgets that are on and ready, coins and rates stay in the bar', () => {
	assert.deepEqual(widgetCards(null), []);
	const widgets = {
		rotate_seconds: 20,
		weather: section({ name: 'Lausanne', temperature_unit: '°C', temperature: 18.4, code: 3, is_day: true }),
		crypto: section({ currency: 'USD', coins: new Array(7).fill({}) }),
		// Waiting for a key, or failing with nothing to show: never a card on a wall.
		stocks: { ...section(null), needs_key: true },
		fx: section({ base: 'EUR', date: '2026-10-16', rates: [] }),
		cve: section(new Array(7).fill({})),
		claude: { ...section(null), error: 'boom' },
		openai: null
	};
	const cards = widgetCards(widgets);
	assert.deepEqual(cards.map((card) => card.key), ['cve:0', 'cve:1']);
	assert.deepEqual(cards.map((card) => [card.from, card.to, card.page, card.pages]), [
		[0, 5, 0, 2],
		[5, 7, 1, 2]
	]);
	assert.deepEqual(widgetCards({ rotate_seconds: 20, cve: section([]), stocks: section({ quotes: [], unknown: [] }) }), []);
	assert.equal(nextCard(5, 6), 0);
	assert.equal(nextCard(9, 3), 1);
	assert.equal(nextCard(0, 0), 0);
});

test('the market bar: coins with their symbol, then currencies against the base', () => {
	assert.deepEqual(marketItems(null), []);
	const items = marketItems({
		rotate_seconds: 20,
		crypto: section({ currency: 'USD', coins: [{ id: 'bitcoin', symbol: 'btc', name: 'Bitcoin', price: 84210, change_24h: 2.4 }] }),
		fx: section({
			base: 'CHF',
			date: '2026-10-16',
			rates: [
				{ code: 'EUR', rate: 1.0638, previous: 1.0627 },
				{ code: 'JPY', rate: 188.7, previous: null }
			]
		}),
		stocks: section({ quotes: [{ symbol: 'AAPL', price: 1, change: null, change_pct: null, at: null }], unknown: [] })
	});
	assert.deepEqual(items.map((i) => [i.group, i.symbol, i.currency]), [
		['crypto', 'BTC', 'USD'],
		['fx', 'EUR', 'CHF'],
		['fx', '100 JPY', 'CHF']
	]);
	// Without a key or data, a source adds nothing.
	assert.deepEqual(marketItems({ rotate_seconds: 20, crypto: { ...section(null), needs_key: true }, fx: section(null) }), []);
});

test('packing: cards stack while they fit, one page of its own when too tall', () => {
	assert.deepEqual(packColumn([100, 100, 100], 340, 12), [[0, 1, 2]]);
	assert.deepEqual(packColumn([100, 100, 100], 300, 12), [[0, 1], [2]]);
	assert.deepEqual(packColumn([500, 100], 300, 12), [[0], [1]]);
	assert.deepEqual(packColumn([], 300, 12), []);
});

test('packing a row: a group label is paid at each page start and group change', () => {
	const groups = ['crypto', 'crypto', 'fx', 'fx'];
	const labels = { crypto: 60, fx: 40 };
	assert.deepEqual(packRow([100, 100, 100, 100], groups, labels, 1000, 10), [[0, 1, 2, 3]]);
	// 60+10+100 + 10+100 = 280, then the fx label 40+10 and 110 more: 440, the last does not fit in 450.
	assert.deepEqual(packRow([100, 100, 100, 100], groups, labels, 450, 10), [[0, 1, 2], [3]]);
	assert.deepEqual(packRow([100], ['fx'], labels, 20, 10), [[0]]);
});

test('scale: grows when there is room, shrinks to fit, within bounds', () => {
	assert.equal(fitScale(100, 110), 1.1);
	assert.equal(fitScale(100, 500), 1.2);
	assert.equal(fitScale(100, 50), 0.7);
	assert.equal(fitScale(0, 50), 1);
	assert.equal(fitScale(100, 0), 1);
});

test('badges: initials and a stable hue from the name', () => {
	assert.equal(initialsOf('Ada Lovelace'), 'AL');
	assert.equal(initialsOf('ada.lovelace@x.io'), 'AL');
	assert.equal(initialsOf('noe'), 'NO');
	assert.equal(initialsOf('élodie'), 'ÉL');
	assert.equal(initialsOf('  '), '?');
	assert.equal(hueOf('Ada'), hueOf(' ada '));
	assert.ok(hueOf('Ada') >= 0 && hueOf('Ada') < 360);
	assert.notEqual(hueOf('Ada'), hueOf('Grace'));
});

test('stale data is noticed', () => {
	const now = Date.parse('2026-10-16T12:10:00Z');
	assert.equal(isStale('2026-10-16T12:00:00', now, 60_000), true);
	assert.equal(isStale('2026-10-16T12:09:30', now, 60_000), false);
	assert.equal(isStale(null, now, 60_000), false);
});

test('AI usage: dollars and compact tokens', () => {
	assert.equal(formatUsd(12.345, 'en'), '$12.35');
	assert.equal(formatUsd(1234.5, 'en'), '$1,235');
	assert.equal(formatTokens(980, 'en'), '980');
	assert.equal(formatTokens(1_200_000, 'en'), '1.2M');
	assert.equal(formatTokens(340_000, 'en'), '340K');
});

test('AI cards: a total card, then one page per four people', () => {
	const usage = (people) => section({ period: 'week', from: '2026-10-12', cost_usd: 4, input_tokens: 10, output_tokens: 5, people });
	const widgets = {
		rotate_seconds: 20,
		claude: usage(new Array(7).fill({})),
		openai: usage(null)
	};
	const cards = widgetCards(widgets);
	assert.deepEqual(cards.map((card) => card.key), ['claude:0', 'claude:1', 'openai:0']);
	assert.deepEqual(cards.filter((card) => card.kind === 'claude').map((card) => [card.from, card.to]), [
		[0, 4],
		[4, 7]
	]);
	// A provider turned on without a key has no card.
	assert.deepEqual(widgetCards({ rotate_seconds: 20, openai: { ...section(null), needs_key: true } }), []);
});
