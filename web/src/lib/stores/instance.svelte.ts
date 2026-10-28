/**
 * What the interface knows about the instance: whether the setup wizard is
 * done (public `GET /api/instance`, read before sign-in) and, once signed in,
 * the instance name shown in the breadcrumb and on the wall.
 */
import { browser } from '$app/env';
import { getInstance, getInstanceSettings } from '#lib/api/instance.js';

/** Key of the former Homelab / Enterprise choice, cleared once on load. */
const LEGACY_MODE_KEY = 'dumbmonit-mode';

class InstanceStore {
	/** `null` until the server answered. */
	setupCompleted = $state<boolean | null>(null);
	/** True once the server answered (or failed to). */
	loaded = $state(false);
	/** Instance name, read once signed in; empty shows "DumbMonit". */
	name = $state('');
	#settingsRead = false;

	constructor() {
		if (!browser) return;
		try {
			localStorage.removeItem(LEGACY_MODE_KEY);
		} catch {
			// Storage unavailable: nothing to clean.
		}
	}

	/** Reads the setup state from the server. Called once from the root layout. */
	async init(): Promise<void> {
		try {
			const info = await getInstance();
			this.setupCompleted = info.setup_completed;
		} catch {
			// An older server, or one that cannot be reached: no wizard offered.
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
		} catch {
			this.#settingsRead = false;
		}
	}

	/** Display name of the instance. */
	get displayName(): string {
		return this.name.trim() || 'DumbMonit';
	}
}

export const instanceStore = new InstanceStore();
