/**
 * The console's theme choice (Settings › Appearance), pure so it is tested
 * with `node --test`: dark by default, light on request, or the system's.
 * The wall is a room monitor and stays dark whatever the choice.
 *
 * Mirrored by the bootstrap script in app.html, which applies it before the
 * first paint: keep the key and the rules in step.
 */

export type ConsoleTheme = 'dark' | 'light' | 'auto';

/** Per browser; absent means the default, dark. */
export const CONSOLE_THEME_KEY = 'dumbmonit-console-theme';

export const CONSOLE_THEMES: readonly ConsoleTheme[] = ['dark', 'light', 'auto'];

/** What was stored, or dark for anything else (nothing, an old value, garbage). */
export function parseConsoleTheme(raw: string | null | undefined): ConsoleTheme {
	return raw === 'light' || raw === 'auto' ? raw : 'dark';
}

/** The theme actually shown on `path`, given the choice and whether the system is dark. */
export function resolveConsoleTheme(choice: ConsoleTheme, systemDark: boolean, path: string): 'light' | 'dark' {
	if (path === '/wall' || path.startsWith('/wall/')) return 'dark';
	if (choice === 'auto') return systemDark ? 'dark' : 'light';
	return choice;
}
