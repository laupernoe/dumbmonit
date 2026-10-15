<script lang="ts">
	/**
	 * What the lines and outlines of the map mean. Shown under the map and
	 * on the printed sheet; the exported image draws its own copy.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { EDGE_DASH, SOURCES, sourceLabel } from './legend';

	interface Props {
		class?: string;
	}

	let { class: className = '' }: Props = $props();
</script>

<div class={`flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-ink-2 ${className}`}>
	<span class="font-semibold text-ink">{m.topology_legend_title()}</span>
	{#each SOURCES as source (source)}
		<span class="inline-flex items-center gap-2">
			<svg width="34" height="10" aria-hidden="true" class="shrink-0">
				<line x1="2" y1="5" x2="32" y2="5" class="stroke-ink-3" stroke-width="1.8" stroke-linecap="round" stroke-dasharray={EDGE_DASH[source] || undefined} />
			</svg>
			{sourceLabel(source)}
		</span>
	{/each}
	<span class="inline-flex items-center gap-2">
		<svg width="26" height="16" aria-hidden="true" class="shrink-0">
			<rect x="1.5" y="1.5" width="23" height="13" rx="3" class="fill-warning-soft stroke-warning" stroke-width="2.5" />
		</svg>
		{m.topology_legend_down()}
	</span>
	<span class="inline-flex items-center gap-2">
		<svg width="26" height="16" aria-hidden="true" class="shrink-0">
			<rect x="1" y="1" width="24" height="14" rx="3" class="fill-surface stroke-advisory" stroke-width="1.5" stroke-dasharray="5 3" />
		</svg>
		{m.topology_legend_impacted()}
	</span>
</div>
