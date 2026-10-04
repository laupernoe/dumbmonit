/**
 * Active alerts, devices and probe states, shared by the whole application.
 *
 * It feeds the nav bar badge, and — since September 2026 — is also the single
 * source Overview, /alerts and /targets read `targets`/`probes` from, instead
 * of each polling its own copy on an uncoordinated clock. Polling is
 * deliberately slow (30 s): the interface is not a real-time console, and a
 * homelab gains nothing from hammering its own server. It also pauses while
 * the tab is hidden (a background tab, a minimised window) and catches up
 * with one immediate refresh as soon as it is visible again.
 */
import { browser } from '$app/environment';
import { listAlerts, listTargets, type Alert, type Target } from '$lib/api';
import { displayState, type ProbeStatus } from '$lib/format';
import { loadProbeStatuses } from '$lib/metrics';

const INTERVAL_MS = 30_000;

class AlertsStore {
	alerts = $state<Alert[]>([]);
	/** Devices, refreshed with the alerts so the badge can count unreachable ones. */
	targets = $state<Target[]>([]);
	probes = $state<Map<number, ProbeStatus>>(new Map());
	/** True until the first response arrives. */
	loading = $state(true);
	/** False if the last refresh failed — not the route being unserved only:
	 *  any page reading `targets`/`probes` from here shows its own error from
	 *  `lastError` when this is false and it has nothing else to show. */
	available = $state(true);
	/** The cause of the last failed refresh; `null` once one succeeds. */
	lastError = $state<unknown>(null);
	#subscribers = 0;

	/**
	 * Alerts that actually need attention: firing right now, not suppressed
	 * by an offline parent, and not acknowledged (someone already knows).
	 * Pending ("building up") and resolved ones are not counted — they are not
	 * yet, or no longer, a problem.
	 */
	get activeCount(): number {
		const firing = this.alerts.filter(
			(alert) => alert.effective_phase === 'firing' && !alert.acked
		);
		const covered = new Set(
			this.alerts
				.filter((alert) => alert.effective_phase === 'firing')
				.map((alert) => alert.target_id)
		);
		// A device that is unreachable but has no firing alert yet (no data, rule
		// still evaluating) still needs attention: count it once, like the overview.
		const unreachable = this.targets.filter(
			(target) => {
				const state = displayState(target, this.probes.get(target.id));
				return (
					(state === 'offline' || state === 'down' || state === 'misconfigured') &&
					!covered.has(target.id)
				);
			}
		).length;
		return firing.length + unreachable;
	}

	async refresh(signal?: AbortSignal): Promise<void> {
		try {
			// Targets share the hard failure path with alerts now that pages read
			// them from here: a page that used to run its own `listTargets` and
			// show the result in detail must still see a real error, not a
			// silently empty list. Probe states stay soft — decoration everywhere
			// they are used, never the reason a page shows an error banner.
			const [alerts, targets] = await Promise.all([listAlerts(signal), listTargets(signal)]);
			const probes = await loadProbeStatuses(signal).catch(() => new Map<number, ProbeStatus>());
			this.alerts = alerts;
			this.targets = targets;
			this.probes = probes;
			this.available = true;
			this.lastError = null;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			// The badge must not pollute the interface with a stale count; the
			// pages that show the error in detail keep their last known targets
			// (`available`/`lastError` tell them whether to trust that list).
			this.alerts = [];
			this.available = false;
			this.lastError = cause;
		} finally {
			this.loading = false;
		}
	}

	/**
	 * Starts polling. Counts its callers so that a single timer runs, and
	 * returns the stop function to hand to an `$effect`. Skips a tick while
	 * the tab is hidden, and refreshes once immediately when it becomes
	 * visible again so the display catches up without waiting for the next
	 * interval.
	 */
	startPolling(): () => void {
		if (!browser) return () => {};
		this.#subscribers += 1;
		const controller = new AbortController();
		const tick = () => {
			if (document.visibilityState === 'hidden') return;
			void this.refresh(controller.signal);
		};
		tick();
		const timer = setInterval(tick, INTERVAL_MS);
		const onVisibilityChange = () => {
			if (document.visibilityState === 'visible') tick();
		};
		document.addEventListener('visibilitychange', onVisibilityChange);
		return () => {
			this.#subscribers -= 1;
			controller.abort();
			clearInterval(timer);
			document.removeEventListener('visibilitychange', onVisibilityChange);
		};
	}
}

export const alertsStore = new AlertsStore();
