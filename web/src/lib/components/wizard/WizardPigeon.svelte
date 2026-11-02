<script lang="ts">
	/**
	 * The pigeon with something to say, for the setup wizard and the add-a-monitor
	 * page. It is the same pigeon as everywhere else, with a speech bubble that
	 * changes with the step: a short sentence, plain words. The bubble is real
	 * text (read in its place, not announced: the step's own title does that).
	 * It eases in over 200 ms when the sentence changes and the pigeon hops
	 * once; under reduced motion both just change.
	 */
	import Mascot from '#lib/components/Mascot.svelte';
	import { reducedMotion } from '#lib/ui/index.js';

	interface Props {
		message: string;
		mood?: 'watch' | 'dizzy' | 'happy' | 'alert' | 'sleep';
		class?: string;
	}
	let { message, mood = 'watch', class: className = '' }: Props = $props();

	let host = $state<HTMLSpanElement | null>(null);
	let previous: string | null = null;
	$effect(() => {
		const next = message;
		if (previous !== null && previous !== next && host && !reducedMotion()) {
			host.animate(
				[{ transform: 'translateY(0)' }, { transform: 'translateY(-4px)', offset: 0.4 }, { transform: 'translateY(0)' }],
				{ duration: 280, easing: 'cubic-bezier(0.2, 0, 0, 1)' }
			);
		}
		previous = next;
	});
</script>

<div class={`flex items-start gap-3 ${className}`}>
	<span bind:this={host} class="mt-0.5 inline-flex shrink-0"><Mascot {mood} class="size-14" /></span>
	{#key message}
		<p class="bubble relative m-0 min-w-0 flex-1 rounded-xl border border-line-strong bg-surface-2 px-3.5 py-2.5 text-[0.8125rem] leading-snug text-ink">{message}</p>
	{/key}
</div>

<style>
	/* The tail points back at the pigeon: a rotated square sharing the bubble's skin. */
	.bubble::before {
		content: '';
		position: absolute;
		top: 1.125rem;
		left: -0.3125rem;
		width: 0.5625rem;
		height: 0.5625rem;
		background: var(--c-surface-2);
		border-left: 1px solid var(--c-line-strong);
		border-bottom: 1px solid var(--c-line-strong);
		transform: rotate(45deg);
	}
	.bubble {
		transform-origin: 0 1.4rem;
		animation: bubble-in 200ms cubic-bezier(0.2, 0, 0, 1) both;
	}
	@keyframes bubble-in {
		from {
			opacity: 0;
			transform: translateX(-4px) scale(0.96);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.bubble {
			animation: none;
		}
	}
</style>
