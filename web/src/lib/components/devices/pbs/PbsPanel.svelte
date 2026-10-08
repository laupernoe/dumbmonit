<script lang="ts">
	/**
	 * What a Proxmox Backup Server has to show beyond charts: the tasks that
	 * failed in the last thirty days, the backup calendar of every machine,
	 * the scheduled jobs and the datastores and disks. Four reads of what the
	 * probe stored, refreshed every minute; PBS itself is only asked when
	 * someone opens a task log or a disk's SMART table.
	 */
	import { untrack } from 'svelte';
	import { getPbsCalendar, getPbsHealth, getPbsJobs, listPbsFailures } from '#lib/api/pbs.js';
	import type { PbsCalendar, PbsFailure, PbsHealth, PbsJobs, Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import ClientDevicesTable from '../ClientDevicesTable.svelte';
	import BackupCalendar from './BackupCalendar.svelte';
	import DatastoreHealth from './DatastoreHealth.svelte';
	import FailureList from './FailureList.svelte';
	import JobTable from './JobTable.svelte';
	import NodeHealth from './NodeHealth.svelte';
	import TapePanel from './TapePanel.svelte';
	import { formatAgo, formatUnix } from './format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	const DAYS = 30;

	let calendar = $state<PbsCalendar | null>(null);
	let failures = $state<PbsFailure[]>([]);
	let jobs = $state<PbsJobs | null>(null);
	let health = $state<PbsHealth | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	const probedAt = $derived(calendar?.probed_at ?? jobs?.probed_at ?? health?.probed_at ?? null);
	const failingJobs = $derived((jobs?.jobs ?? []).filter((j) => j.enabled && j.last_run_ok === false).length);
	/** Units the backup server cannot do without; the rest never raise a count. */
	const REQUIRED_SERVICES = ['proxmox-backup', 'proxmox-backup-proxy', 'proxmox-backup-banner'];
	const stoppedServices = $derived(
		(health?.services ?? []).filter((s) => !s.running && REQUIRED_SERVICES.includes(s.service)).length
	);
	const failingTapeJobs = $derived(
		(health?.tape?.jobs ?? []).filter((j) => {
			const state = j.last_run_state?.toUpperCase();
			return state !== undefined && state !== 'OK' && !state.startsWith('WARNINGS');
		}).length
	);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const [c, f, j, h] = await Promise.all([
				getPbsCalendar(target.id, DAYS, signal),
				listPbsFailures(target.id, DAYS, signal),
				getPbsJobs(target.id, signal),
				getPbsHealth(target.id, signal)
			]);
			calendar = c;
			failures = f;
			jobs = j;
			health = h;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void target.id;
		loading = true;
		calendar = null;
		failures = [];
		jobs = null;
		health = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});
</script>

{#if error}
	<Panel title={m.devicesb_pbs_panel_backups()} class="rise-in">
		<ErrorNotice {error} title={m.devicesb_pbs_panel_error()} onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title={m.devicesb_pbs_panel_backups()} padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label={m.devicesb_pbs_panel_loading()}>
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else}
	<div class="flex flex-col gap-6">
		<Panel
			title={m.devicesb_pbs_panel_failures()}
			description={m.devicesb_pbs_panel_failures_desc({ days: DAYS })}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if failures.length > 0}
					<Plate tone="warning" label={m.devicesb_pbs_panel_failed_tasks({ count: failures.length })} />
				{:else}
					<Plate tone="signal" label={m.devicesb_pbs_panel_all_clear()} />
				{/if}
			{/snippet}
			<FailureList targetId={target.id} {failures} days={DAYS} />
		</Panel>

		<Panel
			title={m.devicesb_pbs_panel_calendar()}
			description={m.devicesb_pbs_panel_calendar_desc({ days: DAYS })}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if probedAt !== null}
					<span class="tnum text-[0.75rem] text-ink-3" title={formatUnix(probedAt)}>{m.devicesb_pbs_panel_read_ago({ ago: formatAgo(probedAt) })}</span>
				{/if}
			{/snippet}
			{#if calendar}
				<BackupCalendar {calendar} />
			{/if}
		</Panel>

		<Panel
			title={m.devicesb_pbs_panel_jobs()}
			description={m.devicesb_pbs_panel_jobs_desc()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if failingJobs > 0}
					<Plate tone="warning" label={m.devicesb_pbs_tape_failing({ count: failingJobs })} />
				{/if}
			{/snippet}
			<JobTable jobs={jobs?.jobs ?? []} />
		</Panel>

		{#if health?.tape}
			<Panel
				title={m.devicesb_pbs_panel_tape()}
				description={m.devicesb_pbs_panel_tape_desc()}
				padded={false}
				class="rise-in"
			>
				{#snippet aside()}
					{#if failingTapeJobs > 0}
						<Plate tone="warning" label={m.devicesb_pbs_tape_failing({ count: failingTapeJobs })} />
					{/if}
				{/snippet}
				<TapePanel tape={health.tape} />
			</Panel>
		{/if}

		<Panel
			title={m.devicesb_pbs_panel_datastores()}
			description={health?.version ? m.devicesb_pbs_panel_version_desc({ version: health.version }) : undefined}
			padded={false}
			class="rise-in"
		>
			<DatastoreHealth
				targetId={target.id}
				datastores={health?.datastores ?? []}
				disks={health?.disks ?? []}
				zpools={health?.zpools ?? []}
			/>
		</Panel>

		<Panel
			title={m.devicesb_pbs_panel_server()}
			description={m.devicesb_pbs_panel_server_desc()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if stoppedServices > 0}
					<Plate tone="warning" label={m.devicesb_pbs_panel_services_stopped({ count: stoppedServices })} />
				{/if}
			{/snippet}
			<NodeHealth
				services={health?.services ?? []}
				packages={health?.packages ?? []}
				certificates={health?.certificates ?? []}
				traffic={health?.traffic ?? []}
			/>
		</Panel>

		<ClientDevicesTable {target} />
	</div>
{/if}
