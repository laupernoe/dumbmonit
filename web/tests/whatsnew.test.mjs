// "What's new": the release list is well formed, and the display rules hold.
// WHATSNEW_REQUIRE=1 also demands an entry for the Cargo.toml version (release check).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { RELEASES, releaseFor, releaseNotesUrl } from '../src/lib/whatsnew/releases.ts';
import { releaseToShow } from '../src/lib/whatsnew/seen.ts';

test('every release is well formed', () => {
	const seen = new Set();
	for (const r of RELEASES) {
		assert.match(r.version, /^\d+\.\d+\.\d+(-[\w.]+)?$/, r.version);
		assert.match(r.date, /^\d{4}-\d{2}-\d{2}$/);
		assert.ok(!seen.has(r.version), `duplicate ${r.version}`);
		seen.add(r.version);
		assert.ok(r.highlights.length >= 3 && r.highlights.length <= 5, `${r.version}: 3 to 5 highlights`);
		for (const h of r.highlights) assert.ok(h.title && h.text);
	}
});

test('display rules', () => {
	const current = RELEASES[0].version;
	assert.equal(releaseToShow(current, null), null, 'first install shows nothing');
	assert.equal(releaseToShow(current, current), null, 'already seen');
	assert.equal(releaseToShow(current, '0.0.1'), releaseFor(current));
	assert.equal(releaseToShow('9.9.9', '0.0.1'), null, 'unlisted version shows nothing');
	assert.equal(releaseNotesUrl('0.1.0-alpha.7'), 'https://github.com/laupernoe/dumbmonit/releases/tag/v0.1.0-alpha.7');
});

test('the Cargo.toml version has an entry', { skip: !process.env.WHATSNEW_REQUIRE }, () => {
	const toml = readFileSync(new URL('../../Cargo.toml', import.meta.url), 'utf8');
	const version = toml.match(/^version\s*=\s*"([^"]+)"/m)[1];
	assert.ok(releaseFor(version), `add ${version} to web/src/lib/whatsnew/releases.ts`);
});
