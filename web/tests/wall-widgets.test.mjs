// Wall widgets: trends, prices, rates, weather codes, badges. The panels are in wall-panels.test.mjs. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	formatPercent,
	formatPrice,
	formatTokens,
	formatUsd,
	fxQuote,
	hueOf,
	initialsOf,
	isStale,
	trend,
	weatherKind
} from '../src/lib/wall/widgets.ts';


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
