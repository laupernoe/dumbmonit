<script lang="ts">
	/**
	 * The instance's deadlines: the 90-day timeline (larger screens), then the
	 * soonest ones as a list, the rest behind "Show all". Renders nothing until
	 * at least one expiry date has been collected, so an instance without
	 * domains, certificates or licences never sees an empty frame.
	 */
	import { onMount } from 'svelte';
	import { listDeadlines } from '#lib/api/deadlines.js';
	import type { Deadline } from '#lib/api/types.js';
	import { m } from '#lib/paraglide/messages.js';
	import { ErrorNotice, Panel, Skeleton } from '#lib/ui/index.js';
	import { TONE_GLYPH, TONE_INK, TONE_SOFT } from '#lib/viz/index.js';
	import type { Tone } from '#lib/viz/index.js';
	import DeadlineList from './DeadlineList.svelte';
	import DeadlineTimeline from './DeadlineTimeline.svelte';
	import { HORIZON_DAYS } from './geometry.js';

	interface Props {
		class?: string;
	}
	let { class: cls = '' }: Props = $props();

	const PREVIEW = 6;
	let items = $state<Deadline[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let all = $state(false);

	const shown = $derived(all ? items : items.slice(0, PREVIEW));
	const LEGEND: { tone: Tone; label: () => string }[] = [
		{ tone: 'warning', label: () => m.deadlines_tier_urgent() },
		{ tone: 'advisory', label: () => m.deadlines_tier_soon() },
		{ tone: 'info', label: () => m.deadlines_tier_notice() },
		{ tone: 'signal', label: () => m.deadlines_tier_calm() }
	];

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			items = await listDeadlines({ signal });
		} catch (e) {
			if (signal?.aborted) return;
			error = e;
		} finally {
			if (!signal?.aborted) loading = false;
		}
	}

	onMount(() => {
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});
</script>

{#if loading}
	<Skeleton class={`h-24 w-full ${cls}`} />
{:else if error}
	<ErrorNotice {error} title={m.deadlines_load_error()} onretry={() => void load()} class={cls} />
{:else if items.length > 0}
	<Panel title={m.deadlines_title()} description={m.deadlines_description({ days: HORIZON_DAYS })} class={cls}>
		<DeadlineTimeline {items} />
		<ul class="m-0 mt-3 flex list-none flex-wrap gap-x-4 gap-y-1 p-0 text-[0.8125rem] text-ink-2 max-sm:hidden">
			{#each LEGEND as entry (entry.tone)}
				<li class="flex items-center gap-1.5">
					<span
						class="grid size-5 place-items-center rounded-full text-[0.6875rem] font-semibold"
						style:background={TONE_SOFT[entry.tone]}
						style:color={TONE_INK[entry.tone]}
						aria-hidden="true">{TONE_GLYPH[entry.tone]}</span
					>
					{entry.label()}
				</li>
			{/each}
		</ul>
		<DeadlineList items={shown} class="mt-3" />
		{#if items.length > PREVIEW}
			<button
				type="button"
				class="mt-2 inline-flex min-h-10 items-center text-sm font-semibold text-ink underline decoration-line-strong underline-offset-2 hover:decoration-ink"
				aria-expanded={all}
				onclick={() => (all = !all)}
			>
				{all ? m.deadlines_show_fewer() : m.deadlines_show_all({ count: items.length })}
			</button>
		{/if}
	</Panel>
{/if}
