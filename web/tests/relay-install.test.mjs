// Relay mode in the install one-liners. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { withRelay } from '../src/lib/components/device-form/relay-install.ts';

const linux = 'curl -sSL http://s/install.sh | sh -s -- --token=dmon_x --url=http://s';
const windows = '& ([scriptblock]::Create((irm http://s/install.ps1))) -Token dmon_x -Url http://s';

test('the Linux one-liner carries relay mode and a quoted site', () => {
	assert.equal(withRelay(linux, 'Lyon office', 'unix'), `${linux} --relay --site='Lyon office'`);
	assert.equal(withRelay(linux, "  l'annexe ", 'unix'), `${linux} --relay --site='l'\\''annexe'`);
	assert.equal(withRelay(linux, '   ', 'unix'), `${linux} --relay`);
});

test('the Windows one-liner carries relay mode and a quoted site', () => {
	assert.equal(withRelay(windows, "l'annexe", 'windows'), `${windows} -Relay -Site 'l''annexe'`);
	assert.equal(withRelay(windows, '', 'windows'), `${windows} -Relay`);
});

test('no command, nothing appended', () => {
	assert.equal(withRelay('', 'Lyon', 'unix'), '');
});
