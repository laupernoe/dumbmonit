<script lang="ts">
	/**
	 * A slim "Did you know?" strip under the sky: the pigeon points at one
	 * thing DumbMonit does that the navigation does not show — an agent on
	 * another network, an AI assistant over MCP, the API, wall mode…
	 *
	 * Shown by the Overview only when nothing needs the reader (the page
	 * decides). One tip per visit, the next one on the next visit; "Another
	 * tip" turns it by hand, and the cross hides the strip for good in this
	 * browser. It never advances on its own: text does not move while read.
	 */
	import { onMount } from 'svelte';
	import { fly } from 'svelte/transition';
	import { ArrowRight, RefreshCw, X } from 'lucide-svelte';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { palette } from '#lib/stores/palette.svelte.js';
	import { Button, reducedMotion } from '#lib/ui/index.js';
	import Mascot from '#lib/components/Mascot.svelte';
	import { TIPS, hideTips, storeNextTip, takeTipIndex, tipsHidden } from './tips';

	const tips = $derived(TIPS.filter((tip) => auth.isAdmin || !tip.admin));

	let index = $state<number | null>(null);
	let hidden = $state(true);
	let bird = $state<HTMLElement | null>(null);

	onMount(() => {
		hidden = tipsHidden();
		if (!hidden) index = takeTipIndex();
	});

	const tip = $derived(index === null || tips.length === 0 ? null : tips[index % tips.length]);
	const enter = () => ({ y: 8, duration: reducedMotion() ? 0 : 320, opacity: 0 });

	function another() {
		if (index === null) return;
		index += 1;
		storeNextTip(index);
		// The pigeon hops as it hands over the next one.
		if (bird && !reducedMotion()) {
			bird.animate(
				[{ transform: 'translateY(0) rotate(-4deg)' }, { transform: 'translateY(-7px) rotate(3deg)', offset: 0.4 }, { transform: 'translateY(0) rotate(-4deg)' }],
				{ duration: 380, easing: 'cubic-bezier(0.22, 1, 0.36, 1)' }
			);
		}
	}

	function dismiss() {
		hideTips();
		hidden = true;
	}
</script>

{#if !hidden && tip}
	<aside
		aria-label="Did you know?"
		class="relative flex flex-col gap-3 rounded-[var(--radius-card)] border border-line bg-surface px-4 py-3 shadow-lift sm:flex-row sm:items-center sm:gap-4 sm:py-2.5 sm:pr-2.5"
	>
		<div class="flex min-w-0 flex-1 items-start gap-3 sm:items-center">
			<span bind:this={bird} class="-my-1 shrink-0 -rotate-[4deg]" aria-hidden="true">
				<Mascot mood="happy" class="size-10" />
			</span>
			<div class="grid min-w-0 flex-1" aria-live="polite">
				{#key tip.id}
					<p class="[grid-area:1/1] text-sm leading-snug text-ink-2" in:fly={enter()}>
						<span class="font-semibold text-ink">Did you know? {tip.title}</span>
						{tip.text}
					</p>
				{/key}
			</div>
		</div>
		<div class="flex shrink-0 items-center gap-1 pl-[3.25rem] sm:pl-0">
			{#if tip.href === 'palette'}
				<Button variant="secondary" size="sm" onclick={() => palette.open()}>
					{tip.action}
					<span class="tnum text-ink-3">{palette.shortcutLabel}</span>
				</Button>
			{:else}
				<Button variant="secondary" size="sm" href={tip.href}>
					<tip.icon class="size-4" aria-hidden="true" />
					{tip.action}
					<ArrowRight class="size-3.5" aria-hidden="true" />
				</Button>
			{/if}
			{#if tips.length > 1}
				<Button variant="ghost" size="sm" onclick={another} aria-label="Another tip" title="Another tip">
					<RefreshCw class="size-4" aria-hidden="true" />
					<span class="hidden lg:inline">Another tip</span>
				</Button>
			{/if}
			<button
				type="button"
				class="inline-flex size-8 items-center justify-center rounded-lg text-ink-3 transition-colors hover:bg-surface-2 hover:text-ink"
				onclick={dismiss}
				aria-label="Hide these tips"
				title="Hide these tips (in this browser)"
			>
				<X class="size-4" aria-hidden="true" />
			</button>
		</div>
	</aside>
{/if}
