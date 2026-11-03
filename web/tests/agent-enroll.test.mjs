// The add-an-agent flow reads the server's install command; it never builds an address. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dockerRun, installOptions } from '../src/lib/components/device-form/agent-enroll.ts';

const pin = 'a'.repeat(64);
const key = `sha256//${'K'.repeat(43)}=`;
const direct = `curl -sSL https://monit.example.com/install.sh | sh -s -- --token=dmon_x --url=https://monit.example.com --sha256=linux-x86_64:${'b'.repeat(64)} --tls-url=https://monit.example.com:8443 --ca-pin=${pin}`;
const relayed = `curl -sSL -k --pinnedpubkey ${key} https://192.168.10.5:8443/install.sh | sh -s -- --token=dmon_x --url=https://192.168.10.5:8443 --pinned-key=${key} --tls-url=https://192.168.10.5:8443 --ca-pin=${pin}`;

test('the addresses come from the install command', () => {
	assert.deepEqual(installOptions(direct), { base: 'https://monit.example.com', tlsUrl: 'https://monit.example.com:8443', caPin: pin, serverPin: null });
	assert.deepEqual(installOptions(relayed), { base: 'https://192.168.10.5:8443', tlsUrl: 'https://192.168.10.5:8443', caPin: pin, serverPin: key });
	assert.deepEqual(installOptions('curl x | sh'), { base: '', tlsUrl: null, caPin: null, serverPin: null });
});

test('the Docker command carries the same channel and pin, through the relay too', () => {
	const command = dockerRun(installOptions(relayed), 'dmon_x', null);
	assert.match(command, /-e DUMBMONIT_AGENT_URL=https:\/\/192\.168\.10\.5:8443 /);
	assert.ok(command.includes(`-e DUMBMONIT_AGENT_SERVER_PIN=${key} `), command);
	assert.match(command, /-e DUMBMONIT_AGENT_TLS_URL=https:\/\/192\.168\.10\.5:8443 /);
	assert.match(command, new RegExp(`-e DUMBMONIT_AGENT_CA_PIN=${pin} `));
	assert.doesNotMatch(command, /-p 8443/, 'an ordinary agent publishes no port');
	const relay = dockerRun(installOptions(direct), 'dmon_x', 'relay-lyon');
	assert.match(relay, /DUMBMONIT_AGENT_HOSTNAME=relay-lyon/);
	assert.match(relay, /-p 8443:8443/, 'a relay publishes its gateway port');
});

test('values a shell could misread are quoted', () => {
	const command = dockerRun({ base: 'https://x', tlsUrl: null, caPin: null, serverPin: null }, "to'ken $x", null);
	assert.match(command, /DUMBMONIT_AGENT_TOKEN='to'\\''ken \$x'/);
});
