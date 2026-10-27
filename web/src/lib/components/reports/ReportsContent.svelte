<script lang="ts">
	/**
	 * The Reports page: the availability (SLA) export of a period on demand,
	 * the scheduled reports (periodic emails for the team — mostly one, weekly
	 * by default, though a few may coexist, for instance a daily one for the
	 * on-call and a monthly one for management) and the history of their
	 * sends. Settings used to hold the last two; `/settings/reports` forwards
	 * here.
	 */
	import ClientAssign from '#lib/components/ClientAssign.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { Info, Mail, Plus } from 'lucide-svelte';
	import { listChannels, listStatusPages, type Channel, type StatusPage } from '#lib/api/index.js';
	import { listReportSchedules, type ReportSchedule } from '#lib/api/reports.js';
	import { Button, EmptyState, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import ReportEditor from '#lib/components/settings/ReportEditor.svelte';
	import SlaDownload from '#lib/components/settings/SlaDownload.svelte';

	const DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/using/reports/';

	let schedules = $state<ReportSchedule[]>([]);
	let channels = $state<Channel[]>([]);
	let pages = $state<StatusPage[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let adding = $state(false);
	let showInfo = $state(false);

	const emailChannels = $derived(channels.filter((channel) => channel.kind === 'smtp'));

	/** The browser's timezone: the best guess for where the team reads its mail. */
	const defaultTimezone = (() => {
		try {
			return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
		} catch {
			return 'UTC';
		}
	})();

	const timezones = (() => {
		try {
			const list = (Intl as unknown as { supportedValuesOf?: (key: string) => string[] }).supportedValuesOf?.('timeZone');
			return list ? ['UTC', ...list] : ['UTC'];
		} catch {
			return ['UTC'];
		}
	})();

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const [list, all, statusPages] = await Promise.all([
				listReportSchedules(signal),
				listChannels(signal),
				listStatusPages(signal)
			]);
			schedules = Array.isArray(list) ? list : [];
			channels = Array.isArray(all) ? all : [];
			pages = Array.isArray(statusPages) ? statusPages : [];
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

	function saved(item: ReportSchedule) {
		schedules = schedules.some((s) => s.id === item.id) ? schedules.map((s) => (s.id === item.id ? item : s)) : [...schedules, item];
		adding = false;
	}

	function removed(id: number) {
		schedules = schedules.filter((s) => s.id !== id);
	}
</script>

<div class="grid gap-6 [&_section[id]]:scroll-mt-20">
<Panel id="reports-sla" title={m.settings_sla_title()} description={m.settings_sla_description()}>
	{#if error}
		<ErrorNotice {error} title={m.settings_reports_load_error()} onretry={() => void load()} />
	{:else if loading}
		<Skeleton class="h-40 w-full" />
	{:else}
		<SlaDownload {pages} {defaultTimezone} {timezones} />
	{/if}
</Panel>

<Panel
	id="reports"
	title={m.settings_reports_title()}
	description={m.settings_reports_description()}
>
	{#snippet aside()}
		<button
			type="button"
			class="inline-flex size-8 items-center justify-center rounded-full border border-line text-ink-2 hover:text-ink focus-visible:outline-2 focus-visible:outline-signal"
			aria-label={m.settings_reports_info_label()}
			aria-expanded={showInfo}
			aria-controls="reports-info"
			onclick={() => (showInfo = !showInfo)}
		>
			<Info class="size-4" aria-hidden="true" />
		</button>
	{/snippet}

	{#if showInfo}
		<div id="reports-info" class="rise-in mb-4 rounded-[var(--radius-card)] border border-line bg-canvas-deep p-4 text-sm text-ink-2">
			<p class="font-semibold text-ink">{m.settings_reports_info_title()}</p>
			<ul class="mt-2 list-disc space-y-1 pl-5">
				<li>{m.settings_reports_info_availability()}</li>
				<li>{m.settings_reports_info_incidents()}</li>
				<li>{m.settings_reports_info_unstable()}</li>
				<li>{m.settings_reports_info_compare()}</li>
			</ul>
			<p class="mt-2">
				{m.settings_reports_info_footer()}
				<a href={DOCS_URL} target="_blank" rel="noreferrer" class="font-semibold text-ink underline decoration-line underline-offset-2">{m.settings_reports_read_more()}</a>
			</p>
		</div>
	{/if}

	{#if error}
		<ErrorNotice {error} title={m.settings_reports_load_error()} onretry={() => void load()} />
	{:else if loading}
		<Skeleton class="h-24 w-full" />
	{:else}
		{#if emailChannels.length === 0}
			<p class="mb-4 flex flex-wrap items-center gap-2 text-sm text-ink-2">
				<Plate tone="advisory" label={m.settings_reports_no_channel_plate()} />
				{m.settings_reports_no_channel_text()}
				<a href="/settings/notifications#notifications-channels" class="font-semibold text-ink underline decoration-line underline-offset-2">{m.settings_reports_no_channel_link()}</a>
			</p>
		{/if}

		<div class="grid gap-6">
			{#each schedules as schedule (schedule.id)}
				<div class="grid gap-3 border-t border-line pt-6 first:border-t-0 first:pt-0">
					<ClientAssign kind="report" id={schedule.id} />
					<ReportEditor {schedule} channels={emailChannels} {defaultTimezone} {timezones} {pages} onsaved={saved} ondeleted={removed} />
				</div>
			{/each}

			{#if adding}
				<div class="border-t border-line pt-6 first:border-t-0 first:pt-0">
					<ReportEditor
						schedule={null}
						channels={emailChannels}
						{defaultTimezone}
						{timezones}
						{pages}
						onsaved={saved}
						ondeleted={removed}
						oncancel={() => (adding = false)}
					/>
				</div>
			{:else if schedules.length === 0}
				<EmptyState icon={Mail} title={m.settings_reports_empty_title()} description={m.settings_reports_empty_description()}>
					{#snippet action()}
						<Button variant="secondary" onclick={() => (adding = true)}>
							<Plus class="size-4" aria-hidden="true" />
							{m.settings_reports_setup()}
						</Button>
					{/snippet}
				</EmptyState>
			{:else}
				<div>
					<Button variant="secondary" onclick={() => (adding = true)}>
						<Plus class="size-4" aria-hidden="true" />
						{m.settings_reports_add()}
					</Button>
				</div>
			{/if}
		</div>
	{/if}
</Panel>

<Panel id="reports-history" title={m.reports_history_title()} description={m.reports_history_description()} padded={false}>
	{#if error}
		<p class="px-5 py-4 text-sm text-ink-2">{m.settings_reports_load_error()}</p>
	{:else if loading}
		<div class="px-5 py-4"><Skeleton class="h-16 w-full" /></div>
	{:else if schedules.length === 0}
		<p class="px-5 py-4 text-sm text-ink-2">{m.reports_history_empty()}</p>
	{:else}
		<ul class="divide-y divide-line" role="list">
			{#each schedules as schedule (schedule.id)}
				<li class="grid gap-1 px-5 py-3 sm:grid-cols-[minmax(0,1.4fr)_minmax(0,1.2fr)_minmax(0,1fr)_auto] sm:items-center sm:gap-4">
					<div class="min-w-0">
						<p class="truncate font-semibold text-ink">{schedule.name}</p>
						<p class="text-[0.8125rem] text-ink-2">{schedule.kind === 'sla' ? m.reports_history_kind_sla() : m.reports_history_kind_digest()}</p>
					</div>
					<p class="text-sm text-ink-2">
						{#if schedule.last_sent_at}
							<time datetime={schedule.last_sent_at} title={formatDateTime(schedule.last_sent_at)}>{m.reports_history_last({ when: formatRelative(schedule.last_sent_at) })}</time>
						{:else}
							{m.reports_history_never()}
						{/if}
					</p>
					<p class="text-sm text-ink-2">
						{#if !schedule.enabled}
							{m.reports_history_paused()}
						{:else if schedule.next_run_at}
							<time datetime={schedule.next_run_at} title={formatDateTime(schedule.next_run_at)}>{m.reports_history_next({ when: formatRelative(schedule.next_run_at) })}</time>
						{/if}
					</p>
					<div class="min-w-0">
						{#if schedule.last_error}
							<Plate tone="warning" label={m.reports_history_failed({ error: schedule.last_error })} />
						{:else if schedule.last_sent_at}
							<Plate tone="signal" label={m.reports_history_sent()} />
						{:else}
							<Plate tone="ghost" label={m.reports_history_waiting()} />
						{/if}
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</Panel>
</div>
