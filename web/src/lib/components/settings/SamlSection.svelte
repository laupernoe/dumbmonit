<script lang="ts">
	/**
	 * Settings → SAML 2.0 sign-in (Entra ID, Okta, Keycloak, ADFS…). The
	 * provider needs three values from here (metadata URL, entity ID, ACS URL);
	 * DumbMonit needs the provider's metadata, pasted or loaded from its XML
	 * file, which only fills the form: nothing is saved until "Save".
	 */
	import { FileUp } from 'lucide-svelte';
	import {
		getSamlConfig,
		importSamlMetadata,
		saveSamlConfig,
		type SamlConfig
	} from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, CopyBlock, Disclosure, ErrorNotice, Field, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import UnsavedBadge from './UnsavedBadge.svelte';
	import { useDirtyTracker } from '#lib/settings/unsaved.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	let config = $state<SamlConfig | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	let form = $state({
		enabled: false,
		idp_entity_id: '',
		idp_sso_url: '',
		certificates: '',
		sp_entity_id: '',
		allow_idp_initiated: false,
		auto_create: false,
		button_label: '',
		attr_email: '',
		attr_name: '',
		attr_groups: '',
		admin_groups: '',
		operator_groups: ''
	});
	let errors = $state<{ idp_sso_url?: string; idp_entity_id?: string; certificates?: string }>({});
	const tracker = useDirtyTracker(() => form);

	const csv = (list: string[]) => list.join(', ');
	const split = (text: string) => text.split(',').map((g) => g.trim()).filter(Boolean);
	/** Every PEM block of a text, trimmed; anything around them is ignored. */
	const pems = (text: string) => text.match(/-----BEGIN CERTIFICATE-----[\s\S]*?-----END CERTIFICATE-----/g)?.map((c) => c.trim()) ?? [];

	function fill(next: SamlConfig) {
		config = next;
		form = {
			enabled: next.enabled,
			idp_entity_id: next.idp_entity_id,
			idp_sso_url: next.idp_sso_url,
			certificates: next.idp_certificates.join('\n\n'),
			sp_entity_id: next.sp_entity_id,
			allow_idp_initiated: next.allow_idp_initiated,
			auto_create: next.auto_create,
			button_label: next.button_label,
			attr_email: next.attr_email,
			attr_name: next.attr_name,
			attr_groups: next.attr_groups,
			admin_groups: csv(next.admin_groups),
			operator_groups: csv(next.operator_groups)
		};
		tracker.markClean();
	}

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			fill(await getSamlConfig(signal));
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

	// --- Import of the provider's metadata -------------------------------------

	let xml = $state('');
	let importing = $state(false);
	let importError = $state<unknown>(null);
	let imported = $state(false);

	async function runImport(text: string) {
		if (!text.trim()) return;
		importing = true;
		importError = null;
		imported = false;
		try {
			const found = await importSamlMetadata(text);
			form.idp_entity_id = found.idp_entity_id;
			form.idp_sso_url = found.idp_sso_url;
			form.certificates = found.idp_certificates.join('\n\n');
			errors = {};
			imported = true;
		} catch (cause) {
			importError = cause;
		} finally {
			importing = false;
		}
	}

	async function pickFile(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		try {
			xml = await file.text();
		} catch (cause) {
			importError = cause;
			return;
		}
		await runImport(xml);
	}

	// --- Save ------------------------------------------------------------------

	let saving = $state(false);
	let saveError = $state<unknown>(null);
	let saved = $state(false);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!config) return;
		saveError = null;
		saved = false;
		const found: typeof errors = {};
		const certs = pems(form.certificates);
		const touched = form.enabled || form.idp_sso_url.trim() !== '' || form.idp_entity_id.trim() !== '';
		if (touched && !/^https?:\/\/\S+$/.test(form.idp_sso_url.trim())) found.idp_sso_url = m.settings_saml_err_sso_url();
		if (touched && !form.idp_entity_id.trim()) found.idp_entity_id = m.settings_saml_err_entity();
		if (touched && certs.length === 0) found.certificates = m.settings_saml_err_certificate();
		errors = found;
		if (Object.keys(found).length > 0) return;

		saving = true;
		try {
			fill(
				await saveSamlConfig({
					...config,
					enabled: form.enabled,
					idp_entity_id: form.idp_entity_id.trim(),
					idp_sso_url: form.idp_sso_url.trim(),
					idp_certificates: certs,
					sp_entity_id: form.sp_entity_id.trim(),
					allow_idp_initiated: form.allow_idp_initiated,
					auto_create: form.auto_create,
					button_label: form.button_label.trim(),
					attr_email: form.attr_email.trim(),
					attr_name: form.attr_name.trim(),
					attr_groups: form.attr_groups.trim(),
					admin_groups: split(form.admin_groups),
					operator_groups: split(form.operator_groups)
				})
			);
			saved = true;
			void auth.refresh();
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = false;
		}
	}
</script>

<Panel id="saml" title={m.settings_saml_title()} description={m.settings_saml_description()}>
	{#snippet aside()}
		{#if config}
			{#if config.enabled}
				<Plate tone="signal" label={m.settings_saml_on()} />
			{:else}
				<Plate tone="ghost" label={m.settings_saml_off()} />
			{/if}
		{/if}
	{/snippet}

	{#if error}
		<ErrorNotice {error} title={m.settings_saml_load_error()} onretry={() => void load()} />
	{:else if loading || !config}
		<Skeleton class="h-10 w-full" rows={6} />
	{:else}
		<form class="grid gap-5" onsubmit={submit} novalidate>
			<section class="grid gap-3" aria-labelledby="saml-sp-heading">
				<div>
					<h3 id="saml-sp-heading" class="text-sm font-semibold text-ink">{m.settings_saml_sp_heading()}</h3>
					<p class="mt-0.5 text-sm text-ink-2">{m.settings_saml_sp_help()}</p>
				</div>
				<div class="grid gap-3">
					<div>
						<p class="mb-1.5 text-sm text-ink-2">{m.settings_saml_metadata_url()}</p>
						<CopyBlock value={config.sp_metadata_url} label={m.settings_saml_copy_metadata()} />
					</div>
					<div>
						<p class="mb-1.5 text-sm text-ink-2">{m.settings_saml_sp_entity()}</p>
						<CopyBlock value={config.sp_entity_id} label={m.settings_saml_copy_entity()} />
					</div>
					<div>
						<p class="mb-1.5 text-sm text-ink-2">{m.settings_saml_acs_url()}</p>
						<CopyBlock value={config.acs_url} label={m.settings_saml_copy_acs()} />
					</div>
				</div>
			</section>

			<section class="grid gap-3" aria-labelledby="saml-idp-heading">
				<div>
					<h3 id="saml-idp-heading" class="text-sm font-semibold text-ink">{m.settings_saml_idp_heading()}</h3>
					<p class="mt-0.5 text-sm text-ink-2">{m.settings_saml_import_help()}</p>
				</div>
				<Field label={m.settings_saml_import_label()} for="saml-xml">
					<textarea id="saml-xml" class="input min-h-24 font-mono text-[0.8125rem]" rows="4" bind:value={xml} spellcheck="false" placeholder="<EntityDescriptor …>" disabled={importing || saving}></textarea>
				</Field>
				<div class="flex flex-wrap items-center gap-2" aria-live="polite">
					<Button variant="secondary" onclick={() => void runImport(xml)} loading={importing} disabled={!xml.trim() || saving}>{m.settings_saml_import_button()}</Button>
					<label class="inline-flex h-10 cursor-pointer items-center gap-2 rounded-lg border border-line-strong px-3 text-sm font-semibold text-ink hover:bg-surface-2 has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-info">
						<FileUp class="size-4" aria-hidden="true" />
						{m.settings_saml_import_file()}
						<input type="file" class="sr-only" accept=".xml,text/xml,application/xml" onchange={pickFile} disabled={importing || saving} />
					</label>
					{#if imported}<Plate tone="signal" label={m.settings_saml_imported()} draw />{/if}
				</div>
				{#if importError}<ErrorNotice error={importError} title={m.settings_saml_import_error()} />{/if}

				<div class="grid gap-4 sm:grid-cols-2">
					<Field label={m.settings_saml_idp_entity()} for="saml-idp-entity" error={errors.idp_entity_id} required>
						<input id="saml-idp-entity" type="text" class="input" bind:value={form.idp_entity_id} autocomplete="off" spellcheck="false" disabled={saving} aria-invalid={errors.idp_entity_id ? 'true' : undefined} oninput={() => (errors = { ...errors, idp_entity_id: undefined })} />
					</Field>
					<Field label={m.settings_saml_idp_sso_url()} for="saml-idp-sso" error={errors.idp_sso_url} required>
						<input id="saml-idp-sso" type="url" class="input" bind:value={form.idp_sso_url} autocomplete="off" spellcheck="false" disabled={saving} aria-invalid={errors.idp_sso_url ? 'true' : undefined} oninput={() => (errors = { ...errors, idp_sso_url: undefined })} />
					</Field>
					<Field label={m.settings_saml_certificates()} for="saml-certs" error={errors.certificates} help={m.settings_saml_certificates_help()} class="sm:col-span-2" required>
						<textarea id="saml-certs" class="input min-h-28 font-mono text-[0.75rem]" rows="6" bind:value={form.certificates} spellcheck="false" placeholder="-----BEGIN CERTIFICATE-----" disabled={saving} aria-invalid={errors.certificates ? 'true' : undefined} oninput={() => (errors = { ...errors, certificates: undefined })}></textarea>
					</Field>
				</div>
			</section>

			<div class="grid gap-4">
				<Field label={m.settings_saml_enable()} for="saml-enabled" inline help={m.settings_saml_enable_help()}>
					<Toggle id="saml-enabled" bind:checked={form.enabled} disabled={saving} />
				</Field>
			</div>

			<Disclosure title={m.settings_saml_more()}>
				<div class="grid gap-4 sm:grid-cols-2">
					<Field label={m.settings_saml_button_label()} for="saml-label" help={m.settings_saml_button_label_help()}>
						<input id="saml-label" type="text" class="input" bind:value={form.button_label} placeholder="SAML" autocomplete="off" disabled={saving} />
					</Field>
					<Field label={m.settings_saml_sp_entity_edit()} for="saml-sp-entity" help={m.settings_saml_sp_entity_edit_help()}>
						<input id="saml-sp-entity" type="text" class="input" bind:value={form.sp_entity_id} autocomplete="off" spellcheck="false" disabled={saving} />
					</Field>
					<Field label={m.settings_saml_attr_email()} for="saml-attr-email">
						<input id="saml-attr-email" type="text" class="input" bind:value={form.attr_email} autocomplete="off" spellcheck="false" disabled={saving} />
					</Field>
					<Field label={m.settings_saml_attr_name()} for="saml-attr-name">
						<input id="saml-attr-name" type="text" class="input" bind:value={form.attr_name} autocomplete="off" spellcheck="false" disabled={saving} />
					</Field>
					<Field label={m.settings_saml_attr_groups()} for="saml-attr-groups" class="sm:col-span-2">
						<input id="saml-attr-groups" type="text" class="input" bind:value={form.attr_groups} autocomplete="off" spellcheck="false" disabled={saving} />
					</Field>
					<Field label={m.settings_saml_admin_groups()} for="saml-admin-groups" help={m.settings_saml_groups_help()} class="sm:col-span-2">
						<input id="saml-admin-groups" type="text" class="input" bind:value={form.admin_groups} placeholder="dumbmonit-admins" autocomplete="off" disabled={saving} />
					</Field>
					<Field label={m.settings_saml_operator_groups()} for="saml-operator-groups" class="sm:col-span-2">
						<input id="saml-operator-groups" type="text" class="input" bind:value={form.operator_groups} placeholder="noc, on-call" autocomplete="off" disabled={saving} />
					</Field>
					<div class="sm:col-span-2">
						<Field label={m.settings_saml_auto_create()} for="saml-auto-create" inline help={m.settings_saml_auto_create_help()}>
							<Toggle id="saml-auto-create" bind:checked={form.auto_create} disabled={saving} />
						</Field>
					</div>
					<div class="sm:col-span-2">
						<Field label={m.settings_saml_idp_initiated()} for="saml-idp-initiated" inline help={m.settings_saml_idp_initiated_help()}>
							<Toggle id="saml-idp-initiated" bind:checked={form.allow_idp_initiated} disabled={saving} />
						</Field>
					</div>
				</div>
			</Disclosure>

			{#if saveError}<ErrorNotice error={saveError} title={m.settings_saml_save_error()} />{/if}

			<div class="flex flex-wrap items-center gap-2" aria-live="polite">
				<Button type="submit" variant="secondary" loading={saving}>{m.settings_saml_save()}</Button>
				<UnsavedBadge dirty={tracker.dirty} />
				{#if saved}<Plate tone="signal" label={m.settings_saml_saved()} draw />{/if}
			</div>
		</form>
	{/if}
</Panel>
