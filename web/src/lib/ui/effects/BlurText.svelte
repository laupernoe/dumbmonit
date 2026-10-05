<script lang="ts">
	/**
	 * Text that arrives word by word, each one sharpening out of a blur as it
	 * settles. Svelte 5 port of the React Bits "BlurText" component (CSS
	 * transitions only, no dependencies). For a page title's first paint —
	 * once per distinct text, never on every re-render. Under
	 * `prefers-reduced-motion` the text is simply there, already sharp.
	 */
	import { onMount } from 'svelte';
	import { reducedMotion } from '../motion';

	interface Props {
		text: string;
		/** Milliseconds between one word and the next. */
		stagger?: number;
		class?: string;
		tag?: 'span' | 'h1' | 'h2' | 'h3' | 'p';
	}

	let { text, stagger = 28, class: className = '', tag = 'span' }: Props = $props();

	const words = $derived(text.split(' '));

	let settled = $state(false);
	let last: string | null = null;

	function run() {
		settled = false;
		if (reducedMotion()) {
			settled = true;
			return;
		}
		// Two frames: the words must paint blurred at least once before the
		// class flips, or the browser coalesces both states into one.
		requestAnimationFrame(() => requestAnimationFrame(() => (settled = true)));
	}

	onMount(run);

	$effect(() => {
		const next = text;
		if (last !== null && next !== last) run();
		last = next;
	});
</script>

<svelte:element this={tag} class={className} aria-label={text}>
	<span aria-hidden="true">
		{#each words as word, i (i)}<span
				class="inline-block [transition:filter_480ms_var(--ease-out-expo),opacity_480ms_var(--ease-out-expo),transform_480ms_var(--ease-out-expo)]"
				style:transition-delay={`${Math.min(i, 14) * stagger}ms`}
				style:filter={settled ? 'blur(0)' : 'blur(10px)'}
				style:opacity={settled ? 1 : 0}
				style:transform={settled ? 'none' : 'translateY(0.35em)'}>{word}</span
			>{i < words.length - 1 ? ' ' : ''}{/each}
	</span>
</svelte:element>
