/**
 * Which rules, channels, maintenance windows and status pages a configuration
 * file declares (`GET /api/config/status`). Loaded once, for administrators:
 * lists use it to show a "Managed by file" badge. The server refuses the edit
 * anyway, so a missing answer only means a missing badge.
 */
import { getConfigStatus } from '#lib/api/configFiles.js';

class ConfigManaged {
	keys = $state<Record<string, string[]>>({});
	#started = false;

	/** Fetches the keys on first use. */
	ensureLoaded(): void {
		if (this.#started) return;
		this.#started = true;
		void this.reload();
	}

	async reload(): Promise<void> {
		try {
			this.keys = (await getConfigStatus()).keys;
		} catch {
			this.keys = {};
		}
	}

	/** `kind`: rule, channel, silence, status_page. A silence also matches `name|monitor`. */
	has(kind: string, key: string): boolean {
		const list = this.keys[kind];
		if (!list) return false;
		return list.includes(key) || (kind === 'silence' && list.some((k) => k.startsWith(`${key}|`)));
	}
}

export const configManaged = new ConfigManaged();
