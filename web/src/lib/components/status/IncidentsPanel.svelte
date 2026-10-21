<script lang="ts">
	/**
	 * Incidents and maintenance windows, from the settings section. Open
	 * announcements first, with an inline "Post update" form (status + message)
	 * and a one-click close; closed ones fold under "Past". A new announcement
	 * is created inline too: title, kind, severity or window, page, first
	 * message.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { Megaphone, Plus, Send, X } from 'lucide-svelte';
	import {
		addIncidentUpdate,
		createIncident,
		deleteIncident,
		type Incident,
		type IncidentKind,
		type IncidentSeverity,
		type IncidentStatus,
		type StatusPage
	} from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Field, Plate } from '#lib/ui/index.js';
	import { INCIDENT_STATUS, KIND_LABEL, STATUSES_FOR, isClosed } from './words';
	import { dateToLocal, localToIso, updatesPayload, validateHistory, type DraftUpdate, type HistoryError } from './incident-history';

	interface Props {
		incidents: Incident[];
		pages: StatusPage[];
		onchange: (incidents: Incident[]) => void;
	}

	let { incidents, pages, onchange }: Props = $props();

	const open = $derived(incidents.filter((i) => !isClosed(i.status)));
	const past = $derived(incidents.filter((i) => isClosed(i.status)));
	const pageTitle = $derived(new Map(pages.map((p) => [p.id, p.title])));

	function scope(incident: Incident): string {
		if (incident.page_id === null) return m.status_incidents_all_pages();
		return pageTitle.get(incident.page_id) ?? m.status_incidents_deleted_page();
	}

	// --- New announcement --------------------------------------------------------

	let creating = $state(false);
	let title = $state('');
	let kind = $state<IncidentKind>('incident');
	let severity = $state<IncidentSeverity>('minor');
	let pageId = $state<string>('');
	let startsAt = $state('');
	let endsAt = $state('');
	let body = $state('');
	// Past incident: dated messages, typed in the browser's local time.
	let history = $state<DraftUpdate[]>([]);
	let historyError = $state<HistoryError | null>(null);
	// Datetime inputs cannot reach into the future for an incident.
	const nowLocal = $derived(dateToLocal(new Date()));
	let titleError = $state<string | null>(null);
	let windowError = $state<string | null>(null);
	let saving = $state(false);
	let createError = $state<unknown>(null);

	/** `datetime-local` value → RFC 3339 in UTC, as the API expects. */
	function toIso(local: string): string | undefined {
		if (!local) return undefined;
		const date = new Date(local);
		return Number.isNaN(date.getTime()) ? undefined : date.toISOString();
	}

	function addHistoryRow() {
		const last = history[history.length - 1];
		history = [...history, { status: last?.status ?? 'investigating', at: last?.at ?? startsAt, body: '' }];
		historyError = null;
	}

	function historyMessage(error: HistoryError): string {
		return {
			future_start: m.status_incidents_err_future_start(),
			future_end: m.status_incidents_err_future_end(),
			end_before_start: m.status_incidents_err_end_before_start(),
			update_empty: m.status_incidents_err_update_empty(),
			update_outside: m.status_incidents_err_update_outside(),
			update_future: m.status_incidents_err_update_future(),
			after_resolution: m.status_incidents_err_after_resolution()
		}[error];
	}

	function resetForm() {
		title = '';
		kind = 'incident';
		severity = 'minor';
		pageId = '';
		startsAt = '';
		endsAt = '';
		body = '';
		history = [];
		historyError = null;
		titleError = null;
		windowError = null;
		createError = null;
	}

	async function create(event: SubmitEvent) {
		event.preventDefault();
		createError = null;
		titleError = title.trim() ? null : m.status_incidents_title_required();
		windowError = null;
		if (kind === 'maintenance') {
			if (!startsAt || !endsAt) windowError = m.status_incidents_window_required();
			else if (new Date(endsAt) <= new Date(startsAt)) windowError = m.status_incidents_window_order();
		}
		historyError = kind === 'incident' ? validateHistory(startsAt, endsAt, history) : null;
		if (titleError || windowError || historyError) return;
		saving = true;
		try {
			const created = await createIncident({
				title: title.trim(),
				kind,
				severity: kind === 'incident' ? severity : undefined,
				page_id: pageId ? Number(pageId) : null,
				starts_at: toIso(startsAt),
				ends_at: toIso(endsAt),
				body: body.trim() || undefined,
				updates: kind === 'incident' && history.length > 0 ? updatesPayload(history) : undefined
			});
			onchange([created, ...incidents]);
			resetForm();
			creating = false;
		} catch (cause) {
			createError = cause;
		} finally {
			saving = false;
		}
	}

	// --- Updates -----------------------------------------------------------------

	let updating = $state<number | null>(null);
	let updateStatus = $state<IncidentStatus>('investigating');
	let updateBody = $state('');
	let updateAt = $state('');
	let updateError = $state<unknown>(null);
	let posting = $state(false);

	function startUpdate(incident: Incident) {
		updating = incident.id;
		updateStatus = incident.status;
		updateBody = '';
		updateAt = '';
		updateError = null;
	}

	function replace(next: Incident) {
		onchange(incidents.map((i) => (i.id === next.id ? next : i)));
	}

	async function postUpdate(event: SubmitEvent, incident: Incident) {
		event.preventDefault();
		if (!updateBody.trim()) {
			updateError = new Error(m.status_incidents_message_required());
			return;
		}
		posting = true;
		updateError = null;
		try {
			replace(await addIncidentUpdate(incident.id, { status: updateStatus, body: updateBody.trim(), at: localToIso(updateAt) }));
			updating = null;
			updateBody = '';
			updateAt = '';
		} catch (cause) {
			updateError = cause;
		} finally {
			posting = false;
		}
	}

	let closing = $state<number | null>(null);
	let closeError = $state<{ id: number; cause: unknown } | null>(null);

	async function close(incident: Incident) {
		closing = incident.id;
		closeError = null;
		const status: IncidentStatus = incident.kind === 'maintenance' ? 'completed' : 'resolved';
		const message = incident.kind === 'maintenance' ? m.status_incidents_maintenance_completed() : m.status_incidents_incident_resolved();
		try {
			replace(await addIncidentUpdate(incident.id, { status, body: message }));
		} catch (cause) {
			closeError = { id: incident.id, cause };
		} finally {
			closing = null;
		}
	}

	let deleting = $state<number | null>(null);
	async function remove(incident: Incident) {
		deleting = incident.id;
		try {
			await deleteIncident(incident.id);
			onchange(incidents.filter((i) => i.id !== incident.id));
		} catch (cause) {
			closeError = { id: incident.id, cause };
		} finally {
			deleting = null;
		}
	}
</script>

{#snippet row(incident: Incident)}
	{@const latest = incident.updates[incident.updates.length - 1]}
	{@const status = INCIDENT_STATUS[incident.status]}
	{@const closed = isClosed(incident.status)}
	<li class={`px-4 py-3 ${closed ? 'bg-canvas-deep/40' : ''}`}>
		<div class="flex flex-wrap items-start gap-x-3 gap-y-2">
			<div class="min-w-0 flex-[1_1_14rem]">
				<div class="flex flex-wrap items-center gap-2">
					<Plate tone={incident.kind === 'maintenance' ? 'info' : closed ? 'ghost' : incident.severity === 'major' ? 'warning' : 'advisory'} label={KIND_LABEL[incident.kind]} bare />
					<Plate tone={status.tone} label={status.label} />
					<span class="font-semibold text-ink">{incident.title}</span>
				</div>
				<p class="mt-1 text-[0.8125rem] text-ink-2">
					{scope(incident)}
					· <time class="tnum" title={formatDateTime(incident.starts_at)}>{incident.kind === 'maintenance' ? formatDateTime(incident.starts_at) : formatRelative(incident.starts_at)}</time>
					{#if incident.ends_at}
						→ <time class="tnum" title={formatDateTime(incident.ends_at)}>{incident.kind === 'maintenance' && !closed ? formatDateTime(incident.ends_at) : formatRelative(incident.ends_at)}</time>
					{/if}
					{#if latest}
						· “{latest.body.length > 90 ? `${latest.body.slice(0, 90)}…` : latest.body}”
					{/if}
				</p>
			</div>
			<div class="flex flex-wrap items-center gap-1.5">
				{#if !closed}
					<Button variant="secondary" size="sm" onclick={() => (updating === incident.id ? (updating = null) : startUpdate(incident))}>
						{updating === incident.id ? m.status_incidents_cancel() : m.status_incidents_post_update()}
					</Button>
					<Button variant="ghost" size="sm" loading={closing === incident.id} onclick={() => close(incident)}>
						{incident.kind === 'maintenance' ? m.status_incidents_complete() : m.status_incidents_resolve()}
					</Button>
				{:else}
					<Confirm confirmLabel={m.status_incidents_delete_confirm()} loading={deleting === incident.id} onconfirm={() => remove(incident)}>{m.status_incidents_delete()}</Confirm>
				{/if}
			</div>
		</div>
		{#if closeError?.id === incident.id}
			<ErrorNotice error={closeError.cause} title={m.status_incidents_update_error()} class="mt-3" />
		{/if}
		{#if updating === incident.id}
			<form class="mt-3 grid gap-3 rounded-lg border border-line bg-surface p-3 sm:grid-cols-[10rem_minmax(0,1fr)_auto] sm:items-start" onsubmit={(event) => postUpdate(event, incident)} novalidate>
				<Field label={m.status_incidents_status()} for="inc-{incident.id}-status">
					<select id="inc-{incident.id}-status" class="input" bind:value={updateStatus} disabled={posting}>
						{#each STATUSES_FOR[incident.kind] as choice (choice)}
							<option value={choice}>{INCIDENT_STATUS[choice].label}</option>
						{/each}
					</select>
				</Field>
				<Field label={m.status_incidents_message()} for="inc-{incident.id}-body" required>
					<textarea id="inc-{incident.id}-body" class="input min-h-10" rows="2" bind:value={updateBody} placeholder={m.status_incidents_message_placeholder()} maxlength="4000" disabled={posting}></textarea>
				</Field>
				<Button type="submit" variant="secondary" class="sm:mt-[1.625rem]" loading={posting}>
					<Send class="size-4" aria-hidden="true" />
					{m.status_incidents_post()}
				</Button>
				<div class="sm:col-span-3 sm:max-w-xs">
					<Field label={m.status_incidents_update_when()} for="inc-{incident.id}-at" help={m.status_incidents_update_when_help()}>
						<input id="inc-{incident.id}-at" type="datetime-local" class="input tnum" bind:value={updateAt} min={dateToLocal(new Date(incident.starts_at.endsWith('Z') ? incident.starts_at : `${incident.starts_at}Z`))} max={nowLocal} disabled={posting} />
					</Field>
				</div>
				{#if updateError}
					<div class="sm:col-span-3"><ErrorNotice error={updateError} title={m.status_incidents_post_error()} /></div>
				{/if}
			</form>
		{/if}
	</li>
{/snippet}

<div class="grid gap-4">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<div>
			<p class="text-sm font-semibold text-ink">{m.status_incidents_heading()}</p>
			<p class="text-[0.8125rem] text-ink-2">{m.status_incidents_lead()}</p>
		</div>
		<Button variant="secondary" size="sm" onclick={() => {
			creating = !creating;
			if (!creating) resetForm();
		}}>
			<Megaphone class="size-4" aria-hidden="true" />
			{creating ? m.status_incidents_cancel() : m.status_incidents_new()}
		</Button>
	</div>

	{#if creating}
		<form class="grid gap-3 rounded-[var(--radius-card)] border border-line bg-canvas-deep/40 p-4" onsubmit={create} novalidate>
			<div class="grid gap-3 sm:grid-cols-[minmax(0,1fr)_10rem]">
				<Field label={m.status_incidents_field_title()} for="inc-new-title" error={titleError} required>
					<input id="inc-new-title" type="text" class="input" bind:value={title} oninput={() => (titleError = null)} placeholder={m.status_incidents_title_placeholder()} maxlength="160" disabled={saving} aria-invalid={titleError ? 'true' : undefined} />
				</Field>
				<Field label={m.status_incidents_kind()} for="inc-new-kind">
					<select id="inc-new-kind" class="input" bind:value={kind} disabled={saving}>
						<option value="incident">{m.status_words_kind_incident()}</option>
						<option value="maintenance">{m.status_words_kind_maintenance()}</option>
					</select>
				</Field>
			</div>
			<div class="grid gap-3 sm:grid-cols-2">
				{#if kind === 'incident'}
					<Field label={m.status_incidents_impact()} for="inc-new-severity">
						<select id="inc-new-severity" class="input" bind:value={severity} disabled={saving}>
							<option value="minor">{m.status_incidents_impact_minor()}</option>
							<option value="major">{m.status_incidents_impact_major()}</option>
						</select>
					</Field>
					<Field label={m.status_incidents_started()} for="inc-new-started" help={m.status_incidents_started_help()}>
						<input id="inc-new-started" type="datetime-local" class="input tnum" bind:value={startsAt} oninput={() => (historyError = null)} max={nowLocal} disabled={saving} />
					</Field>
					<Field label={m.status_incidents_resolved_at()} for="inc-new-resolved" help={m.status_incidents_resolved_help()}>
						<input id="inc-new-resolved" type="datetime-local" class="input tnum" bind:value={endsAt} oninput={() => (historyError = null)} max={nowLocal} min={startsAt || undefined} disabled={saving} />
					</Field>
				{:else}
					<Field label={m.status_incidents_starts()} for="inc-new-starts" error={windowError} required>
						<input id="inc-new-starts" type="datetime-local" class="input tnum" bind:value={startsAt} oninput={() => (windowError = null)} disabled={saving} />
					</Field>
					<Field label={m.status_incidents_ends()} for="inc-new-ends" required>
						<input id="inc-new-ends" type="datetime-local" class="input tnum" bind:value={endsAt} oninput={() => (windowError = null)} disabled={saving} />
					</Field>
				{/if}
				<Field label={m.status_incidents_shown_on()} for="inc-new-page">
					<select id="inc-new-page" class="input" bind:value={pageId} disabled={saving}>
						<option value="">{m.status_incidents_all_pages()}</option>
						{#each pages as p (p.id)}
							<option value={String(p.id)}>{p.title}</option>
						{/each}
					</select>
				</Field>
			</div>
			<Field label={m.status_incidents_first_message()} for="inc-new-body" help={m.status_incidents_first_message_help()}>
				<textarea id="inc-new-body" class="input min-h-10" rows="2" bind:value={body} placeholder={kind === 'incident' ? m.status_incidents_body_placeholder_incident() : m.status_incidents_body_placeholder_maintenance()} maxlength="4000" disabled={saving}></textarea>
			</Field>
			{#if kind === 'incident'}
				<div class="grid gap-3">
					<div class="flex flex-wrap items-center justify-between gap-2">
						<div>
							<p class="text-sm font-semibold text-ink">{m.status_incidents_history_title()}</p>
							<p class="text-[0.8125rem] text-ink-2">{m.status_incidents_history_help()}</p>
						</div>
						<Button type="button" variant="ghost" size="sm" onclick={addHistoryRow} disabled={saving}>
							<Plus class="size-4" aria-hidden="true" />
							{m.status_incidents_history_add()}
						</Button>
					</div>
					{#each history as row, index (index)}
						<div class="grid gap-2 rounded-lg border border-line bg-surface p-3 sm:grid-cols-[9rem_11rem_minmax(0,1fr)_auto] sm:items-start">
							<Field label={m.status_incidents_status()} for="inc-new-h{index}-status">
								<select id="inc-new-h{index}-status" class="input" bind:value={row.status} disabled={saving}>
									{#each STATUSES_FOR.incident as choice (choice)}
										<option value={choice}>{INCIDENT_STATUS[choice].label}</option>
									{/each}
								</select>
							</Field>
							<Field label={m.status_incidents_update_date()} for="inc-new-h{index}-at">
								<input id="inc-new-h{index}-at" type="datetime-local" class="input tnum" bind:value={row.at} oninput={() => (historyError = null)} max={nowLocal} disabled={saving} />
							</Field>
							<Field label={m.status_incidents_message()} for="inc-new-h{index}-body">
								<textarea id="inc-new-h{index}-body" class="input min-h-10" rows="2" bind:value={row.body} oninput={() => (historyError = null)} maxlength="4000" disabled={saving}></textarea>
							</Field>
							<button type="button" class="inline-flex size-10 items-center justify-center rounded-md text-ink-2 hover:bg-surface-2 hover:text-ink sm:mt-[1.625rem]" aria-label={m.status_incidents_history_remove_aria({ number: index + 1 })} onclick={() => (history = history.filter((_, i) => i !== index))} disabled={saving}>
								<X class="size-4" aria-hidden="true" />
							</button>
						</div>
					{/each}
					{#if historyError}
						<p class="text-[0.8125rem] text-warning" role="alert">{historyMessage(historyError)}</p>
					{/if}
				</div>
			{/if}
			{#if createError}
				<ErrorNotice error={createError} title={m.status_incidents_create_error()} />
			{/if}
			<div>
				<Button type="submit" variant="secondary" loading={saving}>
					<Megaphone class="size-4" aria-hidden="true" />
					{kind === 'incident' ? m.status_incidents_open_incident() : m.status_incidents_schedule_maintenance()}
				</Button>
			</div>
		</form>
	{/if}

	{#if incidents.length === 0}
		<EmptyState icon={Megaphone} title={m.status_incidents_empty_title()} description={m.status_incidents_empty_description()} tone="signal" />
	{:else}
		{#if open.length > 0}
			<ul class="divide-y divide-line rounded-[var(--radius-card)] border border-line" role="list" aria-label={m.status_incidents_open_aria()}>
				{#each open as incident (incident.id)}
					{@render row(incident)}
				{/each}
			</ul>
		{:else}
			<p class="text-sm text-ink-2">{m.status_incidents_nothing_open()}</p>
		{/if}
		{#if past.length > 0}
			<details class="group">
				<summary class="cursor-pointer text-sm font-semibold text-ink-2 hover:text-ink">{m.status_incidents_past({ count: past.length })}</summary>
				<ul class="mt-2 divide-y divide-line rounded-[var(--radius-card)] border border-line" role="list" aria-label={m.status_incidents_past_aria()}>
					{#each past as incident (incident.id)}
						{@render row(incident)}
					{/each}
				</ul>
			</details>
		{/if}
	{/if}
</div>
