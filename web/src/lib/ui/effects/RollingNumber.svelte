<script lang="ts">
	/**
	 * A figure that rolls like an odometer when it changes: the old value
	 * leaves upward and the new one arrives from below when it grows, the other
	 * way round when it shrinks. Both share one grid cell, clipped, so the
	 * width never jumps. Under `prefers-reduced-motion` the value simply swaps.
	 */
	import { untrack } from 'svelte';
	import { fly } from 'svelte/transition';
	import { reducedMotion } from '../motion';

	interface Props {
		value: number;
		class?: string;
	}

	let { value, class: className = '' }: Props = $props();

	let shown = $state(untrack(() => value));
	let direction = $state(1);

	$effect(() => {
		const next = value;
		untrack(() => {
			if (next === shown) return;
			direction = next > shown ? 1 : -1;
			shown = next;
		});
	});

	const duration = () => (reducedMotion() ? 0 : 280);
</script>

<span class={`tnum relative inline-grid overflow-hidden ${className}`} data-numeric>
	{#key shown}
		<span
			class="[grid-area:1/1]"
			in:fly={{ y: 9 * direction, duration: duration(), opacity: 0 }}
			out:fly={{ y: -9 * direction, duration: duration(), opacity: 0 }}
		>
			{shown}
		</span>
	{/key}
</span>
