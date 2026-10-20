<script lang="ts">
	/**
	 * Add or edit a log destination. Kinds and fields come from the server
	 * catalogue (`GET /api/logs/kinds`), rendered like notification channels;
	 * an unknown kind is shown as served.
	 *
	 * Secrets are never read back: on edit they show empty ("saved, leave
	 * empty to keep") and are sent only when typed again.
	 */
	import { untrack } from 'svelte';
	import { ExternalLink } from 'lucide-svelte';
	import { createLogDestination, updateLogDestination } from '#lib/api/logs.js';
	import type { LogDestination, LogLevel } from '#lib/api/types.js';
	import { Button, ClickSpark, ErrorNotice, Field, Toggle } from '#lib/ui/index.js';
	import type { KindInfo } from '#lib/components/notifications/kinds';
	import ChannelFieldInput from '#lib/components/notifications/ChannelFieldInput.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import {
		LEVELS,
		buildDestinationPayload,
		destinationChanged as changedDestination,
		isBlank,
		toFieldValue,
		toServerValue,
		type FieldValue
	} from './logs';
	import { levelLabel } from './logLabels';

	interface Props {
		kinds: KindInfo[];
		/** Present when editing, absent when adding. */
		destination?: LogDestination;
		onsaved: (destination: LogDestination) => void;
		oncancel: () => void;
	}

	let { kinds, destination, onsaved, oncancel }: Props = $props();
	const editing = $derived(destination !== undefined);

	// Frozen when the form opens: it must not follow list refreshes while typing.
	let kind = $state(untrack(() => destination?.kind ?? kinds[0]?.kind ?? ''));
	let name = $state(untrack(() => destination?.name ?? ''));
	let enabled = $state(untrack(() => destination?.enabled ?? true));
	let minLevel = $state<LogLevel>(untrack(() => destination?.min_level ?? 'info'));
	let settings = $state<Record<string, FieldValue>>(
		untrack(() => {
			const initial: Record<string, FieldValue> = {};
			const fields = kinds.find((k) => k.kind === destination?.kind)?.settings ?? [];
			for (const field of fields) initial[field.key] = toFieldValue(field, destination?.settings?.[field.key]);
			return initial;
		})
	);
	let secrets = $state<Record<string, string>>({});
	let clearSecrets = $state(false);

	const info = $derived(kinds.find((k) => k.kind === kind));
	const settingFields = $derived(info?.settings ?? []);
	const secretFields = $derived(info?.secrets ?? []);
	const savedSecrets = $derived(editing && destination?.has_secret === true && kind === destination?.kind);
	const changed = $derived(changedDestination(destination, kind, settingFields, settings));
	const keepsSecrets = $derived(editing && destination?.has_secret === true && !clearSecrets && !changed);

	// A saved secret shows a placeholder instead of a value (never pre-filled).
	const shownSecretFields = $derived(
		secretFields.map((f) => (keepsSecrets ? { ...f, placeholder: m.settings_logs_form_secret_saved() } : f))
	);

	function pickKind(next: string) {
		if (next === kind) return;
		kind = next;
		// Fields belong to a kind: switching starts from a blank sheet.
		settings = {};
		secrets = {};
		fieldErrors = {};
	}

	let fieldErrors = $state<Record<string, string>>({});
	let nameError = $state<string | null>(null);
	let apiError = $state<unknown>(null);
	let saving = $state(false);

	function validate(): boolean {
		const errors: Record<string, string> = {};
		nameError = name.trim() ? null : m.settings_logs_form_error_name();
		for (const field of settingFields) {
			if (field.input === 'boolean') continue;
			const value = settings[field.key];
			if (field.required && isBlank(value)) {
				errors[`setting-${field.key}`] = m.settings_logs_form_error_required();
				continue;
			}
			if (isBlank(value)) continue;
			try {
				toServerValue(field, value ?? '');
			} catch (cause) {
				const code = cause instanceof Error ? cause.message : '';
				errors[`setting-${field.key}`] =
					code === 'number' ? m.settings_logs_form_error_number() : m.settings_logs_form_error_invalid();
			}
		}
		for (const field of secretFields) {
			if (field.required && isBlank(secrets[field.key]) && !keepsSecrets) {
				errors[`secret-${field.key}`] = m.settings_logs_form_error_required();
			}
		}
		fieldErrors = errors;
		return !nameError && Object.keys(errors).length === 0;
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		apiError = null;
		if (!validate()) return;
		saving = true;
		try {
			const payload = buildDestinationPayload({
				name,
				kind,
				enabled,
				minLevel,
				settingFields,
				secretFields,
				settings,
				secrets,
				existing: destination,
				clearSecrets,
				destinationChanged: changed
			});
			const saved = destination
				? await updateLogDestination(destination.id, payload)
				: await createLogDestination(payload);
			onsaved(saved);
		} catch (cause) {
			apiError = cause;
		} finally {
			saving = false;
		}
	}
</script>

<form
	class="rounded-[var(--radius-card)] border border-signal/40 bg-surface shadow-lift"
	aria-label={editing ? m.settings_logs_form_edit_title({ name: destination?.name ?? '' }) : m.settings_logs_form_new_title()}
	onsubmit={save}
	novalidate
>
	<div class="flex flex-wrap items-start justify-between gap-3 border-b border-line px-4 py-3 sm:px-5">
		<div class="min-w-0">
			<h3 class="text-base font-semibold tracking-tight text-ink">
				{editing ? m.settings_logs_form_edit_title({ name: destination?.name ?? '' }) : m.settings_logs_form_new_title()}
			</h3>
			{#if info?.summary}<p class="mt-0.5 text-sm text-ink-2">{info.summary}</p>{/if}
		</div>
		{#if info?.doc_url}
			<Button variant="ghost" size="sm" href={info.doc_url} target="_blank" rel="noopener">
				{m.settings_logs_form_documentation()}
				<ExternalLink class="size-3.5" aria-hidden="true" />
			</Button>
		{/if}
	</div>

	<div class="grid gap-5 px-4 py-4 sm:px-5">
		<div class="grid gap-4 sm:grid-cols-2">
			<Field label={m.settings_logs_form_kind()} for="logdest-kind">
				<select id="logdest-kind" class="input" value={kind} disabled={saving} onchange={(e) => pickKind(e.currentTarget.value)}>
					{#each kinds as option (option.kind)}
						<option value={option.kind}>{option.label}</option>
					{/each}
					{#if kind && !info}
						<option value={kind}>{kind}</option>
					{/if}
				</select>
			</Field>
			<Field label={m.settings_logs_form_name()} for="logdest-name" required error={nameError}>
				<input
					id="logdest-name"
					type="text"
					class="input"
					bind:value={name}
					autocomplete="off"
					disabled={saving}
					aria-invalid={nameError ? 'true' : undefined}
					oninput={() => (nameError = null)}
				/>
			</Field>
		</div>

		{#if settingFields.length > 0}
			<fieldset class="grid gap-4 border-t border-line pt-5">
				<legend class="sr-only">{m.settings_logs_form_settings()}</legend>
				{#each settingFields as field (field.key)}
					<ChannelFieldInput
						{field}
						idPrefix="logsetting"
						value={settings[field.key] ?? (field.input === 'boolean' ? false : '')}
						error={fieldErrors[`setting-${field.key}`] ?? null}
						disabled={saving}
						onchange={(value) => {
							settings[field.key] = value;
							delete fieldErrors[`setting-${field.key}`];
						}}
					/>
				{/each}
			</fieldset>
		{/if}

		{#if secretFields.length > 0}
			<fieldset class="grid gap-4 border-t border-line pt-5">
				<legend class="mb-1 text-sm font-semibold text-ink">{m.settings_logs_form_secrets()}</legend>
				<p class="-mt-3 text-[0.8125rem] text-ink-2">
					{#if changed}{m.settings_logs_form_secrets_changed()}{:else if savedSecrets}{m.settings_logs_form_secret_saved()}{:else}{m.settings_logs_form_secrets_note()}{/if}
				</p>
				{#each shownSecretFields as field (field.key)}
					<ChannelFieldInput
						{field}
						idPrefix="logsecret"
						value={secrets[field.key] ?? ''}
						error={fieldErrors[`secret-${field.key}`] ?? null}
						disabled={saving}
						onchange={(value) => {
							secrets[field.key] = typeof value === 'string' ? value : '';
							delete fieldErrors[`secret-${field.key}`];
						}}
					/>
				{/each}
				{#if savedSecrets}
					<Field label={m.settings_logs_form_clear_secrets()} for="logdest-clear" inline>
						<Toggle id="logdest-clear" bind:checked={clearSecrets} disabled={saving} label={m.settings_logs_form_clear_secrets()} />
					</Field>
				{/if}
			</fieldset>
		{/if}

		<div class="grid gap-4 border-t border-line pt-5 sm:grid-cols-2">
			<Field label={m.settings_logs_form_min_level()} for="logdest-level" help={m.settings_logs_form_min_level_help()}>
				<select id="logdest-level" class="input" bind:value={minLevel} disabled={saving}>
					{#each LEVELS as level (level)}
						<option value={level}>{levelLabel(level)}</option>
					{/each}
				</select>
			</Field>
			<Field label={m.settings_logs_form_enabled()} for="logdest-enabled" inline>
				<Toggle id="logdest-enabled" bind:checked={enabled} disabled={saving} label={m.settings_logs_form_enabled()} />
			</Field>
		</div>

		{#if apiError}
			<ErrorNotice error={apiError} title={m.settings_logs_form_save_error()} />
		{/if}
	</div>

	<div class="flex flex-wrap items-center justify-end gap-2 border-t border-line px-4 py-3 sm:px-5">
		<Button variant="ghost" onclick={oncancel} disabled={saving}>{m.alerts_form_cancel()}</Button>
		<ClickSpark>
			<Button type="submit" variant="primary" loading={saving}>
				{editing ? m.settings_logs_form_save() : m.settings_logs_form_add()}
			</Button>
		</ClickSpark>
	</div>
</form>
