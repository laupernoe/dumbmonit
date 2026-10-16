// Wall widgets: trends, prices, rates, weather codes, the rotating deck. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	formatPercent,
	formatPrice,
	fxQuote,
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

test('the deck: no weather card, long lists split in pages, off widgets absent', () => {
	assert.deepEqual(widgetCards(null), []);
	const widgets = {
		rotate_seconds: 20,
		weather: section({ name: 'Lausanne', temperature_unit: '°C', temperature: 18.4, code: 3, is_day: true }),
		crypto: section({ currency: 'USD', coins: new Array(7).fill({}) }),
		stocks: { ...section(null), needs_key: true },
		fx: null,
		cve: section(new Array(3).fill({}))
	};
	const keys = widgetCards(widgets).map((card) => card.key);
	assert.deepEqual(keys, ['crypto:0', 'crypto:1', 'stocks:0', 'cve:0']);
	const crypto = widgetCards(widgets).filter((card) => card.kind === 'crypto');
	assert.deepEqual(
		crypto.map((card) => [card.from, card.to, card.page, card.pages]),
		[
			[0, 5, 0, 2],
			[5, 7, 1, 2]
		]
	);
	assert.equal(nextCard(5, 6), 0);
	assert.equal(nextCard(9, 3), 1);
	assert.equal(nextCard(0, 0), 0);
});

test('stale data is noticed', () => {
	const now = Date.parse('2026-10-16T12:10:00Z');
	assert.equal(isStale('2026-10-16T12:00:00', now, 60_000), true);
	assert.equal(isStale('2026-10-16T12:09:30', now, 60_000), false);
	assert.equal(isStale(null, now, 60_000), false);
});
