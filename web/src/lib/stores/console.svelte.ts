/**
 * State of the Enterprise console shared by its header, its pages and its
 * context panel: the time range, live updates, the inspector, and the monitor
 * the reader picked in a table (shown in the panel on the right).
 *
 * The range and whether the panel is open are remembered per browser; the
 * selection is not — it belongs to the moment.
 */
import { browser } from '$app/env';
import type { TargetId } from '#lib/api/index.js';

export const CONSOLE_RANGES = ['1h', '6h', '24h', '7d', '30d'] as const;
export type ConsoleRange = (typeof CONSOLE_RANGES)[number];

export const RANGE_SECONDS: Record<ConsoleRange, number> = {
	'1h': 3600,
	'6h': 6 * 3600,
	'24h': 24 * 3600,
	'7d': 7 * 86400,
	'30d': 30 * 86400
};

const RANGE_KEY = 'dumbmonit-console-range';
const PANEL_KEY = 'dumbmonit-console-panel';
/** Taps on LIVE before the night-shift note shows. */
const EGG_TAPS = 7;

function read(key: string): string | null {
	if (!browser) return null;
	try {
		return localStorage.getItem(key);
	} catch {
		return null;
	}
}

function write(key: string, value: string) {
	if (!browser) return;
	try {
		localStorage.setItem(key, value);
	} catch {
		// Not remembered: the default comes back on the next visit.
	}
}

function storedRange(): ConsoleRange {
	const value = read(RANGE_KEY);
	return (CONSOLE_RANGES as readonly string[]).includes(value ?? '') ? (value as ConsoleRange) : '24h';
}

class ConsoleStore {
	range = $state<ConsoleRange>(storedRange());
	/** Polling on (the header's LIVE). Off freezes the console's own refreshes. */
	live = $state(true);
	/** The context panel on wide screens; the drawer on narrow ones. */
	panelOpen = $state(read(PANEL_KEY) !== 'closed');
	/** Narrow screens: the panel shows as a drawer only when asked. */
	drawerOpen = $state(false);
	selected = $state<TargetId | null>(null);
	/** True once LIVE was tapped enough times in a row. */
	egg = $state(false);
	#taps = 0;
	#tapTimer: ReturnType<typeof setTimeout> | undefined;

	get rangeSeconds(): number {
		return RANGE_SECONDS[this.range];
	}

	setRange(range: ConsoleRange) {
		this.range = range;
		write(RANGE_KEY, range);
	}

	togglePanel() {
		this.panelOpen = !this.panelOpen;
		write(PANEL_KEY, this.panelOpen ? 'open' : 'closed');
	}

	select(id: TargetId | null) {
		this.selected = id;
	}

	/** LIVE toggles polling; seven quick taps leave a note for the night shift. */
	tapLive() {
		this.live = !this.live;
		clearTimeout(this.#tapTimer);
		this.#taps += 1;
		if (this.#taps >= EGG_TAPS) {
			this.egg = true;
			this.live = true;
			this.#taps = 0;
		}
		this.#tapTimer = setTimeout(() => (this.#taps = 0), 2500);
	}
}

export const consoleState = new ConsoleStore();
