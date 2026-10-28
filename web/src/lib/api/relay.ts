/**
 * Relay agents: agents installed on another network that run probes for the
 * server. Mirrors `crates/server/src/api/relay.rs` (UI side only; the agent
 * side of the channel is not called from the browser).
 */
import { ApiError, request } from './client';
import type { RelayAgent } from './types';

/** Every agent device, relays first, with what each one relays. */
export function listRelays(signal?: AbortSignal): Promise<RelayAgent[]> {
	return request<RelayAgent[]>('/relays', { signal });
}

/**
 * Device kinds a relay agent can probe. `null` on an older server without the
 * route: the caller then offers the relay everywhere, as before.
 */
export async function listRelayKinds(signal?: AbortSignal): Promise<string[] | null> {
	try {
		return await request<string[]>('/relays/kinds', { signal, anticipated: true });
	} catch (cause) {
		if (cause instanceof ApiError && cause.missing) return null;
		throw cause;
	}
}

/** Kinds that never go through a relay, whatever the server says: pushed, or checked on the server. */
const NEVER_RELAYED = new Set(['agent', 'agent_service', 'push', 'website', 'webchange']);

/** True when a monitor of this kind can be probed through a relay agent. */
export function relayable(kind: string, kinds: string[] | null): boolean {
	if (NEVER_RELAYED.has(kind)) return false;
	return kinds === null || kinds.includes(kind);
}
