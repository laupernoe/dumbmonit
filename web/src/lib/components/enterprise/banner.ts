/**
 * The one line at the foot of the Overview banner, read off the real state:
 * "pve-02 is down. 3 guests held back, alert sent." / "All 146 monitors up."
 *
 * Pure on purpose (no messages here): this decides *what* to say, the banner
 * picks the words in the reader's language. Worst news first — down, then
 * needs a look, then waiting, then all up.
 */
import type { ConsoleStatus } from './status.js';

export interface BannerMonitor {
	id: number;
	name: string;
	status: ConsoleStatus;
}

export interface BannerAlert {
	target_id: number | null;
	fingerprint: string;
	effective_phase: string;
	acked: boolean;
	severity: string;
	notify_count: number;
	last_notified_at: string | null;
}

/** What the line says. `name` is set when exactly one monitor is concerned. */
export type BannerLine =
	| { kind: 'empty' }
	| { kind: 'down'; count: number; name: string | null; held: number; sent: boolean }
	| { kind: 'attention'; count: number; name: string | null; alerts: number }
	| { kind: 'waiting'; up: number; total: number; waiting: number }
	| { kind: 'up'; count: number; paused: number };

export function bannerLine(monitors: readonly BannerMonitor[], alerts: readonly BannerAlert[]): BannerLine {
	if (monitors.length === 0) return { kind: 'empty' };
	const firing = alerts.filter((a) => a.effective_phase === 'firing' && !a.acked && a.severity !== 'info');
	const one = (list: readonly BannerMonitor[]) => (list.length === 1 ? list[0].name : null);

	const down = monitors.filter((mon) => mon.status === 'down');
	if (down.length > 0) {
		const ids = new Set(down.map((mon) => mon.id));
		// Guests held back: what a down parent masks, one per monitor (or per alert without one).
		const held = new Set(
			alerts
				.filter((a) => a.effective_phase === 'suppressed')
				.map((a) => (a.target_id !== null ? `t${a.target_id}` : a.fingerprint))
		).size;
		const sent = alerts.some(
			(a) =>
				a.effective_phase === 'firing' &&
				a.target_id !== null &&
				ids.has(a.target_id) &&
				(a.notify_count > 0 || a.last_notified_at !== null)
		);
		return { kind: 'down', count: down.length, name: one(down), held, sent };
	}

	const attention = monitors.filter((mon) => mon.status === 'degraded' || mon.status === 'config');
	if (attention.length > 0 || firing.length > 0) {
		return { kind: 'attention', count: attention.length, name: one(attention), alerts: firing.length };
	}

	const paused = monitors.filter((mon) => mon.status === 'paused').length;
	const active = monitors.length - paused;
	const up = monitors.filter((mon) => mon.status === 'up').length;
	const waiting = monitors.filter((mon) => mon.status === 'waiting').length;
	// `count: 0` with `paused` set: every monitor is paused.
	if (active === 0) return { kind: 'up', count: 0, paused };
	if (waiting > 0) return { kind: 'waiting', up, total: active, waiting };
	return { kind: 'up', count: up, paused };
}
