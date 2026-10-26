/**
 * The console's fleet figures, shared by the overview and the context panel
 * so both read the same numbers from one set of queries. Refreshed every
 * minute while LIVE is on, and again as soon as the range changes.
 */
import { browser } from '$app/env';
import { listAlertHistory, type AlertHistoryEntry, type TargetId } from '#lib/api/index.js';
import { consoleState } from '#lib/stores/console.svelte.js';
import { loadFigures, loadLatency, loadRiver, type FleetFigures, type RiverCell } from './fleet.js';

const INTERVAL_MS = 60_000;

class FleetStore {
	river = $state<Map<TargetId, RiverCell[]>>(new Map());
	latency = $state<Map<TargetId, number[]>>(new Map());
	figures = $state<FleetFigures>({ p95: new Map(), uptime30: new Map(), fleetP95: null });
	history = $state<AlertHistoryEntry[]>([]);
	loading = $state(true);
	error = $state<unknown>(null);
	#subscribers = 0;
	#controller: AbortController | null = null;
	#loadedRange = '';
	#timer: ReturnType<typeof setInterval> | undefined;

	async refresh(): Promise<void> {
		this.#controller?.abort();
		const controller = new AbortController();
		this.#controller = controller;
		const seconds = consoleState.rangeSeconds;
		const since = new Date(Date.now() - Math.max(seconds, 30 * 86400) * 1000).toISOString();
		try {
			const [river, latency, figures, history] = await Promise.all([
				loadRiver(seconds, controller.signal),
				loadLatency(seconds, controller.signal),
				loadFigures(seconds, controller.signal),
				listAlertHistory({ since, limit: 1000 }, controller.signal).catch(() => [] as AlertHistoryEntry[])
			]);
			this.river = river;
			this.latency = latency;
			this.figures = figures;
			this.history = history;
			this.error = null;
			this.#loadedRange = consoleState.range;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			this.error = cause;
		} finally {
			if (this.#controller === controller) this.loading = false;
		}
	}

	/** Starts refreshing while at least one view needs the figures; returns the stop function. */
	start(): () => void {
		if (!browser) return () => {};
		this.#subscribers += 1;
		if (this.#subscribers === 1) {
			this.#timer = setInterval(() => {
				if (!consoleState.live || document.visibilityState === 'hidden') return;
				void this.refresh();
			}, INTERVAL_MS);
			void this.refresh();
		} else if (this.#loadedRange !== consoleState.range) {
			void this.refresh();
		}
		return () => {
			this.#subscribers -= 1;
			if (this.#subscribers > 0) return;
			clearInterval(this.#timer);
			this.#controller?.abort();
		};
	}

	/** Called when the range changes. */
	rangeChanged() {
		if (this.#subscribers > 0 && this.#loadedRange !== consoleState.range) {
			this.loading = true;
			void this.refresh();
		}
	}
}

export const fleetStore = new FleetStore();
