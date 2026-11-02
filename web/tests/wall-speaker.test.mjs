// The wall as a Spotify speaker on a TV: diagnostics, retries, gestures, names. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	autoplayAllowed,
	browserLabel,
	displaySpeakerName,
	retryDelay,
	sdkFailure,
	speakerSupport
} from '../src/lib/wall/spotify.ts';

test('Edge with PlayReady only still makes a speaker', async () => {
	const result = await speakerSupport({
		isSecureContext: true,
		navigator: {
			requestMediaKeySystemAccess: async (system) => {
				if (system !== 'com.microsoft.playready') throw new Error('unsupported');
				return {};
			}
		}
	});
	assert.deepEqual(result, { ok: true });
});

test('a browser without DRM is pointed at what works instead', async () => {
	const none = await speakerSupport({
		isSecureContext: true,
		navigator: {
			requestMediaKeySystemAccess: async () => {
				throw new Error('unsupported');
			}
		}
	});
	assert.equal(none.ok, false);
	assert.match(none.reason, /cannot play Spotify/);
	assert.match(none.fix, /smart-TV and kiosk browsers cannot/);
	assert.match(none.fix, /Chromecast/);
});

test('the SDK errors are said plainly, and only passing ones are retried', () => {
	const drm = sdkFailure('initialization_error', 'Failed to initialize player');
	assert.match(drm.problem, /DRM/);
	assert.match(drm.problem, /Failed to initialize player/);
	assert.match(drm.fix, /Chrome, Edge or Firefox/);
	assert.match(drm.fix, /TV’s own Spotify app/);
	assert.equal(drm.permanent, true);

	const premium = sdkFailure('account_error');
	assert.match(premium.problem, /Premium/);
	assert.equal(premium.permanent, true);
	assert.equal(premium.noPremium, true);

	const auth = sdkFailure('authentication_error', 'Invalid token scopes.');
	assert.equal(auth.permanent, false);
	assert.match(auth.problem, /Invalid token scopes/);
});

test('retries wait longer each time, five minutes at most', () => {
	assert.deepEqual(
		[0, 1, 2, 3, 4, 5, 30].map(retryDelay),
		[15_000, 30_000, 60_000, 120_000, 240_000, 300_000, 300_000]
	);
});

test('a browser that already allows sound needs no tap', () => {
	assert.equal(autoplayAllowed({ getAutoplayPolicy: () => 'allowed' }), true);
	assert.equal(autoplayAllowed({ getAutoplayPolicy: () => 'allowed-muted' }), false);
	assert.equal(autoplayAllowed({ getAutoplayPolicy: () => 'disallowed' }), false);
	assert.equal(autoplayAllowed({}), false, 'unknown: ask for a tap');
	assert.equal(
		autoplayAllowed({
			getAutoplayPolicy: () => {
				throw new TypeError('not supported');
			}
		}),
		false
	);
});

test('a display is recognised by its browser and system', () => {
	assert.equal(
		browserLabel('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36'),
		'Chrome 130 on Linux'
	);
	assert.equal(
		browserLabel('Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:131.0) Gecko/20100101 Firefox/131.0'),
		'Firefox 131 on Windows'
	);
	assert.equal(
		browserLabel(
			'Mozilla/5.0 (SMART-TV; LINUX; Tizen 8.0) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/7.0 Chrome/108.0.5359.1 TV Safari/537.36'
		),
		'Samsung Internet 7 on Tizen'
	);
	assert.equal(
		browserLabel(
			'Mozilla/5.0 (Web0S; Linux/SmartTV) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/94.0.4606.128 Safari/537.36 WebAppManager'
		),
		'Chrome 94 on webOS'
	);
	assert.equal(browserLabel(''), 'A browser');
});

test('a display may announce its own name', () => {
	assert.equal(displaySpeakerName(null, null, 'DumbMonit Wall'), 'DumbMonit Wall');
	assert.equal(displaySpeakerName(' Kitchen ', 'Living room', 'DumbMonit Wall'), 'Kitchen');
	assert.equal(displaySpeakerName(null, 'Living room', 'DumbMonit Wall'), 'Living room');
	assert.equal(displaySpeakerName('', null, 'Office'), 'Office');
	assert.equal(displaySpeakerName('x'.repeat(65), null, 'Office'), 'Office');
});
