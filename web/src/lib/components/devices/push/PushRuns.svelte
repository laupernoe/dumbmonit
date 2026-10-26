<script lang="ts">
	/**
	 * Run history of a heartbeat job: status (icon and word, never colour
	 * alone), when, how long, the exit code, and the output the job sent,
	 * folded away until asked for. Newest first, older pages on demand.
	 * `refreshKey` changes whenever the monitor sees a new call, which reloads
	 * the first page.
	 */
	import { untrack } from 'svelte';
	import { ChevronRight } from 'lucide-svelte';
	import { listPushRuns } from '#lib/api/push.js';
	import type { PushRun, Target } from '#lib/api/index.js';
	import { formatDateTime, formatDuration } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Button, ErrorNotice, Panel, Plate, Skeleton, type Tone } from '#lib/ui/index.js';

	interface Props {
		target: Target;
		refreshKey?: number;
	}

	let { target, refreshKey = 0 }: Props = $props();

	let runs = $state<PushRun[]>([]);
	let nextBefore = $state<number | null>(null);
	let loading = $state(true);
	let loadingMore = $state(false);
	let error = $state<unknown>(null);
	let open = $state<Record<number, boolean>>({});

	async function loadFirst(signal?: AbortSignal) {
		error = null;
		try {
			const page = await listPushRuns(target.id, { limit: 20 }, signal);
			runs = page.runs;
			nextBefore = page.next_before;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	async function loadMore() {
		if (nextBefore === null) return;
		loadingMore = true;
		try {
			const page = await listPushRuns(target.id, { limit: 20, before: nextBefore });
			runs = [...runs, ...page.runs];
			nextBefore = page.next_before;
		} catch (cause) {
			error = cause;
		} finally {
			loadingMore = false;
		}
	}

	$effect(() => {
		void target.id;
		void refreshKey;
		const controller = new AbortController();
		untrack(() => {
			// Keep what is shown during a refresh; only the first load shows a skeleton.
			if (runs.length === 0) loading = true;
		});
		void loadFirst(controller.signal);
		return () => controller.abort();
	});

	function statusOf(run: PushRun): { tone: Tone; label: string } {
		switch (run.status) {
			case 'running':
				return { tone: 'info', label: m.push_runs_status_running() };
			case 'success':
				return { tone: 'signal', label: m.push_runs_status_success() };
			case 'failed':
				return { tone: 'warning', label: m.push_runs_status_failed() };
			default:
				return { tone: 'advisory', label: m.push_runs_status_lost() };
		}
	}

	function when(run: PushRun): string {
		const at = run.finished_at ?? run.started_at;
		return at ? formatDateTime(at) : '';
	}
</script>

<Panel title={m.push_runs_title()} description={m.push_runs_description()}>
	{#if error && runs.length === 0}
		<ErrorNotice {error} title={m.push_runs_load_error()} onretry={() => void loadFirst()} />
	{:else if loading}
		<div class="grid gap-2" aria-busy="true">
			<Skeleton class="h-11 w-full rounded-lg" />
			<Skeleton class="h-11 w-full rounded-lg" />
		</div>
	{:else if runs.length === 0}
		<p class="text-sm text-ink-2">{m.push_runs_empty()}</p>
	{:else}
		<ul class="m-0 flex list-none flex-col divide-y divide-line p-0">
			{#each runs as run (run.id)}
				{@const status = statusOf(run)}
				<li class="py-2.5 first:pt-0 last:pb-0">
					<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
						<Plate tone={status.tone} label={status.label} />
						<span class="tnum text-sm text-ink">{when(run)}</span>
						<span class="text-[0.8125rem] text-ink-2">
							{#if run.duration_secs !== null}
								{m.push_runs_duration({ duration: formatDuration(run.duration_secs) })}
							{:else}
								{m.push_runs_duration_unknown()}
							{/if}
						</span>
						{#if run.exit_code !== null}
							<code class="rounded bg-surface-2 px-1.5 py-0.5 text-[0.8125rem] text-ink">
								{m.push_runs_exit_code({ code: run.exit_code })}
							</code>
						{/if}
						{#if run.output}
							<button
								type="button"
								class="ml-auto inline-flex min-h-10 items-center gap-1 rounded-md px-2 text-[0.8125rem] font-medium text-ink-2 hover:text-ink focus-visible:outline-2 focus-visible:outline-signal"
								aria-expanded={open[run.id] === true}
								aria-controls={`push-run-${run.id}`}
								onclick={() => (open[run.id] = !open[run.id])}
							>
								<ChevronRight
									class={`size-4 transition-transform duration-200 ${open[run.id] ? 'rotate-90' : ''}`}
									aria-hidden="true"
								/>
								{open[run.id] ? m.push_runs_hide_output() : m.push_runs_show_output()}
							</button>
						{/if}
					</div>
					{#if run.output}
						<pre
							id={`push-run-${run.id}`}
							hidden={!open[run.id]}
							class="mt-2 max-h-72 overflow-auto whitespace-pre-wrap break-words rounded-lg border border-line bg-surface-2 p-3 font-mono text-[0.75rem] leading-relaxed text-ink">{run.output}</pre>
					{/if}
				</li>
			{/each}
		</ul>
		{#if nextBefore !== null}
			<div class="mt-3 border-t border-line pt-3">
				<Button variant="secondary" loading={loadingMore} onclick={() => void loadMore()}>
					{m.push_runs_load_more()}
				</Button>
			</div>
		{/if}
		{#if error}
			<div class="mt-3"><ErrorNotice {error} title={m.push_runs_load_error()} /></div>
		{/if}
	{/if}
</Panel>
