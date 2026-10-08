<script lang="ts">
	/**
	 * Settings → Single sign-on: an OpenID Connect provider (Authentik, Keycloak,
	 * Authelia, Pocket ID…). Settings saved here win entirely over the
	 * `DUMBMONIT_OIDC_*` environment variables; those apply only while nothing is
	 * saved. The client secret is stored encrypted and never comes back.
	 */
	import { Link2, Radar } from 'lucide-svelte';
	import {
		clearOidcConfig,
		getOidcConfig,
		saveOidcConfig,
		testOidcDiscovery,
		type OidcConfig,
		type OidcTestReport
	} from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, CopyBlock, ErrorNotice, Field, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import PasswordInput from './PasswordInput.svelte';
	import { m } from '#lib/paraglide/messages.js';

	let config = $state<OidcConfig | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	let form = $state({
		provider_name: '',
		issuer: '',
		client_id: '',
		client_secret: '',
		scopes: '',
		auto_create: false,
		admin_groups: '',
		operator_groups: '',
		groups_claim: '',
		public_url: ''
	});
	let errors = $state<{ issuer?: string; client_id?: string; client_secret?: string; public_url?: string }>({});

	function fill(next: OidcConfig) {
		config = next;
		form = {
			provider_name: next.provider_name,
			issuer: next.issuer,
			client_id: next.client_id,
			client_secret: '',
			scopes: next.scopes,
			auto_create: next.auto_create,
			admin_groups: next.admin_groups.join(', '),
			operator_groups: (next.operator_groups ?? []).join(', '),
			groups_claim: next.groups_claim,
			// Prefilled from the browser: it is the address people use to reach this page.
			public_url: next.public_url || (typeof window !== 'undefined' ? window.location.origin : '')
		};
	}

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			fill(await getOidcConfig(signal));
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

	/** Redirect URI as the server will build it from the public URL being typed. */
	const redirectUri = $derived(`${(form.public_url.trim() || (typeof window !== 'undefined' ? window.location.origin : '')).replace(/\/+$/, '')}/api/auth/oidc/callback`);
	const fromEnv = $derived(config?.source === 'env');

	// --- Save -----------------------------------------------------------------

	let saving = $state(false);
	let saveError = $state<unknown>(null);
	let saved = $state(false);

	function isUrl(value: string): boolean {
		return /^https?:\/\/\S+$/.test(value.trim());
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		saveError = null;
		saved = false;
		const found: typeof errors = {};
		const issuer = form.issuer.trim();
		const clientId = form.client_id.trim();
		const turningOn = issuer !== '' || clientId !== '';
		if (turningOn && !isUrl(issuer)) found.issuer = m.settings_sso_err_issuer();
		if (turningOn && !clientId) found.client_id = m.settings_sso_err_client_id();
		if (turningOn && !form.client_secret && !(config?.has_client_secret && config.source === 'settings')) {
			found.client_secret = m.settings_sso_err_secret();
		}
		if (form.public_url.trim() && !isUrl(form.public_url)) found.public_url = m.settings_sso_err_url();
		errors = found;
		if (Object.keys(found).length > 0) return;

		saving = true;
		try {
			const next = await saveOidcConfig({
				issuer,
				client_id: clientId,
				client_secret: form.client_secret || undefined,
				provider_name: form.provider_name.trim(),
				scopes: form.scopes.trim(),
				auto_create: form.auto_create,
				admin_groups: form.admin_groups.split(',').map((g) => g.trim()).filter(Boolean),
				operator_groups: form.operator_groups.split(',').map((g) => g.trim()).filter(Boolean),
				groups_claim: form.groups_claim.trim(),
				public_url: form.public_url.trim()
			});
			fill(next);
			saved = true;
			// The sign-in button label follows the provider name.
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
			fill(await clearOidcConfig());
			void auth.refresh();
		} catch (cause) {
			saveError = cause;
		} finally {
			clearing = false;
		}
	}

	// --- Test discovery -------------------------------------------------------

	let testing = $state(false);
	let report = $state<OidcTestReport | null>(null);
	let testError = $state<unknown>(null);

	async function test() {
		testing = true;
		report = null;
		testError = null;
		try {
			report = await testOidcDiscovery(form.issuer.trim() || undefined);
		} catch (cause) {
			testError = cause;
		} finally {
			testing = false;
		}
	}
</script>

<Panel id="sso" title={m.settings_sso_title()} description={m.settings_sso_description()}>
	{#snippet aside()}
		{#if config}
			{#if config.enabled}
				<Plate tone="signal" label={m.settings_sso_on({ provider: config.provider_name })} />
			{:else}
				<Plate tone="ghost" label={m.settings_sso_off()} />
			{/if}
		{/if}
	{/snippet}

	{#if error}
		<ErrorNotice {error} title={m.settings_sso_load_error()} onretry={() => void load()} />
	{:else if loading || !config}
		<Skeleton class="h-10 w-full" rows={6} />
	{:else}
		<form class="grid gap-4" onsubmit={submit} novalidate>
			<p class="text-sm text-ink-2">
				{#if fromEnv}
					{m.settings_sso_note_env()}
				{:else if config.source === 'settings'}
					{m.settings_sso_note_settings()}
				{:else}
					{m.settings_sso_note_none()}
				{/if}
			</p>

			<div class="grid gap-4 sm:grid-cols-2">
				<Field label={m.settings_sso_provider_name()} for="sso-name" help={m.settings_sso_provider_help()}>
					<input id="sso-name" type="text" class="input" bind:value={form.provider_name} placeholder="SSO" autocomplete="off" disabled={saving} />
				</Field>
				<Field label={m.settings_sso_issuer()} for="sso-issuer" error={errors.issuer} help={m.settings_sso_issuer_help()} required>
					<input id="sso-issuer" type="url" class="input" bind:value={form.issuer} placeholder="https://id.example.org/application/o/dumbmonit/" autocomplete="off" spellcheck="false" disabled={saving} aria-invalid={errors.issuer ? 'true' : undefined} oninput={() => (errors = { ...errors, issuer: undefined })} />
				</Field>
				<Field label={m.settings_sso_client_id()} for="sso-client-id" error={errors.client_id} required>
					<input id="sso-client-id" type="text" class="input" bind:value={form.client_id} autocomplete="off" spellcheck="false" disabled={saving} aria-invalid={errors.client_id ? 'true' : undefined} oninput={() => (errors = { ...errors, client_id: undefined })} />
				</Field>
				<Field label={m.settings_sso_client_secret()} for="sso-client-secret" error={errors.client_secret} help={config.has_client_secret ? m.settings_sso_secret_stored() : m.settings_sso_secret_help()}>
					<PasswordInput id="sso-client-secret" bind:value={form.client_secret} autocomplete="off" placeholder={config.has_client_secret ? '••••••••' : ''} disabled={saving} invalid={!!errors.client_secret} oninput={() => (errors = { ...errors, client_secret: undefined })} />
				</Field>
				<Field label={m.settings_sso_public_url()} for="sso-public-url" error={errors.public_url} help={m.settings_sso_public_url_help()} class="sm:col-span-2">
					<input id="sso-public-url" type="url" class="input" bind:value={form.public_url} autocomplete="off" spellcheck="false" disabled={saving} aria-invalid={errors.public_url ? 'true' : undefined} oninput={() => (errors = { ...errors, public_url: undefined })} />
				</Field>
			</div>

			<div>
				<p class="mb-1.5 text-sm font-semibold text-ink">{m.settings_sso_redirect_heading()}</p>
				<CopyBlock value={redirectUri} label={m.settings_sso_redirect_copy()} />
			</div>

			{#if config.auto_create_defaulted && config.source !== 'none'}
				<div class="flex flex-wrap items-start gap-2 rounded-[var(--radius-card)] border border-advisory/30 bg-surface p-3 text-sm text-ink-2" role="note">
					<Plate tone="advisory" label={m.settings_sso_autocreate_plate()} />
					<p class="min-w-0 flex-1">
						{m.settings_sso_autocreate_off_body()}
						{#if fromEnv}{m.settings_sso_autocreate_env_hint()}{:else}{m.settings_sso_autocreate_ui_hint()}{/if}
					</p>
				</div>
			{/if}

			<details class="group rounded-[var(--radius-card)] border border-line">
				<summary class="cursor-pointer px-4 py-3 text-sm font-semibold text-ink select-none">{m.settings_sso_more()}</summary>
				<div class="grid gap-4 border-t border-line px-4 py-4 sm:grid-cols-2">
					<Field label={m.settings_sso_scopes()} for="sso-scopes" help={m.settings_sso_scopes_help()}>
						<input id="sso-scopes" type="text" class="input" bind:value={form.scopes} placeholder="openid profile email" autocomplete="off" spellcheck="false" disabled={saving} />
					</Field>
					<Field label={m.settings_sso_groups_claim()} for="sso-groups-claim" help={m.settings_sso_groups_claim_help()}>
						<input id="sso-groups-claim" type="text" class="input" bind:value={form.groups_claim} placeholder="groups" autocomplete="off" spellcheck="false" disabled={saving} />
					</Field>
					<Field label={m.settings_sso_admin_groups()} for="sso-admin-groups" help={m.settings_sso_admin_groups_help()} class="sm:col-span-2">
						<input id="sso-admin-groups" type="text" class="input" bind:value={form.admin_groups} placeholder="dumbmonit-admins, ops" autocomplete="off" disabled={saving} />
					</Field>
					<Field label={m.settings_sso_operator_groups()} for="sso-operator-groups" help={m.settings_sso_operator_groups_help()} class="sm:col-span-2">
						<input id="sso-operator-groups" type="text" class="input" bind:value={form.operator_groups} placeholder="noc, on-call" autocomplete="off" disabled={saving} />
					</Field>
					<div class="sm:col-span-2">
						<Field label={m.settings_sso_autocreate_label()} for="sso-auto-create" inline help={m.settings_sso_autocreate_help()}>
							<Toggle id="sso-auto-create" bind:checked={form.auto_create} disabled={saving} />
						</Field>
					</div>
				</div>
			</details>

			{#if saveError}
				<ErrorNotice error={saveError} title={m.settings_sso_save_error()} />
			{/if}

			<div class="flex flex-wrap items-center gap-2" aria-live="polite">
				<Button type="submit" variant="primary" loading={saving}>{m.settings_sso_save()}</Button>
				<Button variant="secondary" onclick={() => void test()} loading={testing} disabled={saving}>
					<Radar class="size-4" aria-hidden="true" />
					{m.settings_sso_test()}
				</Button>
				{#if config.source === 'settings'}
					<Confirm variant="secondary" size="md" confirmLabel={config.env_available ? m.settings_sso_use_env() : m.settings_sso_turn_off()} loading={clearing} onconfirm={clearSaved}>
						{m.settings_sso_forget()}
					</Confirm>
				{/if}
				{#if saved}
					<Plate tone="signal" label={m.settings_sso_saved()} draw />
				{/if}
			</div>
		</form>

		<div aria-live="polite" class="mt-4 empty:hidden">
			{#if testError}
				<ErrorNotice error={testError} title={m.settings_sso_test_error()} />
			{:else if report && !report.ok}
				<ErrorNotice title={m.settings_sso_discovery_failed()} error={new Error(report.error ?? m.settings_sso_no_answer())} />
			{:else if report?.discovery}
				{@const found = report.discovery}
				<div class="rise-in rounded-[var(--radius-card)] border border-signal/30 bg-surface p-4">
					<div class="flex flex-wrap items-center gap-2">
						<Link2 class="size-4 text-signal-ink" aria-hidden="true" />
						<p class="font-semibold text-ink">{m.settings_sso_found()}</p>
						<Plate tone="signal" label={m.settings_sso_discovery_ok()} />
					</div>
					<dl class="mt-3 grid gap-x-4 gap-y-1.5 text-sm sm:grid-cols-[auto_minmax(0,1fr)]">
						<dt class="text-ink-2">{m.settings_sso_row_issuer()}</dt>
						<dd class="min-w-0 truncate font-mono text-[0.8125rem] text-ink" title={found.issuer}>{found.issuer}</dd>
						<dt class="text-ink-2">{m.settings_sso_row_authorization()}</dt>
						<dd class="min-w-0 truncate font-mono text-[0.8125rem] text-ink" title={found.authorization_endpoint}>{found.authorization_endpoint}</dd>
						<dt class="text-ink-2">{m.settings_sso_row_token()}</dt>
						<dd class="min-w-0 truncate font-mono text-[0.8125rem] text-ink" title={found.token_endpoint}>{found.token_endpoint}</dd>
						<dt class="text-ink-2">{m.settings_sso_row_keys()}</dt>
						<dd class="min-w-0 truncate font-mono text-[0.8125rem] text-ink" title={found.jwks_uri}>{found.jwks_uri}</dd>
						{#if found.id_token_signing_alg_values_supported?.length}
							<dt class="text-ink-2">{m.settings_sso_row_signing()}</dt>
							<dd class="text-ink">{found.id_token_signing_alg_values_supported.join(', ')}</dd>
						{/if}
						{#if found.scopes_supported?.length}
							<dt class="text-ink-2">{m.settings_sso_row_scopes()}</dt>
							<dd class="text-ink">{found.scopes_supported.join(', ')}</dd>
						{/if}
					</dl>
					{#if found.id_token_signing_alg_values_supported && !found.id_token_signing_alg_values_supported.some((alg) => alg === 'RS256' || alg === 'ES256')}
						<p class="mt-3 text-sm text-advisory-ink">{m.settings_sso_alg_warning()}</p>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</Panel>
