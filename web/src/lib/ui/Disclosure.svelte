<script lang="ts">
	/**
	 * A collapsible block: a full-width header button (chevron, title, optional
	 * summary on the right) over a body that is closed by default. The body stays
	 * mounted while closed (`hidden`), so a form inside keeps what was typed and
	 * its fields stay reachable by id. The header is a real button: Enter and
	 * Space toggle it, `aria-expanded` carries the state, and the chevron turns
	 * so the state is never conveyed by anything but shape and text.
	 */
	import type { Snippet } from 'svelte';
	import { ChevronRight } from 'lucide-svelte';

	interface Props {
		title: string;
		/** A few words on the right of the header: a status, a count. */
		summary?: string;
		/** Extra header content after the title (a plate). */
		aside?: Snippet;
		open?: boolean;
		id?: string;
		children: Snippet;
		class?: string;
	}

	let { title, summary, aside, open = $bindable(false), id, children, class: className = '' }: Props = $props();

	const uid = $props.id();
	const bodyId = $derived(`${id ?? `disclosure-${uid}`}-body`);
</script>

<div {id} class={`rounded-[var(--radius-card)] border border-line bg-surface ${className}`}>
	<h3 class="m-0 text-sm font-semibold text-ink">
		<button
			type="button"
			class="flex min-h-11 w-full items-center gap-2 rounded-[inherit] px-4 py-2.5 text-left hover:bg-surface-2 focus-visible:outline-2 focus-visible:outline-signal"
			aria-expanded={open}
			aria-controls={bodyId}
			onclick={() => (open = !open)}
		>
			<ChevronRight
				class={`size-4 shrink-0 text-ink-3 transition-transform duration-200 ease-out-expo ${open ? 'rotate-90' : ''}`}
				aria-hidden="true"
			/>
			<span class="min-w-0 flex-1 truncate">{title}</span>
			{#if aside}<span class="shrink-0">{@render aside()}</span>{/if}
			{#if summary}<span class="shrink-0 text-[0.8125rem] font-normal text-ink-2">{summary}</span>{/if}
		</button>
	</h3>
	<div id={bodyId} hidden={!open} class="border-t border-line px-4 py-4">
		{@render children()}
	</div>
</div>
