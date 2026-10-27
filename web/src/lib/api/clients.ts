/** MSP mode: clients, what belongs to each, their logo. Admin routes, but the logo. */
import { request } from './client';
import type {
	Client,
	ClientAssignments,
	ClientHealth,
	ClientInput,
	ClientKind,
	ClientsResponse,
	MspSettings
} from './types';

export function listClients(signal?: AbortSignal): Promise<ClientsResponse> {
	return request<ClientsResponse>('/clients', { signal });
}

export function createClient(payload: ClientInput): Promise<Client> {
	return request<Client>('/clients', { method: 'POST', body: payload });
}

export function updateClient(id: number, payload: ClientInput): Promise<Client> {
	return request<Client>(`/clients/${id}`, { method: 'PUT', body: payload });
}

/** Refused while the client still has targets, status pages or agent tokens. */
export function deleteClient(id: number): Promise<void> {
	return request<void>(`/clients/${id}`, { method: 'DELETE' });
}

export function saveMspSettings(enabled: boolean): Promise<MspSettings> {
	return request<MspSettings>('/clients/settings', { method: 'PUT', body: { enabled } });
}

export function getClientsOverview(signal?: AbortSignal): Promise<ClientHealth[]> {
	return request<ClientHealth[]>('/clients/overview', { signal });
}

export function getClientAssignments(signal?: AbortSignal): Promise<ClientAssignments> {
	return request<ClientAssignments>('/clients/assignments', { signal });
}

/**
 * Moves objects to a client; `null` shares them with the whole instance (not
 * for targets, status pages' targets or agent tokens). Moving a target takes
 * what depends on it along.
 */
export function assignToClient(kind: ClientKind, ids: number[], clientId: number | null): Promise<void> {
	return request<void>('/clients/assign', { method: 'PUT', body: { kind, ids, client_id: clientId } });
}

/** `data` is a `data:` URL or plain base64 of a PNG, JPEG or WebP file. */
export function uploadClientLogo(id: number, data: string): Promise<void> {
	return request<void>(`/clients/${id}/logo`, { method: 'PUT', body: { data } });
}

export function deleteClientLogo(id: number): Promise<void> {
	return request<void>(`/clients/${id}/logo`, { method: 'DELETE' });
}

/** Image URL of a client logo; `version` busts the browser cache after a change. */
export function clientLogoUrl(id: number, version?: string): string {
	return `/api/clients/${id}/logo${version ? `?v=${encodeURIComponent(version)}` : ''}`;
}
