/**
 * Light / dark theme.
 *
 * The app is one console (the `obsidian` class on `<html>`: Geist, the
 * console's tokens). It is deep black by default; Settings › Appearance can
 * turn it light (`console-light` alongside `obsidian`) or make it follow the
 * system. The wall, a room monitor, stays dark. That choice is kept per
 * browser (`theme-choice.ts`).
 *
 * The public status pages (`/s/…`, or a status page's own domain) keep their
 * own light and dark versions; there `auto` follows `prefers-color-scheme`,
 * and a visitor's `light` or `dark` is remembered. The effective theme is
 * applied to `<html>` as a `dark` class, as Tailwind expects.
 */
import { browser } from '$app/env';
import { page } from '$app/state';
import { hostedStatusSlug } from '#lib/status-host.js';
import { CONSOLE_THEME_KEY, parseConsoleTheme, resolveConsoleTheme, type ConsoleTheme } from './theme-choice.js';

export type { ConsoleTheme } from './theme-choice.js';

/** Class of the console tokens on `<html>` (app.css). */
export const CONSOLE_CLASS = 'obsidian';
/** Added next to it for the light console. */
export const CONSOLE_LIGHT_CLASS = 'console-light';

export type ThemePreference = 'auto' | 'light' | 'dark';

const STORAGE_KEY = 'dumbmonit-theme';
/** Key used before the product rename; read once, then moved to the new one. */
const LEGACY_STORAGE_KEY = 'ezymonit-theme';

/** Browser bar colour per look (matches each canvas). */
const BAR_COLOR = {
	console: { dark: '#07080a', light: '#f6f6f7' },
	status: { dark: '#0a1020', light: '#f3efe6' }
} as const;

function readPreference(): ThemePreference {
	if (!browser) return 'auto';
	try {
		let stored = localStorage.getItem(STORAGE_KEY);
		if (stored === null) {
			stored = localStorage.getItem(LEGACY_STORAGE_KEY);
			if (stored !== null) {
				localStorage.setItem(STORAGE_KEY, stored);
				localStorage.removeItem(LEGACY_STORAGE_KEY);
			}
		}
		if (stored === 'light' || stored === 'dark') return stored;
	} catch {
		// Storage unavailable (strict private browsing): stay in automatic mode.
	}
	return 'auto';
}

function readConsoleChoice(): ConsoleTheme {
	if (!browser) return 'dark';
	try {
		return parseConsoleTheme(localStorage.getItem(CONSOLE_THEME_KEY));
	} catch {
		return 'dark';
	}
}

function systemIsDark(): boolean {
	return browser && window.matchMedia('(prefers-color-scheme: dark)').matches;
}

class Theme {
	/** A visitor's choice on the public status pages. */
	preference = $state<ThemePreference>('auto');
	/** The console's choice, from Settings › Appearance. */
	consoleChoice = $state<ConsoleTheme>('dark');
	/** True when the system is dark; tracked live for automatic mode. */
	#systemDark = $state(false);

	constructor() {
		this.preference = readPreference();
		this.consoleChoice = readConsoleChoice();
		this.#systemDark = systemIsDark();
	}

	/** False on a public status page, which keeps its own light; the console everywhere else. */
	get console(): boolean {
		if (!browser) return true;
		return !hostedStatusSlug && !page.url.pathname.startsWith('/s/');
	}

	/** Theme actually applied, once the preference is resolved. */
	get resolved(): 'light' | 'dark' {
		if (this.console) {
			return resolveConsoleTheme(this.consoleChoice, this.#systemDark, browser ? page.url.pathname : '/');
		}
		if (this.preference === 'auto') return this.#systemDark ? 'dark' : 'light';
		return this.preference;
	}

	/**
	 * Listens for system theme changes. Call once from the root layout; returns
	 * the unsubscribe function.
	 */
	watchSystem(): () => void {
		if (!browser) return () => {};
		const query = window.matchMedia('(prefers-color-scheme: dark)');
		const onChange = (event: MediaQueryListEvent) => {
			this.#systemDark = event.matches;
		};
		query.addEventListener('change', onChange);
		return () => query.removeEventListener('change', onChange);
	}

	/** Applies the theme to the document. Called by an effect in the root layout. */
	apply() {
		if (!browser) return;
		const root = document.documentElement;
		const resolved = this.resolved;
		const inConsole = this.console;
		root.classList.toggle(CONSOLE_CLASS, inConsole);
		root.classList.toggle(CONSOLE_LIGHT_CLASS, inConsole && resolved === 'light');
		root.classList.toggle('dark', resolved === 'dark');
		document
			.querySelector('meta[name="apple-mobile-web-app-status-bar-style"]')
			?.setAttribute('content', resolved === 'dark' ? 'black-translucent' : 'default');
		document
			.querySelector('meta[name="theme-color"]')
			?.setAttribute('content', BAR_COLOR[inConsole ? 'console' : 'status'][resolved]);
	}

	/** A visitor's light or dark on a public status page. */
	set(preference: ThemePreference) {
		this.preference = preference;
		if (!browser) return;
		try {
			if (preference === 'auto') localStorage.removeItem(STORAGE_KEY);
			else localStorage.setItem(STORAGE_KEY, preference);
		} catch {
			// The theme will simply not be remembered from one visit to the next.
		}
	}

	/** Toggles between light and dark, starting from the theme currently displayed. */
	toggle() {
		this.set(this.resolved === 'dark' ? 'light' : 'dark');
	}

	/** The console's theme (Settings › Appearance), remembered in this browser. */
	setConsole(choice: ConsoleTheme) {
		this.consoleChoice = choice;
		if (!browser) return;
		try {
			if (choice === 'dark') localStorage.removeItem(CONSOLE_THEME_KEY);
			else localStorage.setItem(CONSOLE_THEME_KEY, choice);
		} catch {
			// Not remembered: the console is dark again on the next visit.
		}
	}
}

export const theme = new Theme();
