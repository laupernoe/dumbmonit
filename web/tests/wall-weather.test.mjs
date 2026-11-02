// The wall's weather follows the network: clear, clouds, rain, thunderstorm. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	weatherFor,
	sceneMoodFor,
	nightToneFor,
	weatherForTone,
	parseForcedWeather
} from '../src/lib/components/wall/wallWeather.ts';

const sky = (attention, warnings = 0, unreachable = 0) => ({ attention, warnings, unreachable });

test('nothing to look at is clear skies', () => {
	assert.equal(weatherFor(sky(0)), 'clear');
});

test('advisories alone are clouds', () => {
	assert.equal(weatherFor(sky(3)), 'cloudy');
});

test('a device down or a critical alert is rain', () => {
	assert.equal(weatherFor(sky(1, 0, 1)), 'rain');
	assert.equal(weatherFor(sky(1, 1, 0)), 'rain');
	assert.equal(weatherFor(sky(2, 1, 1)), 'rain');
});

test('two critical alerts, or three things down or critical, are a thunderstorm', () => {
	assert.equal(weatherFor(sky(2, 2, 0)), 'thunder');
	assert.equal(weatherFor(sky(3, 1, 2)), 'thunder');
	assert.equal(weatherFor(sky(5, 0, 3)), 'thunder');
});

test('each scene family reads the weather in its own words', () => {
	assert.deepEqual(['clear', 'cloudy', 'rain', 'thunder'].map(sceneMoodFor), ['calm', 'clouded', 'storm', 'storm']);
	assert.deepEqual(['clear', 'cloudy', 'rain', 'thunder'].map(nightToneFor), ['ok', 'warn', 'critical', 'critical']);
	assert.deepEqual(['ok', 'warn', 'critical'].map(weatherForTone), ['clear', 'cloudy', 'thunder']);
});

test('a ?weather= value names a state or nothing', () => {
	assert.equal(parseForcedWeather('thunder'), 'thunder');
	assert.equal(parseForcedWeather('snow'), null);
	assert.equal(parseForcedWeather(null), null);
});
