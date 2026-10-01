// Spotify on the wall: speaker support, now playing, progress. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	cardVisible,
	deviceLabel,
	formatTime,
	fromSdkState,
	pickCover,
	coverUrl,
	pickPlaying,
	progressAt,
	speakerSupport,
	PAUSE_GRACE_MS,
	SDK_URL
} from '../src/lib/wall/spotify.ts';

const track = (over = {}) => ({
	playing: true,
	kind: 'track',
	title: 'Teardrop',
	artists: ['Massive Attack'],
	album: 'Mezzanine',
	cover_url: 'https://i.scdn.co/image/300',
	duration_ms: 330_000,
	progress_ms: 61_000,
	device_name: 'Living room TV',
	device_type: 'TV',
	...over
});

test('a page over plain HTTP is told why it cannot be a speaker', async () => {
	const result = await speakerSupport({ isSecureContext: false, navigator: {} });
	assert.equal(result.ok, false);
	assert.match(result.reason, /plain HTTP/);
	assert.match(result.fix, /HTTPS/);
});

test('a browser without protected media is told so', async () => {
	const result = await speakerSupport({ isSecureContext: true, navigator: {} });
	assert.equal(result.ok, false);
	assert.match(result.reason, /Encrypted Media/);
});

test('Widevine, or FairPlay on Safari, makes a speaker', async () => {
	const asked = [];
	const widevine = await speakerSupport({
		isSecureContext: true,
		navigator: {
			requestMediaKeySystemAccess: async (system) => {
				asked.push(system);
				return {};
			}
		}
	});
	assert.deepEqual(widevine, { ok: true });
	assert.deepEqual(asked, ['com.widevine.alpha']);

	const safari = await speakerSupport({
		isSecureContext: true,
		navigator: {
			requestMediaKeySystemAccess: async (system) => {
				if (system !== 'com.apple.fps') throw new Error('unsupported');
				return {};
			}
		}
	});
	assert.deepEqual(safari, { ok: true });

	const none = await speakerSupport({
		isSecureContext: true,
		navigator: {
			requestMediaKeySystemAccess: async () => {
				throw new Error('unsupported');
			}
		}
	});
	assert.equal(none.ok, false);
	assert.match(none.reason, /Widevine/);
	assert.match(none.fix, /Firefox/);
});

test('the SDK script is the one the CSP allows', () => {
	assert.equal(SDK_URL, 'https://sdk.scdn.co/spotify-player.js');
});

test("the wall's own player state reads like the server's", () => {
	const now = fromSdkState(
		{
			paused: false,
			position: 1_000,
			duration: 200_000,
			track_window: {
				current_track: {
					name: 'Windowlicker',
					type: 'track',
					artists: [{ name: 'Aphex Twin' }],
					album: {
						name: 'Windowlicker',
						images: [
							{ url: 'https://i.scdn.co/image/64', width: 64 },
							{ url: 'https://i.scdn.co/image/640', width: 640 },
							{ url: 'https://i.scdn.co/image/300', width: 300 }
						]
					}
				}
			}
		},
		'DumbMonit Wall'
	);
	assert.equal(now.playing, true);
	assert.equal(now.title, 'Windowlicker');
	assert.deepEqual(now.artists, ['Aphex Twin']);
	assert.equal(now.cover_url, 'https://i.scdn.co/image/300');
	assert.equal(now.device_name, 'DumbMonit Wall');
	assert.equal(now.progress_ms, 1_000);
	assert.equal(fromSdkState(null, 'x'), null);
	assert.equal(fromSdkState({ paused: true, position: 0, duration: 0, track_window: {} }, 'x'), null);
});

test('covers come from Spotify only, on the origin the CSP allows', () => {
	assert.equal(pickCover([{ url: 'https://evil.example/c.png', width: 640 }]), null);
	assert.equal(pickCover([{ url: 'http://i.scdn.co/image/a', width: 640 }]), null);
	assert.equal(pickCover([{ url: 'https://i.scdn.co/image/small', width: 64 }]), 'https://i.scdn.co/image/small');
	assert.equal(pickCover(undefined), null);
	const id = 'ab67616d00001e02f907de96b9a4fbc04accc0d5';
	assert.equal(coverUrl(`https://image-cdn-fa.spotifycdn.com/image/${id}`), `https://i.scdn.co/image/${id}`);
	assert.equal(coverUrl(`https://image-cdn-fa.spotifycdn.com.evil.test/image/${id}`), null);
	assert.equal(coverUrl(`https://mosaic.scdn.co/640/${id}`), null);
	assert.equal(coverUrl(`https://i.scdn.co/image/${id}?x=1`), null);
});

test('this display wins while it plays, otherwise the account', () => {
	const local = track({ device_name: 'DumbMonit Wall', title: 'Local' });
	const remote = track({ title: 'Remote' });
	assert.equal(pickPlaying(local, remote).source, 'speaker');
	assert.equal(pickPlaying({ ...local, playing: false }, remote).now.title, 'Remote');
	assert.equal(pickPlaying(null, remote).source, 'account');
	assert.equal(pickPlaying(null, null), null);
});

test('the card hides when nothing plays, after a short grace for a pause', () => {
	const at = 1_000_000;
	assert.equal(cardVisible(track(), null, at), true);
	assert.equal(cardVisible(null, at, at), false);
	assert.equal(cardVisible(track({ kind: 'ad' }), at, at), false);
	const paused = track({ playing: false });
	assert.equal(cardVisible(paused, at - 5_000, at), true);
	assert.equal(cardVisible(paused, at - PAUSE_GRACE_MS - 1, at), false);
	assert.equal(cardVisible(paused, null, at), false, 'paused since before the wall opened');
});

test('the progress moves on between two reads, and stops at the end', () => {
	assert.equal(progressAt(track(), 10_000, 13_000), 64_000);
	assert.equal(progressAt(track({ playing: false }), 10_000, 13_000), 61_000);
	assert.equal(progressAt(track({ progress_ms: 329_000 }), 0, 10_000), 330_000);
	assert.equal(formatTime(0), '0:00');
	assert.equal(formatTime(187_400), '3:07');
	assert.equal(formatTime(3_765_000), '1:02:45');
});

test('where it plays, in words', () => {
	assert.equal(deviceLabel(track(), 'account', 'DumbMonit Wall'), 'Living room TV');
	assert.equal(deviceLabel(track({ device_name: 'DumbMonit Wall' }), 'account', 'DumbMonit Wall'), 'this display');
	assert.equal(deviceLabel(track(), 'speaker', 'DumbMonit Wall'), 'this display');
	assert.equal(deviceLabel(track({ device_name: null, device_type: 'Smartphone' }), 'account', 'x'), 'a phone');
	assert.equal(deviceLabel(track({ device_name: null, device_type: 'Toaster' }), 'account', 'x'), 'another device');
});
