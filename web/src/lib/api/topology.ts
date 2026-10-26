/**
 * Topology map: nodes and links the server derives from what it already
 * knows, then the corrections made by hand and the rules learned from them.
 * Mirrors `crates/server/src/api/topology.rs` and `topology_edits.rs`.
 */
import { request } from './client';
import type {
	Topology,
	TopologyCorrectionCreated,
	TopologyCorrectionInput,
	TopologyRule,
	TopologyRuleSpec,
	TopologyRuleStatus
} from './types';

export function getTopology(signal?: AbortSignal): Promise<Topology> {
	return request<Topology>('/topology', { signal });
}

export function createCorrection(input: TopologyCorrectionInput): Promise<TopologyCorrectionCreated> {
	return request<TopologyCorrectionCreated>('/topology/corrections', { method: 'POST', body: input });
}

export function deleteCorrection(id: number): Promise<void> {
	return request<void>(`/topology/corrections/${id}`, { method: 'DELETE' });
}

/** Accepts (`active`) or declines (`dismissed`) a rule, usually a suggestion. */
export function saveRule(spec: TopologyRuleSpec, status: TopologyRuleStatus, evidence = 0): Promise<TopologyRule> {
	const body = {
		kind: spec.kind,
		subnet: spec.subnet,
		address: spec.address,
		target_kind: spec.target_kind,
		source: spec.source,
		parent: spec.parent,
		status,
		evidence
	};
	return request<TopologyRule>('/topology/rules', { method: 'POST', body });
}

export function deleteRule(id: number): Promise<void> {
	return request<void>(`/topology/rules/${id}`, { method: 'DELETE' });
}
