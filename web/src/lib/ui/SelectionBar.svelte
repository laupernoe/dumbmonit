<script lang="ts">
	/**
	 * The action bar of a selection mode: floats over the bottom of the page
	 * (above the phone tab bar), so turning selection on never pushes the list
	 * it acts on. It says how many are chosen, offers "select all" and the
	 * page's own actions, and Escape or "Done" leaves the mode.
	 *
	 * Text comes in through props, like every primitive here.
	 */
	import type { Snippet } from 'svelte';
	import { fly } from 'svelte/transition';
	import { X } from 'lucide-svelte';
	import { reducedMotion } from './motion';

	interface Props {
		/** "3 selected". */
		countLabel: string;
		/** "Select all (8)". */
		selectAllLabel: string;
		/** Accessible name of the toolbar. */
		label: string;
		/** "Done": leaves the selection mode. */
		exitLabel: string;
		/** Whether none, some or all of the rows are chosen. */
		selection: 'none' | 'some' | 'all';
		/** No row to pick: "select all" is disabled. */
		empty?: boolean;
		onselectall: (on: boolean) => void;
		onexit: () => void;
		/** A result to read out after an action ran (in place of nothing). */
		message?: { text: string; error: boolean } | null;
		children?: Snippet;
	}

	let {
		countLabel,
		selectAllLabel,
		label,
		exitLabel,
		selection,
		empty = false,
		onselectall,
		onexit,
		message = null,
		children
	}: Props = $props();

	let box = $state<HTMLInputElement | null>(null);
	$effect(() => {
		if (box) box.indeterminate = selection === 'some';
	});

	function onkeydown(event: KeyboardEvent) {
		// An open menu or field handles its own Escape first (they stop it).
		if (event.key === 'Escape' && !event.defaultPrevented) onexit();
	}

	const enter = () => ({ y: 16, duration: reducedMotion() ? 0 : 220 });
</script>

<svelte:window {onkeydown} />

<div
	class="pointer-events-none fixed inset-x-0 bottom-[calc(4.25rem+env(safe-area-inset-bottom))] z-40 flex justify-center px-4 sm:bottom-6"
	transition:fly={enter()}
>
	<div
		class="pointer-events-auto flex w-full max-w-4xl flex-wrap items-center gap-x-3 gap-y-2 rounded-[var(--radius-card)] border border-line-strong bg-surface px-3 py-2 shadow-float"
		role="toolbar"
		aria-label={label}
	>
		<label class="inline-flex min-h-10 cursor-pointer items-center gap-2 px-1 text-[0.8125rem] font-semibold text-ink">
			<input
				bind:this={box}
				type="checkbox"
				class="size-4 accent-[var(--c-signal)]"
				checked={selection === 'all'}
				disabled={empty}
				onchange={(e) => onselectall((e.currentTarget as HTMLInputElement).checked)}
			/>
			<span class="max-sm:sr-only">{selectAllLabel}</span>
		</label>
		<span
			class={`tnum min-w-0 flex-1 text-[0.8125rem] sm:whitespace-nowrap ${message?.error ? 'font-medium text-warning-ink' : 'text-ink-2'}`}
			aria-live="polite"
			role={message?.error ? 'alert' : undefined}
		>
			{message ? message.text : countLabel}
		</span>
		<button
			type="button"
			class="inline-flex h-10 items-center gap-1.5 rounded-lg px-3 text-[0.8125rem] font-semibold text-ink-2 transition-colors hover:bg-surface-2 hover:text-ink sm:order-last sm:h-8"
			onclick={onexit}
		>
			<X class="size-3.5" aria-hidden="true" />
			{exitLabel}
		</button>
		<div class="flex flex-wrap items-center gap-1.5 max-sm:order-last max-sm:w-full">
			{@render children?.()}
		</div>
	</div>
</div>
