// Test-connection gating and scan default subnet. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	defaultSubnet,
	firstReason,
	identitySummary,
	testBlocker,
	usableSubnets
} from '../src/lib/components/device-form/test-connection.ts';

test('the test button explains what is missing, address first', () => {
	assert.equal(testBlocker({ address: '  ', credentialErrors: { community: 'x' }, viaRelay: false }), 'address');
	assert.equal(testBlocker({ address: '10.0.0.1', credentialErrors: { community: 'x' }, viaRelay: false }), 'credential');
	assert.equal(testBlocker({ address: '10.0.0.1', credentialErrors: {}, viaRelay: true }), 'relay');
	assert.equal(testBlocker({ address: '10.0.0.1', credentialErrors: {}, viaRelay: false }), null);
});

test('firstReason skips empty entries', () => {
	assert.equal(firstReason([null, undefined, '', 'Name is required.', 'other']), 'Name is required.');
	assert.equal(firstReason([null, '']), null);
});

test('usableSubnets keeps valid, unique CIDRs in server order', () => {
	const raw = [
		{ cidr: '192.168.1.0/24', source: 'devices' },
		{ cidr: 'nonsense', source: 'devices' },
		{ cidr: '192.168.1.0/24', source: 'config' },
		{ cidr: '10.0.0.0/24', source: 'server', interface: 'eth0' },
		{ cidr: '10.1.0.0/24', source: 'weird' }
	];
	assert.deepEqual(
		usableSubnets(raw).map((s) => s.cidr),
		['192.168.1.0/24', '10.0.0.0/24', '10.1.0.0/24']
	);
	const kept = usableSubnets(raw);
	assert.deepEqual(kept.map((s) => s.source), ['devices', 'server', 'server']);
	assert.equal(kept[0].interface, undefined);
	assert.equal(kept[1].interface, 'eth0');
	assert.deepEqual(usableSubnets(null), []);
	assert.deepEqual(usableSubnets('no'), []);
});

test('the default subnet never overwrites what was typed', () => {
	const subnets = usableSubnets([{ cidr: '10.0.0.0/24', source: 'config' }]);
	assert.equal(defaultSubnet('', subnets), '10.0.0.0/24');
	assert.equal(defaultSubnet('192.168.5.0/24', subnets), null);
	assert.equal(defaultSubnet('', []), null);
});

test('identitySummary joins name and description and clips long text', () => {
	assert.equal(identitySummary('sw1', 'Linux  switch\n6.1'), 'sw1 — Linux switch 6.1');
	assert.equal(identitySummary(null, null), '');
	assert.ok(identitySummary('a', 'x'.repeat(500)).length < 130);
});
