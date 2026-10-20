// Editing a saved credential: pre-fill, kept secrets, re-entry. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	draftChanged,
	prefillDraft,
	secretPolicy,
	toCredential
} from '../src/lib/components/device-form/credential-edit.ts';

const text = (key) => ({ key, label: key, help: '', placeholder: '', input: 'text', choices: [], required: true });
const secret = (key, required = true) => ({ key, label: key, help: '', placeholder: '', input: 'password', choices: [], required });
const select = (key, choices) => ({ key, label: key, help: '', placeholder: '', input: 'select', choices, required: false });

const PROXMOX = { kind: 'api_token', label: 'API token', help: '', fields: [text('token_id'), secret('secret')] };
const LOGIN = { kind: 'username_password', label: 'Login', help: '', fields: [text('username'), secret('password')] };
const SNMP_V3 = {
	kind: 'snmp_v3',
	label: 'SNMP v3',
	help: '',
	fields: [
		text('username'),
		select('auth_protocol', ['sha256', 'sha512']),
		secret('auth_passphrase'),
		select('privacy_protocol', ['aes128', 'aes256', 'none']),
		secret('privacy_passphrase', false)
	]
};

const savedToken = { kind: 'api_token', has_secret: true, public: { token_id: 'dumbmonit@pve!monitor' } };

test('the token ID comes back pre-filled, the secret blank', () => {
	assert.deepEqual(prefillDraft(PROXMOX, savedToken), { token_id: 'dumbmonit@pve!monitor', secret: '' });
	// Another family: nothing to pre-fill.
	assert.deepEqual(prefillDraft(LOGIN, savedToken), { username: '', password: '' });
});

test('an untouched or token-ID-only edit keeps the saved secret, never sends it empty', () => {
	const draft = prefillDraft(PROXMOX, savedToken);
	assert.equal(draftChanged(draft, prefillDraft(PROXMOX, savedToken)), false);
	const policy = secretPolicy(PROXMOX, draft, savedToken, false);
	assert.deepEqual(policy.kept, ['secret']);
	assert.deepEqual(toCredential(PROXMOX, draft, policy.kept), { type: 'api_token', token_id: 'dumbmonit@pve!monitor' });
	// Typed: the new secret replaces the old one.
	assert.deepEqual(toCredential(PROXMOX, { ...draft, secret: 'new' }, policy.kept), {
		type: 'api_token',
		token_id: 'dumbmonit@pve!monitor',
		secret: 'new'
	});
});

test('another token ID or a new destination asks for the secret again', () => {
	const draft = { token_id: 'other@pve!monitor', secret: '' };
	const policy = secretPolicy(PROXMOX, draft, savedToken, false);
	assert.deepEqual(policy.kept, []);
	assert.deepEqual(policy.reenter, { secret: 'identity' });
	assert.equal(policy.identityKey, 'token_id');
	const moved = secretPolicy(PROXMOX, prefillDraft(PROXMOX, savedToken), savedToken, true);
	assert.deepEqual(moved.reenter, { secret: 'destination' });
});

test('a blank password is left out, not sent empty', () => {
	const saved = { kind: 'username_password', has_secret: true, public: { username: 'root@pam' } };
	const draft = prefillDraft(LOGIN, saved);
	const policy = secretPolicy(LOGIN, draft, saved, false);
	assert.deepEqual(toCredential(LOGIN, draft, policy.kept), { type: 'username_password', username: 'root@pam' });
});

test('SNMP v3 keeps the privacy passphrase left blank, and "none" turns privacy off', () => {
	const saved = {
		kind: 'snmp_v3',
		has_secret: true,
		public: { username: 'monitor', auth_protocol: 'sha512', privacy_protocol: 'aes256' }
	};
	const draft = prefillDraft(SNMP_V3, saved);
	assert.equal(draft.auth_protocol, 'sha512');
	assert.equal(draft.privacy_protocol, 'aes256');
	const policy = secretPolicy(SNMP_V3, draft, saved, false);
	assert.deepEqual(policy.kept, ['auth_passphrase', 'privacy_passphrase']);
	// The former loss: retyping only the auth passphrase sent `privacy: null`.
	assert.deepEqual(toCredential(SNMP_V3, { ...draft, auth_passphrase: 'new-auth' }, policy.kept), {
		type: 'snmp_v3',
		username: 'monitor',
		auth: { protocol: 'sha512', passphrase: 'new-auth' },
		privacy: { protocol: 'aes256' }
	});
	assert.equal(toCredential(SNMP_V3, { ...draft, privacy_protocol: 'none' }, policy.kept).privacy, null);
	// A saved user without privacy shows "none".
	const noPriv = { ...saved, public: { username: 'monitor', auth_protocol: 'sha256' } };
	assert.equal(prefillDraft(SNMP_V3, noPriv).privacy_protocol, 'none');
});
