<script lang="ts">
	/**
	 * Every scheduled job PBS knows — sync, verify, prune, and the garbage
	 * collection of each datastore — with its schedule, last run, outcome and
	 * next run. A failed last run gets a red plate and the error text; a
	 * disabled job a ghost one. Failures sort first, the server does that.
	 */
	import type { PbsJob } from '#lib/api/index.js';
	import { Plate, type Tone } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { formatAgo, formatUnix, jobKindLabel } from './format';

	interface Props {
		jobs: PbsJob[];
	}

	let { jobs }: Props = $props();

	function status(job: PbsJob): { tone: Tone; label: string } {
		if (!job.enabled) return { tone: 'ghost', label: m.devicesb_pbs_jobs_disabled() };
		if (job.last_run_ok === false) return { tone: 'warning', label: m.devicesb_pbs_jobs_last_failed() };
		if (job.last_run_ok === null) return { tone: 'ghost', label: m.devicesb_pbs_jobs_never_ran() };
		if (job.last_run_state?.toUpperCase().startsWith('WARNINGS')) return { tone: 'advisory', label: job.last_run_state };
		return { tone: 'signal', label: m.devicesb_pbs_jobs_last_ok() };
	}

	function nextRun(job: PbsJob): string {
		if (job.next_run === null) return job.schedule ? '—' : m.devicesb_pbs_jobs_not_scheduled();
		if (job.next_run * 1000 < Date.now()) return m.devicesb_pbs_jobs_overdue({ ago: formatAgo(job.next_run) });
		return formatAgo(job.next_run);
	}

	function place(job: PbsJob): string {
		let text = job.datastore;
		if (job.namespace) text += ` / ${job.namespace}`;
		if (job.kind === 'sync' && job.remote) text = `${job.remote} → ${text}`;
		return text;
	}
</script>

{#if jobs.length === 0}
	<p class="px-5 py-4 text-sm text-ink-2">
		{m.devicesb_pbs_jobs_empty()}
	</p>
{:else}
	<div class="overflow-x-auto">
		<table class="w-full min-w-[40rem] text-sm">
			<thead>
				<tr class="border-b border-line text-left text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase">
					<th class="px-5 py-2 font-semibold">{m.devicesb_pbs_jobs_col_job()}</th>
					<th class="px-3 py-2 font-semibold">{m.devicesb_pbs_jobs_col_datastore()}</th>
					<th class="px-3 py-2 font-semibold">{m.devicesb_pbs_jobs_col_schedule()}</th>
					<th class="px-3 py-2 font-semibold">{m.devicesb_pbs_jobs_col_last_run()}</th>
					<th class="px-3 py-2 font-semibold">{m.devicesb_pbs_jobs_col_next_run()}</th>
					<th class="px-5 py-2 font-semibold">{m.devicesb_pbs_jobs_col_status()}</th>
				</tr>
			</thead>
			<tbody class="divide-y divide-line">
				{#each jobs as job (`${job.kind}/${job.datastore}/${job.id}`)}
					{@const plate = status(job)}
					<tr class={job.last_run_ok === false && job.enabled ? 'bg-warning-soft/40' : ''}>
						<td class="px-5 py-2.5 align-top">
							<p class="font-semibold text-ink">{job.kind === 'gc' ? jobKindLabel(job.kind) : m.devicesb_pbs_jobs_title({ kind: jobKindLabel(job.kind), id: job.id })}</p>
							{#if job.comment}<p class="text-[0.75rem] text-ink-3">{job.comment}</p>{/if}
							{#if job.retention}<p class="tnum text-[0.75rem] text-ink-3">{m.devicesb_pbs_jobs_keeps({ retention: job.retention })}</p>{/if}
						</td>
						<td class="px-3 py-2.5 align-top text-ink-2 break-all">{place(job)}</td>
						<td class="tnum px-3 py-2.5 align-top font-mono text-[0.8125rem] text-ink-2">{job.schedule ?? '—'}</td>
						<td class="tnum px-3 py-2.5 align-top text-ink-2" title={formatUnix(job.last_run_end)}>{formatAgo(job.last_run_end)}</td>
						<td class="tnum px-3 py-2.5 align-top text-ink-2" title={job.next_run === null ? '' : formatUnix(job.next_run)}>{nextRun(job)}</td>
						<td class="px-5 py-2.5 align-top">
							<Plate tone={plate.tone} label={plate.label} />
							{#if job.error}<p class="mt-1 max-w-xs text-[0.8125rem] text-warning-ink break-words">{job.error}</p>{/if}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}
