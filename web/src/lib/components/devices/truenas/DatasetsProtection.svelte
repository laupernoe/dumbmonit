<script lang="ts">
	/**
	 * Is the data protected, and is it still? The datasets against their
	 * quotas — a backup share that hits its quota stops taking backups, quietly
	 * — then the last scrub of each pool, because a scrub is the only thing that
	 * finds a rotting block before it is needed, then the replication and
	 * snapshot tasks, failures first. A replication that has been failing for a
	 * month looks exactly like one that works until the day it is needed.
	 */
	import type { TruenasDatasetRow, TruenasScrubRow, TruenasTaskRow } from '#lib/api/index.js';
	import { Button, Plate } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import {
		FILL,
		formatAgo,
		formatBytes,
		formatCount,
		formatUnix,
		percentOf,
		reading,
		scanLabel,
		taskKindLabel,
		taskPlate
	} from './format';

	interface Props {
		datasets: TruenasDatasetRow[];
		scrubs: TruenasScrubRow[];
		tasks: TruenasTaskRow[];
	}

	let { datasets, scrubs, tasks }: Props = $props();

	/** Past this many datasets, the list folds; the server puts the fullest first. */
	const FOLD = 12;
	let showAll = $state(false);
	const shownDatasets = $derived(showAll ? datasets : datasets.slice(0, FOLD));
</script>

{#if datasets.length === 0 && scrubs.length === 0 && tasks.length === 0}
	<p class="px-5 py-4 text-sm text-ink-2">
		{m.devicesb_truenas_protection_empty()}
	</p>
{:else}
	<div class="flex flex-col divide-y divide-line">
		{#if datasets.length > 0}
			<div class="flex flex-col gap-2 px-5 py-4">
				<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_truenas_protection_datasets()}</p>
				<ul class="flex flex-col divide-y divide-line">
					{#each shownDatasets as dataset (dataset.name)}
						{@const used = reading(dataset.used_bytes)}
						{@const available = reading(dataset.available_bytes)}
						{@const quota = reading(dataset.quota_bytes)}
						{@const quotaPercent =
							quota !== null
								? (reading(dataset.quota_used_percent) ?? percentOf(used, quota))
								: null}
						{@const snapshots = reading(dataset.snapshot_count)}
						<li class="flex flex-col gap-1 py-2.5 lg:flex-row lg:items-center lg:gap-4">
							<div class="flex min-w-0 items-center gap-2 lg:w-72 lg:shrink-0">
								<span class="truncate font-semibold text-ink" title={dataset.name}>{dataset.name}</span>
								{#if dataset.locked}
									<Plate
										tone="warning"
										label={m.devicesb_truenas_protection_locked()}
										title={m.devicesb_truenas_protection_locked_title()}
									/>
								{/if}
							</div>
							<div class="tnum flex min-w-0 flex-1 flex-wrap items-center gap-x-4 gap-y-1 text-[0.8125rem] text-ink-2">
								{#if used !== null}<span>{m.devicesb_truenas_protection_used({ size: formatBytes(used) })}</span>{/if}
								{#if available !== null}<span class="text-ink-3">{m.devicesb_truenas_protection_free({ size: formatBytes(available) })}</span>{/if}
								{#if quota !== null && quotaPercent !== null}
									{@const tone = dataset.near_quota ? 'advisory' : 'signal'}
									<span class="inline-flex items-center gap-2">
										<span
											class="h-1.5 w-20 overflow-hidden rounded-full bg-surface-2"
											role="meter"
											aria-valuemin="0"
											aria-valuemax="100"
											aria-valuenow={Math.round(quotaPercent)}
											aria-label={m.devicesb_truenas_protection_quota_usage({ name: dataset.name })}
										>
											<span
												class={`block h-full rounded-full ${FILL[tone]}`}
												style={`width: ${Math.min(100, quotaPercent)}%`}
											></span>
										</span>
										{m.devicesb_truenas_protection_quota_of({ percent: quotaPercent.toFixed(0), size: formatBytes(quota) })}
									</span>
								{:else if quota !== null}
									<span>{m.devicesb_truenas_protection_quota({ size: formatBytes(quota) })}</span>
								{/if}
								{#if snapshots !== null}
									<span>{snapshots === 1 ? m.devicesb_truenas_protection_snapshots_one({ count: formatCount(snapshots) }) : m.devicesb_truenas_protection_snapshots_other({ count: formatCount(snapshots) })}</span>
								{/if}
								{#if dataset.newest_snapshot_at !== null}
									<span class="text-ink-3" title={formatUnix(dataset.newest_snapshot_at)}>
										{m.devicesb_truenas_protection_last_snapshot({ ago: formatAgo(dataset.newest_snapshot_at) })}
									</span>
								{/if}
							</div>
							{#if dataset.near_quota}
								<div class="flex shrink-0 items-center">
									<Plate tone="advisory" label={m.devicesb_truenas_protection_near_quota()} />
								</div>
							{/if}
						</li>
					{/each}
				</ul>
				{#if datasets.length > FOLD}
					<div>
						<Button variant="ghost" size="sm" class="min-h-10 sm:min-h-0" onclick={() => (showAll = !showAll)}>
							{showAll ? m.devicesb_truenas_protection_show_fewer() : m.devicesb_truenas_protection_show_all({ count: formatCount(datasets.length) })}
						</Button>
					</div>
				{/if}
			</div>
		{/if}

		{#if scrubs.length > 0}
			<div class="flex flex-col gap-2 px-5 py-4">
				<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_truenas_protection_scrubs()}</p>
				<ul class="flex flex-col gap-1.5">
					{#each scrubs as scrub (scrub.pool)}
						{@const errors = reading(scrub.last_scrub_errors)}
						<li class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
							<span class="font-semibold text-ink">{scrub.pool}</span>
							{#if scrub.running}
								<Plate tone="info" label={scanLabel(scrub.function, scrub.percent, scrub.seconds_left)} />
							{/if}
							{#if scrub.last_scrub_at !== null}
								<span class="tnum text-[0.8125rem] text-ink-2" title={formatUnix(scrub.last_scrub_at)}>
									{#if errors === null}
										{m.devicesb_truenas_protection_last_scrub({ ago: formatAgo(scrub.last_scrub_at) })}
									{:else if errors === 1}
										{m.devicesb_truenas_protection_last_scrub_errors_one({ ago: formatAgo(scrub.last_scrub_at), count: formatCount(errors) })}
									{:else}
										{m.devicesb_truenas_protection_last_scrub_errors_other({ ago: formatAgo(scrub.last_scrub_at), count: formatCount(errors) })}
									{/if}
								</span>
							{:else if !scrub.running}
								<span class="text-[0.8125rem] text-ink-3">
									{scrub.function?.toUpperCase() === 'RESILVER'
										? m.devicesb_truenas_protection_no_scrub_resilver()
										: m.devicesb_truenas_protection_no_scrub()}
								</span>
							{/if}
							{#if errors !== null && errors > 0}
								<Plate tone="warning" label={m.devicesb_truenas_protection_scrub_errors()} />
							{/if}
							{#if scrub.overdue}
								<Plate
									tone="advisory"
									label={m.devicesb_truenas_protection_scrub_overdue()}
									title={m.devicesb_truenas_protection_scrub_overdue_title({ days: formatCount(scrub.threshold_days) })}
								/>
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/if}

		<div class="flex flex-col gap-2 px-5 py-4">
			<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_truenas_protection_tasks()}</p>
			{#if tasks.length === 0}
				<p class="text-sm text-ink-2">
					{m.devicesb_truenas_protection_no_tasks()}
				</p>
			{:else}
				<ul class="flex flex-col divide-y divide-line">
					{#each tasks as task, index (`${task.kind}/${task.name}/${index}`)}
						{@const plate = taskPlate(task)}
						<li class="flex flex-col gap-1 py-2.5">
							<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
								<Plate tone={plate.tone} label={plate.label} />
								<span class="text-[0.75rem] tracking-wide text-ink-3 uppercase">
									{taskKindLabel(task.kind)}
								</span>
								<span class="font-semibold break-all text-ink">{task.name}</span>
								{#if task.detail}
									<span class="text-[0.8125rem] text-ink-3">{task.detail}</span>
								{/if}
								{#if !task.enabled}
									<Plate tone="ghost" label={m.devicesb_truenas_protection_disabled()} />
								{/if}
								{#if task.stale}
									<Plate tone="advisory" label={m.devicesb_truenas_protection_stale()} />
								{/if}
							</div>
							{#if task.last_run_at !== null || task.last_snapshot}
								<p class="tnum flex min-w-0 flex-wrap gap-x-4 gap-y-0.5 text-[0.8125rem] text-ink-2">
									{#if task.last_run_at !== null}
										<span title={formatUnix(task.last_run_at)}>{m.devicesb_truenas_protection_last_run({ ago: formatAgo(task.last_run_at) })}</span>
									{/if}
									{#if task.last_snapshot}
										<span class="max-w-full truncate font-mono text-[0.75rem] text-ink-3" title={task.last_snapshot}>
											{task.last_snapshot}
										</span>
									{/if}
								</p>
							{/if}
							{#if task.error}
								<p
									class="text-[0.8125rem] break-words {task.failed
										? 'text-warning-ink'
										: 'text-ink-3'}"
								>
									{task.error}
								</p>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>
{/if}
