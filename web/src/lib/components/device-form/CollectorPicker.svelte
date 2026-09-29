<script lang="ts">
	/**
	 * Step 1 of adding anything: what do you want to watch?
	 *
	 * A search box over a radio group of every choice, grouped by how it is
	 * collected: devices read over the network, what the agent reports from
	 * inside a machine, services probed from here. Each thing the agent reports
	 * (Docker, Plakar, services, disks…) is a choice of its own that leads to the
	 * agent's install. Arrow keys move between choices, Space/Enter select.
	 * Every kind comes from `GET /api/collectors`; only icons, groups, search
	 * words and the agent shortcuts are ours.
	 */
	import { Check, Search } from 'lucide-svelte';
	import type { CollectorInfo } from '$lib/api';
	import { filterGroups, groupCollectors } from './kinds';

	interface Props {
		collectors: CollectorInfo[];
		/** Id of the current choice: a kind, or `agent:<feature>`. */
		selected: string | null;
		onselect: (kind: string, feature: string | null) => void;
		/** Three columns on large screens, when nothing sits beside the picker. */
		wide?: boolean;
		/** Shown when the search finds nothing: a way out. */
		empty?: import('svelte').Snippet<[string]>;
	}

	let { collectors, selected, onselect, wide = false, empty }: Props = $props();

	let query = $state('');
	const groups = $derived(groupCollectors(collectors));
	const visible = $derived(filterGroups(groups, query));
	const flat = $derived(visible.flatMap((group) => group.choices));
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
			placeholder="Search: switch, NAS, Docker, backup, website…"
			bind:value={query}
			onkeydown={onsearchkey}
			autocomplete="off"
			spellcheck="false"
			aria-label="Search what to monitor"
		/>
	</div>

	{#if flat.length === 0}
		<div class="rounded-[var(--radius-card)] border border-dashed border-line px-4 py-6 text-center">
			<p class="text-sm text-ink">Nothing here matches “{query.trim()}”.</p>
			{#if empty}{@render empty(query.trim())}{/if}
		</div>
	{:else}
		<div bind:this={host} role="radiogroup" aria-label="What to monitor" class="grid gap-5">
			{#each visible as group (group.id)}
				<section aria-labelledby={`picker-${group.id}`}>
					<div class="mb-2 flex flex-wrap items-baseline gap-x-3 gap-y-0.5">
						<h3 id={`picker-${group.id}`} class="label-tape">{group.title}</h3>
						{#if group.hint}<p class="text-[0.8125rem] text-ink-3">{group.hint}</p>{/if}
					</div>
					<ul class={`grid gap-2 sm:grid-cols-2 ${wide ? 'xl:grid-cols-3' : ''}`}>
						{#each group.choices as choice, i (choice.id)}
							{@const Icon = choice.icon}
							{@const active = choice.id === selected}
							<li class="rise-in min-w-0" style={`--rise-delay: ${Math.min(i, 10) * 25}ms`}>
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
										<Icon class="size-4" aria-hidden="true" />
									</span>
									<span class="min-w-0 flex-1 pr-5">
										<span class="block text-[0.9375rem] leading-tight font-semibold text-ink">{choice.label}</span>
										{#if choice.summary}
											<span class="mt-1 line-clamp-2 text-[0.8125rem] leading-snug text-ink-2">{choice.summary}</span>
										{/if}
									</span>
									{#if active}
										<span
											class="absolute top-2.5 right-2.5 flex size-5 items-center justify-center rounded-full bg-signal text-on-signal"
											aria-hidden="true"
										>
											<Check class="size-3.5" />
										</span>
									{/if}
								</button>
							</li>
						{/each}
					</ul>
				</section>
			{/each}
		</div>
	{/if}
</div>
