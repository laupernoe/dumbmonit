/**
 * Dismissing alerts, from any screen: one click, the alert leaves the lists at
 * once (optimistic), a toast offers "Undo". The toast is mounted once in the
 * root layout (`DismissToast`); pages that keep their own copy of the alerts
 * (the dismissed list, a monitor's alerts) watch `version`, which moves on
 * every dismissal and every undo.
 */
import { dismissAlert, undismissAlert, type Alert } from '#lib/api/index.js';
import { alertsStore } from '#lib/stores/alerts.svelte.js';
import { withoutFingerprints } from '#lib/alerts-dismiss.js';

interface Pending {
	id: number;
	fingerprints: string[];
}

class Dismissals {
	/** Moves after every dismissal or undo, once the server has answered. */
	version = $state(0);
	/** What the toast offers to undo; `null` when no toast is shown. */
	pending = $state<Pending | null>(null);
	/** The last failure, shown by the toast host; cleared by the next gesture. */
	error = $state<string | null>(null);
	#seq = 0;

	/** Dismisses every given alert (a folded row stands for several). */
	async dismiss(alerts: readonly Alert[]): Promise<void> {
		const fingerprints = alerts.map((a) => a.fingerprint);
		if (fingerprints.length === 0) return;
		this.error = null;
		alertsStore.alerts = withoutFingerprints(alertsStore.alerts, fingerprints);
		const done: string[] = [];
		try {
			for (const fingerprint of fingerprints) {
				await dismissAlert(fingerprint);
				done.push(fingerprint);
			}
		} catch (cause) {
			this.error = cause instanceof Error ? cause.message : String(cause);
		}
		if (done.length > 0) this.pending = { id: ++this.#seq, fingerprints: done };
		this.version += 1;
		await alertsStore.refresh();
	}

	/** Brings back what the toast (or the Dismissed list) names. */
	async restore(fingerprints: readonly string[]): Promise<void> {
		this.error = null;
		this.pending = null;
		try {
			for (const fingerprint of fingerprints) await undismissAlert(fingerprint);
		} catch (cause) {
			this.error = cause instanceof Error ? cause.message : String(cause);
		}
		this.version += 1;
		await alertsStore.refresh();
	}

	clearPending(id?: number) {
		if (id === undefined || this.pending?.id === id) this.pending = null;
	}

	clearError() {
		this.error = null;
	}
}

export const dismissals = new Dismissals();
