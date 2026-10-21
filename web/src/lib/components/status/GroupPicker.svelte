<script lang="ts">
	/**
	 * Monitor folders as a whole: a checked folder is dynamic, so a monitor added
	 * to it later shows up on the page by itself and a removed one disappears.
	 * Each folder opens to show its monitors, which can also be ticked one by
	 * one; the folder's box is "partly ticked" when only some of them are.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { ChevronRight } from 'lucide-svelte';
	import type { Target } from '#lib/api/index.js';

	interface Props {
		id: string;
		targets: Target[];
		/** Checked folders. */
		groups: string[];
		/** Monitors ticked one by one. */
		picked: Set<number>;
		ontogglegroup: (name: string, on: boolean) => void;
		ontoggletarget: (target: Target, on: boolean) => void;
		disabled?: boolean;
	}

	let { id, targets, groups, picked, ontogglegroup, ontoggletarget, disabled = false }: Props = $props();

	let expanded = $state<Set<string>>(new Set());

	const members = $derived.by(() => {
		const map = new Map<string, Target[]>();
		for (const target of targets) {
			if (!target.group_name) continue;
			map.set(target.group_name, [...(map.get(target.group_name) ?? []), target]);
		}
		for (const name of groups) if (!map.has(name)) map.set(name, []);
		return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
	});

	function toggleOpen(name: string) {
		const next = new Set(expanded);
		if (next.has(name)) next.delete(name);
		else next.add(name);
		expanded = next;
	}

	/** Partly ticked: not the whole folder, but some of its monitors one by one. */
	function partial(name: string, list: Target[]) {
		return !groups.includes(name) && list.some((t) => picked.has(t.id));
	}

	function indeterminate(node: HTMLInputElement, value: boolean) {
		node.indeterminate = value;
		return { update: (next: boolean) => (node.indeterminate = next) };
	}
</script>

{#if members.length > 0}
	<div class="grid gap-1.5">
		<span class="text-sm font-semibold text-ink" id="{id}-label">{m.status_groups_title()}</span>
		<ul class="max-h-72 divide-y divide-line overflow-auto rounded-[var(--radius-card)] border border-line" aria-labelledby="{id}-label">
			{#each members as [name, list], i (name)}
				{@const checked = groups.includes(name)}
				{@const open = expanded.has(name)}
				<li>
					<div class="flex min-h-10 items-center gap-1 px-2">
						<button
							type="button"
							class="inline-flex size-10 shrink-0 items-center justify-center rounded-md text-ink-2 hover:bg-surface-2 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ink"
							aria-expanded={open}
							aria-controls="{id}-g{i}"
							aria-label={m.status_groups_expand_aria({ name })}
							onclick={() => toggleOpen(name)}
						>
							<ChevronRight class={`size-4 transition-transform ${open ? 'rotate-90' : ''}`} aria-hidden="true" />
						</button>
						<label class="flex min-h-10 min-w-0 flex-1 cursor-pointer items-center gap-2 text-sm text-ink">
							<input
								type="checkbox"
								class="size-4 shrink-0 accent-[var(--color-signal)]"
								{checked}
								{disabled}
								use:indeterminate={partial(name, list)}
								onchange={(event) => ontogglegroup(name, event.currentTarget.checked)}
							/>
							<span class="truncate">{name}</span>
							<span class="ml-auto shrink-0 text-[0.75rem] text-ink-3 tnum">{m.status_groups_count({ count: list.length })}</span>
						</label>
					</div>
					{#if open}
						<ul id="{id}-g{i}" class="border-t border-line bg-canvas-deep py-1">
							{#each list as target (target.id)}
								<li>
									<label class="flex min-h-10 cursor-pointer items-center gap-2 pl-12 pr-3 text-sm text-ink">
										<input
											type="checkbox"
											class="size-4 shrink-0 accent-[var(--color-signal)]"
											checked={checked || picked.has(target.id)}
											disabled={disabled || checked}
											onchange={(event) => ontoggletarget(target, event.currentTarget.checked)}
										/>
										<span class="truncate">{target.name}</span>
									</label>
								</li>
							{:else}
								<li class="py-2 pl-12 text-[0.8125rem] text-ink-2">{m.status_groups_empty()}</li>
							{/each}
						</ul>
					{/if}
				</li>
			{/each}
		</ul>
		<p class="text-[0.8125rem] text-ink-2">{m.status_groups_help()}</p>
	</div>
{/if}
