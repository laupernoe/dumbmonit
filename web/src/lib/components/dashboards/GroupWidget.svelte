<script lang="ts">
	/** State of the monitors in one folder, from the shared store the rack reads. */
	import { m } from '#lib/paraglide/messages.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { STATE_LABEL, STATE_TONE, displayState, type TargetState } from '#lib/format.js';
	import { EmptyState, Led, Plate, Skeleton } from '#lib/ui/index.js';
	import type { Tone } from '#lib/ui/index.js';

	interface Props {
		folder: string;
	}

	let { folder }: Props = $props();

	/** From the worst: what to read first. */
	const SEVERITY: TargetState[] = ['offline', 'down', 'misconfigured', 'pending', 'unknown', 'disabled', 'online'];

	const rows = $derived(
		alertsStore.targets
			.filter((t) => t.group_name === folder)
			.map((t) => ({ target: t, state: displayState(t, alertsStore.probes.get(t.id)) }))
			.sort((a, b) => SEVERITY.indexOf(a.state) - SEVERITY.indexOf(b.state) || a.target.name.localeCompare(b.target.name))
	);
	const worst = $derived(rows.find((r) => r.state !== 'disabled')?.state ?? 'disabled');
	const problems = $derived(rows.filter((r) => r.state !== 'online' && r.state !== 'disabled').length);
	const summaryTone = $derived<Tone>(problems === 0 && rows.length > 0 ? 'signal' : STATE_TONE[worst]);
</script>

{#if alertsStore.loading}
	<Skeleton class="h-16 w-full" />
{:else if rows.length === 0}
	<EmptyState title={m.dash_group_empty_title()} description={m.dash_group_empty_description()} />
{:else}
	<div class="flex flex-col gap-2">
		<Plate tone={summaryTone} label={problems === 0 ? m.dash_group_all_ok({ count: rows.length }) : m.dash_group_problems({ problems, count: rows.length })} />
		<ul class="flex flex-col">
			{#each rows as row (row.target.id)}
				<li>
					<a href={`/targets/${row.target.id}`} class="flex min-h-10 items-center gap-2.5 rounded-lg px-1 text-sm hover:bg-surface-2">
						<Led tone={STATE_TONE[row.state]} size="sm" />
						<span class="min-w-0 flex-1 truncate text-ink">{row.target.name}</span>
						<span class="shrink-0 text-xs text-ink-2">{STATE_LABEL[row.state]}</span>
					</a>
				</li>
			{/each}
		</ul>
	</div>
{/if}
