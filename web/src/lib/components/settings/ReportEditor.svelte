<script lang="ts">
	/**
	 * One periodic report: its form, its status line and its actions. `schedule`
	 * is `null` for a report not saved yet. The parent owns the list; this
	 * component reports back with `onsaved` and `ondeleted`.
	 */
	import { untrack } from 'svelte';
	import { Send } from 'lucide-svelte';
	import { toApiError, type Channel } from '$lib/api';
	import {
		MAX_REPORT_RECIPIENTS,
		createReportSchedule,
		deleteReportSchedule,
		sendReportPreview,
		updateReportSchedule,
		type ReportFrequency,
		type ReportSchedule,
		type ReportSchedulePayload
	} from '$lib/api/reports';
	import { formatDateTime, formatRelative } from '$lib/format';
	import { Button, Confirm, ErrorNotice, Field, Plate, Toggle } from '$lib/ui';

	interface Props {
		schedule: ReportSchedule | null;
		/** Enabled-or-not email (`smtp`) channels the report may use. */
		channels: Channel[];
		/** Timezone suggested for a new report. */
		defaultTimezone: string;
		timezones: string[];
		onsaved: (saved: ReportSchedule) => void;
		ondeleted: (id: number) => void;
		oncancel?: () => void;
	}

	let { schedule, channels, defaultTimezone, timezones, onsaved, ondeleted, oncancel }: Props = $props();

	const WEEKDAYS = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
	const FREQUENCIES: { value: ReportFrequency; label: string }[] = [
		{ value: 'daily', label: 'Daily' },
		{ value: 'weekly', label: 'Weekly' },
		{ value: 'monthly', label: 'Monthly' }
	];

	// The form starts from the saved report; it is a draft until saved.
	const initial = untrack(() => schedule);
	let name = $state(initial?.name ?? 'Team report');
	let enabled = $state(initial?.enabled ?? true);
	let frequency = $state<ReportFrequency>(initial?.frequency ?? 'weekly');
	let weekday = $state(initial?.weekday ?? 0);
	let dayOfMonth = $state(initial?.day_of_month ?? 1);
	let hour = $state(initial?.hour ?? 8);
	let timezone = $state(initial?.timezone ?? untrack(() => defaultTimezone));
	let recipients = $state((initial?.recipients ?? []).join('\n'));
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
			name: name.trim() || 'Team report',
			enabled,
			frequency,
			weekday,
			day_of_month: dayOfMonth,
			hour,
			timezone: timezone.trim(),
			recipients: parsed,
			channel_id: channelId === '' ? null : Number(channelId)
		};
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		notice = null;
		recipientsError = null;
		if (parsed.length > MAX_REPORT_RECIPIENTS) {
			recipientsError = `At most ${MAX_REPORT_RECIPIENTS} recipients.`;
			return;
		}
		if (enabled && parsed.length === 0) {
			recipientsError = 'Add at least one address before turning the report on.';
			return;
		}
		saving = true;
		try {
			const saved = schedule ? await updateReportSchedule(schedule.id, payload()) : await createReportSchedule(payload());
			notice = 'Saved.';
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
					? `Preview sent to ${result.sent}, failed for ${result.failed}.`
					: `Preview sent to ${result.sent} ${result.sent === 1 ? 'recipient' : 'recipients'}.`;
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
				frequency !== schedule.frequency ||
				weekday !== schedule.weekday ||
				dayOfMonth !== schedule.day_of_month ||
				hour !== schedule.hour ||
				timezone !== schedule.timezone ||
				parsed.join(',') !== schedule.recipients.join(',') ||
				channelId !== (schedule.channel_id === null ? '' : String(schedule.channel_id)))
	);

	const idPrefix = $derived(`report-${schedule?.id ?? 'new'}`);
</script>

<form class="grid gap-4 sm:grid-cols-2" onsubmit={save} novalidate>
	<Field label="Name" for={`${idPrefix}-name`} help="Only shown in this list.">
		<input id={`${idPrefix}-name`} type="text" class="input" bind:value={name} maxlength="80" autocomplete="off" disabled={saving} />
	</Field>
	<Field label="Send reports" inline>
		<Toggle bind:checked={enabled} label="Send reports" disabled={saving} />
	</Field>

	<fieldset class="grid min-w-0 gap-1.5" disabled={saving}>
		<legend class="mb-1.5 block text-sm font-semibold text-ink">Frequency</legend>
		<div class="flex w-fit max-w-full flex-wrap gap-1 rounded-lg border border-line bg-canvas-deep p-1" role="radiogroup" aria-label="Frequency">
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

	<div class="grid grid-cols-2 gap-3">
		{#if frequency === 'weekly'}
			<Field label="Day" for={`${idPrefix}-weekday`}>
				<select id={`${idPrefix}-weekday`} class="input" bind:value={weekday} disabled={saving}>
					{#each WEEKDAYS as label, index (label)}
						<option value={index}>{label}</option>
					{/each}
				</select>
			</Field>
		{:else if frequency === 'monthly'}
			<Field label="Day of the month" for={`${idPrefix}-dom`} help="1 to 28.">
				<input id={`${idPrefix}-dom`} type="number" class="input" min="1" max="28" bind:value={dayOfMonth} disabled={saving} />
			</Field>
		{/if}
		<Field label="Hour" for={`${idPrefix}-hour`}>
			<select id={`${idPrefix}-hour`} class="input" bind:value={hour} disabled={saving}>
				{#each Array.from({ length: 24 }, (_, i) => i) as value (value)}
					<option {value}>{String(value).padStart(2, '0')}:00</option>
				{/each}
			</select>
		</Field>
	</div>

	<Field label="Timezone" for={`${idPrefix}-tz`} help="An IANA name such as Europe/Paris. Daylight saving time is followed.">
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

	<Field label="Email channel" for={`${idPrefix}-channel`} help="Only its mail server is used; its own recipients are ignored.">
		<select id={`${idPrefix}-channel`} class="input" bind:value={channelId} disabled={saving}>
			<option value="">First enabled email channel</option>
			{#each channels as channel (channel.id)}
				<option value={String(channel.id)}>{channel.name}{channel.enabled ? '' : ' (disabled)'}</option>
			{/each}
		</select>
	</Field>

	<Field
		label="Recipients"
		for={`${idPrefix}-recipients`}
		error={recipientsError}
		help={`One per line, or separated by commas. Up to ${MAX_REPORT_RECIPIENTS}; they do not have to be DumbMonit users. ${parsed.length} entered.`}
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
		<Button type="submit" variant="primary" loading={saving}>{schedule ? 'Save' : 'Create report'}</Button>
		{#if schedule}
			<Button variant="secondary" onclick={preview} loading={previewing} disabled={dirty || saving}>
				<Send class="size-4" aria-hidden="true" />
				Send a preview
			</Button>
			<Confirm confirmLabel="Delete for good?" onconfirm={remove}>Delete</Confirm>
		{:else if oncancel}
			<Button variant="ghost" onclick={oncancel}>Cancel</Button>
		{/if}
		{#if dirty}<span class="text-sm text-ink-2">Unsaved changes: save before sending a preview.</span>{/if}
	</div>
</form>

<div aria-live="polite" class="mt-3 grid gap-2">
	{#if notice}<p class="text-sm text-ink">{notice}</p>{/if}
	{#if error}<ErrorNotice {error} title="Could not save the report" />{/if}
	{#if previewError}<ErrorNotice error={toApiError(previewError)} title="Could not send the preview" />{/if}
</div>

{#if schedule}
	<div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-ink-2">
		{#if !schedule.enabled}
			<Plate tone="ghost" label="Off" />
		{:else if schedule.next_run_at}
			<span>Next report <time title={formatDateTime(schedule.next_run_at)}>{formatRelative(schedule.next_run_at)}</time></span>
		{/if}
		{#if schedule.last_sent_at}
			<span>Last sent <time title={formatDateTime(schedule.last_sent_at)}>{formatRelative(schedule.last_sent_at)}</time></span>
		{:else}
			<span>Never sent</span>
		{/if}
		{#if schedule.last_error}<Plate tone="warning" label={`Last send failed: ${schedule.last_error}`} />{/if}
	</div>
{/if}
