<script lang="ts">
	/** A small line of values, decorative: the figure next to it carries the number. */
	import { sparkPath } from './fleet.js';

	interface Props {
		values: number[];
		width?: number;
		height?: number;
		/** Stroke colour class (`stroke-signal`…). */
		stroke?: string;
		/** Draw the line in once, on first show. */
		draw?: boolean;
		class?: string;
	}
	let { values, width = 130, height = 20, stroke = 'stroke-ink-3', draw = false, class: cls = '' }: Props = $props();
	const path = $derived(sparkPath(values, width, height));
</script>

<svg {width} {height} viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" aria-hidden="true" class={`block max-w-full ${cls}`}>
	{#if path}
		<path d={path} fill="none" stroke-width="1.4" stroke-linejoin="round" stroke-linecap="round" class={`${stroke} ${draw ? 'ob-draw' : ''}`} />
	{:else}
		<line x1="0" x2={width} y1={height / 2} y2={height / 2} stroke-width="1" stroke-dasharray="2 4" class="stroke-line-strong" />
	{/if}
</svg>
