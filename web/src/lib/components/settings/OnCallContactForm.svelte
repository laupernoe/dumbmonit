<script lang="ts">
	/**
	 * Add or change one of my on-call contacts. Two flavours: "via a team
	 * channel" (the channel's server and credentials do the sending, I only give
	 * the recipient) or "my own service" (fields from the channel catalogue).
	 * Secrets are never shown again: a blank secret on edit keeps the stored one.
	 */
	import { untrack } from 'svelte';
	import { createContact, updateContact, type Contact, type ContactOptions, type ContactPayload } from '#lib/api/index.js';
	import { Button, ErrorNotice, Field, Toggle } from '#lib/ui/index.js';
	import ChannelFieldInput from '#lib/components/notifications/ChannelFieldInput.svelte';
	import type { KindField, KindInfo } from '#lib/components/notifications/kinds.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		contact?: Contact;
		options: ContactOptions;
		kinds: KindInfo[];
		onsaved: (contact: Contact) => void;
		oncancel: () => void;
	}
	let { contact, options, kinds, onsaved, oncancel }: Props = $props();
	const editing = $derived(contact !== undefined);

	type Value = string | boolean;
	type Mode = 'channel' | 'own';

	const ownKinds = $derived(options.kinds.map((k) => kinds.find((x) => x.kind === k) ?? ({ kind: k, label: k, summary: '', doc_url: '', settings: [], secrets: [] } as KindInfo)));

	let mode = $state<Mode>(untrack(() => (contact ? (contact.via_channel_id !== null ? 'channel' : 'own') : options.team_channels.length > 0 ? 'channel' : 'own')));
	let label = $state(untrack(() => contact?.label ?? ''));
	let enabled = $state(untrack(() => contact?.enabled ?? true));
	let minSeverity = $state<'info' | 'warning' | 'critical'>(untrack(() => contact?.min_severity ?? 'warning'));
	let channelId = $state(untrack(() => (contact?.via_channel_id != null ? String(contact.via_channel_id) : '')));
	let address = $state(untrack(() => contact?.address ?? ''));
	let kind = $state(untrack(() => (contact && contact.via_channel_id === null ? contact.kind : '')));
	let settings = $state<Record<string, Value>>({});
	let secrets = $state<Record<string, string>>({});
	let clearSecrets = $state(false);
	let fieldErrors = $state<Record<string, string>>({});
	let formError = $state<string | null>(null);
	let apiError = $state<unknown>(null);
	let saving = $state(false);

	const info = $derived(kinds.find((k) => k.kind === kind));
	const settingFields = $derived(info?.settings ?? []);
	const secretFields = $derived(info?.secrets ?? []);
	const channel = $derived(options.team_channels.find((c) => String(c.id) === channelId));
	const recipientLabel = $derived(
		channel?.recipient === 'phone' ? m.oncall_contact_recipient_phone() : channel?.recipient === 'chat_id' ? m.oncall_contact_recipient_chat() : m.oncall_contact_recipient_email()
	);
	const keepsSecrets = $derived(editing && contact?.has_secret === true && !clearSecrets && contact?.kind === kind);

	function toField(field: KindField, raw: unknown): Value {
		if (field.input === 'boolean') return raw === true;
		if (raw === null || raw === undefined) return '';
		if (field.shape === 'list') return Array.isArray(raw) ? raw.map(String).join('\n') : String(raw);
		if (field.shape === 'object') return typeof raw === 'string' ? raw : JSON.stringify(raw, null, 2);
		return typeof raw === 'string' ? raw : String(raw);
	}
	// Seeds the edit fields once the catalogue entry of the contact's kind is known.
	let seeded = '';
	$effect(() => {
		if (!contact || kind !== contact.kind || seeded === kind) return;
		seeded = kind;
		untrack(() => {
			const initial: Record<string, Value> = {};
			for (const f of settingFields) initial[f.key] = toField(f, contact.settings?.[f.key]);
			settings = initial;
		});
	});

	const isBlank = (v: Value | undefined) => v === undefined || (typeof v === 'string' && v.trim() === '');

	function toServer(field: KindField, value: Value): unknown {
		if (field.input === 'boolean') return value === true;
		if (typeof value !== 'string') return value;
		const text = value.trim();
		if (field.shape === 'list') return text.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
		if (field.shape === 'object') {
			let parsed: unknown;
			try {
				parsed = JSON.parse(text);
			} catch {
				throw new Error(m.notifications_form_error_json());
			}
			if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error(m.notifications_form_error_json_object());
			return parsed;
		}
		if (field.input === 'number') {
			const n = Number(text);
			if (Number.isNaN(n)) throw new Error(m.notifications_form_error_number());
			return n;
		}
		return text;
	}

	function validate(): boolean {
		const errors: Record<string, string> = {};
		formError = null;
		if (!label.trim()) errors.label = m.oncall_contact_error_label();
		if (mode === 'channel') {
			if (!channelId) errors.channel = m.oncall_contact_error_channel();
			if (!address.trim()) errors.address = m.oncall_contact_error_address();
		} else {
			if (!kind) errors.kind = m.oncall_contact_error_kind();
			for (const f of settingFields) {
				if (f.input === 'boolean') continue;
				const v = settings[f.key];
				if (f.required && isBlank(v)) errors[`setting-${f.key}`] = m.notifications_form_error_required();
				else if (!isBlank(v)) {
					try {
						toServer(f, v ?? '');
					} catch (cause) {
						errors[`setting-${f.key}`] = cause instanceof Error ? cause.message : m.notifications_form_error_invalid();
					}
				}
			}
			for (const f of secretFields) {
				if (f.required && isBlank(secrets[f.key]) && !keepsSecrets) errors[`secret-${f.key}`] = m.notifications_form_error_secret_required();
			}
		}
		fieldErrors = errors;
		return Object.keys(errors).length === 0;
	}

	function buildPayload(): ContactPayload {
		const base = { label: label.trim(), enabled, min_severity: minSeverity };
		if (mode === 'channel') {
			const chan = options.team_channels.find((c) => String(c.id) === channelId);
			return { ...base, kind: chan?.kind, via_channel_id: Number(channelId), address: address.trim() };
		}
		const outSettings: Record<string, unknown> = {};
		for (const f of settingFields) {
			const v = settings[f.key];
			if (f.input !== 'boolean' && isBlank(v)) continue;
			outSettings[f.key] = toServer(f, v ?? '');
		}
		const typed: Record<string, unknown> = {};
		for (const f of secretFields) {
			if (!isBlank(secrets[f.key])) typed[f.key] = secrets[f.key].trim();
		}
		const payload: ContactPayload = { ...base, kind, via_channel_id: null, address: null, settings: outSettings };
		// New contact or a changed kind: always send them. On edit, only when typed or a clear is asked.
		if (!editing || clearSecrets || contact?.kind !== kind || Object.keys(typed).length > 0) payload.secrets = clearSecrets ? {} : typed;
		return payload;
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		apiError = null;
		if (!validate()) return;
		saving = true;
		try {
			const payload = buildPayload();
			onsaved(contact ? await updateContact(contact.id, payload) : await createContact(payload));
		} catch (cause) {
			apiError = cause;
		} finally {
			saving = false;
		}
	}
</script>

<form onsubmit={save} class="grid gap-4 rounded-[var(--radius-card)] border border-signal/40 bg-surface p-4 shadow-lift sm:p-5" aria-label={editing ? m.oncall_contact_edit() : m.oncall_contact_add()}>
	<h3 class="text-base font-semibold tracking-tight text-ink">{editing ? m.oncall_contact_edit() : m.oncall_contact_add()}</h3>

	<div class="inline-flex max-w-full flex-wrap gap-1 self-start rounded-lg border border-line bg-surface-2 p-1" role="group" aria-label={m.oncall_contact_mode()}>
		{#each [['channel', m.oncall_contact_mode_channel()], ['own', m.oncall_contact_mode_own()]] as [id, text] (id)}
			<button
				type="button"
				aria-pressed={mode === id}
				disabled={editing}
				class={`inline-flex min-h-10 items-center rounded-md px-3 py-1.5 text-sm font-medium transition disabled:cursor-default ${mode === id ? 'bg-surface text-ink shadow-lift' : 'text-ink-2 hover:text-ink disabled:opacity-50'}`}
				onclick={() => (mode = id as Mode)}
			>
				{text}
			</button>
		{/each}
	</div>
	<p class="-mt-2 text-[0.8125rem] text-ink-2">{mode === 'channel' ? m.oncall_contact_mode_channel_help() : m.oncall_contact_mode_own_help()}</p>

	<Field label={m.oncall_contact_label()} for="contact-label" help={m.oncall_contact_label_help()} error={fieldErrors.label} required>
		<input id="contact-label" class="input" bind:value={label} autocomplete="off" aria-invalid={fieldErrors.label ? 'true' : undefined} />
	</Field>

	{#if mode === 'channel'}
		{#if options.team_channels.length === 0}
			<p class="text-[0.8125rem] text-ink-2">{m.oncall_contact_no_channels()}</p>
		{:else}
			<div class="grid gap-4 sm:grid-cols-2">
				<Field label={m.oncall_contact_channel()} for="contact-channel" error={fieldErrors.channel} required>
					<select id="contact-channel" class="input" bind:value={channelId} aria-invalid={fieldErrors.channel ? 'true' : undefined}>
						<option value="">{m.oncall_contact_channel_pick()}</option>
						{#each options.team_channels as c (c.id)}<option value={String(c.id)}>{c.name} ({kinds.find((k) => k.kind === c.kind)?.label ?? c.kind})</option>{/each}
					</select>
				</Field>
				<Field label={recipientLabel} for="contact-address" error={fieldErrors.address} required>
					<input id="contact-address" class="input" bind:value={address} autocomplete="off" spellcheck="false" aria-invalid={fieldErrors.address ? 'true' : undefined} />
				</Field>
			</div>
		{/if}
	{:else}
		<Field label={m.oncall_contact_kind()} for="contact-kind" error={fieldErrors.kind} required>
			<select
				id="contact-kind"
				class="input sm:max-w-xs"
				value={kind}
				disabled={editing}
				onchange={(e) => {
					kind = e.currentTarget.value;
					settings = {};
					secrets = {};
				}}
			>
				<option value="">{m.oncall_contact_kind_pick()}</option>
				{#each ownKinds as k (k.kind)}<option value={k.kind}>{k.label}</option>{/each}
			</select>
		</Field>
		{#if info}
			{#if info.summary}<p class="-mt-2 text-[0.8125rem] text-ink-2">{info.summary}</p>{/if}
			{#each settingFields as field (field.key)}
				<ChannelFieldInput {field} idPrefix="contact-setting" value={settings[field.key] ?? (field.input === 'boolean' ? false : '')} error={fieldErrors[`setting-${field.key}`]} onchange={(v) => (settings[field.key] = v)} />
			{/each}
			{#if secretFields.length > 0}
				<fieldset class="grid gap-3 rounded-lg border border-line bg-canvas-deep p-3">
					<legend class="px-1 text-sm font-semibold text-ink">{m.notifications_form_secrets()}</legend>
					<p class="text-[0.8125rem] text-ink-2">{m.notifications_form_secrets_note()} {#if keepsSecrets}{m.notifications_form_secrets_saved()}{/if}</p>
					{#each secretFields as field (field.key)}
						<ChannelFieldInput
							{field}
							idPrefix="contact-secret"
							value={secrets[field.key] ?? ''}
							note={keepsSecrets ? m.notifications_form_secrets_keep() : undefined}
							error={fieldErrors[`secret-${field.key}`]}
							onchange={(v) => (secrets[field.key] = typeof v === 'string' ? v : '')}
						/>
					{/each}
					{#if editing && contact?.has_secret}
						<Field label={m.notifications_form_clear_secrets()} for="contact-clear" inline help={m.notifications_form_clear_secrets_help()}>
							<Toggle id="contact-clear" bind:checked={clearSecrets} label={m.notifications_form_clear_secrets()} />
						</Field>
					{/if}
				</fieldset>
			{/if}
		{/if}
	{/if}

	<div class="grid gap-4 sm:grid-cols-2">
		<Field label={m.oncall_contact_min_severity()} for="contact-sev" help={m.oncall_contact_min_severity_help()}>
			<select id="contact-sev" class="input" bind:value={minSeverity}>
				<option value="info">{m.notifications_form_floor_info()}</option>
				<option value="warning">{m.notifications_form_floor_warning()}</option>
				<option value="critical">{m.notifications_form_floor_critical()}</option>
			</select>
		</Field>
		<Field label={m.oncall_contact_enabled()} for="contact-enabled" inline>
			<Toggle id="contact-enabled" bind:checked={enabled} label={m.oncall_contact_enabled()} />
		</Field>
	</div>

	{#if formError}<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{formError}</p>{/if}
	{#if apiError}<ErrorNotice error={apiError} title={m.oncall_error_save()} />{/if}
	<div class="flex flex-wrap justify-end gap-2">
		<Button variant="ghost" onclick={oncancel}>{m.alerts_form_cancel()}</Button>
		<Button type="submit" variant="primary" loading={saving}>{m.oncall_save()}</Button>
	</div>
</form>
