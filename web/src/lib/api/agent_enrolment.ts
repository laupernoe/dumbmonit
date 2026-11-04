/**
 * Enrolment approval (Settings → Agents): which new machines enter on their
 * own, the ones waiting for approval, the hourly rate per token, and fleet
 * tokens put to sleep after a long idle time. Administrator session only.
 */
import { request } from './client';
import type { AgentEnrolmentOverview, AgentEnrolmentSettings } from './types';

export function getAgentEnrolment(signal?: AbortSignal): Promise<AgentEnrolmentOverview> {
	return request<AgentEnrolmentOverview>('/agent/enrolment', { signal });
}

export function setAgentEnrolmentSettings(settings: AgentEnrolmentSettings): Promise<AgentEnrolmentOverview> {
	return request<AgentEnrolmentOverview>('/agent/enrolment', { method: 'PUT', body: settings });
}

/** Accepts or rejects one machine or many; resolves to how many were decided. */
export function decideAgentEnrolments(ids: number[], approve: boolean): Promise<{ decided: number }> {
	return request<{ decided: number }>('/agent/enrolment/decide', { method: 'POST', body: { ids, approve } });
}

/** Puts a sleeping fleet token back in service. */
export function reactivateAgentToken(id: number): Promise<void> {
	return request<void>(`/agent/tokens/${id}/reactivate`, { method: 'POST' });
}
