<script lang="ts">
	/**
	 * Deadlines as a list, soonest first: one row per expiry, a glyph and the
	 * time left in words (never colour alone), the object and where it lives,
	 * the date. This is the form phones get; the timeline above it is only a
	 * quicker read on larger screens.
	 */
	import type { Deadline } from '#lib/api/types.js';
	import { formatDateTime } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { TONE_GLYPH, TONE_INK, TONE_SOFT } from '#lib/viz/index.js';
	import { deadlineTone } from './geometry.js';
	import { kindName, whenLabel } from './labels.js';

	interface Props {
		items: Deadline[];
		/** Hide the monitor name: the list sits on that monitor's own page. */
		hideTarget?: boolean;
		class?: string;
	}
	let { items, hideTarget = false, class: cls = '' }: Props = $props();
</script>

<ul class={`m-0 flex list-none flex-col p-0 ${cls}`} aria-label={m.deadlines_list_label()}>
	{#each items as item (`${item.target}|${item.kind}|${item.what}|${item.expires_at}`)}
		{@const tone = deadlineTone(item.days_left)}
		<li class="graticule min-w-0">
			<a
				href={`/targets/${item.target}`}
				class="flex min-h-11 min-w-0 items-center gap-3 py-1.5 hover:bg-surface-2"
				aria-label={m.deadlines_open({ name: item.target_name })}
			>
				<span
					class="grid size-7 shrink-0 place-items-center rounded-full text-[0.8125rem] font-semibold"
					style:background={TONE_SOFT[tone]}
					style:color={TONE_INK[tone]}
					aria-hidden="true">{TONE_GLYPH[tone]}</span
				>
				<span class="min-w-0 flex-1">
					<span class="block truncate text-sm font-semibold text-ink">{item.what}</span>
					<span class="block truncate text-[0.8125rem] text-ink-2">
						{kindName(item.kind)}{hideTarget ? '' : ` · ${item.target_name}`}
					</span>
				</span>
				<span class="shrink-0 text-right">
					<span class="tnum block text-sm font-semibold" style:color={TONE_INK[tone]}>{whenLabel(item.days_left)}</span>
					<span class="tnum block text-[0.8125rem] text-ink-3 max-sm:hidden">{formatDateTime(item.expires_at)}</span>
				</span>
			</a>
		</li>
	{/each}
</ul>
