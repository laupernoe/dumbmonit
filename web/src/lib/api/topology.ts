/**
 * Topology map: nodes and links the server derives from what it already
 * knows. Mirrors `crates/server/src/api/topology.rs`.
 */
import { request } from './client';
import type { Topology } from './types';

export function getTopology(signal?: AbortSignal): Promise<Topology> {
	return request<Topology>('/topology', { signal });
}
