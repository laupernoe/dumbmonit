<script lang="ts">
	/**
	 * MetricsQL box with name completion. Suggestions open while typing (or on
	 * Ctrl+Space); Arrow keys pick, Enter or Tab accepts, Escape closes. It is
	 * a plain combobox: the textarea owns focus, the list is `aria-activedescendant`.
	 * Ctrl/Cmd+Enter calls `onrun`.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import type { CatalogMetric } from '#lib/api/types.js';
	import { apply, suggest, type Suggestions } from '#lib/dashboards/suggest.js';

	interface Props {
		value: string;
		catalog: readonly CatalogMetric[];
		id: string;
		rows?: number;
		placeholder?: string;
		onrun?: () => void;
	}

	let { value = $bindable(''), catalog, id, rows = 3, placeholder = '', onrun }: Props = $props();

	let field = $state<HTMLTextAreaElement | null>(null);
	let found = $state<Suggestions | null>(null);
	let active = $state(0);

	function refresh(force = false) {
		if (!field) return;
		found = suggest(field.value, field.selectionStart ?? field.value.length, catalog, force);
		active = 0;
	}

	function accept(item: string) {
		if (!found || !field) return;
		const next = apply(value, found, item);
		value = next.text;
		found = null;
		const caret = next.caret;
		queueMicrotask(() => {
			field?.focus();
			field?.setSelectionRange(caret, caret);
		});
	}

	function onkeydown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
			event.preventDefault();
			onrun?.();
			return;
		}
		if (event.ctrlKey && event.key === ' ') {
			event.preventDefault();
			refresh(true);
			return;
		}
		if (!found) return;
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			active = (active + 1) % found.items.length;
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			active = (active - 1 + found.items.length) % found.items.length;
		} else if (event.key === 'Enter' || event.key === 'Tab') {
			event.preventDefault();
			accept(found.items[active]);
		} else if (event.key === 'Escape') {
			event.preventDefault();
			found = null;
		}
	}
</script>

<div class="relative">
	<textarea
		bind:this={field}
		bind:value
		{id}
		{rows}
		{placeholder}
		spellcheck="false"
		autocapitalize="off"
		autocomplete="off"
		role="combobox"
		aria-expanded={found !== null}
		aria-controls={`${id}-list`}
		aria-autocomplete="list"
		aria-activedescendant={found ? `${id}-opt-${active}` : undefined}
		aria-describedby={`${id}-hint`}
		class="input block min-h-24 resize-y font-mono text-sm"
		oninput={() => refresh()}
		onclick={() => (found = null)}
		onblur={() => setTimeout(() => (found = null), 120)}
		{onkeydown}
	></textarea>
	{#if found}
		<ul
			id={`${id}-list`}
			role="listbox"
			aria-label={m.dash_query_suggestions()}
			class="absolute z-30 mt-1 max-h-64 w-full max-w-lg overflow-auto rounded-lg border border-line-strong bg-surface py-1 shadow-lift"
		>
			{#each found.items as item, i (item)}
				<li
					id={`${id}-opt-${i}`}
					role="option"
					aria-selected={i === active}
					class={`flex min-h-10 cursor-pointer items-center px-3 font-mono text-sm ${i === active ? 'bg-surface-2 text-ink' : 'text-ink-2'}`}
					onmousedown={(e) => {
						e.preventDefault();
						accept(item);
					}}
				>
					{item}
				</li>
			{/each}
		</ul>
	{/if}
	<p id={`${id}-hint`} class="mt-1 text-xs text-ink-3">{m.dash_query_hint()}</p>
</div>
