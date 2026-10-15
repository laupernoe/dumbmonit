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
		{ cidr: '192.168.1.0/24', interface: 'eth0' },
		{ cidr: 'nonsense', interface: 'x' },
		{ cidr: '192.168.1.0/24', interface: 'dup' },
		{ cidr: '172.17.0.0/24', interface: 'docker0' }
	];
	assert.deepEqual(
		usableSubnets(raw).map((s) => s.cidr),
		['192.168.1.0/24', '172.17.0.0/24']
	);
	assert.deepEqual(usableSubnets(null), []);
	assert.deepEqual(usableSubnets('no'), []);
});

test('the default subnet never overwrites what was typed', () => {
	const subnets = usableSubnets([{ cidr: '10.0.0.0/24', interface: 'eth0' }]);
	assert.equal(defaultSubnet('', subnets), '10.0.0.0/24');
	assert.equal(defaultSubnet('192.168.5.0/24', subnets), null);
	assert.equal(defaultSubnet('', []), null);
});

test('identitySummary joins name and description and clips long text', () => {
	assert.equal(identitySummary('sw1', 'Linux  switch\n6.1'), 'sw1 — Linux switch 6.1');
	assert.equal(identitySummary(null, null), '');
	assert.ok(identitySummary('a', 'x'.repeat(500)).length < 130);
});
