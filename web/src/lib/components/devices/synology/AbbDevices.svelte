<script lang="ts">
	/**
	 * Active Backup for Business, device by device. A laptop is not a server:
	 * the server learned each device's own rhythm (which days it is on, how
	 * often it backs up) and only calls it overdue against that rhythm — never
	 * on its usual off-days. Each row says the state in a word, the last
	 * success, the rhythm in words and a thirty-day strip; the tasks above give
	 * ABB's own view.
	 */
	import type { AbbDayCell, AbbDevice, AbbDeviceState, AbbTask, SynologyAbb } from '#lib/api/index.js';
	import { Panel, Plate, type Tone } from '#lib/ui/index.js';
	import { formatAgo, formatSpan, formatUnix } from '../pbs/format';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		abb: SynologyAbb;
	}

	let { abb }: Props = $props();

	function stateOf(state: AbbDeviceState): { tone: Tone; word: string } {
		switch (state) {
			case 'ok': return { tone: 'signal', word: m.devicesb_synology_abb_state_ok() };
			case 'idle': return { tone: 'ghost', word: m.devicesb_synology_abb_state_idle() };
			case 'learning': return { tone: 'info', word: m.devicesb_synology_abb_state_learning() };
			case 'running': return { tone: 'info', word: m.devicesb_synology_abb_state_running() };
			case 'overdue': return { tone: 'advisory', word: m.devicesb_synology_abb_state_overdue() };
			case 'failing': return { tone: 'warning', word: m.devicesb_synology_abb_state_failing() };
			default: return { tone: 'advisory', word: m.devicesb_synology_abb_state_never() };
		}
	}

	const DAY_BG: Record<AbbDayCell['outcome'], string> = {
		success: 'bg-signal',
		failure: 'bg-warning',
		cancelled: 'bg-advisory',
		running: 'bg-info',
		none: 'ghost-cell bg-ghost opacity-70'
	};

	function dayWord(outcome: AbbDayCell['outcome']): string {
		switch (outcome) {
			case 'success': return m.devicesb_synology_abb_day_success();
			case 'failure': return m.devicesb_synology_abb_day_failure();
			case 'cancelled': return m.devicesb_synology_abb_day_cancelled();
			case 'running': return m.devicesb_synology_abb_day_running();
			default: return m.devicesb_synology_abb_day_none();
		}
	}

	/** A task's last result as a plate label. */
	function resultLabel(result: string): string {
		switch (result) {
			case 'success': return m.devicesb_synology_abb_res_success();
			case 'partial_success': return m.devicesb_synology_abb_res_partial();
			case 'fail': return m.devicesb_synology_abb_res_fail();
			case 'cancel': return m.devicesb_synology_abb_res_cancel();
			case 'no_backup': return m.devicesb_synology_abb_res_no_backup();
			case 'running': return m.devicesb_synology_abb_res_running();
			case 'none': return m.devicesb_synology_abb_res_none();
			case 'unknown': return m.devicesb_synology_abb_res_unknown();
			default: return result;
		}
	}

	/** "last run failed 20 min ago", for a run that did not succeed. */
	function lastRunLine(outcome: string, ago: string): string {
		switch (outcome) {
			case 'partial_success': return m.devicesb_synology_abb_lastrun_partial({ ago });
			case 'fail': return m.devicesb_synology_abb_lastrun_fail({ ago });
			case 'cancel': return m.devicesb_synology_abb_lastrun_cancel({ ago });
			case 'no_backup': return m.devicesb_synology_abb_lastrun_no_backup({ ago });
			case 'running': return m.devicesb_synology_abb_lastrun_running({ ago });
			case 'none': return m.devicesb_synology_abb_lastrun_none({ ago });
			case 'unknown': return m.devicesb_synology_abb_lastrun_unknown({ ago });
			default: return m.devicesb_synology_abb_lastrun_other({ outcome, ago });
		}
	}

	function sourceWord(source: string): string {
		switch (source) {
			case 'pc': return 'PC';
			case 'vm': return m.devicesb_synology_abb_source_vm();
			case 'physical_server': return m.devicesb_synology_abb_source_physical();
			case 'file_server': return m.devicesb_synology_abb_source_file();
			case 'nas': return 'NAS';
			case 'unknown': return '';
			default: return source;
		}
	}

	/** Failing first, then overdue, then by name. */
	const ORDER: Record<AbbDeviceState, number> = { failing: 0, never: 1, overdue: 2, running: 3, learning: 4, ok: 5, idle: 6 };
	const devices = $derived(
		[...abb.devices].sort((a, b) => ORDER[a.state] - ORDER[b.state] || a.device_name.localeCompare(b.device_name, 'en'))
	);
	const attention = $derived(abb.devices.filter((d) => d.state === 'failing' || d.state === 'overdue' || d.state === 'never').length);
	const taskFailures = $derived(abb.tasks.filter((t) => t.last_status === 0).length);

	function taskPlate(task: AbbTask): { tone: Tone; label: string } {
		if (task.last_status === 2) return { tone: 'info', label: m.devicesb_synology_abb_res_running() };
		if (task.last_status === 0) return { tone: 'warning', label: task.result === 'partial_success' ? m.devicesb_synology_abb_res_partial() : m.devicesb_synology_abb_res_fail() };
		if (task.last_status === 1) return { tone: 'signal', label: m.devicesb_synology_abb_res_success() };
		return { tone: 'ghost', label: resultLabel(task.result) };
	}

	/** "Last success 3 h ago · last run failed 20 min ago". */
	function lastLine(device: AbbDevice): string {
		const parts: string[] = [];
		parts.push(device.last_success_s === null ? m.devicesb_synology_abb_no_success_record() : m.devicesb_synology_abb_last_success({ ago: formatAgo(device.last_success_s) }));
		if (device.last_run_s !== null && device.last_outcome && device.last_outcome !== 'success') {
			parts.push(lastRunLine(device.last_outcome, formatAgo(device.last_run_s)));
		}
		return parts.join(' · ');
	}

	/** Why the verdict, in one clause the row can carry as a tooltip. */
	function reason(device: AbbDevice): string {
		const allowance = formatSpan(device.allowance_s);
		switch (device.state) {
			case 'overdue': {
				const elapsed = formatSpan(device.active_elapsed_s ?? 0);
				return device.typical_interval_s
					? m.devicesb_synology_abb_reason_overdue({ elapsed, allowance, interval: formatSpan(device.typical_interval_s) })
					: m.devicesb_synology_abb_reason_overdue_unknown({ elapsed, allowance });
			}
			case 'failing':
				return m.devicesb_synology_abb_reason_failing({ count: device.consecutive_failures, threshold: abb.failing_streak });
			case 'idle':
				return m.devicesb_synology_abb_reason_idle();
			case 'learning':
				return m.devicesb_synology_abb_reason_learning({ allowance: formatSpan(abb.learning_allowance_s) });
			case 'never':
				return m.devicesb_synology_abb_reason_never();
			case 'running':
				return m.devicesb_synology_abb_reason_running();
			default:
				return m.devicesb_synology_abb_reason_ok({ allowance });
		}
	}

	function tooltip(cell: AbbDayCell): string {
		const outcome = dayWord(cell.outcome);
		if (cell.runs === 0) return m.devicesb_synology_abb_tooltip({ day: cell.day, outcome });
		const runs = cell.runs === 1 ? m.devicesb_synology_abb_runs_one() : m.devicesb_synology_abb_runs_other({ count: cell.runs });
		return m.devicesb_synology_abb_tooltip_runs({ day: cell.day, outcome, runs });
	}
</script>

<Panel
	title={m.devicesb_synology_abb_title()}
	description={m.devicesb_synology_abb_desc()}
	padded={false}
	class="rise-in"
>
	{#snippet aside()}
		{#if attention > 0}
			<Plate tone="warning" label={attention === 1 ? m.devicesb_synology_abb_attention_one() : m.devicesb_synology_abb_attention_other({ count: attention })} />
		{:else if abb.devices.length > 0}
			<Plate tone="signal" label={m.devicesb_synology_abb_all_on_rhythm()} />
		{/if}
	{/snippet}

	{#if abb.tasks.length > 0}
		<ul class="divide-y divide-line border-b border-line bg-surface-2/40">
			{#each abb.tasks as task (task.task_id)}
				{@const plate = taskPlate(task)}
				<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-5 py-2 text-[0.8125rem]">
					<span class="font-semibold text-ink">{task.name}</span>
					<span class="text-ink-3">{[sourceWord(task.source_type), task.device_count === null ? '' : task.device_count === 1 ? m.devicesb_synology_abb_devices_one() : m.devicesb_synology_abb_devices_other({ count: task.device_count })].filter(Boolean).join(' · ')}</span>
					<span class="tnum text-ink-2">
						{task.last_success_seconds === null ? m.devicesb_synology_abb_no_success_yet() : m.devicesb_synology_abb_last_success({ ago: formatAgo(task.last_success_seconds) })}
					</span>
					{#if task.enabled === false}<Plate tone="muted" label={m.devicesb_synology_abb_no_schedule()} bare size="sm" />{/if}
					<span class="ml-auto"><Plate tone={plate.tone} label={plate.label} size="sm" /></span>
				</li>
			{/each}
		</ul>
	{/if}

	{#if devices.length === 0}
		<p class="px-5 py-4 text-sm text-ink-2">
			{taskFailures > 0 ? m.devicesb_synology_abb_no_history_failures() : m.devicesb_synology_abb_no_history()}
		</p>
	{:else}
		<div class="flex flex-wrap items-center gap-x-4 gap-y-1.5 border-b border-line px-5 py-2.5 text-[0.75rem] text-ink-2">
			<span class="tnum">{devices.length === 1 ? m.devicesb_synology_abb_summary_one() : m.devicesb_synology_abb_summary_other({ count: devices.length })}</span>
			<span class="flex flex-wrap items-center gap-x-3 gap-y-1 sm:ml-auto">
				<span class="inline-flex items-center gap-1.5"><span class="inline-block size-2.5 rounded-full bg-signal" aria-hidden="true"></span>{m.devicesb_synology_abb_day_success()}</span>
				<span class="inline-flex items-center gap-1.5"><span class="inline-block size-2.5 rounded-full bg-warning" aria-hidden="true"></span>{m.devicesb_synology_abb_legend_failed()}</span>
				<span class="inline-flex items-center gap-1.5"><span class="inline-block size-2.5 rounded-full bg-advisory" aria-hidden="true"></span>{m.devicesb_synology_abb_legend_cancelled()}</span>
				<span class="inline-flex items-center gap-1.5"><span class="inline-block size-2.5 rounded-full bg-ghost" aria-hidden="true"></span>{m.devicesb_synology_abb_day_none()}</span>
			</span>
		</div>
		<ul class="divide-y divide-line">
			{#each devices as device, i (device.device_id)}
				{@const state = stateOf(device.state)}
				<li class="rise-in px-5 py-3" style="--rise-delay: {Math.min(i, 8) * 40}ms" title={reason(device)}>
					<div class="flex flex-col gap-2 lg:flex-row lg:items-center lg:gap-4">
						<div class="min-w-0 lg:w-64 lg:shrink-0">
							<p class="truncate font-semibold text-ink" title={device.device_name}>{device.device_name || m.devicesb_synology_abb_device_fallback({ id: device.device_id })}</p>
							<p class="truncate text-[0.75rem] text-ink-3" title={device.task_name}>{device.task_name || m.devicesb_synology_abb_task_unknown()} · {device.rhythm}</p>
						</div>
						<div class="flex min-w-0 flex-1 items-center gap-[3px]" role="img" aria-label={m.devicesb_synology_abb_aria({ device: device.device_name, successes: device.successes_30d, days: device.calendar.length, failures: device.failures_30d })}>
							{#each device.calendar as cell (cell.day)}
								<span class={`h-4 min-w-0 flex-1 rounded-full ${DAY_BG[cell.outcome]}`} title={tooltip(cell)}></span>
							{/each}
						</div>
						<div class="flex shrink-0 flex-wrap items-center gap-2 lg:w-72 lg:justify-end">
							<span class="tnum text-[0.75rem] text-ink-2" title={device.last_success_s === null ? undefined : formatUnix(device.last_success_s)}>{lastLine(device)}</span>
							<Plate tone={state.tone} label={state.word} />
						</div>
					</div>
				</li>
			{/each}
		</ul>
		<p class="px-5 py-3 text-[0.75rem] leading-relaxed text-ink-3">
			{m.devicesb_synology_abb_footnote({ allowance: formatSpan(abb.min_allowance_s), streak: abb.failing_streak })}
		</p>
	{/if}
</Panel>
