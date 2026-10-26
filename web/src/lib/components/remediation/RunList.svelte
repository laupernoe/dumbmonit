<script lang="ts">
	/**
	 * The audit log of remediation runs: who, what, where, when, and how it
	 * ended. A run's output (already truncated by the server) unfolds in place.
	 */
	import type { RemediationRun } from '#lib/api/index.js';
	import { Plate } from '#lib/ui/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { runLabel, runTone, utc } from './status';

	interface Props {
		runs: RemediationRun[];
		/** Show the device name (off on a device's own page). */
		showTarget?: boolean;
	}

	let { runs, showTarget = true }: Props = $props();
	let open = $state<number | null>(null);
</script>

{#if runs.length === 0}
	<p class="text-sm text-ink-2">{m.remediation_runs_empty()}</p>
{:else}
	<ul class="divide-y divide-line">
		{#each runs as run (run.id)}
			<li class="py-2.5">
				<button
					type="button"
					class="flex min-h-10 w-full min-w-0 flex-wrap items-center gap-x-3 gap-y-1 rounded-md text-left focus-visible:outline-2 focus-visible:outline-signal"
					aria-expanded={open === run.id}
					onclick={() => (open = open === run.id ? null : run.id)}
				>
					<Plate tone={runTone(run.status)} label={runLabel(run.status)} class="shrink-0" />
					<span class="min-w-0 flex-1">
						<span class="block truncate font-medium text-ink">{run.action_name}</span>
						<span class="block truncate text-[0.8125rem] text-ink-2">
							{#if showTarget && run.target_name}{m.remediation_runs_on({ target: run.target_name })} · {/if}
							{run.trigger === 'auto'
								? m.remediation_runs_auto({ id: String(run.policy_id ?? '') })
								: m.remediation_runs_manual({ user: run.requested_by })}
						</span>
					</span>
					<time class="tnum shrink-0 text-[0.8125rem] text-ink-2" datetime={utc(run.created_at)} title={formatDateTime(utc(run.created_at))}>
						{formatRelative(utc(run.created_at))}
					</time>
				</button>
				{#if open === run.id}
					<div class="mt-2">
						<p class="text-[0.75rem] font-medium uppercase tracking-wide text-ink-3">{m.remediation_runs_output()}</p>
						<pre class="mt-1 max-h-64 overflow-auto whitespace-pre-wrap break-words rounded-md border border-line bg-canvas-deep p-3 font-mono text-[0.75rem] text-ink">{run.output || '—'}</pre>
					</div>
				{/if}
			</li>
		{/each}
	</ul>
{/if}
