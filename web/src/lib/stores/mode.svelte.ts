/**
 * Product mode: Homelab (the simple interface) or Enterprise (the dense, very
 * dark one). Chosen by an administrator, stored on the instance, read here
 * from the public `GET /api/instance` before sign-in.
 *
 * The last mode seen is cached in the browser so the bootstrap script in
 * `app.html` puts the right class on `<html>` before the first paint: an
 * Enterprise console must not flash cream on every reload.
 *
 * The mode only picks a shell (layout) and a set of colour tokens
 * (`html.mode-enterprise` in app.css). Pages and data components are shared.
 */
import { browser } from '$app/env';
import { getInstance, getInstanceSettings } from '#lib/api/instance.js';
import type { ProductMode } from '#lib/api/index.js';

export const MODE_STORAGE_KEY = 'dumbmonit-mode';
export const MODE_CLASS = 'mode-enterprise';

function cached(): ProductMode {
	if (!browser) return 'homelab';
	try {
		return localStorage.getItem(MODE_STORAGE_KEY) === 'enterprise' ? 'enterprise' : 'homelab';
	} catch {
		return 'homelab';
	}
}

class ModeStore {
	current = $state<ProductMode>(cached());
	/** `null` until the server answered. */
	setupCompleted = $state<boolean | null>(null);
	/** True once the server answered (or failed to). */
	loaded = $state(false);
	/** Instance name, read once signed in; empty shows "DumbMonit". */
	name = $state('');
	#settingsRead = false;

	get enterprise(): boolean {
		return this.current === 'enterprise';
	}

	get homelab(): boolean {
		return this.current !== 'enterprise';
	}

	/** Reads the mode from the server. Called once from the root layout. */
	async init(): Promise<void> {
		try {
			const info = await getInstance();
			this.setupCompleted = info.setup_completed;
			this.set(info.product_mode);
		} catch {
			// An older server, or one that cannot be reached: keep what was cached.
		} finally {
			this.loaded = true;
		}
	}

	/** Reads the instance name once a session exists. */
	async loadSettings(): Promise<void> {
		if (this.#settingsRead) return;
		this.#settingsRead = true;
		try {
			const settings = await getInstanceSettings();
			this.name = settings.name;
			this.setupCompleted = settings.setup_completed;
			this.set(settings.product_mode);
		} catch {
			this.#settingsRead = false;
		}
	}

	/** Display name of the instance. */
	get displayName(): string {
		return this.name.trim() || 'DumbMonit';
	}

	/** Applies a mode locally (after a save, or a server read). */
	set(mode: ProductMode) {
		this.current = mode;
		if (!browser) return;
		try {
			localStorage.setItem(MODE_STORAGE_KEY, mode);
		} catch {
			// Not remembered: the next load reads the server again anyway.
		}
	}

	/** Puts the mode class on `<html>`. Called by an effect in the root layout. */
	apply() {
		if (!browser) return;
		document.documentElement.classList.toggle(MODE_CLASS, this.enterprise);
	}
}

export const mode = new ModeStore();
