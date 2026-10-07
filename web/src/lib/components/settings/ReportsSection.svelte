<script lang="ts">
	/**
	 * Settings → Reports: periodic availability emails for the team. Mostly one
	 * report (weekly by default), though a few may coexist, for instance a
	 * daily one for the on-call and a monthly one for management.
	 */
	import { Info, Mail, Plus } from 'lucide-svelte';
	import { listChannels, type Channel } from '$lib/api';
	import { listReportSchedules, type ReportSchedule } from '$lib/api/reports';
	import { Button, EmptyState, ErrorNotice, Panel, Plate, Skeleton } from '$lib/ui';
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
	title="Reports"
	description="A short email for the team: how the infrastructure did over the last day, week or month."
>
	{#snippet aside()}
		<button
			type="button"
			class="inline-flex size-8 items-center justify-center rounded-full border border-line text-ink-2 hover:text-ink focus-visible:outline-2 focus-visible:outline-signal"
			aria-label="What does a report contain?"
			aria-expanded={showInfo}
			aria-controls="reports-info"
			onclick={() => (showInfo = !showInfo)}
		>
			<Info class="size-4" aria-hidden="true" />
		</button>
	{/snippet}

	{#if showInfo}
		<div id="reports-info" class="rise-in mb-4 rounded-[var(--radius-card)] border border-line bg-canvas-deep p-4 text-sm text-ink-2">
			<p class="font-semibold text-ink">What a report contains</p>
			<ul class="mt-2 list-disc space-y-1 pl-5">
				<li>Availability over the period, overall, by group and by device, computed like on a status page.</li>
				<li>The incidents of the period: when each started, how long it lasted, on which device.</li>
				<li>The five least stable devices.</li>
				<li>How all of it compares with the previous period of the same length.</li>
			</ul>
			<p class="mt-2">
				Plain tables, no remote image. Each recipient gets their own email. The period is the last 24 hours, 7 days or 30 days before sending.
				<a href={DOCS_URL} target="_blank" rel="noreferrer" class="font-semibold text-ink underline decoration-line underline-offset-2">Read more</a>.
			</p>
		</div>
	{/if}

	{#if error}
		<ErrorNotice {error} title="Could not load the reports" onretry={() => void load()} />
	{:else if loading}
		<Skeleton class="h-24 w-full" />
	{:else}
		{#if emailChannels.length === 0}
			<p class="mb-4 flex flex-wrap items-center gap-2 text-sm text-ink-2">
				<Plate tone="advisory" label="No email channel" />
				Reports are sent through an email (SMTP) channel. Add one in
				<a href="/alerts#notifications" class="font-semibold text-ink underline decoration-line underline-offset-2">Alerts → Notifications</a>.
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
				<EmptyState icon={Mail} title="No report yet." description="Weekly by default, on Monday morning, in your timezone.">
					{#snippet action()}
						<Button variant="primary" onclick={() => (adding = true)}>
							<Plus class="size-4" aria-hidden="true" />
							Set up a report
						</Button>
					{/snippet}
				</EmptyState>
			{:else}
				<div>
					<Button variant="secondary" onclick={() => (adding = true)}>
						<Plus class="size-4" aria-hidden="true" />
						Add a report
					</Button>
				</div>
			{/if}
		</div>
	{/if}
</Panel>
