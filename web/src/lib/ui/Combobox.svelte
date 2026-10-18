<script lang="ts" module>
	export interface ComboOption {
		value: string;
		label: string;
		/** Secondary text, searched too (a type, an address). */
		hint?: string;
		/** Options sharing a group are listed under its heading. */
		group?: string;
	}
</script>

<script lang="ts">
	/**
	 * Multi-select combobox: a search box that opens a listbox of options.
	 * Picking keeps the list open so several can be ticked in a row; what is
	 * selected is shown by the caller (chips, rows), so it can add its own
	 * controls. Keyboard: arrows, Home/End, Enter or Space-less toggle, Escape.
	 * Filters on label, hint and group; options are grouped under headings when
	 * any carries a group.
	 */
	import { ChevronDown, Check } from 'lucide-svelte';

	interface Props {
		options: ComboOption[];
		selected: string[];
		ontoggle: (value: string, on: boolean) => void;
		/** Accessible name and placeholder of the search box. */
		label: string;
		/** Shown when the filter matches nothing. */
		emptyText: string;
		/** Heading of options without a group, when others have one. */
		ungroupedLabel?: string;
		/** Text on the right of the box, e.g. "3 selected". */
		summary?: string;
		disabled?: boolean;
		id: string;
	}

	let { options, selected, ontoggle, label, emptyText, ungroupedLabel = '', summary = '', disabled = false, id }: Props = $props();

	let query = $state('');
	let open = $state(false);
	let active = $state(0);
	let root = $state<HTMLDivElement | null>(null);
	let list = $state<HTMLUListElement | null>(null);

	const chosen = $derived(new Set(selected));
	const hasGroups = $derived(options.some((o) => o.group));

	/** Filtered options, ordered by group (ungrouped first) then label. */
	const shown = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const found = q ? options.filter((o) => `${o.label} ${o.hint ?? ''} ${o.group ?? ''}`.toLowerCase().includes(q)) : options;
		return [...found].sort((a, b) => (a.group ?? '').localeCompare(b.group ?? '') || a.label.localeCompare(b.label));
	});

	const listId = $derived(`${id}-list`);
	const optionId = (index: number) => `${id}-opt-${index}`;

	function show() {
		if (!disabled) open = true;
	}
	function move(delta: number) {
		show();
		if (shown.length === 0) return;
		active = (active + delta + shown.length) % shown.length;
		scrollActive();
	}
	function scrollActive() {
		queueMicrotask(() => list?.querySelector(`#${CSS.escape(optionId(active))}`)?.scrollIntoView({ block: 'nearest' }));
	}
	function toggleAt(index: number) {
		const option = shown[index];
		if (option) ontoggle(option.value, !chosen.has(option.value));
	}
	function onkeydown(event: KeyboardEvent) {
		switch (event.key) {
			case 'ArrowDown':
				event.preventDefault();
				move(open ? 1 : 0);
				break;
			case 'ArrowUp':
				event.preventDefault();
				move(open ? -1 : 0);
				break;
			case 'Home':
				if (open) {
					event.preventDefault();
					active = 0;
					scrollActive();
				}
				break;
			case 'End':
				if (open) {
					event.preventDefault();
					active = Math.max(0, shown.length - 1);
					scrollActive();
				}
				break;
			case 'Enter':
				// Enter inside a form would submit it: here it picks.
				event.preventDefault();
				if (open) toggleAt(active);
				else show();
				break;
			case 'Escape':
				if (open) {
					event.preventDefault();
					event.stopPropagation();
					open = false;
				}
				break;
			case 'Tab':
				open = false;
				break;
		}
	}
	function onfocusout(event: FocusEvent) {
		if (!root?.contains(event.relatedTarget as Node | null)) open = false;
	}
</script>

<div bind:this={root} class="relative" onfocusout={onfocusout}>
	<div class="relative">
		<input
			{id}
			type="text"
			role="combobox"
			class="input !pr-24"
			aria-expanded={open}
			aria-controls={listId}
			aria-autocomplete="list"
			aria-activedescendant={open && shown.length > 0 ? optionId(active) : undefined}
			aria-label={label}
			placeholder={label}
			autocomplete="off"
			spellcheck="false"
			{disabled}
			bind:value={query}
			onfocus={show}
			onclick={show}
			oninput={() => {
				open = true;
				active = 0;
			}}
			{onkeydown}
		/>
		<span class="pointer-events-none absolute inset-y-0 right-2 flex items-center gap-1.5 text-[0.75rem] text-ink-2">
			{#if summary}<span class="tabular-nums">{summary}</span>{/if}
			<ChevronDown class={`size-4 transition-transform ${open ? 'rotate-180' : ''}`} aria-hidden="true" />
		</span>
	</div>
	{#if open}
		<ul
			bind:this={list}
			id={listId}
			role="listbox"
			aria-multiselectable="true"
			aria-label={label}
			class="absolute inset-x-0 top-full z-30 mt-1 max-h-72 overflow-y-auto rounded-[var(--radius-card)] border border-line-strong bg-surface p-1 shadow-lift"
		>
			{#each shown as option, index (option.value)}
				{@const on = chosen.has(option.value)}
				{#if hasGroups && (index === 0 || (shown[index - 1].group ?? '') !== (option.group ?? ''))}
					<li role="presentation" class="px-2.5 pt-2 pb-1 text-[0.6875rem] font-semibold tracking-wide text-ink-3 uppercase">{option.group || ungroupedLabel}</li>
				{/if}
				<!-- Keyboard goes through the input (activedescendant); the mouse picks here. -->
				<li
					id={optionId(index)}
					role="option"
					aria-selected={on}
					class={`flex min-h-10 cursor-pointer items-center gap-2 rounded-md px-2.5 py-1.5 text-sm text-ink ${index === active ? 'bg-surface-2' : ''}`}
					onmousedown={(event) => event.preventDefault()}
					onclick={() => toggleAt(index)}
					onmousemove={() => (active = index)}
				>
					<span class={`flex size-4 shrink-0 items-center justify-center rounded border ${on ? 'border-signal bg-signal text-canvas' : 'border-line-strong'}`} aria-hidden="true">
						{#if on}<Check class="size-3" />{/if}
					</span>
					<span class="min-w-0 flex-1 truncate">{option.label}</span>
					{#if option.hint}<span class="shrink-0 text-[0.75rem] text-ink-3">{option.hint}</span>{/if}
				</li>
			{:else}
				<li role="presentation" class="px-3 py-4 text-center text-sm text-ink-2">{emptyText}</li>
			{/each}
		</ul>
	{/if}
</div>
