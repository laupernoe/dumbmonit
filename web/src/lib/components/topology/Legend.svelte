<script lang="ts">
	/**
	 * What the lines, rings and badges of the map mean, shown under it. The
	 * exported sheet draws its own copy.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { EDGE_DASH, SOURCES, sourceLabel } from './legend';

	interface Props {
		/** Sites are drawn: the WAN link is part of the legend. */
		wan?: boolean;
		class?: string;
	}

	let { wan = false, class: className = '' }: Props = $props();
</script>

<div class={`flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-ink-2 ${className}`}>
	<span class="font-semibold text-ink">{m.topology_legend_title()}</span>
	{#each SOURCES as source (source)}
		<span class="inline-flex items-center gap-2">
			<svg width="34" height="10" aria-hidden="true" class="shrink-0">
				{#if source === 'parent'}
					<line x1="2" y1="5" x2="26" y2="5" class="stroke-ink-2" stroke-width="1.6" stroke-linecap="round" />
					<polygon points="32,5 24,1 24,9" class="fill-ink-2" />
				{:else}
					<line x1="2" y1="5" x2="32" y2="5" class="stroke-ink-3" stroke-width="1.3" stroke-linecap="round" stroke-dasharray={EDGE_DASH[source]} />
				{/if}
			</svg>
			{sourceLabel(source)}
		</span>
	{/each}
	{#if wan}
		<span class="inline-flex items-center gap-2">
			<svg width="34" height="10" aria-hidden="true" class="shrink-0">
				<line x1="4" y1="5" x2="30" y2="5" class="stroke-ink-3" stroke-width="7" stroke-linecap="round" opacity="0.14" />
				<line x1="4" y1="5" x2="30" y2="5" class="stroke-ink-3" stroke-width="2.2" stroke-linecap="round" stroke-dasharray={EDGE_DASH.wan} />
			</svg>
			{sourceLabel('wan')}
		</span>
	{/if}
	<span class="inline-flex items-center gap-2">
		<svg width="18" height="18" aria-hidden="true" class="shrink-0">
			<circle cx="9" cy="9" r="7" class="fill-warning-soft stroke-warning" stroke-width="2.5" />
			<path d="M9,5.6 L9,9.6 M9,12.2 L9,12.3" class="stroke-warning-ink" stroke-width="1.8" stroke-linecap="round" />
		</svg>
		{m.topology_legend_down()}
	</span>
	<span class="inline-flex items-center gap-2">
		<svg width="18" height="18" aria-hidden="true" class="shrink-0">
			<circle cx="9" cy="9" r="7" class="fill-surface stroke-advisory" stroke-width="1.8" stroke-dasharray="3 2.5" />
			<path d="M9,5.6 L9,12 M6.4,9.6 L9,12.2 L11.6,9.6" fill="none" class="stroke-advisory-ink" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
		</svg>
		{m.topology_legend_suppressed()}
	</span>
</div>
