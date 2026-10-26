/**
 * Passkeys (WebAuthn): sign in without a password, and manage the passkeys of
 * the current account. Mirrors `crates/server/src/api/passkeys.rs`.
 *
 * The server builds the ceremony options (challenge, relying-party id) and
 * verifies the answer; this file only moves bytes between the browser's
 * `navigator.credentials` and the API, as base64url strings.
 */
import { request } from './client';
import type { Passkey } from './types';

/** Passkeys need a secure context (HTTPS, or http://localhost) and a capable browser. */
export function passkeysSupported(): boolean {
	return typeof window !== 'undefined' && window.isSecureContext && 'PublicKeyCredential' in window && !!navigator.credentials;
}

function fromBase64Url(text: string): ArrayBuffer {
	const padded = text.replace(/-/g, '+').replace(/_/g, '/').padEnd(Math.ceil(text.length / 4) * 4, '=');
	const raw = atob(padded);
	const bytes = new Uint8Array(raw.length);
	for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
	return bytes.buffer;
}

function toBase64Url(buffer: ArrayBuffer | null | undefined): string {
	if (!buffer) return '';
	let raw = '';
	for (const byte of new Uint8Array(buffer)) raw += String.fromCharCode(byte);
	return btoa(raw).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

interface Descriptor {
	type: 'public-key';
	id: string;
}

interface CreationOptions {
	rp: { id: string; name: string };
	user: { id: string; name: string; displayName: string };
	challenge: string;
	pubKeyCredParams: { type: 'public-key'; alg: number }[];
	timeout?: number;
	attestation?: AttestationConveyancePreference;
	excludeCredentials?: Descriptor[];
	authenticatorSelection?: AuthenticatorSelectionCriteria;
}

interface RequestOptions {
	challenge: string;
	rpId: string;
	timeout?: number;
	userVerification?: UserVerificationRequirement;
	allowCredentials?: Descriptor[];
}

export function listPasskeys(signal?: AbortSignal): Promise<Passkey[]> {
	return request<Passkey[]>('/auth/passkeys', { signal });
}

export function renamePasskey(id: number, name: string): Promise<void> {
	return request<void>(`/auth/passkeys/${id}`, { method: 'PUT', body: { name } });
}

export function deletePasskey(id: number): Promise<void> {
	return request<void>(`/auth/passkeys/${id}`, { method: 'DELETE' });
}

/**
 * Registers a new passkey for the signed-in account. The password is asked again
 * (a stolen session must not be able to install its own key); a 401 means
 * "wrong password", never "session expired".
 */
export async function addPasskey(password: string, name: string): Promise<Passkey> {
	const begin = await request<{ challenge_id: string; options: CreationOptions }>('/auth/passkeys/register/begin', {
		method: 'POST',
		body: { password },
		allowUnauthorized: true
	});
	const o = begin.options;
	const created = (await navigator.credentials.create({
		publicKey: {
			rp: o.rp,
			user: { id: fromBase64Url(o.user.id), name: o.user.name, displayName: o.user.displayName },
			challenge: fromBase64Url(o.challenge),
			pubKeyCredParams: o.pubKeyCredParams,
			timeout: o.timeout,
			attestation: o.attestation,
			excludeCredentials: (o.excludeCredentials ?? []).map((c) => ({ type: c.type, id: fromBase64Url(c.id) })),
			authenticatorSelection: o.authenticatorSelection
		}
	})) as PublicKeyCredential | null;
	if (!created) throw new DOMException('No credential was created', 'NotAllowedError');
	const response = created.response as AuthenticatorAttestationResponse;
	return request<Passkey>('/auth/passkeys/register/finish', {
		method: 'POST',
		body: {
			challenge_id: begin.challenge_id,
			name,
			credential: {
				id: created.id,
				response: {
					clientDataJSON: toBase64Url(response.clientDataJSON),
					attestationObject: toBase64Url(response.attestationObject)
				},
				transports: typeof response.getTransports === 'function' ? response.getTransports() : []
			}
		}
	});
}

/**
 * Signs in with a discoverable passkey (no username). Resolves once the server
 * has set the session cookie; the caller refreshes the auth state.
 */
export async function signInWithPasskey(): Promise<void> {
	const begin = await request<{ challenge_id: string; options: RequestOptions }>('/auth/passkey/login/begin', {
		method: 'POST',
		body: {},
		allowUnauthorized: true
	});
	const o = begin.options;
	const asserted = (await navigator.credentials.get({
		publicKey: {
			challenge: fromBase64Url(o.challenge),
			rpId: o.rpId,
			timeout: o.timeout,
			userVerification: o.userVerification,
			allowCredentials: (o.allowCredentials ?? []).map((c) => ({ type: c.type, id: fromBase64Url(c.id) }))
		}
	})) as PublicKeyCredential | null;
	if (!asserted) throw new DOMException('No credential was selected', 'NotAllowedError');
	const response = asserted.response as AuthenticatorAssertionResponse;
	await request<void>('/auth/passkey/login/finish', {
		method: 'POST',
		allowUnauthorized: true,
		body: {
			challenge_id: begin.challenge_id,
			credential: {
				id: asserted.id,
				response: {
					clientDataJSON: toBase64Url(response.clientDataJSON),
					authenticatorData: toBase64Url(response.authenticatorData),
					signature: toBase64Url(response.signature),
					userHandle: toBase64Url(response.userHandle)
				}
			}
		}
	});
}
