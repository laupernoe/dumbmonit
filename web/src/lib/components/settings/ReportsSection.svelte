<script lang="ts">
	/**
	 * Settings → Reports: periodic availability emails for the team. Mostly one
	 * report (weekly by default), though a few may coexist, for instance a
	 * daily one for the on-call and a monthly one for management.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { Info, Mail, Plus } from 'lucide-svelte';
	import { listChannels, type Channel } from '#lib/api/index.js';
	import { listReportSchedules, type ReportSchedule } from '#lib/api/reports.js';
	import { Button, EmptyState, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import ReportEditor from './ReportEditor.svelte';

	const DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/using/reports/';

	let schedules = $state<ReportSchedule[]>([]);
	let channels = $state<Channel[]>([]);
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
			const [list, all] = await Promise.all([listReportSchedules(signal), listChannels(signal)]);
			schedules = Array.isArray(list) ? list : [];
			channels = Array.isArray(all) ? all : [];
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
				<a href="/alerts#notifications" class="font-semibold text-ink underline decoration-line underline-offset-2">{m.settings_reports_no_channel_link()}</a>
			</p>
		{/if}

		<div class="grid gap-6">
			{#each schedules as schedule (schedule.id)}
				<div class="rounded-[var(--radius-card)] border border-line p-4">
					<ReportEditor {schedule} channels={emailChannels} {defaultTimezone} {timezones} onsaved={saved} ondeleted={removed} />
				</div>
			{/each}

			{#if adding}
				<div class="rounded-[var(--radius-card)] border border-line p-4">
					<ReportEditor
						schedule={null}
						channels={emailChannels}
						{defaultTimezone}
						{timezones}
						onsaved={saved}
						ondeleted={removed}
						oncancel={() => (adding = false)}
					/>
				</div>
			{:else if schedules.length === 0}
				<EmptyState icon={Mail} title={m.settings_reports_empty_title()} description={m.settings_reports_empty_description()}>
					{#snippet action()}
						<Button variant="primary" onclick={() => (adding = true)}>
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
