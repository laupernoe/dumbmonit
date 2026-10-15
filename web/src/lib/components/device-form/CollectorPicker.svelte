<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Step 1 of adding anything: what do you want to watch?
	 *
	 * A search box first, then the few things most people add (website, machine
	 * with the agent, network device, hypervisor, NAS…), then every other kind
	 * grouped by category and folded away. Typing searches the whole catalog and
	 * lists the matches flat. The agent is one choice: what it reports is found
	 * by search and detected once it runs. Every kind comes from
	 * `GET /api/collectors`; only icons, groups and search words are ours.
	 * Arrow keys move between choices, Space/Enter select.
	 */
	import { Check, ChevronDown, Radar, Search } from 'lucide-svelte';
	import type { CollectorInfo } from '#lib/api/index.js';
	import { Plate } from '#lib/ui/index.js';
	import KindIcon from '#lib/components/KindIcon.svelte';
	import { filterGroups, groupCollectors, POPULAR_KINDS, type KindChoice } from './kinds';

	interface Props {
		collectors: CollectorInfo[];
		/** Id of the current choice: a kind, or `agent:<feature>`. */
		selected: string | null;
		onselect: (kind: string, feature: string | null) => void;
		/** Three columns on large screens, when nothing sits beside the picker. */
		wide?: boolean;
		/** Adds a "Scan the network" shortcut among the common choices. */
		onscan?: () => void;
		/** Shown when the search finds nothing: a way out. */
		empty?: import('svelte').Snippet<[string]>;
	}

	let { collectors, selected, onselect, onscan, wide = false, empty }: Props = $props();

	let query = $state('');
	const groups = $derived(groupCollectors(collectors));
	const visible = $derived(filterGroups(groups, query));
	/** Typing searches the whole catalog, flat: no folded section hides a match. */
	const searching = $derived(query.trim().length > 0);
	const popular = $derived(
		POPULAR_KINDS.flatMap((kind) => groups.flatMap((g) => g.choices).filter((c) => c.id === kind))
	);
	const rest = $derived(
		groups
			.map((g) => ({ ...g, choices: g.choices.filter((c) => !POPULAR_KINDS.includes(c.id)) }))
			.filter((g) => g.choices.length > 0)
	);
	/** Which categories are open. Empty: every one starts folded. */
	let expanded = $state<Set<string>>(new Set());
	function toggleExpanded(id: string) {
		const next = new Set(expanded);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		expanded = next;
	}
	const flat = $derived<KindChoice[]>(
		searching ? visible.flatMap((g) => g.choices) : [...popular, ...rest.filter((g) => expanded.has(g.id)).flatMap((g) => g.choices)]
	);
	/** Roving tabindex: the selected choice, or the first one, is the tab stop. */
	const tabStop = $derived(selected && flat.some((c) => c.id === selected) ? selected : flat[0]?.id);

	let host = $state<HTMLDivElement | null>(null);

	function choose(id: string) {
		const choice = flat.find((c) => c.id === id);
		if (choice) onselect(choice.kind, choice.feature);
	}

	function move(from: string, delta: number) {
		const index = flat.findIndex((c) => c.id === from);
		if (index === -1 || flat.length === 0) return;
		const next = flat[(index + delta + flat.length) % flat.length];
		// Moving only moves the focus: selecting would scroll to the form each time.
		host?.querySelector<HTMLButtonElement>(`[data-choice="${next.id}"]`)?.focus();
	}

	function onkeydown(event: KeyboardEvent, id: string) {
		switch (event.key) {
			case 'ArrowRight':
			case 'ArrowDown':
				event.preventDefault();
				move(id, 1);
				break;
			case 'ArrowLeft':
			case 'ArrowUp':
				event.preventDefault();
				move(id, -1);
				break;
			case ' ':
			case 'Enter':
				event.preventDefault();
				choose(id);
				break;
		}
	}

	/** Enter in the search box takes the only, or first, match. */
	function onsearchkey(event: KeyboardEvent) {
		if (event.key === 'Enter' && query.trim() && flat.length > 0) {
			event.preventDefault();
			choose(flat[0].id);
		} else if (event.key === 'ArrowDown' && flat.length > 0) {
			event.preventDefault();
			host?.querySelector<HTMLButtonElement>(`[data-choice="${flat[0].id}"]`)?.focus();
		}
	}
</script>

<div class="grid gap-5">
	<div class="relative">
		<Search class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-ink-3" aria-hidden="true" />
		<input
			type="search"
			class="input"
			style="padding-left: 2.25rem"
			placeholder={m.deviceform_picker_search_placeholder()}
			bind:value={query}
			onkeydown={onsearchkey}
			autocomplete="off"
			spellcheck="false"
			aria-label={m.deviceform_picker_search_label()}
		/>
	</div>

	{#if flat.length === 0}
		<div class="rounded-[var(--radius-card)] border border-dashed border-line px-4 py-6 text-center">
			<p class="text-sm text-ink">{m.deviceform_picker_no_match({ query: query.trim() })}</p>
			{#if empty}{@render empty(query.trim())}{/if}
		</div>
	{:else}
		<div bind:this={host} role="radiogroup" aria-label={m.deviceform_picker_group_label()} class="grid gap-5">
			{#if searching}
				{@render tiles(flat)}
			{:else}
				<section aria-labelledby="picker-popular">
					<h3 id="picker-popular" class="label-tape mb-2">{m.deviceform_picker_popular()}</h3>
					{@render tiles(popular, true)}
				</section>
				<div class="grid gap-1 border-t border-line pt-3">
					<h3 class="label-tape mb-1">{m.deviceform_picker_more()}</h3>
					{#each rest as group (group.id)}
						{@const open = expanded.has(group.id)}
						<section>
							<button
								type="button"
								class="flex min-h-10 w-full items-center gap-2 rounded text-left text-sm font-semibold text-ink-2 hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal"
								aria-expanded={open}
								onclick={() => toggleExpanded(group.id)}
							>
								<ChevronDown class={`size-4 shrink-0 transition-transform duration-200 ${open ? '' : '-rotate-90'}`} aria-hidden="true" />
								<span>{group.title} ({group.choices.length})</span>
								{#if group.hint}<span class="hidden min-w-0 truncate font-normal text-ink-3 sm:inline">{group.hint}</span>{/if}
							</button>
							{#if open}
								<div class="mt-2 mb-3">{@render tiles(group.choices)}</div>
							{/if}
						</section>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>

{#snippet tiles(items: KindChoice[], withScan = false)}
	<ul class={`grid gap-2 sm:grid-cols-2 ${wide ? 'xl:grid-cols-3' : ''}`}>
		{#each items as choice, i (choice.id)}
			{@const Icon = choice.icon}
			{@const active = choice.id === selected}
			<li class="min-w-0" style={`--rise-delay: ${Math.min(i, 10) * 25}ms`}>
				<button
					type="button"
					role="radio"
					aria-checked={active}
					tabindex={choice.id === tabStop ? 0 : -1}
					data-choice={choice.id}
					data-kind={choice.feature ? undefined : choice.kind}
					onclick={() => onselect(choice.kind, choice.feature)}
					onkeydown={(event) => onkeydown(event, choice.id)}
					class={`relative flex h-full w-full items-start gap-3 rounded-[var(--radius-card)] border px-3.5 py-3 text-left transition-[transform,box-shadow,border-color,background-color] duration-200 ease-out-expo focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal ${
						active
							? 'border-signal bg-signal-soft shadow-lift'
							: 'border-line bg-surface hover:-translate-y-px hover:border-line-strong hover:shadow-float'
					}`}
				>
					<span
						class={`mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg border ${
							active ? 'border-signal/30 bg-surface text-signal-ink' : 'border-line bg-surface-2 text-ink-2'
						}`}
					>
						{#if choice.feature}
							<Icon class="size-4" aria-hidden="true" />
						{:else}
							<KindIcon kind={choice.kind} />
						{/if}
					</span>
					<span class="min-w-0 flex-1 pr-5">
						<span class="flex flex-wrap items-center gap-x-2 gap-y-1">
							<span class="text-[0.9375rem] leading-tight font-semibold text-ink">{choice.label}</span>
							{#if choice.pack}<Plate tone="info" label={m.deviceform_picker_pack()} title={m.deviceform_picker_pack_title()} />{/if}
						</span>
						{#if choice.summary}
							<span class="mt-1 line-clamp-2 text-[0.8125rem] leading-snug text-ink-2">{choice.summary}</span>
						{/if}
					</span>
					{#if active}
						<span class="absolute top-2.5 right-2.5 flex size-5 items-center justify-center rounded-full bg-signal text-on-signal" aria-hidden="true">
							<Check class="size-3.5" />
						</span>
					{/if}
				</button>
			</li>
		{/each}
		{#if withScan && onscan}
			<li class="min-w-0">
				<button
					type="button"
					data-choice="scan"
					onclick={onscan}
					class="flex h-full w-full items-start gap-3 rounded-[var(--radius-card)] border border-line bg-surface px-3.5 py-3 text-left transition-[transform,box-shadow,border-color] duration-200 ease-out-expo hover:-translate-y-px hover:border-line-strong hover:shadow-float focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal"
				>
					<span class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg border border-line bg-surface-2 text-ink-2">
						<Radar class="size-4" aria-hidden="true" />
					</span>
					<span class="min-w-0 flex-1">
						<span class="text-[0.9375rem] leading-tight font-semibold text-ink">{m.deviceform_new_scan()}</span>
						<span class="mt-1 line-clamp-2 block text-[0.8125rem] leading-snug text-ink-2">{m.deviceform_new_door_scan_desc()}</span>
					</span>
				</button>
			</li>
		{/if}
	</ul>
{/snippet}
