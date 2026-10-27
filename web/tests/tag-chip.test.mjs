// Option chips on a monitor's page: never a raw PEM or a long multi-line value. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { tagValueSummary, tagChip } from '../src/lib/tag-chip.ts';

test('a PEM certificate is summarised, never shown', () => {
	const pem = '-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----';
	assert.equal(tagValueSummary(pem), 'PEM');
	const chip = tagChip('ca_cert', pem);
	assert.equal(chip.label, 'ca_cert = PEM');
	assert.ok(!chip.title.includes('MIIB'));
});

test('short values pass through, long and multi-line ones are cut', () => {
	assert.equal(tagValueSummary('636'), '636');
	assert.equal(tagValueSummary('a\nb'), 'a…');
	const long = tagValueSummary('x'.repeat(200));
	assert.equal(long.length, 48);
	assert.ok(long.endsWith('…'));
});
