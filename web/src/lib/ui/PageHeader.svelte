<script lang="ts">
	/** Page title row: the heading carries its own weight, no eyebrow above it. */
	import type { Snippet } from 'svelte';
	import BlurText from './effects/BlurText.svelte';

	interface Props {
		title: string;
		description?: string;
		actions?: Snippet;
		/** Breadcrumb-ish back link. */
		back?: { href: string; label: string };
		/** The title settles in from a blur, once, instead of appearing flat. */
		reveal?: boolean;
		class?: string;
	}
	let { title, description, actions, back, reveal = true, class: className = '' }: Props = $props();
</script>

<div class={`mb-3 flex flex-wrap items-end justify-between gap-x-6 gap-y-2 sm:mb-6 sm:gap-y-3 ${className}`}>
	<div class="min-w-0">
		{#if back}
			<a href={back.href} class="-mt-2 mb-1 inline-flex min-h-10 items-center gap-1 text-sm sm:mt-0 sm:min-h-0 text-ink-2 hover:text-ink hover:underline">← {back.label}</a>
		{/if}
		{#if reveal}
			<BlurText tag="h1" text={title} class="display text-xl text-ink sm:text-3xl" />
		{:else}
			<h1 class="display text-xl text-ink sm:text-3xl">{title}</h1>
		{/if}
		<!-- Phones: the title alone; the sentence under it is for the larger screen. -->
		{#if description}<p class="mt-1.5 max-w-2xl text-sm text-ink-2 max-sm:hidden sm:text-[0.9375rem]">{description}</p>{/if}
	</div>
	{#if actions}<div class="flex max-w-full flex-wrap items-center gap-2">{@render actions()}</div>{/if}
</div>
