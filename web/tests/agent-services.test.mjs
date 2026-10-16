// Services an agent found, each a device under its machine. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { agentHostOf, isDockerDevice, serviceModule } from '../src/lib/components/devices/agent/services.ts';

const machine = { id: 5, kind: 'agent', profile_id: null, parent_id: null };
const docker = { id: 9, kind: 'agent_service', profile_id: 'docker', parent_id: 5 };
const zfs = { id: 10, kind: 'agent_service', profile_id: 'zfs', parent_id: 5 };
const snmp = { id: 11, kind: 'snmp', profile_id: 'docker', parent_id: 5 };

test('a service device names its module, any other device none', () => {
	assert.equal(serviceModule(docker), 'docker');
	assert.equal(serviceModule(zfs), 'zfs');
	assert.equal(serviceModule(machine), null);
	// A profile on another kind (an SNMP profile) is not a module.
	assert.equal(serviceModule(snmp), null);
});

test('only the Docker service shows containers', () => {
	assert.equal(isDockerDevice(docker), true);
	assert.equal(isDockerDevice(zfs), false);
	assert.equal(isDockerDevice(machine), false, 'the machine shows its own metrics only');
});

test('commands go to the machine whose agent runs them', () => {
	assert.equal(agentHostOf(docker), 5);
	assert.equal(agentHostOf(machine), 5);
	assert.equal(agentHostOf({ ...docker, parent_id: null }), 9, 'an orphan keeps its own id');
});
