/**
 * What the Docker panel derives from a container's latest readings. Pure
 * (no `#lib` import), so `node --test` can run it as is.
 */

/** Restarts over fifteen minutes from which a container counts as looping (same as the "Container restarting" rule). */
export const CRASHLOOP_RESTARTS = 3;

export interface UsageReading {
	up: boolean;
	cpu_percent?: number | null;
	memory_bytes?: number | null;
	memory_limit_bytes?: number | null;
	restarts_15m?: number | null;
}

export function isCrashLooping(c: Pick<UsageReading, 'restarts_15m'>): boolean {
	return (c.restarts_15m ?? 0) >= CRASHLOOP_RESTARTS;
}

/** Memory against its limit as 0..1; `null` without a limit or a reading. */
export function memoryRatio(c: UsageReading): number | null {
	const limit = c.memory_limit_bytes;
	if (c.memory_bytes == null || limit == null || limit <= 0) return null;
	return c.memory_bytes / limit;
}

/** Tone of a ratio: critical from 90 %, warning from 80 %. */
export function ratioTone(ratio: number): 'signal' | 'advisory' | 'warning' {
	return ratio >= 0.9 ? 'warning' : ratio >= 0.8 ? 'advisory' : 'signal';
}

/** Running containers with a reading for `field`, largest first, at most `limit`. */
export function topBy<T extends UsageReading & { name: string }>(
	containers: readonly T[],
	field: 'cpu_percent' | 'memory_bytes',
	limit = 5
): { key: string; label: string; value: number }[] {
	return containers
		.filter((c) => c.up && c[field] != null && Number.isFinite(c[field]))
		.map((c) => ({ key: c.name, label: c.name, value: c[field] as number }))
		.sort((a, b) => b.value - a.value)
		.slice(0, limit);
}
