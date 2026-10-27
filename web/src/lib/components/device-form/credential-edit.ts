/**
 * Editing a saved credential: what the form may pre-fill, and which secret
 * fields can stay blank. No i18n here: the reasons are codes, the components
 * turn them into messages. Mirrors `CredentialInput::resolve` in
 * `crates/proto/src/credential.rs`.
 *
 * The API never returns a secret. It returns the non-secret parts
 * (`credential_public`: user name, Proxmox token ID, SNMP v3 protocols) under
 * the form's field keys, and `has_secret`. A secret field left blank keeps the
 * saved secret as long as the identity it belongs to (user name, token ID) and
 * the destination (address, relay, connection settings) are unchanged;
 * otherwise it must be typed again, and the form says why.
 */
import type { Credential, CredentialView, SnmpV3AuthProtocol, SnmpV3PrivacyProtocol } from '#lib/api/types.js';

export type Draft = Record<string, string>;

/** What the server says about the saved credential. */
export interface SavedCredential {
	/** Family kind (`api_token`…), already mapped from the label. */
	kind: string;
	has_secret: boolean;
	public: Record<string, string>;
}

/** Fields that name who the secret belongs to: changing one voids the saved secret. */
const IDENTITY_KEYS = ['username', 'token_id'];

/** Why a blank secret field cannot keep the saved one. */
export type ReenterReason = 'identity' | 'destination';

export interface SecretPolicy {
	/** Secret fields whose blank value keeps the saved secret. */
	kept: string[];
	/** Secret fields that must be typed again, and why. */
	reenter: Record<string, ReenterReason>;
	/** The identity field whose change voided the saved secrets, if any. */
	identityKey: string | null;
}

/** Empty draft for a family: selects start on their first choice. */
export function blankDraft(view: CredentialView): Draft {
	const draft: Draft = {};
	for (const field of view.fields) {
		draft[field.key] = field.input === 'select' ? (field.choices[0] ?? '') : '';
	}
	return draft;
}

/** The draft an edit form starts from: the saved public parts, secrets blank. */
export function prefillDraft(view: CredentialView, saved: SavedCredential | null): Draft {
	const draft = blankDraft(view);
	if (!saved || saved.kind !== view.kind) return draft;
	for (const field of view.fields) {
		if (field.input === 'password') continue;
		const value = saved.public[field.key];
		if (value === undefined) continue;
		if (field.input === 'select' && !field.choices.includes(value)) continue;
		draft[field.key] = value;
	}
	// A saved SNMP v3 user without encryption: say so rather than offer AES.
	const privacy = view.fields.find((field) => field.key === 'privacy_protocol');
	if (view.kind === 'snmp_v3' && privacy && !('privacy_protocol' in saved.public) && privacy.choices.includes('none')) {
		draft.privacy_protocol = 'none';
	}
	return draft;
}

/** Is a secret stored for this field of the saved credential? */
function savedSecret(key: string, saved: SavedCredential): boolean {
	if (!saved.has_secret) return false;
	switch (key) {
		case 'auth_passphrase':
			return 'auth_protocol' in saved.public;
		case 'privacy_passphrase':
			return 'privacy_protocol' in saved.public;
		case 'secret':
			// Only a token saved as `user@realm!name=secret` can be split again.
			return 'token_id' in saved.public;
		default:
			return true;
	}
}

/** True when this secret field is switched off by its select (SNMP v3 privacy "none"). */
export function secretDisabled(key: string, draft: Draft): boolean {
	return key === 'privacy_passphrase' && draft.privacy_protocol === 'none';
}

export function secretPolicy(
	view: CredentialView,
	draft: Draft,
	saved: SavedCredential | null,
	destinationChanged: boolean
): SecretPolicy {
	const policy: SecretPolicy = { kept: [], reenter: {}, identityKey: null };
	if (!saved || saved.kind !== view.kind) return policy;
	policy.identityKey =
		IDENTITY_KEYS.find(
			(key) =>
				view.fields.some((field) => field.key === key) &&
				key in saved.public &&
				(draft[key] ?? '').trim() !== saved.public[key]
		) ?? null;
	for (const field of view.fields) {
		if (field.input !== 'password' || !savedSecret(field.key, saved)) continue;
		if (policy.identityKey) policy.reenter[field.key] = 'identity';
		else if (destinationChanged) policy.reenter[field.key] = 'destination';
		else policy.kept.push(field.key);
	}
	return policy;
}

/** True when the draft differs from what the edit form started with. */
export function draftChanged(draft: Draft, initial: Draft): boolean {
	const keys = new Set([...Object.keys(draft), ...Object.keys(initial)]);
	return [...keys].some((key) => (draft[key] ?? '') !== (initial[key] ?? ''));
}

/**
 * Cleans a pasted value: surrounding whitespace and line breaks, and the
 * quotes a shell snippet or a JSON export wraps a secret in.
 */
export function sanitizeSecret(value: string): string {
	let out = value.replace(/[\r\n]+/g, '').trim();
	for (const quote of ['"', "'"]) {
		if (out.length >= 2 && out.startsWith(quote) && out.endsWith(quote)) {
			out = out.slice(1, -1).trim();
			break;
		}
	}
	return out;
}

/**
 * A Proxmox token pasted whole (`user@realm!name=secret`) into the Token ID
 * field: the two halves, so the form can move the secret where it belongs.
 */
export function splitPastedToken(value: string): { token_id: string; secret: string } | null {
	const clean = sanitizeSecret(value);
	const at = clean.indexOf('=');
	if (at <= 0 || !clean.slice(0, at).includes('!')) return null;
	return { token_id: clean.slice(0, at), secret: clean.slice(at + 1) };
}

function value(draft: Draft, key: string): string {
	return (draft[key] ?? '').trim();
}

/**
 * The `credential` to send. `kept` lists the secret fields whose blank value
 * keeps the saved secret: they are left out of the payload rather than sent
 * empty (an empty secret would replace the saved one).
 */
export function toCredential(view: CredentialView, draft: Draft, kept: string[] = []): Credential {
	const keeps = (key: string) => kept.includes(key) && !(draft[key] ?? '');
	switch (view.kind) {
		case 'none':
			return { type: 'none' };
		case 'snmp_community':
			return keeps('community') ? { type: 'snmp_community' } : { type: 'snmp_community', community: value(draft, 'community') };
		case 'snmp_v3': {
			const authPassphrase = draft.auth_passphrase ?? '';
			const privacyPassphrase = draft.privacy_passphrase ?? '';
			const authProtocol = (draft.auth_protocol || 'sha256') as SnmpV3AuthProtocol;
			const privacyProtocol = (draft.privacy_protocol || 'aes128') as SnmpV3PrivacyProtocol;
			return {
				type: 'snmp_v3',
				username: value(draft, 'username'),
				auth: authPassphrase
					? { protocol: authProtocol, passphrase: authPassphrase }
					: keeps('auth_passphrase')
						? { protocol: authProtocol }
						: null,
				// "none" turns encryption off, whatever is typed or saved.
				privacy: secretDisabled('privacy_passphrase', draft)
					? null
					: privacyPassphrase
						? { protocol: privacyProtocol, passphrase: privacyPassphrase }
						: keeps('privacy_passphrase')
							? { protocol: privacyProtocol }
							: null
			};
		}
		case 'api_token':
			// Two pieces (Proxmox VE, PBS) or one string (bearer token): the
			// server assembles the former and stores both the same way.
			if (view.fields.some((field) => field.key === 'token_id')) {
				const tokenId = value(draft, 'token_id');
				return keeps('secret')
					? { type: 'api_token', token_id: tokenId }
					: { type: 'api_token', token_id: tokenId, secret: sanitizeSecret(draft.secret ?? '') };
			}
			return keeps('token') ? { type: 'api_token' } : { type: 'api_token', token: sanitizeSecret(draft.token ?? '') };
		case 'username_password':
			return keeps('password')
				? { type: 'username_password', username: value(draft, 'username') }
				: { type: 'username_password', username: value(draft, 'username'), password: draft.password ?? '' };
		case 'ssh_login': {
			// A blank enable password is left out: the server keeps the saved one
			// (or the login password stands in for it).
			const enable = draft.enable_secret ?? '';
			return {
				type: 'ssh_login',
				username: value(draft, 'username'),
				...(keeps('password') ? {} : { password: draft.password ?? '' }),
				...(enable ? { enable_secret: enable } : {})
			};
		}
		case 'radius':
			return {
				type: 'radius',
				username: value(draft, 'username'),
				...(keeps('password') ? {} : { password: draft.password ?? '' }),
				...(keeps('shared_secret') ? {} : { shared_secret: sanitizeSecret(draft.shared_secret ?? '') })
			};
		default: {
			// A family this build does not know: send every field under its key.
			const out: Record<string, string> = { type: view.kind };
			for (const field of view.fields) {
				if (!keeps(field.key)) out[field.key] = draft[field.key] ?? '';
			}
			return out as Credential;
		}
	}
}
