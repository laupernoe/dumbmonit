<script lang="ts">
	/**
	 * Scheduled maintenance: the silences, each as a row. A window covering now
	 * is "Active now"; otherwise it is "Scheduled". Removing one is a two-step
	 * confirm — a silence that vanishes by accident lets a real alert through.
	 */
	import type { Silence, Target } from '#lib/api/index.js';
	import { Confirm, EmptyState, Plate, Button } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { CalendarClock } from 'lucide-svelte';
	import { formatDateTime } from '#lib/format.js';
	import { scheduleLabel, silenceScope } from './helpers';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		silences: Silence[];
		targets: Map<number, Target>;
		removingId?: number | null;
		onremove: (id: number) => void;
		onschedule: () => void;
	}

	let { silences, targets, removingId = null, onremove, onschedule }: Props = $props();
</script>

{#if silences.length === 0}
	<EmptyState
		icon={CalendarClock}
		title={m.alerts_silences_empty_title()}
		description={m.alerts_silences_empty_description()}
	>
		{#snippet action()}
			{#if auth.canOperate}
				<Button variant="primary" onclick={onschedule}>{m.alerts_silences_schedule()}</Button>
			{/if}
		{/snippet}
	</EmptyState>
{:else}
	<div class="space-y-2.5">
		{#each silences as silence, i (silence.id)}
			<div
				class="rise-in flex flex-wrap items-start gap-x-4 gap-y-2 rounded-[var(--radius-card)] border border-line bg-surface px-4 py-3 shadow-lift"
				style={`--rise-delay: ${i * 30}ms`}
			>
				<div class="min-w-0 flex-1">
					<div class="flex flex-wrap items-center gap-2">
						{#if silence.active_now}
							<Plate tone="signal" label={m.alerts_silences_active_now()} pulse />
						{:else}
							<Plate tone="ghost" label={m.alerts_silences_scheduled()} />
						{/if}
						<span class="truncate font-semibold text-ink">{silence.name}</span>
					</div>
					<div class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[0.8125rem] text-ink-2">
						<span>{silenceScope(silence, targets)}</span>
						<span class="text-ink-3" aria-hidden="true">·</span>
						<span class="tnum">{scheduleLabel(silence.schedule)}</span>
						{#if silence.active_now && silence.active_until}
							<span class="text-ink-3" aria-hidden="true">·</span>
							<span class="tnum">{m.alerts_silences_until({ when: formatDateTime(silence.active_until) })}</span>
						{:else if silence.next_start_at}
							<span class="text-ink-3" aria-hidden="true">·</span>
							<span class="tnum">{m.alerts_silences_next({ when: formatDateTime(silence.next_start_at) })}</span>
						{/if}
					</div>
					{#if silence.comment}
						<p class="mt-1 text-[0.8125rem] text-ink-2">{silence.comment}</p>
					{/if}
				</div>
				{#if auth.canOperate}
					<Confirm
						size="sm"
						variant="danger"
						confirmLabel={m.alerts_silences_remove_confirm()}
						loading={removingId === silence.id}
						onconfirm={() => onremove(silence.id)}
					>
						{m.alerts_silences_remove()}
					</Confirm>
				{/if}
			</div>
		{/each}
	</div>
{/if}
