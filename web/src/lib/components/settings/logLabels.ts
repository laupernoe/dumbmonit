/** Message-backed labels of the Logs section (kept apart from `logs.ts`, which has no messages). */
import { m } from '#lib/paraglide/messages.js';
import type { LogLevel } from '#lib/api/types.js';

export function levelLabel(level: LogLevel | string): string {
	switch (level) {
		case 'error':
			return m.settings_logs_level_error();
		case 'warn':
			return m.settings_logs_level_warn();
		case 'info':
			return m.settings_logs_level_info();
		case 'debug':
			return m.settings_logs_level_debug();
		default:
			return level;
	}
}
