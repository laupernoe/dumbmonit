<script lang="ts">
	/**
	 * Settings → Account & security: change your own password, sign out.
	 * Accounts that sign in through the identity provider have no password
	 * here; when the server runs unprotected there is nothing to change either.
	 */
	import { LogOut } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { ApiError, changePassword } from '#lib/api/index.js';
	import { auth, PASSWORD_MIN_LENGTH, validatePassword } from '#lib/stores/auth.svelte.js';
	import { Button, ErrorNotice, Field, Panel, Plate } from '#lib/ui/index.js';
	import PasswordInput from './PasswordInput.svelte';
	import TwoFactorSection from './TwoFactorSection.svelte';

	let current = $state('');
	let next = $state('');
	let confirmation = $state('');
	let errors = $state<{ current?: string; next?: string; confirmation?: string }>({});
	let apiError = $state<unknown>(null);
	let saving = $state(false);
	let changed = $state(false);
	let signingOut = $state(false);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		apiError = null;
		changed = false;

		const found: typeof errors = {};
		if (!current) found.current = m.settings_security_err_current();
		const problem = validatePassword(next);
		if (problem) found.next = problem;
		else if (next === current) found.next = m.settings_security_err_same();
		if (!found.next && next !== confirmation) found.confirmation = m.settings_security_err_mismatch();
		errors = found;
		if (Object.keys(found).length > 0) return;

		saving = true;
		try {
			await changePassword(current, next);
			current = '';
			next = '';
			confirmation = '';
			changed = true;
		} catch (cause) {
			if (cause instanceof ApiError && cause.status === 401) {
				errors = { current: m.settings_security_err_current_wrong() };
			} else {
				apiError = cause;
			}
		} finally {
			saving = false;
		}
	}

	async function signOut() {
		signingOut = true;
		// The layout guard redirects to the sign-in screen once the session is closed.
		await auth.logout();
	}
</script>

<Panel id="security" title={m.settings_security_title()} description={auth.user ? m.settings_security_description_user({ username: auth.user.username }) : m.settings_security_description()}>
	{#if !auth.available}
		<Plate tone="info" size="md" label={m.settings_security_no_protection()} />
	{:else if auth.user?.auth === 'oidc'}
		<div class="flex flex-wrap items-center gap-3">
			<Plate tone="info" size="md" label={m.settings_security_managed_by({ provider: auth.oidc.provider_name || m.settings_security_default_provider() })} />
			<p class="text-sm text-ink-2">{m.settings_security_oidc_note()}</p>
		</div>
	{:else}
		<form class="grid max-w-md gap-4" onsubmit={submit} novalidate>
			<Field label={m.settings_security_current_label()} for="current-password" error={errors.current}>
				<PasswordInput
					id="current-password"
					bind:value={current}
					autocomplete="current-password"
					disabled={saving}
					invalid={!!errors.current}
					oninput={() => (errors = { ...errors, current: undefined })}
				/>
			</Field>
			<Field label={m.settings_security_new_label()} for="next-password" error={errors.next} help={m.settings_security_new_help({ min: PASSWORD_MIN_LENGTH })}>
				<PasswordInput
					id="next-password"
					bind:value={next}
					autocomplete="new-password"
					disabled={saving}
					invalid={!!errors.next}
					oninput={() => (errors = { ...errors, next: undefined })}
				/>
			</Field>
			<Field label={m.settings_security_confirm_label()} for="confirm-next-password" error={errors.confirmation}>
				<PasswordInput
					id="confirm-next-password"
					bind:value={confirmation}
					autocomplete="new-password"
					disabled={saving}
					invalid={!!errors.confirmation}
					oninput={() => (errors = { ...errors, confirmation: undefined })}
				/>
			</Field>

			{#if apiError}
				<ErrorNotice error={apiError} title={m.settings_security_change_error()} />
			{/if}

			<div class="flex flex-wrap items-center gap-3" aria-live="polite">
				<Button type="submit" variant="secondary" loading={saving}>{m.settings_security_change()}</Button>
				{#if changed}
					<Plate tone="signal" label={m.settings_security_changed()} />
				{/if}
			</div>
		</form>
	{/if}

	<TwoFactorSection />

	{#if auth.available}
		<div class="mt-6 flex flex-wrap items-center justify-between gap-3 border-t border-line pt-5">
			<div>
				<p class="text-sm font-semibold text-ink">{m.settings_security_session_title()}</p>
				<p class="mt-0.5 text-[0.8125rem] text-ink-2">{m.settings_security_session_hint()}</p>
			</div>
			<Button variant="secondary" loading={signingOut} onclick={() => void signOut()}>
				<LogOut class="size-4" aria-hidden="true" />
				{m.settings_security_sign_out()}
			</Button>
		</div>
	{/if}
</Panel>
