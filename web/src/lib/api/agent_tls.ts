/**
 * Encrypted agent channel (Settings → Agents): TLS 1.3, client certificates,
 * post-quantum key exchange. Administrator session only.
 */
import { request } from './client';
import type { AgentChannelOverview } from './types';

export function getAgentChannel(signal?: AbortSignal): Promise<AgentChannelOverview> {
	return request<AgentChannelOverview>('/agent/tls', { signal });
}

/** Require the encrypted channel: agents on the plain channel of the web port are refused. */
export function setAgentChannelRequired(required: boolean): Promise<AgentChannelOverview> {
	return request<AgentChannelOverview>('/agent/tls', { method: 'PUT', body: { required } });
}

export function startAgentCaRotation(): Promise<AgentChannelOverview> {
	return request<AgentChannelOverview>('/agent/tls/rotation', { method: 'POST' });
}

export function completeAgentCaRotation(): Promise<AgentChannelOverview> {
	return request<AgentChannelOverview>('/agent/tls/rotation/complete', { method: 'POST' });
}

export function cancelAgentCaRotation(): Promise<void> {
	return request<void>('/agent/tls/rotation', { method: 'DELETE' });
}
