<script lang="ts">
	/**
	 * A tiny burst of sparks at the click point, around any control. A primary
	 * `Button` already sparks on its own (`use:sparkle`); this wrapper is for
	 * the rest. The burst is drawn over the page (see `spark.ts`): the wrapper
	 * adds no canvas and nothing for a screen reader.
	 */
	import type { Snippet } from 'svelte';
	import { sparkFor } from './spark';

	interface Props {
		children: Snippet;
		count?: number;
		size?: number;
		radius?: number;
		duration?: number;
		color?: string;
		class?: string;
	}

	let { children, count = 8, size = 9, radius = 18, duration = 420, color = 'var(--c-signal)', class: className = '' }: Props = $props();

	let host = $state<HTMLDivElement | null>(null);

	function burst(event: MouseEvent) {
		if (!host) return;
		sparkFor(event, host, { count, size, radius, duration, color });
	}
</script>

<!-- The wrapper only listens for the click that its interactive child already
     handles; keyboard activation of that child triggers a click too. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div bind:this={host} class={`relative inline-flex ${className}`} onclick={burst}>
	{@render children()}
</div>
