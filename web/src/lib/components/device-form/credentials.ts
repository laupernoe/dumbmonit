import { m } from '#lib/paraglide/messages.js';
/**
 * Credential drafting: the flat form state and its translation to the API.
 *
 * The fields come from the server (`CollectorInfo.credentials`): one entry per
 * field key, always a string. `toCredential` recomposes the JSON the API
 * expects for the families it knows (nested SNMP v3, Proxmox token in two
 * pieces) and sends the flat keys as they are for a family it does not.
 *
 * The API never returns a secret. On edit the form starts from the saved
 * non-secret parts (`credential_public`: user name, token ID, protocols) and
 * blank secret fields; a secret left blank is omitted from the payload, which
 * keeps the saved one on the server (see `credential-edit.ts`). The form only
 * sends a `credential` at all when something changed.
 */
import { secretDisabled, type SecretPolicy } from './credential-edit';
import type { CredentialField, CredentialKind, CredentialView } from '#lib/api/index.js';

export type CredentialDraft = Record<string, string>;

/** Empty draft for a family: selects start on their first choice. */
export { blankDraft as emptyDraft } from './credential-edit';

/**
 * Maps the human label returned in `Target.credential_kind` back to a family
 * kind. Older servers answer in French, newer ones in English: both are
 * accepted, case-insensitively, so edit works against either build.
 */
const KIND_BY_LABEL: Record<string, CredentialKind> = {
	'snmp community': 'snmp_community',
	'communauté snmp': 'snmp_community',
	'snmp v3': 'snmp_v3',
	'api token': 'api_token',
	"jeton d'api": 'api_token',
	'username / password': 'username_password',
	'utilisateur / mot de passe': 'username_password',
	none: 'none',
	aucune: 'none'
};

export function kindFromLabel(label: string | undefined, fallback: string): string {
	const key = label?.trim().toLowerCase().replace(/\s+/g, ' ') ?? '';
	return KIND_BY_LABEL[key] ?? fallback;
}

/** The family to show first: what the target uses, else the server's first. */
export function initialView(views: CredentialView[], savedLabel: string | undefined): CredentialView {
	const first = views[0] ?? { kind: 'none', label: m.deviceform_cred_no_auth(), help: '', fields: [] };
	const kind = kindFromLabel(savedLabel, first.kind);
	return views.find((view) => view.kind === kind) ?? first;
}

export { sanitizeSecret, splitPastedToken, toCredential } from './credential-edit';

function value(draft: CredentialDraft, key: string): string {
	return (draft[key] ?? '').trim();
}

/** Field-level problems of a draft, by field key; empty when it can be sent. */
export type CredentialErrors = Record<string, string>;

function requiredMessage(field: CredentialField): string {
	return m.deviceform_cred_required({ label: field.label });
}

/**
 * Field-level problems. With a `policy` (edit), a kept secret may stay blank,
 * and a secret that must be typed again says why.
 */
export function validateCredential(
	view: CredentialView,
	draft: CredentialDraft,
	policy: SecretPolicy | null = null
): CredentialErrors {
	const errors: CredentialErrors = {};
	for (const field of view.fields) {
		const current = field.input === 'password' ? (draft[field.key] ?? '') : value(draft, field.key);
		const reason = policy?.reenter[field.key];
		if (policy?.kept.includes(field.key) && !current) continue;
		if (reason && !current && !secretDisabled(field.key, draft)) {
			const identity = view.fields.find((f) => f.key === policy?.identityKey);
			errors[field.key] =
				reason === 'identity'
					? m.deviceform_cred_reenter_identity({ label: identity?.label ?? '' })
					: m.deviceform_cred_reenter_destination();
		} else if (field.required && !current && !secretDisabled(field.key, draft)) {
			errors[field.key] = requiredMessage(field);
		} else if (field.key === 'token_id' && current && !/^[^@!=\s]+@[^@!=\s]+![^@!=\s]+$/.test(current)) {
			errors[field.key] = m.deviceform_cred_token_format();
		}
	}
	return errors;
}
