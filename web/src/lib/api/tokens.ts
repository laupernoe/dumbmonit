/**
 * API tokens: what scripts, dashboards and MCP assistants present instead of a
 * session cookie. Mirrors `crates/server/src/api/tokens.rs`.
 *
 * Managing tokens is a session-only gesture: a token can never list, create or
 * revoke another token, so it can never mint one more powerful than itself.
 */
import { request } from './client';

/** What a token may do. `read` can never change anything. */
export type ApiTokenScope = 'read' | 'write';

export interface ApiToken {
	id: number;
	name: string;
	/** Start of the token (`dmt_` + a few characters), to identify it in a list. */
	prefix: string;
	scope: ApiTokenScope;
	/** RFC 3339, UTC (`2026-10-01T09:12:00Z`). */
	created_at: string;
	/** Account that created the token; `null` for tokens older than accounts. */
	created_by: string | null;
	/** `null`: never expires. */
	expires_at: string | null;
	/** Computed by the server: `expires_at` is in the past. */
	expired: boolean;
	/** Networks the token may be used from, normalised (`192.168.1.0/24`). Empty: anywhere. */
	allowed_networks: string[];
	last_used_at: string | null;
	/** Client address of the last use, as the server established it. */
	last_used_ip: string | null;
	revoked_at: string | null;
}

/** Response of `POST /api/tokens`: the only chance to see the token in clear. */
export interface CreatedApiToken extends ApiToken {
	secret: string;
}

export interface ApiTokenPayload {
	name: string;
	scope?: ApiTokenScope;
	/** 1 to 3650; `null` or absent: never expires. */
	expires_in_days?: number | null;
	/** CIDR networks or single addresses, 32 at most. Absent or empty: anywhere. */
	allowed_networks?: string[];
}

/** Lifetimes offered when creating a token; `null` is "never". */
export const TOKEN_EXPIRY_CHOICES: { days: number | null; label: string }[] = [
	{ days: 30, label: '30 days' },
	{ days: 90, label: '90 days' },
	{ days: 365, label: '1 year' },
	{ days: null, label: 'Never' }
];

/** The default lifetime: long enough not to be a chore, short enough to be forgotten safely. */
export const DEFAULT_TOKEN_EXPIRY_DAYS = 90;

/** The OpenAPI 3.1 description of this instance's API (public, no secret in it). */
export const OPENAPI_PATH = '/api/openapi.json';

/** Where the HTTP API is documented. The interface links to it, never serves it. */
export const API_DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/reference/api/';

/** Where connecting an assistant (MCP) is documented. */
export const ASSISTANT_DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/using/assistant/';

export function listApiTokens(signal?: AbortSignal): Promise<ApiToken[]> {
	return request<ApiToken[]>('/tokens', { signal });
}

/** Creates an API token. The secret is only ever returned here. */
export function createApiToken(payload: ApiTokenPayload): Promise<CreatedApiToken> {
	return request<CreatedApiToken>('/tokens', { method: 'POST', body: payload });
}

export function revokeApiToken(id: number): Promise<void> {
	return request<void>(`/tokens/${id}`, { method: 'DELETE' });
}

/**
 * Splits what was typed in the "allowed networks" field: commas, spaces and
 * line breaks all separate entries. Validation is the server's job — it
 * answers 400 with the offending entry.
 */
export function parseNetworks(raw: string): string[] {
	return raw
		.split(/[\s,;]+/)
		.map((entry) => entry.trim())
		.filter((entry) => entry.length > 0);
}
