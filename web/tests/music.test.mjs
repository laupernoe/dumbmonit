// Share link → embed URL for the wall's music player. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseMusicLink, EMBED_ORIGINS } from '../src/lib/wall/music.ts';

const SP = '37i9dQZF1DXcBWIGoYBM5M';
const src = (link) => {
	const r = parseMusicLink(link);
	assert.ok(r.ok, `${link} should be accepted: ${r.message}`);
	return r.embed.src;
};
const refused = (link) => {
	const r = parseMusicLink(link);
	assert.equal(r.ok, false, `${link} should be refused, got ${r.embed?.src}`);
	assert.ok(r.message.length > 10);
	return r.message;
};

test('Spotify links become open.spotify.com/embed', () => {
	for (const kind of ['track', 'album', 'playlist', 'episode']) {
		assert.equal(src(`https://open.spotify.com/${kind}/${SP}?si=abc123`), `https://open.spotify.com/embed/${kind}/${SP}`);
	}
	assert.equal(src(`https://open.spotify.com/intl-fr/album/${SP}`), `https://open.spotify.com/embed/album/${SP}`);
	assert.equal(src(`open.spotify.com/playlist/${SP}`), `https://open.spotify.com/embed/playlist/${SP}`);
	assert.equal(src(`spotify:track:${SP}`), `https://open.spotify.com/embed/track/${SP}`);
	assert.equal(src(`https://open.spotify.com/embed/track/${SP}`), `https://open.spotify.com/embed/track/${SP}`);
	assert.equal(parseMusicLink(`https://open.spotify.com/track/${SP}`).embed.provider, 'spotify');
});

test('Deezer links become widget.deezer.com/widget/auto', () => {
	assert.equal(src('https://www.deezer.com/fr/playlist/1313621735'), 'https://widget.deezer.com/widget/auto/playlist/1313621735');
	assert.equal(src('https://www.deezer.com/track/3135556?utm=x'), 'https://widget.deezer.com/widget/auto/track/3135556');
	assert.equal(src('deezer.com/en/album/302127'), 'https://widget.deezer.com/widget/auto/album/302127');
	assert.equal(src('https://widget.deezer.com/widget/dark/album/302127'), 'https://widget.deezer.com/widget/auto/album/302127');
});

test('YouTube and YouTube Music links become youtube-nocookie embeds', () => {
	const v = 'dQw4w9WgXcQ';
	const l = 'PLFgquLnL59alCl_2TQvOiD5Vgm1hCaGSI';
	assert.equal(src(`https://www.youtube.com/watch?v=${v}`), `https://www.youtube-nocookie.com/embed/${v}`);
	assert.equal(src(`https://youtu.be/${v}?si=xyz`), `https://www.youtube-nocookie.com/embed/${v}`);
	assert.equal(src(`https://m.youtube.com/watch?v=${v}&list=${l}`), `https://www.youtube-nocookie.com/embed/${v}?list=${l}`);
	assert.equal(src(`https://www.youtube.com/playlist?list=${l}`), `https://www.youtube-nocookie.com/embed/videoseries?list=${l}`);
	assert.equal(src(`https://www.youtube.com/shorts/${v}`), `https://www.youtube-nocookie.com/embed/${v}`);
	assert.equal(src(`https://music.youtube.com/playlist?list=${l}`), `https://www.youtube-nocookie.com/embed/videoseries?list=${l}`);
	assert.equal(src(`https://music.youtube.com/watch?v=${v}&list=${l}`), `https://www.youtube-nocookie.com/embed/${v}?list=${l}`);
	assert.equal(parseMusicLink(`https://music.youtube.com/watch?v=${v}`).embed.label, 'YouTube Music');
});

test('every accepted link lands on one of the allowed origins', () => {
	for (const link of [`https://open.spotify.com/track/${SP}`, 'https://www.deezer.com/track/1', 'https://youtu.be/dQw4w9WgXcQ']) {
		assert.ok(EMBED_ORIGINS.includes(new URL(src(link)).origin), link);
	}
});

test('anything else is refused, never framed', () => {
	refused('');
	refused('   ');
	refused('not a link at all');
	refused('https://example.com/watch?v=dQw4w9WgXcQ');
	refused('https://open.spotify.com.evil.test/track/' + SP);
	refused('https://evil.test/open.spotify.com/track/' + SP);
	refused('javascript:alert(1)');
	refused('data:text/html,<script>alert(1)</script>');
	refused('ftp://open.spotify.com/track/' + SP);
	refused(`https://user:pw@open.spotify.com/track/${SP}`);
	refused(`https://open.spotify.com:8443/track/${SP}`);
	refused('https://open.spotify.com/artist/' + SP);
	refused('https://open.spotify.com/track/short');
	refused('https://open.spotify.com/track/' + SP.slice(0, 21) + '"');
	refused('https://www.deezer.com/fr/artist/27');
	refused('https://www.deezer.com/track/12ab');
	refused('https://www.youtube.com/@somechannel');
	refused('https://www.youtube.com/watch?v=short');
	refused('https://www.youtube.com/watch?v=dQw4w9WgXc"&x');
	refused('https://www.youtube.com/playlist?list=<bad>');
	assert.match(refused('https://spotify.link/AbCdEf'), /full/);
	assert.match(refused('https://deezer.page.link/xyz'), /full/);
	assert.match(refused('https://link.deezer.com/s/30abc'), /full/);
});
