<script lang="ts">
	/**
	 * The thirty-day strip of a Proxmox Backup Server: one row per backed-up
	 * machine, one cell per day, drawn by the shared `BackupMatrix`. This file
	 * only translates PBS's calendar into matrix rows: a day with a snapshot whose
	 * verification failed is a warning (the backup is there, its check failed),
	 * and each row adds PBS's own line — snapshots, retention, last failure.
	 */
	import type { PbsCalendar, PbsCalendarDay, PbsCalendarGroup } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import BackupMatrix from '../backup/BackupMatrix.svelte';
	import type { DayState, MatrixDay, MatrixJob } from '../backup/history';
	import { formatAgo, formatBytes, formatUnix, groupPlace, groupTitle } from './format';

	interface Props {
		calendar: PbsCalendar;
	}

	let { calendar }: Props = $props();

	const STATE: Record<PbsCalendarDay['state'], DayState> = {
		ok: 'ok',
		verify_failed: 'warning',
		failed: 'failed',
		running: 'running',
		none: 'none'
	};

	/** A group is failing when its newest task failed after its newest success. */
	function failing(group: PbsCalendarGroup): boolean {
		const failure = group.last_failure?.time ?? null;
		if (failure === null) return false;
		return (group.last_success ?? 0) < failure;
	}

	function excerpt(status: string): string {
		const text = status.replace(/^TASK ERROR:\s*/i, '').trim();
		return text.length > 90 ? `${text.slice(0, 90)}…` : text;
	}

	function dayOf(day: PbsCalendarDay): MatrixDay {
		const notes: string[] = [];
		if (day.snapshot) {
			let line: string = m.devicesb_pbs_calendar_tip_snapshot({
				time: formatUnix(day.snapshot.time).split(', ').at(-1) ?? ''
			});
			if (day.snapshot.size !== null) line += ` · ${formatBytes(day.snapshot.size)}`;
			if (day.snapshot.verified === false) line += ` · ${m.devicesb_pbs_calendar_tip_verification_failed()}`;
			else if (day.snapshot.verified === true) line += ` · ${m.devicesb_pbs_calendar_tip_verified()}`;
			notes.push(line);
		}
		return {
			date: day.date,
			state: STATE[day.state],
			runs: day.runs.map((run) => ({
				at: run.start,
				outcome: run.ok === null ? 'running' : run.ok ? 'ok' : 'failed',
				durationSeconds: run.end === null ? null : Math.max(0, run.end - run.start),
				bytes: null,
				note: run.ok === false && run.status ? excerpt(run.status) : undefined
			})),
			notes
		};
	}

	const jobs = $derived<MatrixJob[]>(
		calendar.groups.map((group) => ({
			key: `${group.datastore}/${group.namespace}/${group.backup_type}/${group.backup_id}`,
			title: groupTitle(group),
			subtitle: groupPlace(group),
			days: group.days.map(dayOf),
			lastSuccess: group.last_success,
			failing: failing(group),
			running: group.days.at(-1)?.state === 'running',
			durationSeconds: null,
			bytes: null
		}))
	);

	const groups = $derived(new Map(calendar.groups.map((g) => [`${g.datastore}/${g.namespace}/${g.backup_type}/${g.backup_id}`, g])));
</script>

{#if calendar.groups.length === 0}
	<p class="px-5 py-4 text-sm text-ink-2">
		{#if calendar.probed_at === null}
			{m.devicesb_pbs_calendar_empty_waiting()}
		{:else}
			{m.devicesb_pbs_calendar_empty_none({ days: calendar.days })}
		{/if}
	</p>
{:else}
	<BackupMatrix {jobs}>
		{#snippet meta(job)}
			{@const group = groups.get(job.key)}
			{#if group}
				<span>{group.count === 1 ? m.devicesb_pbs_calendar_snapshots_one({ count: group.count }) : m.devicesb_pbs_calendar_snapshots_other({ count: group.count })}</span>
				{#if group.last_size !== null}<span>{m.devicesb_pbs_calendar_last_size({ size: formatBytes(group.last_size) })}</span>{/if}
				{#if group.retention}<span>{m.devicesb_pbs_calendar_keeps({ retention: group.retention })}</span>{/if}
				{#if group.last_verified === false}<span class="text-advisory-ink">{m.devicesb_pbs_calendar_latest_unverified()}</span>{/if}
				{#if group.last_failure}
					<span class="min-w-0 break-words text-warning-ink" title={group.last_failure.error}>
						{m.devicesb_pbs_calendar_failed_line({ ago: formatAgo(group.last_failure.time), error: excerpt(group.last_failure.error) })}
					</span>
				{/if}
			{/if}
		{/snippet}
	</BackupMatrix>
{/if}
