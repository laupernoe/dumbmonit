// Checklist and rows options of a collector form. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	checkedValues,
	encodeChecked,
	emptyRow,
	parseRows,
	serializeRows,
	rowErrors,
	firstRowError,
	seedChecklists
} from '../src/lib/components/device-form/option-values.ts';

const item = (value) => ({ value, label: value, help: '', badges: [] });
const checklist = {
	key: 'lists',
	input: 'checklist',
	default: 'a,c',
	items: ['a', 'b', 'c'].map(item),
	columns: []
};
const zone = {
	key: 'zone',
	label: 'Zone',
	placeholder: '',
	input: 'text',
	choices: [],
	pattern: '^[a-z0-9-]+(\\.[a-z0-9-]+)*\\.[a-z0-9-]*[a-z][a-z0-9-]*$',
	invalid: 'Not a zone',
	unique: true
};
const rows = {
	key: 'custom_lists',
	input: 'rows',
	default: '',
	items: [],
	columns: [
		zone,
		{ key: 'subject', label: 'Type', placeholder: '', input: 'select', choices: ['ip', 'domain'], pattern: '', invalid: '', unique: false },
		{ key: 'major', label: 'Major', placeholder: '', input: 'boolean', choices: [], pattern: '', invalid: '', unique: false }
	]
};
const messages = { invalid: 'Invalid', duplicate: 'Duplicate' };

test('an absent checklist tag ticks everything, "-" nothing', () => {
	assert.deepEqual(checkedValues(checklist, undefined), ['a', 'b', 'c']);
	assert.deepEqual(checkedValues(checklist, ''), ['a', 'b', 'c']);
	assert.deepEqual(checkedValues(checklist, '-'), []);
	assert.deepEqual(checkedValues(checklist, 'c, a, gone'), ['a', 'c']);
});

test('a checklist encodes to a blank tag only when everything is ticked', () => {
	assert.equal(encodeChecked(checklist, ['a', 'b', 'c']), '');
	assert.equal(encodeChecked(checklist, ['c', 'a']), 'a,c');
	assert.equal(encodeChecked(checklist, []), '-');
});

test('a new device starts from the described default, an edited one is left alone', () => {
	assert.deepEqual(seedChecklists([checklist, rows], {}), { lists: 'a,c' });
	assert.deepEqual(seedChecklists([checklist], { lists: '-' }), { lists: '-' });
});

test('rows round-trip through the tag, unfilled rows are dropped', () => {
	assert.deepEqual(emptyRow(rows.columns), { zone: '', subject: 'ip', major: false });
	const filled = [
		{ zone: ' dnsbl.example.org ', subject: 'domain', major: true },
		{ zone: '', subject: 'ip', major: true }
	];
	const tag = serializeRows(rows, filled);
	assert.deepEqual(JSON.parse(tag), [{ zone: 'dnsbl.example.org', subject: 'domain', major: true }]);
	assert.deepEqual(parseRows(rows, tag), [{ zone: 'dnsbl.example.org', subject: 'domain', major: true }]);
	assert.equal(serializeRows(rows, [emptyRow(rows.columns)]), '');
	assert.deepEqual(parseRows(rows, 'not json'), []);
	assert.deepEqual(parseRows(rows, '{"a":1}'), []);
});

test('rows are validated: pattern, no IP address, no duplicate', () => {
	const row = (value) => ({ zone: value, subject: 'ip', major: false });
	const errors = rowErrors(rows, [row('dnsbl.example.org'), row('192.0.2.1'), row('nodot'), row('DNSBL.example.org'), emptyRow(rows.columns)], messages);
	assert.equal(errors[0], undefined);
	assert.equal(errors[1].zone, 'Not a zone');
	assert.equal(errors[2].zone, 'Not a zone');
	assert.equal(errors[3].zone, 'Duplicate');
	assert.equal(errors[4], undefined, 'an unfilled row is not an error');
	assert.equal(firstRowError(errors), 'Not a zone');
	assert.equal(firstRowError(rowErrors(rows, [row('a.example.org')], messages)), null);
});
