/**
 * On-call: schedules, replacements, escalation policies, the escalation log and
 * the signed-in person's own contacts and hours. Mirrors the `/api/oncall` and
 * `/api/account` routes. Schedules and policies are written by admins,
 * replacements by admins and operators; the `/account` routes are for the
 * signed-in user, session only.
 */
import { request } from './client';
import type {
	Contact,
	ContactOptions,
	ContactPayload,
	Escalation,
	EscalationPolicy,
	EscalationPolicyPayload,
	OnCallEntry,
	OnCallProfile,
	OnCallProfilePayload,
	Override,
	OverridePayload,
	Person,
	Schedule,
	SchedulePayload,
	ShiftsResponse
} from './types';

export function listOnCallPeople(signal?: AbortSignal): Promise<Person[]> {
	return request<Person[]>('/oncall/people', { signal });
}

export function listSchedules(signal?: AbortSignal): Promise<Schedule[]> {
	return request<Schedule[]>('/oncall/schedules', { signal });
}

export function createSchedule(payload: SchedulePayload): Promise<Schedule> {
	return request<Schedule>('/oncall/schedules', { method: 'POST', body: payload });
}

export function updateSchedule(id: number, payload: SchedulePayload): Promise<Schedule> {
	return request<Schedule>(`/oncall/schedules/${id}`, { method: 'PUT', body: payload });
}

export function deleteSchedule(id: number): Promise<void> {
	return request<void>(`/oncall/schedules/${id}`, { method: 'DELETE' });
}

/** Shifts between two instants (RFC 3339, at most 92 days apart). */
export function getScheduleShifts(id: number, from: string, to: string, signal?: AbortSignal): Promise<ShiftsResponse> {
	return request<ShiftsResponse>(`/oncall/schedules/${id}/shifts`, { query: { from, to }, signal });
}

export function listOverrides(scheduleId: number, signal?: AbortSignal): Promise<Override[]> {
	return request<Override[]>(`/oncall/schedules/${scheduleId}/overrides`, { signal });
}

export function createOverride(scheduleId: number, payload: OverridePayload): Promise<Override> {
	return request<Override>(`/oncall/schedules/${scheduleId}/overrides`, { method: 'POST', body: payload });
}

export function deleteOverride(scheduleId: number, overrideId: number): Promise<void> {
	return request<void>(`/oncall/schedules/${scheduleId}/overrides/${overrideId}`, { method: 'DELETE' });
}

export function getOnCallNow(signal?: AbortSignal): Promise<OnCallEntry[]> {
	return request<OnCallEntry[]>('/oncall/now', { signal });
}

export function listEscalationPolicies(signal?: AbortSignal): Promise<EscalationPolicy[]> {
	return request<EscalationPolicy[]>('/oncall/policies', { signal });
}

export function createEscalationPolicy(payload: EscalationPolicyPayload): Promise<EscalationPolicy> {
	return request<EscalationPolicy>('/oncall/policies', { method: 'POST', body: payload });
}

export function updateEscalationPolicy(id: number, payload: EscalationPolicyPayload): Promise<EscalationPolicy> {
	return request<EscalationPolicy>(`/oncall/policies/${id}`, { method: 'PUT', body: payload });
}

export function deleteEscalationPolicy(id: number): Promise<void> {
	return request<void>(`/oncall/policies/${id}`, { method: 'DELETE' });
}

export function listEscalations(
	params: { status?: 'active' | 'ended'; fingerprint?: string; limit?: number } = {},
	signal?: AbortSignal
): Promise<Escalation[]> {
	return request<Escalation[]>('/oncall/escalations', { query: params, signal });
}

// --- My account -----------------------------------------------------------------

export function getContactOptions(signal?: AbortSignal): Promise<ContactOptions> {
	return request<ContactOptions>('/account/contacts/options', { signal });
}

export function listContacts(signal?: AbortSignal): Promise<Contact[]> {
	return request<Contact[]>('/account/contacts', { signal });
}

export function createContact(payload: ContactPayload): Promise<Contact> {
	return request<Contact>('/account/contacts', { method: 'POST', body: payload });
}

export function updateContact(id: number, payload: ContactPayload): Promise<Contact> {
	return request<Contact>(`/account/contacts/${id}`, { method: 'PUT', body: payload });
}

export function deleteContact(id: number): Promise<void> {
	return request<void>(`/account/contacts/${id}`, { method: 'DELETE' });
}

export function testContact(id: number): Promise<{ ok: boolean; message: string }> {
	return request<{ ok: boolean; message: string }>(`/account/contacts/${id}/test`, { method: 'POST' });
}

export function getOnCallProfile(signal?: AbortSignal): Promise<OnCallProfile> {
	return request<OnCallProfile>('/account/oncall', { signal });
}

export function updateOnCallProfile(payload: OnCallProfilePayload): Promise<OnCallProfile> {
	return request<OnCallProfile>('/account/oncall', { method: 'PUT', body: payload });
}

/** New iCal link; the previous one stops working. */
export function createOnCallCalendar(): Promise<{ url: string }> {
	return request<{ url: string }>('/account/oncall/calendar', { method: 'POST' });
}

export function revokeOnCallCalendar(): Promise<void> {
	return request<void>('/account/oncall/calendar', { method: 'DELETE' });
}
