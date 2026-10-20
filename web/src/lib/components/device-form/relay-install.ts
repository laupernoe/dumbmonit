/**
 * Relay mode in the install one-liners, as `crates/agent/install/install.sh`
 * (`--relay --site=NAME`) and `install.ps1` (`-Relay -Site NAME`) know it: the
 * scripts write `relay: true` and `site:` into agent.yaml, so a relay installs
 * in one command, without editing a file afterwards.
 */

/** A POSIX shell word: single quotes, a quote inside closed and reopened. */
function shellQuote(value: string): string {
	return `'${value.replace(/'/g, `'\\''`)}'`;
}

/** A PowerShell literal string: single quotes, a quote inside doubled. */
function psQuote(value: string): string {
	return `'${value.replace(/'/g, "''")}'`;
}

/** The install command with relay mode (and its site, when one is given) appended. */
export function withRelay(command: string, site: string, shell: 'unix' | 'windows'): string {
	if (!command) return command;
	const name = site.trim();
	if (shell === 'unix') return name ? `${command} --relay --site=${shellQuote(name)}` : `${command} --relay`;
	return name ? `${command} -Relay -Site ${psQuote(name)}` : `${command} -Relay`;
}
