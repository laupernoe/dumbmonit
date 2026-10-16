/**
 * Services an agent found on its machine (Docker, Plakar, ZFS, tunnels…).
 *
 * Each one is its own device of kind `agent_service`, under the machine
 * (`parent_id`), with the module id in `profile_id`. Its series carry its own
 * `target`; the machine keeps CPU, memory, disks, network and OS health.
 */
import type { Target, TargetId } from '#lib/api/index.js';

export const AGENT_SERVICE_KIND = 'agent_service';

type Device = Pick<Target, 'id' | 'kind' | 'profile_id' | 'parent_id'>;

/** The module a service device stands for (`docker`, `zfs`…); `null` for any other device. */
export function serviceModule(target: Device): string | null {
	return target.kind === AGENT_SERVICE_KIND ? target.profile_id : null;
}

/** The device whose page shows the containers: the Docker service of a machine. */
export function isDockerDevice(target: Device): boolean {
	return serviceModule(target) === 'docker';
}

/** The machine whose agent runs the commands: the parent of a service, the device itself otherwise. */
export function agentHostOf(target: Device): TargetId {
	return target.kind === AGENT_SERVICE_KIND && target.parent_id !== null ? target.parent_id : target.id;
}
