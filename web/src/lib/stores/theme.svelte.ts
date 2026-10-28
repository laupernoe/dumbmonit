/**
 * Light / dark theme.
 *
 * The app itself is one dark console (the `obsidian` tokens in app.css): it is
 * always dark. Only the public status pages (`/s/…`, or a status page's own
 * domain) keep a light and a dark version; there `auto` follows
 * `prefers-color-scheme`, and a visitor's `light` or `dark` is remembered.
 * The effective theme is applied to `<html>` as a `dark` class, as Tailwind
 * expects, and the console tokens as an `obsidian` class.
 */
import { browser } from '$app/env';
import { page } from '$app/state';
import { hostedStatusSlug } from '#lib/status-host.js';

/** Class of the console tokens on `<html>` (app.css). */
export const CONSOLE_CLASS = 'obsidian';

export type ThemePreference = 'auto' | 'light' | 'dark';

const STORAGE_KEY = 'dumbmonit-theme';
/** Key used before the product rename; read once, then moved to the new one. */
const LEGACY_STORAGE_KEY = 'ezymonit-theme';

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

function systemIsDark(): boolean {
	return browser && window.matchMedia('(prefers-color-scheme: dark)').matches;
}

class Theme {
	preference = $state<ThemePreference>('auto');
	/** True when the system is dark; tracked live for automatic mode. */
	#systemDark = $state(false);

	constructor() {
		this.preference = readPreference();
		this.#systemDark = systemIsDark();
	}

	/** False on a public status page, which keeps its own light; the console everywhere else. */
	get console(): boolean {
		if (!browser) return true;
		return !hostedStatusSlug && !page.url.pathname.startsWith('/s/');
	}

	/** Theme actually applied, once the preference is resolved. */
	get resolved(): 'light' | 'dark' {
		// The console is dark by design: one palette, tuned for long shifts.
		if (this.console) return 'dark';
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
		document.documentElement.classList.toggle(CONSOLE_CLASS, this.console);
		document.documentElement.classList.toggle('dark', this.resolved === 'dark');
		document
			.querySelector('meta[name="apple-mobile-web-app-status-bar-style"]')
			?.setAttribute('content', this.resolved === 'dark' ? 'black-translucent' : 'default');
	}

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
}

export const theme = new Theme();
