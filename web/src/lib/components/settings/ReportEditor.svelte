<script lang="ts">
	/**
	 * One periodic report: its form, its status line and its actions. `schedule`
	 * is `null` for a report not saved yet. The parent owns the list; this
	 * component reports back with `onsaved` and `ondeleted`.
	 */
	import { untrack } from 'svelte';
	import { Send } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { toApiError, type Channel, type StatusPage } from '#lib/api/index.js';
	import {
		MAX_REPORT_RECIPIENTS,
		createReportSchedule,
		deleteReportSchedule,
		sendReportPreview,
		updateReportSchedule,
		type ReportFrequency,
		type ReportKind,
		type ReportSchedule,
		type ReportSchedulePayload
	} from '#lib/api/reports.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { Button, Confirm, ErrorNotice, Field, Plate, Toggle } from '#lib/ui/index.js';
	import UnsavedBadge from './UnsavedBadge.svelte';
	import { useUnsavedGuard } from '#lib/settings/unsaved.svelte.js';

	interface Props {
		schedule: ReportSchedule | null;
		/** Enabled-or-not email (`smtp`) channels the report may use. */
		channels: Channel[];
		/** Timezone suggested for a new report. */
		defaultTimezone: string;
		timezones: string[];
		/** Status pages whose title can head an SLA report. */
		pages?: StatusPage[];
		onsaved: (saved: ReportSchedule) => void;
		ondeleted: (id: number) => void;
		oncancel?: () => void;
	}

	let { schedule, channels, defaultTimezone, timezones, pages = [], onsaved, ondeleted, oncancel }: Props = $props();

	// 2024-01-01 was a Monday: seven consecutive days give the localized weekday names.
	const WEEKDAYS = Array.from({ length: 7 }, (_, i) =>
		new Intl.DateTimeFormat(getLocale(), { weekday: 'long', timeZone: 'UTC' }).format(new Date(Date.UTC(2024, 0, 1 + i)))
	);
	const FREQUENCIES: { value: ReportFrequency; label: string }[] = [
		{ value: 'daily', label: m.settings_report_freq_daily() },
		{ value: 'weekly', label: m.settings_report_freq_weekly() },
		{ value: 'monthly', label: m.settings_report_freq_monthly() }
	];

	const KINDS: { value: ReportKind; label: string }[] = [
		{ value: 'digest', label: m.settings_report_kind_digest() },
		{ value: 'sla', label: m.settings_report_kind_sla() }
	];

	// The form starts from the saved report; it is a draft until saved.
	const initial = untrack(() => schedule);
	let name = $state(initial?.name ?? m.settings_report_default_name());
	let enabled = $state(initial?.enabled ?? true);
	let frequency = $state<ReportFrequency>(initial?.frequency ?? 'weekly');
	let weekday = $state(initial?.weekday ?? 0);
	let dayOfMonth = $state(initial?.day_of_month ?? 1);
	let hour = $state(initial?.hour ?? 8);
	let timezone = $state(initial?.timezone ?? untrack(() => defaultTimezone));
	let recipients = $state((initial?.recipients ?? []).join('\n'));
	let kind = $state<ReportKind>(initial?.kind ?? 'digest');
	let attachPdf = $state(initial?.attach_pdf ?? true);
	let attachCsv = $state(initial?.attach_csv ?? true);
	let objective = $state(initial?.objective === null || initial === null ? '' : String(initial.objective));
	let pageId = $state<string>(initial?.page_id === null || initial === null ? '' : String(initial.page_id));
	let objectiveError = $state<string | null>(null);
	let channelId = $state<string>(initial?.channel_id === null || initial === null ? '' : String(initial.channel_id));

	let saving = $state(false);
	let previewing = $state(false);
	let error = $state<unknown>(null);
	let previewError = $state<unknown>(null);
	let notice = $state<string | null>(null);
	let recipientsError = $state<string | null>(null);

	/** Addresses split on commas, spaces and line breaks; blanks dropped. */
	function parseRecipients(raw: string): string[] {
		return raw
			.split(/[\s,;]+/)
			.map((item) => item.trim())
			.filter(Boolean);
	}

	const parsed = $derived(parseRecipients(recipients));

	function payload(): ReportSchedulePayload {
		return {
			name: name.trim() || m.settings_report_default_name(),
			enabled,
			// An SLA report is the previous calendar month: always monthly.
			frequency: kind === 'sla' ? 'monthly' : frequency,
			weekday,
			day_of_month: dayOfMonth,
			hour,
			timezone: timezone.trim(),
			recipients: parsed,
			channel_id: channelId === '' ? null : Number(channelId),
			kind,
			attach_pdf: attachPdf,
			attach_csv: attachCsv,
			page_id: kind === 'sla' && pageId !== '' ? Number(pageId) : null,
			objective: kind === 'sla' && objective.trim() !== '' ? Number(objective.replace(',', '.')) : null
		};
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		notice = null;
		recipientsError = null;
		objectiveError = null;
		const goal = Number(objective.replace(',', '.'));
		if (kind === 'sla' && objective.trim() !== '' && !(goal > 0 && goal <= 100)) {
			objectiveError = m.settings_report_objective_invalid();
			return;
		}
		if (parsed.length > MAX_REPORT_RECIPIENTS) {
			recipientsError = m.settings_report_recipients_max({ max: MAX_REPORT_RECIPIENTS });
			return;
		}
		if (enabled && parsed.length === 0) {
			recipientsError = m.settings_report_recipients_required();
			return;
		}
		saving = true;
		try {
			const saved = schedule ? await updateReportSchedule(schedule.id, payload()) : await createReportSchedule(payload());
			notice = m.settings_report_saved();
			onsaved(saved);
		} catch (cause) {
			error = cause;
		} finally {
			saving = false;
		}
	}

	async function preview() {
		if (!schedule) return;
		previewError = null;
		notice = null;
		previewing = true;
		try {
			const result = await sendReportPreview(schedule.id);
			notice =
				result.failed > 0
					? m.settings_report_preview_failed({ sent: result.sent, failed: result.failed })
					: m.settings_report_preview_sent({ sent: result.sent });
		} catch (cause) {
			previewError = cause;
		} finally {
			previewing = false;
		}
	}

	async function remove() {
		if (!schedule) return;
		error = null;
		try {
			await deleteReportSchedule(schedule.id);
			ondeleted(schedule.id);
		} catch (cause) {
			error = cause;
		}
	}

	/** The saved settings differ from the form: a preview would use the old ones. */
	const dirty = $derived(
		schedule !== null &&
			(name !== schedule.name ||
				enabled !== schedule.enabled ||
				(kind === 'sla' ? 'monthly' : frequency) !== schedule.frequency ||
				kind !== schedule.kind ||
				attachPdf !== schedule.attach_pdf ||
				attachCsv !== schedule.attach_csv ||
				pageId !== (schedule.page_id === null ? '' : String(schedule.page_id)) ||
				objective.trim() !== (schedule.objective === null ? '' : String(schedule.objective)) ||
				weekday !== schedule.weekday ||
				dayOfMonth !== schedule.day_of_month ||
				hour !== schedule.hour ||
				timezone !== schedule.timezone ||
				parsed.join(',') !== schedule.recipients.join(',') ||
				channelId !== (schedule.channel_id === null ? '' : String(schedule.channel_id)))
	);

	// Edits to a saved report warn before leaving; a brand-new draft does not.
	useUnsavedGuard(() => dirty);

	const idPrefix = $derived(`report-${schedule?.id ?? 'new'}`);
</script>

<form class="grid gap-4 sm:grid-cols-2" onsubmit={save} novalidate>
	<Field label={m.settings_report_name()} for={`${idPrefix}-name`} help={m.settings_report_name_help()}>
		<input id={`${idPrefix}-name`} type="text" class="input" bind:value={name} maxlength="80" autocomplete="off" disabled={saving} />
	</Field>
	<Field label={m.settings_report_send()} inline>
		<Toggle bind:checked={enabled} label={m.settings_report_send()} disabled={saving} />
	</Field>

	<fieldset class="grid min-w-0 content-start gap-1.5 sm:col-span-2" disabled={saving}>
		<legend class="mb-1.5 block text-sm font-semibold text-ink">{m.settings_report_kind()}</legend>
		<div class="flex w-fit max-w-full flex-wrap gap-1 rounded-lg border border-line bg-canvas-deep p-1" role="radiogroup" aria-label={m.settings_report_kind()}>
			{#each KINDS as option (option.value)}
				<label
					class={`cursor-pointer rounded-md px-3 py-1.5 text-sm transition-colors has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-signal ${kind === option.value ? 'bg-surface font-semibold text-ink shadow-lift' : 'text-ink-2 hover:text-ink'}`}
				>
					<input type="radio" class="sr-only" name={`${idPrefix}-kind`} value={option.value} bind:group={kind} />
					{option.label}
				</label>
			{/each}
		</div>
		<p class="text-[0.8125rem] text-ink-2">{m.settings_report_kind_help()}</p>
	</fieldset>

	{#if kind === 'sla'}
		<Field label={m.settings_report_objective()} for={`${idPrefix}-objective`} error={objectiveError} help={m.settings_report_objective_help()}>
			<input id={`${idPrefix}-objective`} type="text" inputmode="decimal" class="input" bind:value={objective} placeholder="99.9" autocomplete="off" disabled={saving} aria-invalid={objectiveError ? 'true' : undefined} />
		</Field>
		<Field label={m.settings_report_brand()} for={`${idPrefix}-brand`} help={m.settings_report_brand_help()}>
			<select id={`${idPrefix}-brand`} class="input" bind:value={pageId} disabled={saving}>
				<option value="">{m.settings_report_brand_none()}</option>
				{#each pages as statusPage (statusPage.id)}
					<option value={String(statusPage.id)}>{statusPage.title}</option>
				{/each}
			</select>
		</Field>
		<Field label={m.settings_report_attach_pdf()} inline>
			<Toggle bind:checked={attachPdf} label={m.settings_report_attach_pdf()} disabled={saving} />
		</Field>
		<Field label={m.settings_report_attach_csv()} inline>
			<Toggle bind:checked={attachCsv} label={m.settings_report_attach_csv()} disabled={saving} />
		</Field>
	{/if}

	{#if kind === 'digest'}
	<fieldset class="grid min-w-0 gap-1.5" disabled={saving}>
		<legend class="mb-1.5 block text-sm font-semibold text-ink">{m.settings_report_frequency()}</legend>
		<div class="flex w-fit max-w-full flex-wrap gap-1 rounded-lg border border-line bg-canvas-deep p-1" role="radiogroup" aria-label={m.settings_report_frequency()}>
			{#each FREQUENCIES as option (option.value)}
				<label
					class={`cursor-pointer rounded-md px-3 py-1.5 text-sm transition-colors has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-signal ${frequency === option.value ? 'bg-surface font-semibold text-ink shadow-lift' : 'text-ink-2 hover:text-ink'}`}
				>
					<input type="radio" class="sr-only" name={`${idPrefix}-frequency`} value={option.value} bind:group={frequency} />
					{option.label}
				</label>
			{/each}
		</div>
	</fieldset>
	{/if}

	<div class="grid grid-cols-2 gap-3">
		{#if kind === 'sla' || frequency === 'monthly'}
			<Field label={m.settings_report_day_of_month()} for={`${idPrefix}-dom`} help={m.settings_report_day_of_month_help()}>
				<input id={`${idPrefix}-dom`} type="number" class="input" min="1" max="28" bind:value={dayOfMonth} disabled={saving} />
			</Field>
		{:else if frequency === 'weekly'}
			<Field label={m.settings_report_day()} for={`${idPrefix}-weekday`}>
				<select id={`${idPrefix}-weekday`} class="input" bind:value={weekday} disabled={saving}>
					{#each WEEKDAYS as label, index (label)}
						<option value={index}>{label}</option>
					{/each}
				</select>
			</Field>
		{/if}
		<Field label={m.settings_report_hour()} for={`${idPrefix}-hour`}>
			<select id={`${idPrefix}-hour`} class="input" bind:value={hour} disabled={saving}>
				{#each Array.from({ length: 24 }, (_, i) => i) as value (value)}
					<option {value}>{String(value).padStart(2, '0')}:00</option>
				{/each}
			</select>
		</Field>
	</div>

	<Field label={m.settings_report_timezone()} for={`${idPrefix}-tz`} help={m.settings_report_timezone_help()}>
		<input
			id={`${idPrefix}-tz`}
			type="text"
			class="input font-mono"
			list={`${idPrefix}-tz-list`}
			bind:value={timezone}
			autocomplete="off"
			spellcheck="false"
			disabled={saving}
		/>
		<datalist id={`${idPrefix}-tz-list`}>
			{#each timezones as zone (zone)}<option value={zone}></option>{/each}
		</datalist>
	</Field>

	<Field label={m.settings_report_channel()} for={`${idPrefix}-channel`} help={m.settings_report_channel_help()}>
		<select id={`${idPrefix}-channel`} class="input" bind:value={channelId} disabled={saving}>
			<option value="">{m.settings_report_channel_default()}</option>
			{#each channels as channel (channel.id)}
				<option value={String(channel.id)}>{channel.enabled ? channel.name : m.settings_report_channel_disabled({ name: channel.name })}</option>
			{/each}
		</select>
	</Field>

	<Field
		label={m.settings_report_recipients()}
		for={`${idPrefix}-recipients`}
		error={recipientsError}
		help={m.settings_report_recipients_help({ max: MAX_REPORT_RECIPIENTS, count: parsed.length })}
		class="sm:col-span-2"
	>
		<textarea
			id={`${idPrefix}-recipients`}
			class="input min-h-24 font-mono"
			bind:value={recipients}
			placeholder="ops@example.org"
			autocomplete="off"
			spellcheck="false"
			disabled={saving}
			aria-invalid={recipientsError ? 'true' : undefined}
			oninput={() => (recipientsError = null)}
		></textarea>
	</Field>

	<div class="flex flex-wrap items-center gap-2 sm:col-span-2">
		<Button type="submit" variant="primary" loading={saving}>{schedule ? m.settings_report_save() : m.settings_report_create()}</Button>
		{#if schedule}
			<Button variant="secondary" onclick={preview} loading={previewing} disabled={dirty || saving}>
				<Send class="size-4" aria-hidden="true" />
				{m.settings_report_send_preview()}
			</Button>
			<Confirm confirmLabel={m.settings_report_delete_confirm()} onconfirm={remove}>{m.settings_report_delete()}</Confirm>
		{:else if oncancel}
			<Button variant="ghost" onclick={oncancel}>{m.settings_report_cancel()}</Button>
		{/if}
		<UnsavedBadge {dirty} />
	</div>
</form>

<div aria-live="polite" class="mt-3 grid gap-2">
	{#if notice}<p class="text-sm text-ink">{notice}</p>{/if}
	{#if error}<ErrorNotice {error} title={m.settings_report_save_error()} />{/if}
	{#if previewError}<ErrorNotice error={toApiError(previewError)} title={m.settings_report_preview_error()} />{/if}
</div>

{#if schedule}
	<div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-ink-2">
		{#if !schedule.enabled}
			<Plate tone="ghost" label={m.settings_report_off()} />
		{:else if schedule.next_run_at}
			<time title={formatDateTime(schedule.next_run_at)}>{m.settings_report_next({ when: formatRelative(schedule.next_run_at) })}</time>
		{/if}
		{#if schedule.last_sent_at}
			<time title={formatDateTime(schedule.last_sent_at)}>{m.settings_report_last_sent({ when: formatRelative(schedule.last_sent_at) })}</time>
		{:else}
			<span>{m.settings_report_never_sent()}</span>
		{/if}
		{#if schedule.last_error}<Plate tone="warning" label={m.settings_report_last_failed({ error: schedule.last_error })} />{/if}
	</div>
{/if}
