/**
 * Sentinel mode of an agent machine (administrators): the agent watches that the
 * server answers and notifies by itself when it does not. Mirrors
 * `crates/server/src/api/agent_sentinel.rs`.
 */
import { request } from './client';
import type { AgentSentinel, AgentSentinelInput, TargetId } from './types';

export function getAgentSentinel(id: TargetId, signal?: AbortSignal): Promise<AgentSentinel> {
	return request<AgentSentinel>(`/targets/${id}/agent/sentinel`, { signal });
}

/** The agent applies it at its next report. */
export function setAgentSentinel(id: TargetId, body: AgentSentinelInput): Promise<AgentSentinel> {
	return request<AgentSentinel>(`/targets/${id}/agent/sentinel`, { method: 'PUT', body });
}
