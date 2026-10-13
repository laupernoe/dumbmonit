/**
 * "Run with sudo" for copyable Unix shell commands.
 *
 * `sudo` has to wrap the shell that executes the script, not the downloader:
 * `curl … | sh` becomes `curl … | sudo sh`. A plain command (or each part of an
 * `a && b` chain) simply gets a `sudo ` prefix. Not for Windows/PowerShell.
 */

const STORAGE_KEY = 'dumbmonit.copy.sudo';
const SHELL_PIPE = /\|\s*(?:sudo\s+)?((?:ba|da|z)?sh)\b/;

/** The command without any `sudo` we may have added (or that was already there). */
export function stripSudo(command: string): string {
	return command
		.replace(/\|\s*sudo\s+/g, '| ')
		.split('\n')
		.map((line) => line.replace(/(^\s*|&&\s*)sudo\s+/g, '$1'))
		.join('\n');
}

/** Adds or removes sudo. Idempotent. */
export function applySudo(command: string, on: boolean): string {
	const bare = stripSudo(command);
	if (!on) return bare;
	if (SHELL_PIPE.test(bare)) return bare.replace(new RegExp(SHELL_PIPE, 'g'), '| sudo $1');
	return bare
		.split('\n')
		.map((line) => line.replace(/(^\s*|&&\s*)(?=\S)/g, '$1sudo '))
		.join('\n');
}

export const hasSudo = (command: string): boolean => /(^|\s)sudo\s/.test(command);

export function loadSudoPreference(): boolean | null {
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		return stored === null ? null : stored === '1';
	} catch {
		return null;
	}
}

export function saveSudoPreference(on: boolean): void {
	try {
		localStorage.setItem(STORAGE_KEY, on ? '1' : '0');
	} catch {
		/* storage unavailable: the choice just isn't remembered */
	}
}
