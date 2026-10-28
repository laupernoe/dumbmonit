/**
 * What the add-an-agent flow derives from the server's install command: the
 * addresses it carries, and the same install as a Docker command. The server
 * is the single source of the command (`GET /api/agent/install`, token
 * creation); nothing here guesses an address.
 */

export interface InstallOptions {
	/** `--url`: web port of the server, or the relay's gateway. */
	base: string;
	/** `--tls-url`: encrypted agent channel (the relay's gateway behind a relay). */
	tlsUrl: string | null;
	/** `--ca-pin`: fingerprint of the server's authority, pinned end to end. */
	caPin: string | null;
}

/** Reads `--url=`, `--tls-url=` and `--ca-pin=` from the Unix install command. */
export function installOptions(command: string): InstallOptions {
	const value = (flag: string) => {
		const match = command.match(new RegExp(`(?:^|\\s)--${flag}=(\\S+)`));
		return match ? match[1] : null;
	};
	return { base: value('url') ?? '', tlsUrl: value('tls-url'), caPin: value('ca-pin') };
}

/** Quotes a value for a POSIX shell when it needs it. */
function shell(value: string): string {
	return /^[\w@%+=:,./-]+$/.test(value) ? value : `'${value.replace(/'/g, `'\\''`)}'`;
}

export const AGENT_IMAGE = 'ghcr.io/laupernoe/dumbmonit-agent:latest';

/**
 * `docker run` for the agent image, with the same server, channel and pin as
 * the install command. `hostname` names a relay container after its site (a
 * container's own hostname is a random id).
 */
export function dockerRun(options: InstallOptions, token: string, hostname: string | null): string {
	const env: [string, string][] = [
		['DUMBMONIT_AGENT_URL', options.base],
		['DUMBMONIT_AGENT_TOKEN', token]
	];
	if (options.tlsUrl) env.push(['DUMBMONIT_AGENT_TLS_URL', options.tlsUrl]);
	if (options.caPin) env.push(['DUMBMONIT_AGENT_CA_PIN', options.caPin]);
	if (hostname) env.push(['DUMBMONIT_AGENT_HOSTNAME', hostname]);
	env.push(['DUMBMONIT_AGENT_SECRET_FILE', '/var/lib/dumbmonit-agent/agent-secret']);
	return [
		'docker run -d --name dumbmonit-agent --restart unless-stopped \\',
		...env.map(([key, value]) => `  -e ${key}=${shell(value)} \\`),
		// A relay's gateway, for the agents of its site, once it is switched on.
		...(hostname ? ['  -p 8443:8443 \\'] : []),
		'  -v dumbmonit-agent:/var/lib/dumbmonit-agent \\',
		'  -v /var/run/docker.sock:/var/run/docker.sock:ro \\',
		`  ${AGENT_IMAGE}`
	].join('\n');
}
