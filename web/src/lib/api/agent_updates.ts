/**
 * Updates of the DumbMonit agent binary itself (Settings → Agents). Not the
 * Docker container `auto_update` policies. Administrator session only.
 */
import { request } from './client';
import type { AgentUpdateSettings, AgentUpdates } from './types';

export function getAgentUpdates(signal?: AbortSignal): Promise<AgentUpdates> {
	return request<AgentUpdates>('/agent/updates', { signal });
}

export function setAgentUpdateSettings(settings: Partial<AgentUpdateSettings>): Promise<AgentUpdates> {
	return request<AgentUpdates>('/agent/updates', { method: 'PUT', body: settings });
}

/** `enabled: null` follows the global setting. */
export function setAgentUpdatePolicy(id: number, policy: { enabled?: boolean | null; canary?: boolean }): Promise<AgentUpdates> {
	return request<AgentUpdates>(`/agent/updates/${id}`, { method: 'PUT', body: policy });
}

export function updateAgentNow(id: number, allowDowngrade = false): Promise<AgentUpdates> {
	return request<AgentUpdates>(`/agent/updates/${id}/now`, { method: 'POST', body: { allow_downgrade: allowDowngrade } });
}

export function updateAllAgentsNow(): Promise<AgentUpdates> {
	return request<AgentUpdates>('/agent/updates/all', { method: 'POST' });
}
