<script lang="ts">
	/**
	 * Settings → My account → On-call hours: my timezone, my quiet hours, what I
	 * am on call for now and next, and a private iCal link for my calendar app.
	 * The rule: while I am on call I am always paged; when I am not, my quiet
	 * hours let only critical alerts through.
	 */
	import {
		createOnCallCalendar,
		getOnCallProfile,
		revokeOnCallCalendar,
		updateOnCallProfile,
		type OnCallProfile,
		type QuietHours
	} from '#lib/api/index.js';
	import { formatDateTime } from '#lib/format.js';
	import { Button, Confirm, CopyBlock, ErrorNotice, Field, Panel, Skeleton } from '#lib/ui/index.js';
	import QuietHoursEditor from '#lib/components/notifications/QuietHoursEditor.svelte';
	import { browserTimezone, timezoneList } from '#lib/components/oncall/helpers.js';
	import { m } from '#lib/paraglide/messages.js';

	let profile = $state<OnCallProfile | null>(null);
	let loadError = $state<unknown>(null);
	let actionError = $state<unknown>(null);
	let timezone = $state(browserTimezone());
	let quiet = $state<QuietHours | null>(null);
	let saving = $state(false);
	let saved = $state(false);
	let calendarUrl = $state<string | null>(null);
	let calendarBusy = $state(false);

	function adopt(next: OnCallProfile) {
		profile = next;
		timezone = next.timezone || browserTimezone();
		quiet = next.quiet_hours;
	}

	async function load(signal?: AbortSignal) {
		loadError = null;
		try {
			adopt(await getOnCallProfile(signal));
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			loadError = cause;
		}
	}
	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	const zones = $derived(timezoneList(timezone));
	const dirty = $derived(
		profile !== null && (timezone !== profile.timezone || JSON.stringify(quiet) !== JSON.stringify(profile.quiet_hours))
	);

	async function save() {
		saving = true;
		saved = false;
		actionError = null;
		try {
			adopt(await updateOnCallProfile({ timezone, quiet_hours: quiet ? { ...quiet, timezone } : null }));
			saved = true;
		} catch (cause) {
			actionError = cause;
		} finally {
			saving = false;
		}
	}

	const absolute = (url: string) => (url.startsWith('/') ? `${window.location.origin}${url}` : url);

	async function makeCalendar() {
		calendarBusy = true;
		actionError = null;
		try {
			calendarUrl = absolute((await createOnCallCalendar()).url);
			if (profile) profile = { ...profile, calendar: true };
		} catch (cause) {
			actionError = cause;
		} finally {
			calendarBusy = false;
		}
	}

	async function revokeCalendar() {
		calendarBusy = true;
		actionError = null;
		try {
			await revokeOnCallCalendar();
			calendarUrl = null;
			if (profile) profile = { ...profile, calendar: false };
		} catch (cause) {
			actionError = cause;
		} finally {
			calendarBusy = false;
		}
	}
</script>

<Panel id="oncall-hours" title={m.oncall_hours_title()} description={m.oncall_hours_description()}>
	{#if loadError}
		<ErrorNotice error={loadError} title={m.oncall_hours_load_error()} onretry={() => void load()} />
	{:else if !profile}
		<Skeleton class="h-12 w-full" rows={2} />
	{:else}
		<div class="grid gap-5">
			<!-- What I am on call for. -->
			<div class="rounded-lg border border-line bg-canvas-deep px-4 py-3 text-sm">
				{#if profile.on_call.length > 0}
					<p class="font-semibold text-ink">{m.oncall_hours_on_call_now()}</p>
					<ul class="mt-1 grid gap-0.5 text-ink-2">
						{#each profile.on_call as entry (entry.schedule_id)}
							<li class="tnum">
								{#if entry.until}{m.oncall_hours_on_call_until({ schedule: entry.schedule_name, when: formatDateTime(entry.until) })}{:else}{entry.schedule_name}{/if}
							</li>
						{/each}
					</ul>
				{:else}
					<p class="font-semibold text-ink">{m.oncall_hours_not_on_call()}</p>
				{/if}
				{#if profile.next_shift}
					<p class="tnum mt-1.5 text-ink-2">
						{m.oncall_hours_next({ schedule: profile.next_shift.schedule_name, from: formatDateTime(profile.next_shift.start), to: formatDateTime(profile.next_shift.end) })}
					</p>
				{:else if profile.on_call.length === 0}
					<p class="mt-1 text-ink-2">{m.oncall_hours_no_next()}</p>
				{/if}
			</div>

			<Field label={m.oncall_hours_timezone()} for="oncall-tz" help={m.oncall_hours_timezone_help()} class="sm:max-w-sm">
				<select id="oncall-tz" class="input" bind:value={timezone}>
					{#each zones as zone (zone)}<option value={zone}>{zone}</option>{/each}
				</select>
			</Field>

			<div class="grid gap-2">
				{#key profile.quiet_hours === null}
					<QuietHoursEditor idPrefix="oncall-quiet" value={quiet} onchange={(next) => (quiet = next)} />
				{/key}
				<p class="text-[0.8125rem] text-ink-2">{m.oncall_hours_rule()}</p>
			</div>

			<div class="flex flex-wrap items-center gap-3">
				<Button variant="primary" loading={saving} disabled={!dirty} onclick={() => void save()}>{m.oncall_save()}</Button>
				{#if saved && !dirty}<span class="text-[0.8125rem] font-medium text-signal-ink" role="status">{m.oncall_saved()}</span>{/if}
			</div>

			<!-- Calendar link. -->
			<div class="grid gap-2 border-t border-line pt-4">
				<h3 class="text-sm font-semibold text-ink">{m.oncall_ical_title()}</h3>
				<p class="text-[0.8125rem] text-ink-2">{m.oncall_ical_help()}</p>
				{#if calendarUrl}
					<CopyBlock value={calendarUrl} label={m.oncall_ical_copy()} />
					<p class="text-[0.8125rem] text-ink-2">{m.oncall_ical_shown_once()}</p>
				{:else if profile.calendar}
					<p class="text-[0.8125rem] text-ink-2">{m.oncall_ical_exists()}</p>
				{/if}
				<div class="flex flex-wrap gap-2">
					{#if profile.calendar}
						<Confirm variant="secondary" size="md" confirmLabel={m.oncall_ical_regenerate_confirm()} loading={calendarBusy} onconfirm={makeCalendar}>{m.oncall_ical_regenerate()}</Confirm>
						<Confirm confirmLabel={m.oncall_ical_revoke_confirm()} size="md" loading={calendarBusy} onconfirm={revokeCalendar}>{m.oncall_ical_revoke()}</Confirm>
					{:else}
						<Button variant="secondary" loading={calendarBusy} onclick={() => void makeCalendar()}>{m.oncall_ical_create()}</Button>
					{/if}
				</div>
			</div>

			{#if actionError}<ErrorNotice error={actionError} title={m.oncall_hours_error()} />{/if}
		</div>
	{/if}
</Panel>
