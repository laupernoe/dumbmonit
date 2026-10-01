<script lang="ts">
	/**
	 * Label + control + help + error, in one rhythm. The control is rendered by
	 * the caller (an `.input`, a select, a toggle) so this stays agnostic.
	 * A new error gives the field a short "no" shake (not under reduced motion):
	 * the eye goes to the field that refused, not to the top of the form.
	 */
	import { untrack, type Snippet } from 'svelte';
	import { shake } from './motion';

	interface Props {
		children: Snippet;
		label: string;
		for?: string;
		help?: string;
		error?: string | null;
		required?: boolean;
		/** Inline layout: label on the left (for toggles and short controls). */
		inline?: boolean;
		class?: string;
	}

	let { children, label, for: htmlFor, help, error = null, required = false, inline = false, class: className = '' }: Props = $props();

	let root = $state<HTMLDivElement | null>(null);
	let shownError: string | null = untrack(() => error ?? null);
	$effect(() => {
		const next = error ?? null;
		untrack(() => {
			if (next && next !== shownError) shake(root);
			shownError = next;
		});
	});
</script>

<div bind:this={root} class={`${inline ? 'flex items-center justify-between gap-4' : 'grid gap-1.5'} ${className}`}>
	<div class={inline ? 'min-w-0' : ''}>
		<label for={htmlFor} class="block text-sm font-semibold text-ink">
			{label}{#if required}<span class="ml-0.5 text-warning" aria-hidden="true">*</span>{/if}
		</label>
		{#if inline && help}<p class="mt-0.5 text-[0.8125rem] text-ink-2">{help}</p>{/if}
	</div>
	<div class={inline ? 'shrink-0' : 'contents'}>
		{@render children()}
	</div>
	{#if !inline && error}
		<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{error}</p>
	{:else if !inline && help}
		<p class="text-[0.8125rem] text-ink-2">{help}</p>
	{/if}
</div>
