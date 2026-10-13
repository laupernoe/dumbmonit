/**
 * Optional agent modules, as `crates/agent/src/config.rs` (`MODULES`) and the
 * install scripts know them. System metrics (CPU, memory, disks, network) are
 * not a module: the agent always collects them.
 */
export const AGENT_MODULES = [
	'docker',
	'plakar',
	'system_health',
	'sensors',
	'smart',
	'zfs',
	'wireguard',
	'perf_counters'
] as const;

export type AgentModule = (typeof AGENT_MODULES)[number];

/**
 * The install command with the chosen modules appended. `null` means "detect
 * everything", the agent's default: the command is left as the server built
 * it. An empty choice keeps system metrics only (`none`).
 */
export function withModules(command: string, modules: readonly AgentModule[] | null, shell: 'unix' | 'windows'): string {
	if (modules === null || !command) return command;
	const ordered = AGENT_MODULES.filter((module) => modules.includes(module));
	const value = ordered.length > 0 ? ordered.join(',') : 'none';
	return shell === 'unix' ? `${command} --modules=${value}` : `${command} -Modules ${value}`;
}
