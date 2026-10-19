<script lang="ts">
	/**
	 * Settings → LDAP / Active Directory: people sign in on the usual sign-in
	 * screen with their directory username and password. The server binds a
	 * service account, finds the user with the filter, then binds as that user
	 * with the typed password; groups map to roles. Settings saved here win
	 * entirely over the `DUMBMONIT_LDAP_*` environment variables. The service
	 * account password is stored encrypted and never comes back.
	 */
	import { Check, Radar, X } from 'lucide-svelte';
	import {
		clearLdapConfig,
		getLdapConfig,
		saveLdapConfig,
		testLdapConnection,
		type LdapConfig,
		type LdapConfigPayload,
		type LdapGroupMode,
		type LdapTestReport,
		type LdapTestStep
	} from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, ErrorNotice, Field, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import PasswordInput from './PasswordInput.svelte';
	import UnsavedBadge from './UnsavedBadge.svelte';
	import { useDirtyTracker } from '#lib/settings/unsaved.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	const USERNAME = '{username}';
	const DN = '{dn}';

	/** Starting points: the filters and attributes each directory family uses. */
	const PRESETS = {
		ad: {
			user_filter: `(&(objectClass=user)(sAMAccountName=${USERNAME}))`,
			username_attribute: 'sAMAccountName',
			display_name_attribute: 'displayName',
			email_attribute: 'mail',
			group_mode: 'memberof' as LdapGroupMode,
			// Nested groups, resolved by the domain controller (LDAP_MATCHING_RULE_IN_CHAIN).
			group_filter: `(&(objectClass=group)(member:1.2.840.113556.1.4.1941:=${DN}))`
		},
		openldap: {
			user_filter: `(&(objectClass=inetOrgPerson)(uid=${USERNAME}))`,
			username_attribute: 'uid',
			display_name_attribute: 'cn',
			email_attribute: 'mail',
			group_mode: 'search' as LdapGroupMode,
			group_filter: `(&(objectClass=groupOfNames)(member=${DN}))`
		}
	};

	let config = $state<LdapConfig | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	let form = $state({
		url: '',
		starttls: false,
		tls_verify: true,
		ca_cert: '',
		bind_dn: '',
		bind_password: '',
		base_dn: '',
		user_filter: '',
		username_attribute: '',
		display_name_attribute: '',
		email_attribute: '',
		group_mode: 'memberof' as LdapGroupMode,
		group_base_dn: '',
		group_filter: '',
		admin_groups: '',
		operator_groups: '',
		viewer_groups: '',
		timeout_seconds: 5
	});
	let errors = $state<{ url?: string; base_dn?: string; bind_password?: string; user_filter?: string; group_filter?: string }>({});
	let testUsername = $state('');

	// The password is compared as typed or not: its text never goes into the snapshot.
	const tracker = useDirtyTracker(() => ({ ...form, bind_password: form.bind_password !== '' }));

	function fill(next: LdapConfig) {
		config = next;
		form = {
			url: next.url,
			starttls: next.starttls,
			tls_verify: next.tls_verify,
			ca_cert: next.ca_cert,
			bind_dn: next.bind_dn,
			bind_password: '',
			base_dn: next.base_dn,
			user_filter: next.user_filter,
			username_attribute: next.username_attribute,
			display_name_attribute: next.display_name_attribute,
			email_attribute: next.email_attribute,
			group_mode: next.group_mode,
			group_base_dn: next.group_base_dn,
			group_filter: next.group_filter,
			admin_groups: next.admin_groups.join('\n'),
			operator_groups: next.operator_groups.join('\n'),
			viewer_groups: next.viewer_groups.join('\n'),
			timeout_seconds: next.timeout_seconds
		};
		tracker.markClean();
	}

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			fill(await getLdapConfig(signal));
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	const fromEnv = $derived(config?.source === 'env');
	const isLdaps = $derived(/^ldaps:\/\//i.test(form.url.trim()));

	function applyPreset(name: keyof typeof PRESETS) {
		form = { ...form, ...PRESETS[name] };
		errors = { ...errors, user_filter: undefined, group_filter: undefined };
	}

	function lines(value: string): string[] {
		return value
			.split('\n')
			.map((line) => line.trim())
			.filter(Boolean);
	}

	function payload(): LdapConfigPayload {
		return {
			url: form.url.trim(),
			starttls: !isLdaps && form.starttls,
			tls_verify: form.tls_verify,
			ca_cert: form.ca_cert.trim(),
			bind_dn: form.bind_dn.trim(),
			bind_password: form.bind_password || undefined,
			base_dn: form.base_dn.trim(),
			user_filter: form.user_filter.trim(),
			username_attribute: form.username_attribute.trim(),
			display_name_attribute: form.display_name_attribute.trim(),
			email_attribute: form.email_attribute.trim(),
			group_mode: form.group_mode,
			group_base_dn: form.group_base_dn.trim(),
			group_filter: form.group_filter.trim(),
			admin_groups: lines(form.admin_groups),
			operator_groups: lines(form.operator_groups),
			viewer_groups: lines(form.viewer_groups),
			timeout_seconds: Math.min(30, Math.max(1, Math.round(Number(form.timeout_seconds) || 5)))
		};
	}

	function filterOk(value: string, markers: string[]): boolean {
		const trimmed = value.trim();
		return trimmed.startsWith('(') && trimmed.endsWith(')') && markers.some((marker) => trimmed.includes(marker));
	}

	/** The stored password is kept only for the same server and the same service account. */
	const storedPasswordUsable = $derived(
		!!config &&
			config.source === 'settings' &&
			config.has_bind_password &&
			config.url.toLowerCase() === form.url.trim().replace(/\/+$/, '').toLowerCase() &&
			config.bind_dn === form.bind_dn.trim()
	);

	// --- Save -----------------------------------------------------------------

	let saving = $state(false);
	let saveError = $state<unknown>(null);
	let saved = $state(false);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		saveError = null;
		saved = false;
		const found: typeof errors = {};
		const next = payload();
		if (next.url) {
			if (!/^ldaps?:\/\/\S+$/i.test(next.url)) found.url = m.settings_ldap_err_url();
			else if (!isLdaps && !next.starttls && !config?.allow_plain) found.url = m.settings_ldap_err_plain();
			if (!next.base_dn) found.base_dn = m.settings_ldap_err_base_dn();
			if (next.user_filter && !filterOk(next.user_filter, [USERNAME])) {
				found.user_filter = m.settings_ldap_err_filter({ placeholder: USERNAME });
			}
			if (next.group_mode === 'search' && next.group_filter && !filterOk(next.group_filter, [DN, USERNAME])) {
				found.group_filter = m.settings_ldap_err_filter({ placeholder: DN });
			}
			if (next.bind_dn && !form.bind_password && !storedPasswordUsable) {
				found.bind_password = m.settings_ldap_err_secret();
			}
		}
		errors = found;
		if (Object.keys(found).length > 0) return;

		saving = true;
		try {
			fill(await saveLdapConfig(next));
			saved = true;
			// The sign-in screen mentions directory accounts while LDAP is on.
			void auth.refresh();
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = false;
		}
	}

	let clearing = $state(false);
	async function clearSaved() {
		clearing = true;
		saveError = null;
		saved = false;
		try {
			fill(await clearLdapConfig());
			void auth.refresh();
		} catch (cause) {
			saveError = cause;
		} finally {
			clearing = false;
		}
	}

	// --- Test -----------------------------------------------------------------

	let testing = $state(false);
	let report = $state<LdapTestReport | null>(null);
	let testError = $state<unknown>(null);

	const STEP_LABELS: Record<LdapTestStep['step'], () => string> = {
		connect: () => m.settings_ldap_step_connect(),
		service_bind: () => m.settings_ldap_step_service_bind(),
		user_search: () => m.settings_ldap_step_user_search(),
		groups: () => m.settings_ldap_step_groups()
	};
	const ROLE_LABELS = {
		admin: () => m.settings_users_role_admin(),
		operator: () => m.settings_users_role_operator(),
		viewer: () => m.settings_users_role_viewer()
	};

	async function test() {
		testing = true;
		report = null;
		testError = null;
		try {
			report = await testLdapConnection(payload(), testUsername.trim() || undefined);
		} catch (cause) {
			testError = cause;
		} finally {
			testing = false;
		}
	}
</script>

<Panel id="ldap" title={m.settings_ldap_title()} description={m.settings_ldap_description()}>
	{#snippet aside()}
		{#if config}
			{#if config.enabled}
				<Plate tone="signal" label={m.settings_ldap_on()} />
			{:else}
				<Plate tone="ghost" label={m.settings_ldap_off()} />
			{/if}
		{/if}
	{/snippet}

	{#if error}
		<ErrorNotice {error} title={m.settings_ldap_load_error()} onretry={() => void load()} />
	{:else if loading || !config}
		<Skeleton class="h-10 w-full" rows={6} />
	{:else}
		<form class="grid gap-4" onsubmit={submit} novalidate>
			<p class="text-sm text-ink-2">
				{#if fromEnv}
					{m.settings_ldap_note_env()}
				{:else if config.source === 'settings'}
					{m.settings_ldap_note_settings()}
				{:else}
					{m.settings_ldap_note_none()}
				{/if}
				{m.settings_ldap_local_note()}
			</p>

			<div class="flex flex-wrap items-center gap-2">
				<span class="text-sm font-semibold text-ink">{m.settings_ldap_preset_label()}</span>
				<Button variant="ghost" size="sm" onclick={() => applyPreset('ad')} disabled={saving}>{m.settings_ldap_preset_ad()}</Button>
				<Button variant="ghost" size="sm" onclick={() => applyPreset('openldap')} disabled={saving}>{m.settings_ldap_preset_openldap()}</Button>
			</div>

			<div class="grid gap-4 sm:grid-cols-2">
				<Field label={m.settings_ldap_url()} for="ldap-url" error={errors.url} help={m.settings_ldap_url_help()} required class="sm:col-span-2">
					<input id="ldap-url" type="url" class="input" bind:value={form.url} placeholder="ldaps://dc01.corp.example:636" autocomplete="off" spellcheck="false" disabled={saving} aria-invalid={errors.url ? 'true' : undefined} oninput={() => (errors = { ...errors, url: undefined })} />
				</Field>
				{#if !isLdaps}
					<div class="sm:col-span-2">
						<Field label={m.settings_ldap_starttls()} for="ldap-starttls" inline help={m.settings_ldap_starttls_help()}>
							<Toggle id="ldap-starttls" bind:checked={form.starttls} disabled={saving} onchange={() => (errors = { ...errors, url: undefined })} />
						</Field>
					</div>
				{/if}
				<Field label={m.settings_ldap_bind_dn()} for="ldap-bind-dn" help={m.settings_ldap_bind_dn_help()}>
					<input id="ldap-bind-dn" type="text" class="input" bind:value={form.bind_dn} placeholder="CN=svc-dumbmonit,OU=Service,DC=corp,DC=example" autocomplete="off" spellcheck="false" disabled={saving} />
				</Field>
				<Field label={m.settings_ldap_bind_password()} for="ldap-bind-password" error={errors.bind_password} help={storedPasswordUsable ? m.settings_ldap_bind_password_stored() : m.settings_ldap_bind_password_help()}>
					<PasswordInput id="ldap-bind-password" bind:value={form.bind_password} autocomplete="off" placeholder={storedPasswordUsable ? '••••••••' : ''} disabled={saving} invalid={!!errors.bind_password} oninput={() => (errors = { ...errors, bind_password: undefined })} />
				</Field>
				<Field label={m.settings_ldap_base_dn()} for="ldap-base-dn" error={errors.base_dn} help={m.settings_ldap_base_dn_help()} required class="sm:col-span-2">
					<input id="ldap-base-dn" type="text" class="input" bind:value={form.base_dn} placeholder="DC=corp,DC=example" autocomplete="off" spellcheck="false" disabled={saving} aria-invalid={errors.base_dn ? 'true' : undefined} oninput={() => (errors = { ...errors, base_dn: undefined })} />
				</Field>
				<Field label={m.settings_ldap_user_filter()} for="ldap-user-filter" error={errors.user_filter} help={m.settings_ldap_user_filter_help({ placeholder: USERNAME })} class="sm:col-span-2">
					<input id="ldap-user-filter" type="text" class="input font-mono text-[0.8125rem]" bind:value={form.user_filter} placeholder={PRESETS.ad.user_filter} autocomplete="off" spellcheck="false" disabled={saving} aria-invalid={errors.user_filter ? 'true' : undefined} oninput={() => (errors = { ...errors, user_filter: undefined })} />
				</Field>
			</div>

			<fieldset class="grid gap-4 rounded-[var(--radius-card)] border border-line p-4 sm:grid-cols-2">
				<legend class="px-1 text-sm font-semibold text-ink">{m.settings_ldap_groups_heading()}</legend>
				<Field label={m.settings_ldap_group_mode()} for="ldap-group-mode" help={m.settings_ldap_group_mode_help()} class="sm:col-span-2">
					<select id="ldap-group-mode" class="input" bind:value={form.group_mode} disabled={saving}>
						<option value="memberof">{m.settings_ldap_group_mode_memberof()}</option>
						<option value="search">{m.settings_ldap_group_mode_search()}</option>
					</select>
				</Field>
				{#if form.group_mode === 'search'}
					<Field label={m.settings_ldap_group_base_dn()} for="ldap-group-base" help={m.settings_ldap_group_base_dn_help()}>
						<input id="ldap-group-base" type="text" class="input" bind:value={form.group_base_dn} placeholder="OU=Groups,DC=corp,DC=example" autocomplete="off" spellcheck="false" disabled={saving} />
					</Field>
					<Field label={m.settings_ldap_group_filter()} for="ldap-group-filter" error={errors.group_filter} help={m.settings_ldap_group_filter_help({ dn: DN, username: USERNAME })}>
						<input id="ldap-group-filter" type="text" class="input font-mono text-[0.8125rem]" bind:value={form.group_filter} placeholder={PRESETS.openldap.group_filter} autocomplete="off" spellcheck="false" disabled={saving} aria-invalid={errors.group_filter ? 'true' : undefined} oninput={() => (errors = { ...errors, group_filter: undefined })} />
					</Field>
				{/if}
				<Field label={m.settings_ldap_admin_groups()} for="ldap-admin-groups">
					<textarea id="ldap-admin-groups" class="input min-h-20 font-mono text-[0.8125rem]" rows="3" bind:value={form.admin_groups} placeholder="DumbMonit Admins" spellcheck="false" autocomplete="off" disabled={saving}></textarea>
				</Field>
				<Field label={m.settings_ldap_operator_groups()} for="ldap-operator-groups">
					<textarea id="ldap-operator-groups" class="input min-h-20 font-mono text-[0.8125rem]" rows="3" bind:value={form.operator_groups} placeholder="NOC" spellcheck="false" autocomplete="off" disabled={saving}></textarea>
				</Field>
				<Field label={m.settings_ldap_viewer_groups()} for="ldap-viewer-groups">
					<textarea id="ldap-viewer-groups" class="input min-h-20 font-mono text-[0.8125rem]" rows="3" bind:value={form.viewer_groups} placeholder="CN=IT Staff,OU=Groups,DC=corp,DC=example" spellcheck="false" autocomplete="off" disabled={saving}></textarea>
				</Field>
				<p class="text-sm text-ink-2 sm:col-span-2">{m.settings_ldap_groups_help()}</p>
			</fieldset>

			<details class="group rounded-[var(--radius-card)] border border-line">
				<summary class="cursor-pointer px-4 py-3 text-sm font-semibold text-ink select-none">{m.settings_ldap_more()}</summary>
				<div class="grid gap-4 border-t border-line px-4 py-4 sm:grid-cols-2">
					<div class="sm:col-span-2">
						<Field label={m.settings_ldap_tls_verify()} for="ldap-tls-verify" inline help={m.settings_ldap_tls_verify_help()}>
							<Toggle id="ldap-tls-verify" bind:checked={form.tls_verify} disabled={saving} />
						</Field>
					</div>
					<Field label={m.settings_ldap_ca_cert()} for="ldap-ca" help={m.settings_ldap_ca_cert_help()} class="sm:col-span-2">
						<textarea id="ldap-ca" class="input min-h-24 font-mono text-[0.75rem]" rows="4" bind:value={form.ca_cert} placeholder="-----BEGIN CERTIFICATE-----" spellcheck="false" autocomplete="off" disabled={saving}></textarea>
					</Field>
					<Field label={m.settings_ldap_username_attribute()} for="ldap-username-attr">
						<input id="ldap-username-attr" type="text" class="input" bind:value={form.username_attribute} placeholder="sAMAccountName" autocomplete="off" spellcheck="false" disabled={saving} />
					</Field>
					<Field label={m.settings_ldap_display_name_attribute()} for="ldap-display-attr">
						<input id="ldap-display-attr" type="text" class="input" bind:value={form.display_name_attribute} placeholder="displayName" autocomplete="off" spellcheck="false" disabled={saving} />
					</Field>
					<Field label={m.settings_ldap_email_attribute()} for="ldap-email-attr" help={m.settings_ldap_email_attribute_help()}>
						<input id="ldap-email-attr" type="text" class="input" bind:value={form.email_attribute} placeholder="mail" autocomplete="off" spellcheck="false" disabled={saving} />
					</Field>
					<Field label={m.settings_ldap_timeout()} for="ldap-timeout" help={m.settings_ldap_timeout_help()}>
						<input id="ldap-timeout" type="number" min="1" max="30" class="input tnum" bind:value={form.timeout_seconds} disabled={saving} />
					</Field>
				</div>
			</details>

			{#if !form.tls_verify}
				<div class="flex flex-wrap items-start gap-2 rounded-[var(--radius-card)] border border-warning/35 bg-surface p-3 text-sm text-ink-2" role="note">
					<Plate tone="warning" label={m.settings_ldap_tls_off_plate()} />
					<p class="min-w-0 flex-1">{m.settings_ldap_tls_off_warning()}</p>
				</div>
			{/if}

			{#if saveError}
				<ErrorNotice error={saveError} title={m.settings_ldap_save_error()} />
			{/if}

			<div class="flex flex-wrap items-center gap-2" aria-live="polite">
				<Button type="submit" variant="secondary" loading={saving}>{m.settings_ldap_save()}</Button>
				<UnsavedBadge dirty={tracker.dirty} />
				{#if config.source === 'settings'}
					<Confirm variant="secondary" size="md" confirmLabel={config.env_available ? m.settings_ldap_use_env() : m.settings_ldap_turn_off()} loading={clearing} onconfirm={clearSaved}>
						{m.settings_ldap_forget()}
					</Confirm>
				{/if}
				{#if saved}
					<Plate tone="signal" label={m.settings_ldap_saved()} draw />
				{/if}
			</div>
		</form>

		<div class="mt-6 grid gap-3 border-t border-line pt-4">
			<div class="flex flex-wrap items-end gap-2">
				<Field label={m.settings_ldap_test_username()} for="ldap-test-user" help={m.settings_ldap_test_username_help()} class="min-w-0 flex-[1_1_14rem]">
					<input id="ldap-test-user" type="text" class="input" bind:value={testUsername} placeholder="jdoe" autocomplete="off" autocapitalize="off" spellcheck="false" disabled={testing} />
				</Field>
				<Button variant="secondary" onclick={() => void test()} loading={testing} disabled={saving || !form.url.trim()}>
					<Radar class="size-4" aria-hidden="true" />
					{m.settings_ldap_test()}
				</Button>
			</div>

			<div aria-live="polite" class="empty:hidden">
				{#if testError}
					<ErrorNotice error={testError} title={m.settings_ldap_test_error()} />
				{:else if report}
					<div class={`rise-in rounded-[var(--radius-card)] border bg-surface p-4 ${report.ok ? 'border-signal/30' : 'border-warning/35'}`}>
						<div class="flex flex-wrap items-center gap-2">
							<p class="font-semibold text-ink">{report.ok ? m.settings_ldap_test_ok() : m.settings_ldap_test_failed()}</p>
							<Plate tone={report.ok ? 'signal' : 'warning'} label={report.ok ? m.settings_ldap_test_ok_plate() : m.settings_ldap_test_failed_plate()} />
						</div>
						<ol class="mt-3 grid gap-2 text-sm">
							{#each report.steps as step (step.step)}
								<li class="flex items-start gap-2">
									{#if step.ok}
										<Check class="mt-0.5 size-4 shrink-0 text-signal-ink" aria-hidden="true" />
									{:else}
										<X class="mt-0.5 size-4 shrink-0 text-warning-ink" aria-hidden="true" />
									{/if}
									<span class="sr-only">{step.ok ? m.settings_ldap_step_ok() : m.settings_ldap_step_failed()}</span>
									<span class="min-w-0 flex-1">
										<span class="font-semibold text-ink">{STEP_LABELS[step.step]?.() ?? step.step}</span>
										<span class="text-ink-2"> — {step.detail}</span>
										<span class="tnum text-ink-3"> · {m.settings_ldap_step_ms({ ms: step.elapsed_ms })}</span>
									</span>
								</li>
							{/each}
						</ol>
						{#if report.user}
							{@const found = report.user}
							<dl class="mt-4 grid gap-x-4 gap-y-1.5 border-t border-line pt-3 text-sm sm:grid-cols-[auto_minmax(0,1fr)]">
								<dt class="text-ink-2">{m.settings_ldap_row_dn()}</dt>
								<dd class="min-w-0 font-mono text-[0.8125rem] break-all text-ink">{found.dn}</dd>
								<dt class="text-ink-2">{m.settings_ldap_row_username()}</dt>
								<dd class="min-w-0 text-ink">{found.username}</dd>
								{#if found.display_name}
									<dt class="text-ink-2">{m.settings_ldap_row_display_name()}</dt>
									<dd class="min-w-0 text-ink">{found.display_name}</dd>
								{/if}
								{#if found.email}
									<dt class="text-ink-2">{m.settings_ldap_row_email()}</dt>
									<dd class="min-w-0 break-all text-ink">{found.email}</dd>
								{/if}
								<dt class="text-ink-2">{m.settings_ldap_row_groups()}</dt>
								<dd class="min-w-0 text-ink">
									{#if found.groups.length}
										<ul class="grid gap-0.5">
											{#each found.groups as group (group)}
												<li class="font-mono text-[0.75rem] break-all">{group}</li>
											{/each}
										</ul>
									{:else}
										{m.settings_ldap_no_groups()}
									{/if}
								</dd>
								<dt class="text-ink-2">{m.settings_ldap_row_role()}</dt>
								<dd class="min-w-0">
									{#if found.allowed && found.role}
										<Plate tone="signal" bare label={ROLE_LABELS[found.role]()} />
									{:else}
										<Plate tone="warning" bare label={m.settings_ldap_access_denied()} />
									{/if}
								</dd>
							</dl>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	{/if}
</Panel>
