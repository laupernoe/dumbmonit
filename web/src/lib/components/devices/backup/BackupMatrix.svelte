<script lang="ts">
	/**
	 * The thirty-day matrix of a backup integration: one row per job, one cell
	 * per day, shared by PBS, Veeam, Duplicati, Kopia, Active Backup and Plakar.
	 * A cell is a shape and a word, never a colour alone: a filled pill for a
	 * run that succeeded, a hatched one for a warning, a crossed one for a
	 * failure, a dotted one for a day nothing ran, a dashed outline before the
	 * first data. Hover names the day, each run (time, outcome, duration, size);
	 * tap or press a cell to read the same lines under the row. The row carries
	 * the age of the last success in a plate.
	 *
	 * Presentational: it takes finished `MatrixJob`s (see `history.ts`) and the
	 * `meta` snippet adds an integration's own line under each row.
	 */
	import type { Snippet } from 'svelte';
	import { Plate, type Tone } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { formatAgo, formatBytes, formatSpan, formatUnix } from '../pbs/format';
	import type { DayState, MatrixDay, MatrixJob, MatrixRun, RunOutcome } from './history';

	interface Props {
		jobs: MatrixJob[];
		/** Under this age a success is "on time" (a day by default). */
		lateAfterSeconds?: number;
		/** Beyond this age the plate warns (two days by default). */
		missingAfterSeconds?: number;
		/** One more line under a row, after duration and size. */
		meta?: Snippet<[MatrixJob]>;
	}

	let { jobs, lateAfterSeconds = 86_400, missingAfterSeconds = 172_800, meta }: Props = $props();

	/** Token backgrounds, plus a shape per state (see the style block). */
	const CELL: Record<DayState, string> = {
		ok: 'bg-signal rounded-full',
		warning: 'cell-warning bg-advisory rounded-sm',
		failed: 'cell-failed bg-warning rounded-sm',
		running: 'bg-info rounded-full',
		none: 'ghost-cell bg-ghost opacity-70 rounded-full',
		nodata: 'border border-dashed border-line-strong rounded-full'
	};

	const LEGEND: DayState[] = ['ok', 'warning', 'failed', 'running', 'none', 'nodata'];

	function stateWord(state: DayState): string {
		switch (state) {
			case 'ok':
				return m.backup_matrix_state_ok();
			case 'warning':
				return m.backup_matrix_state_warning();
			case 'failed':
				return m.backup_matrix_state_failed();
			case 'running':
				return m.backup_matrix_state_running();
			case 'nodata':
				return m.backup_matrix_state_nodata();
			default:
				return m.backup_matrix_state_none();
		}
	}

	function outcomeWord(outcome: RunOutcome): string {
		switch (outcome) {
			case 'ok':
				return m.backup_matrix_outcome_ok();
			case 'warning':
				return m.backup_matrix_outcome_warning();
			case 'failed':
				return m.backup_matrix_outcome_failed();
			default:
				return m.backup_matrix_outcome_running();
		}
	}

	function runLine(run: MatrixRun): string {
		const time = formatUnix(run.at).split(', ').at(-1) ?? '';
		const parts = [time, outcomeWord(run.outcome)];
		if (run.durationSeconds !== null) parts.push(formatSpan(run.durationSeconds));
		if (run.bytes !== null) parts.push(formatBytes(run.bytes));
		if (run.note) parts.push(run.note);
		return parts.join(' · ');
	}

	/** The day's lines: its date and state, each run, then the adapter's notes. */
	function lines(day: MatrixDay): string[] {
		return [`${day.date} · ${stateWord(day.state)}`, ...day.runs.map(runLine), ...(day.notes ?? [])];
	}

	/** Rows that are failing first, then the oldest last success, then by name. */
	const rows = $derived.by(() => {
		const list = [...jobs];
		list.sort((a, b) => {
			const fa = a.failing ? 0 : 1;
			const fb = b.failing ? 0 : 1;
			if (fa !== fb) return fa - fb;
			const la = a.lastSuccess ?? 0;
			const lb = b.lastSuccess ?? 0;
			if (la !== lb) return la - lb;
			return a.title.localeCompare(b.title, 'en');
		});
		return list;
	});

	const failingCount = $derived(jobs.filter((j) => j.failing).length);
	const okToday = $derived(jobs.filter((j) => j.days.at(-1)?.state === 'ok').length);
	const firstDate = $derived(jobs[0]?.days[0]?.date ?? '');

	/** The state of the newest day that holds any information. */
	function newestState(job: MatrixJob): DayState {
		for (let i = job.days.length - 1; i >= 0; i--) {
			const state = job.days[i].state;
			if (state !== 'none' && state !== 'nodata') return state;
		}
		return 'none';
	}

	/** The plate word does the alerting: failure, then the age of the last success. */
	function plateOf(job: MatrixJob): { tone: Tone; label: string } {
		if (job.failing) return { tone: 'warning', label: m.backup_matrix_plate_failed() };
		if (job.running) return { tone: 'info', label: m.backup_matrix_plate_running() };
		if (job.lastSuccess === null) return { tone: 'ghost', label: m.backup_matrix_plate_none() };
		const age = Date.now() / 1000 - job.lastSuccess;
		const ago = formatAgo(job.lastSuccess);
		if (age < lateAfterSeconds) {
			return newestState(job) === 'warning'
				? { tone: 'advisory', label: m.backup_matrix_plate_warning({ ago }) }
				: { tone: 'signal', label: m.backup_matrix_plate_ok({ ago }) };
		}
		if (age <= missingAfterSeconds) return { tone: 'advisory', label: m.backup_matrix_plate_late({ ago }) };
		return { tone: 'warning', label: m.backup_matrix_plate_missing({ span: formatSpan(age) }) };
	}

	function ariaSummary(job: MatrixJob): string {
		const ok = job.days.filter((d) => d.state === 'ok' || d.state === 'warning').length;
		const failed = job.days.filter((d) => d.state === 'failed').length;
		return m.backup_matrix_aria_summary({ ok, total: job.days.length, failed });
	}

	/** The cell whose lines are written under its row (tap, Enter or Space). */
	let picked = $state<{ key: string; index: number } | null>(null);

	function pick(key: string, index: number) {
		picked = picked && picked.key === key && picked.index === index ? null : { key, index };
	}
</script>

{#if jobs.length > 0}
	<div class="flex flex-wrap items-center gap-x-4 gap-y-1.5 border-b border-line px-4 py-2.5 sm:px-5 text-[0.75rem] text-ink-2">
		<span class="tnum">
			{failingCount > 0
				? m.backup_matrix_summary_today_failing({ ok: okToday, total: jobs.length, failing: failingCount })
				: m.backup_matrix_summary_today({ ok: okToday, total: jobs.length })}
		</span>
		<ul class="m-0 flex list-none flex-wrap items-center gap-x-3 gap-y-1 p-0 sm:ml-auto" aria-label={m.backup_matrix_legend()}>
			{#each LEGEND as state (state)}
				<li class="inline-flex items-center gap-1.5">
					<span class={`inline-block h-3 w-3 ${CELL[state]}`} aria-hidden="true"></span>{stateWord(state)}
				</li>
			{/each}
		</ul>
	</div>
	<ul class="divide-y divide-line">
		{#each rows as job, i (job.key)}
			{@const plate = plateOf(job)}
			<li class="rise-in px-4 py-3 sm:px-5" style="--rise-delay: {Math.min(i, 8) * 40}ms">
				<div class="flex flex-col gap-2 lg:flex-row lg:items-center lg:gap-4">
					<div class="min-w-0 lg:w-64 lg:shrink-0">
						<p class="truncate font-semibold text-ink" title={job.title}>{job.title}</p>
						{#if job.subtitle}
							<p class="truncate text-[0.75rem] text-ink-3" title={job.subtitle}>{job.subtitle}</p>
						{/if}
					</div>
					<div
						class="flex min-w-0 flex-1 items-center gap-[3px]"
						role="group"
						aria-label={m.backup_matrix_aria_group({ group: job.title, summary: ariaSummary(job) })}
					>
						{#each job.days as day, index (day.date)}
							<button
								type="button"
								class="cell-hit group flex h-10 min-w-0 flex-1 items-center justify-center"
								title={lines(day).join('\n')}
								aria-label={lines(day).join(', ')}
								aria-pressed={picked?.key === job.key && picked.index === index}
								onclick={() => pick(job.key, index)}
							>
								<span class={`block h-4 w-full ${CELL[day.state]} ${picked?.key === job.key && picked.index === index ? 'outline outline-2 outline-offset-1 outline-ink' : ''}`}></span>
							</button>
						{/each}
					</div>
					<div class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1 lg:w-56 lg:justify-end">
						<Plate tone={plate.tone} label={plate.label} />
					</div>
				</div>
				{#if picked?.key === job.key}
					<ul class="tnum mt-2 flex list-none flex-col gap-0.5 rounded-md bg-surface-2 px-3 py-2 text-[0.8125rem] text-ink-2" aria-live="polite">
						{#each lines(job.days[picked.index]) as line, n (n)}
							<li class={n === 0 ? 'font-semibold text-ink' : ''}>{line}</li>
						{/each}
					</ul>
				{/if}
				<p class="tnum mt-1.5 flex flex-wrap gap-x-3 text-[0.8125rem] text-ink-2">
					{#if job.durationSeconds !== null}<span>{m.backup_matrix_meta_duration({ duration: formatSpan(job.durationSeconds) })}</span>{/if}
					{#if job.bytes !== null}<span>{m.backup_matrix_meta_size({ size: formatBytes(job.bytes) })}</span>{/if}
					{@render meta?.(job)}
				</p>
			</li>
		{/each}
	</ul>
	<div class="flex items-center justify-between gap-3 border-t border-line px-5 py-2 text-[0.75rem] text-ink-3">
		<span class="tnum">{firstDate}</span>
		<span>{m.backup_matrix_today()}</span>
	</div>
{/if}

<style>
	/* Warning: hatched, so it differs from a success without its colour. */
	.cell-warning {
		background-image: repeating-linear-gradient(135deg, transparent 0 2px, var(--c-surface) 2px 3.5px);
	}
	/* Failure: a cross in the cell, readable down to a few pixels. */
	.cell-failed {
		position: relative;
	}
	.cell-failed::before,
	.cell-failed::after {
		content: '';
		position: absolute;
		left: 50%;
		top: 50%;
		width: 2px;
		height: 70%;
		background: var(--c-surface);
		transform: translate(-50%, -50%) rotate(45deg);
	}
	.cell-failed::after {
		transform: translate(-50%, -50%) rotate(-45deg);
	}
	.cell-hit:focus-visible {
		outline: 2px solid var(--c-focus);
		outline-offset: 1px;
		border-radius: 0.25rem;
	}
</style>
