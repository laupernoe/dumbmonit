/**
 * Shared scene wiring: how the network feels (`mood`) and the one-off gestures
 * scenes play when it changes (`pulse`). Pure functions plus a small debouncing
 * tracker, used by the wall and by public status pages.
 */

export type SceneTone = 'ok' | 'warn' | 'critical';
export type WallMood = 'calm' | 'clouded' | 'storm';
export type PulseKind = 'down' | 'up' | 'resolved';
export interface ScenePulse {
	kind: PulseKind;
	at: number;
}

/** What a diff needs: devices currently down, and every open issue (down ones included). */
export interface MoodSnapshot {
	down: ReadonlySet<string>;
	issues: ReadonlySet<string>;
}

/** ok = nothing firing, warn = warnings/advisories, critical = any critical or a device down. */
export function toneFrom(input: { down: number; critical: number; warnings: number }): SceneTone {
	if (input.down > 0 || input.critical > 0) return 'critical';
	return input.warnings > 0 ? 'warn' : 'ok';
}

const WALL: Record<SceneTone, WallMood> = { ok: 'calm', warn: 'clouded', critical: 'storm' };
export const toWallMood = (tone: SceneTone): WallMood => WALL[tone];

/** The gesture a change deserves; a down beats an up, an up beats a resolution. */
export function diffPulse(prev: MoodSnapshot, next: MoodSnapshot): PulseKind | null {
	for (const key of next.down) if (!prev.down.has(key)) return 'down';
	for (const key of prev.down) if (!next.down.has(key)) return 'up';
	for (const key of prev.issues) if (!next.issues.has(key) && !prev.down.has(key)) return 'resolved';
	return null;
}

const RANK: Record<PulseKind, number> = { down: 3, up: 2, resolved: 1 };

/**
 * Feeds successive snapshots, emits at most one pulse per burst: changes within
 * `debounceMs` merge into the strongest one. The first snapshot is a baseline.
 */
export function createPulseTracker(onPulse: (pulse: ScenePulse) => void, debounceMs = 1500) {
	let last: MoodSnapshot | null = null;
	let pending: PulseKind | null = null;
	let timer: ReturnType<typeof setTimeout> | null = null;
	const reset = () => {
		last = null;
		pending = null;
		if (timer) clearTimeout(timer);
		timer = null;
	};
	return {
		push(next: MoodSnapshot) {
			const prev = last;
			last = next;
			if (!prev) return;
			const kind = diffPulse(prev, next);
			if (!kind) return;
			if (!pending || RANK[kind] > RANK[pending]) pending = kind;
			if (timer) clearTimeout(timer);
			timer = setTimeout(() => {
				timer = null;
				const fire = pending;
				pending = null;
				if (fire) onPulse({ kind: fire, at: Date.now() });
			}, debounceMs);
		},
		/** Forget the baseline (data reloaded from scratch) and drop anything pending. */
		reset,
		dispose: reset
	};
}

export interface DevOverride {
	tone: SceneTone | null;
	pulse: PulseKind | null;
}

/** Dev aid: `?mood=ok|warn|critical` and `?pulse=down|up|resolved`. */
export function parseDevOverride(search: string): DevOverride {
	const q = new URLSearchParams(search);
	const mood = q.get('mood');
	const pulse = q.get('pulse');
	return {
		tone: mood === 'ok' || mood === 'warn' || mood === 'critical' ? mood : null,
		pulse: pulse === 'down' || pulse === 'up' || pulse === 'resolved' ? pulse : null
	};
}

export const DEV_REPLAY_MS = 8000;
