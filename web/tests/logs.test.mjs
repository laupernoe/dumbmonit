// Logs section logic: export period, sizes, destination payload. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	buildDestinationPayload,
	clampDays,
	destinationChanged,
	exportChoices,
	formatSize,
	inRange
} from '../src/lib/components/settings/logs.ts';

const field = (key, extra = {}) => ({
	key, label: key, required: false, input: 'text', help: '', placeholder: '', options: [], shape: 'scalar', default: '', ...extra
});

test('export choices beyond the retention are disabled', () => {
	assert.deepEqual(exportChoices(14).map((c) => c.disabled), [false, false, false, true]);
	assert.deepEqual(exportChoices(1).map((c) => c.disabled), [false, true, true, true]);
	assert.deepEqual(exportChoices(365).map((c) => c.disabled), [false, false, false, false]);
});

test('custom days are whole numbers bounded by the retention', () => {
	assert.equal(clampDays('10', 14), 10);
	assert.equal(clampDays(99, 14), 14);
	assert.equal(clampDays(3.9, 14), 3);
	assert.equal(clampDays(0, 14), null);
	assert.equal(clampDays('', 14), null);
	assert.equal(clampDays('abc', 14), null);
});

test('range check for retention and cap', () => {
	assert.equal(inRange('14', 1, 365), true);
	assert.equal(inRange(0, 1, 365), false);
	assert.equal(inRange(366, 1, 365), false);
	assert.equal(inRange(1.5, 1, 365), false);
	assert.equal(inRange('', 16, 102400), false);
});

test('sizes are readable', () => {
	assert.equal(formatSize(0), '0 B');
	assert.equal(formatSize(512), '512 B');
	assert.equal(formatSize(1536), '1.5 kB');
	assert.equal(formatSize(5 * 1024 * 1024), '5.0 MB');
	assert.equal(formatSize(300 * 1024 * 1024), '300 MB');
});

const settingFields = [
	field('transport', { input: 'select', options: ['udp', 'tcp'] }),
	field('address', { destination: true }),
	field('tls', { input: 'boolean' })
];
const secretFields = [field('password', { input: 'password' })];

test('creating always sends secrets, even empty', () => {
	const p = buildDestinationPayload({
		name: ' Graylog ', kind: 'gelf', enabled: true, minLevel: 'info', settingFields, secretFields,
		settings: { transport: 'udp', address: 'graylog:12201', tls: false }, secrets: {}
	});
	assert.equal(p.name, 'Graylog');
	assert.deepEqual(p.secrets, {});
	assert.deepEqual(p.settings, { transport: 'udp', address: 'graylog:12201', tls: false });
});

const existing = {
	id: 1, name: 'g', kind: 'gelf', enabled: true, min_level: 'info', has_secret: true, stats: null,
	settings: { transport: 'udp', address: 'graylog:12201', tls: false }
};
const base = {
	name: 'g', kind: 'gelf', enabled: true, minLevel: 'info', settingFields, secretFields, existing,
	settings: { transport: 'udp', address: 'graylog:12201', tls: false }, secrets: { password: '  ' }
};

test('editing with empty secrets omits them so the server keeps them', () => {
	const p = buildDestinationPayload(base);
	assert.equal('secrets' in p, false);
});

test('editing with a typed secret sends only that one; clearing sends null', () => {
	assert.deepEqual(buildDestinationPayload({ ...base, secrets: { password: ' x ' } }).secrets, { password: 'x' });
	assert.deepEqual(buildDestinationPayload({ ...base, clearSecrets: true }).secrets, { password: null });
});

test('a changed destination field asks for the secrets again', () => {
	assert.equal(destinationChanged(existing, 'gelf', settingFields, base.settings), false);
	assert.equal(destinationChanged(existing, 'gelf', settingFields, { ...base.settings, address: 'other:1' }), true);
	assert.equal(destinationChanged(existing, 'loki', settingFields, base.settings), true);
	assert.equal(destinationChanged({ ...existing, has_secret: false }, 'loki', settingFields, base.settings), false);
});

test('blank optional settings are dropped, unknown stored ones kept on same kind', () => {
	const p = buildDestinationPayload({
		...base,
		existing: { ...existing, settings: { ...existing.settings, future: 1 } },
		settings: { transport: '', address: 'a:1', tls: true }
	});
	assert.deepEqual(p.settings, { address: 'a:1', tls: true, future: 1 });
});
