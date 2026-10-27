/** Teams (per-folder rights), SAML sign-in and SCIM provisioning. Admin routes. */
import { request } from './client';
import type {
	SamlConfig,
	SamlMetadataImport,
	ScimDefaultRole,
	ScimSettings,
	ScimTokenReply,
	Team,
	TeamInput,
	TeamsResponse,
	UnassignedAccess
} from './types';

export function listTeams(signal?: AbortSignal): Promise<TeamsResponse> {
	return request<TeamsResponse>('/teams', { signal });
}

export function createTeam(payload: TeamInput): Promise<Team> {
	return request<Team>('/teams', { method: 'POST', body: payload });
}

export function updateTeam(id: number, payload: TeamInput): Promise<Team> {
	return request<Team>(`/teams/${id}`, { method: 'PUT', body: payload });
}

export function deleteTeam(id: number): Promise<void> {
	return request<void>(`/teams/${id}`, { method: 'DELETE' });
}

export function saveTeamsSettings(unassigned_access: UnassignedAccess): Promise<{ unassigned_access: UnassignedAccess }> {
	return request<{ unassigned_access: UnassignedAccess }>('/teams/settings', { method: 'PUT', body: { unassigned_access } });
}

export function getSamlConfig(signal?: AbortSignal): Promise<SamlConfig> {
	return request<SamlConfig>('/auth/saml/config', { signal });
}

/** The read-only URLs are ignored by the server; they may stay in the payload. */
export function saveSamlConfig(payload: SamlConfig): Promise<SamlConfig> {
	return request<SamlConfig>('/auth/saml/config', { method: 'PUT', body: payload });
}

export function importSamlMetadata(xml: string): Promise<SamlMetadataImport> {
	return request<SamlMetadataImport>('/auth/saml/metadata/import', { method: 'POST', body: { xml } });
}

export function getScimSettings(signal?: AbortSignal): Promise<ScimSettings> {
	return request<ScimSettings>('/auth/scim', { signal });
}

export function saveScimSettings(payload: { enabled: boolean; default_role: ScimDefaultRole }): Promise<ScimSettings> {
	return request<ScimSettings>('/auth/scim', { method: 'PUT', body: payload });
}

/** Generates or rotates the bearer token; the reply is the only time it is visible. */
export function rotateScimToken(): Promise<ScimTokenReply> {
	return request<ScimTokenReply>('/auth/scim/token', { method: 'POST' });
}

export function revokeScimToken(): Promise<void> {
	return request<void>('/auth/scim/token', { method: 'DELETE' });
}
