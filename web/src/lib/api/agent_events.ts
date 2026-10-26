/**
 * Windows event log rules of an agent machine, and the latest events they
 * matched. Rules reach the agent with the acknowledgement of its next batch.
 */
import { ApiError, request } from './client';
import type { AgentEvent, AgentEventRule, AgentEventRuleInput, AgentEventRules, TargetId } from './types';

/** The machine's rules and the suggested examples; `null` on a server without the feature. */
export async function getAgentEventRules(id: TargetId, signal?: AbortSignal): Promise<AgentEventRules | null> {
	try {
		return await request<AgentEventRules>(`/targets/${id}/agent/event-rules`, { signal, anticipated: true });
	} catch (cause) {
		if (cause instanceof ApiError && cause.missing) return null;
		throw cause;
	}
}

export function createAgentEventRule(id: TargetId, rule: AgentEventRuleInput): Promise<AgentEventRule> {
	return request<AgentEventRule>(`/targets/${id}/agent/event-rules`, { method: 'POST', body: rule });
}

export function updateAgentEventRule(id: TargetId, ruleId: number, rule: AgentEventRuleInput): Promise<AgentEventRule> {
	return request<AgentEventRule>(`/targets/${id}/agent/event-rules/${ruleId}`, { method: 'PUT', body: rule });
}

export function deleteAgentEventRule(id: TargetId, ruleId: number): Promise<void> {
	return request<void>(`/targets/${id}/agent/event-rules/${ruleId}`, { method: 'DELETE' });
}

/** Latest matched events, newest first. */
export function listAgentEvents(id: TargetId, limit = 50, signal?: AbortSignal): Promise<AgentEvent[]> {
	return request<AgentEvent[]>(`/targets/${id}/agent/events?limit=${limit}`, { signal });
}
