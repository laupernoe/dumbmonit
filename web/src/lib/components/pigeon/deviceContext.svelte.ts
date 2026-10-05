/**
 * What device kind the reader is looking at right now, for Pip's benefit
 * only: the device detail page and the "add a device" page already fetch
 * `GET /api/collectors` and resolve their own kind's `CollectorInfo` to build
 * their form — they just hand the result here (one line, in an `$effect`
 * with a `deviceContext.set(...)` / cleanup `deviceContext.clear()`) instead
 * of Pip fetching it all over again.
 *
 * `info` is `null` once the page unmounts or has nothing selected yet, so
 * Pip falls back to its generic device tips.
 */
import type { CollectorInfo } from '$lib/api';

class DeviceContext {
	info = $state<CollectorInfo | null>(null);

	set(info: CollectorInfo | null) {
		this.info = info;
	}

	clear() {
		this.info = null;
	}
}

export const deviceContext = new DeviceContext();
