/**
 * Runbooks and remediation. Mirrors `crates/server/src/api/remediation.rs`;
 * types live in `types.ts`. Remediation routes are session only (API tokens
 * are refused by the server).
 */
import { request } from './client';
import type {
	AlertRemediation,
	ArmChallenge,
	RemediationAction,
	RemediationActionInput,
	RemediationKind,
	RemediationPolicy,
	RemediationPolicyInput,
	RemediationRun,
	Runbook,
	RunbookInput,
	TargetId,
	TargetRemediation
} from './types';

export function listRunbooks(signal?: AbortSignal): Promise<Runbook[]> {
	return request<Runbook[]>('/runbooks', { signal });
}

export function getRunbook(id: number, signal?: AbortSignal): Promise<Runbook> {
	return request<Runbook>(`/runbooks/${id}`, { signal });
}

export function createRunbook(input: RunbookInput): Promise<Runbook> {
	return request<Runbook>('/runbooks', { method: 'POST', body: input });
}

export function updateRunbook(id: number, input: RunbookInput): Promise<Runbook> {
	return request<Runbook>(`/runbooks/${id}`, { method: 'PUT', body: input });
}

export function deleteRunbook(id: number): Promise<void> {
	return request<void>(`/runbooks/${id}`, { method: 'DELETE' });
}

export function listRemediationKinds(signal?: AbortSignal): Promise<RemediationKind[]> {
	return request<RemediationKind[]>('/remediation/kinds', { signal });
}

export function listRemediationActions(signal?: AbortSignal): Promise<RemediationAction[]> {
	return request<RemediationAction[]>('/remediation/actions', { signal });
}

export function createRemediationAction(input: RemediationActionInput): Promise<RemediationAction> {
	return request<RemediationAction>('/remediation/actions', { method: 'POST', body: input });
}

export function updateRemediationAction(id: number, input: RemediationActionInput): Promise<RemediationAction> {
	return request<RemediationAction>(`/remediation/actions/${id}`, { method: 'PUT', body: input });
}

export function deleteRemediationAction(id: number): Promise<void> {
	return request<void>(`/remediation/actions/${id}`, { method: 'DELETE' });
}

/** Runs an action by hand, from an active alert (operators) or on a device (admins). */
export function runRemediationAction(
	id: number,
	context: { fingerprint?: string; target_id?: TargetId }
): Promise<RemediationRun> {
	return request<RemediationRun>(`/remediation/actions/${id}/run`, { method: 'POST', body: context });
}

export function listRemediationPolicies(signal?: AbortSignal): Promise<RemediationPolicy[]> {
	return request<RemediationPolicy[]>('/remediation/policies', { signal });
}

export function createRemediationPolicy(input: RemediationPolicyInput): Promise<RemediationPolicy> {
	return request<RemediationPolicy>('/remediation/policies', { method: 'POST', body: input });
}

export function updateRemediationPolicy(id: number, input: RemediationPolicyInput): Promise<RemediationPolicy> {
	return request<RemediationPolicy>(`/remediation/policies/${id}`, { method: 'PUT', body: input });
}

export function deleteRemediationPolicy(id: number): Promise<void> {
	return request<void>(`/remediation/policies/${id}`, { method: 'DELETE' });
}

/** First step of arming: a single-use code and a plain summary. */
export function armRemediationPolicy(id: number): Promise<ArmChallenge> {
	return request<ArmChallenge>(`/remediation/policies/${id}/arm`, { method: 'POST' });
}

/** Second step: the code, by the same account, within two minutes. */
export function confirmRemediationPolicy(id: number, token: string): Promise<RemediationPolicy> {
	return request<RemediationPolicy>(`/remediation/policies/${id}/confirm`, { method: 'POST', body: { token } });
}

export function disableRemediationPolicy(id: number): Promise<RemediationPolicy> {
	return request<RemediationPolicy>(`/remediation/policies/${id}/disable`, { method: 'POST' });
}

export function listRemediationRuns(
	filter: { target_id?: TargetId; fingerprint?: string; policy_id?: number; before?: number; limit?: number } = {},
	signal?: AbortSignal
): Promise<RemediationRun[]> {
	return request<RemediationRun[]>('/remediation/runs', { query: filter, signal });
}

export function getAlertRemediation(fingerprint: string, signal?: AbortSignal): Promise<AlertRemediation> {
	return request<AlertRemediation>(`/remediation/alert/${encodeURIComponent(fingerprint)}`, { signal });
}

export function getTargetRemediation(id: TargetId, signal?: AbortSignal): Promise<TargetRemediation> {
	return request<TargetRemediation>(`/remediation/targets/${id}`, { signal });
}
