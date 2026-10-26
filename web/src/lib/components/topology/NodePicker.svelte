<script lang="ts">
	/**
	 * Pick another node of the map by typing part of its name or type: the
	 * keyboard and touch way to do what a drag does. Results are plain
	 * buttons (Tab, Enter), the closest names first.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import KindIcon from '#lib/components/KindIcon.svelte';
	import { Monitor } from 'lucide-svelte';
	import { searchNodes } from './edits';
	import type { MapNode } from './layout';

	interface Props {
		nodes: Iterable<MapNode>;
		/** The node being corrected: never offered. */
		except: string;
		label: string;
		id: string;
		onpick: (id: string) => void;
	}

	let { nodes, except, label, id, onpick }: Props = $props();

	let query = $state('');
	const found = $derived(searchNodes(nodes, query, except, 6));
</script>

<div class="flex flex-col gap-1.5">
	<label for={id} class="text-xs font-semibold text-ink-2">{label}</label>
	<input {id} class="input" type="search" autocomplete="off" bind:value={query} placeholder={m.topology_edit_search_placeholder()} />
	{#if query.trim() && found.length === 0}
		<p class="text-xs text-ink-3">{m.topology_edit_no_match()}</p>
	{:else if query.trim()}
		<ul class="flex flex-col gap-1" aria-label={label}>
			{#each found as node (node.id)}
				<li>
					<button
						type="button"
						class="inline-flex min-h-10 w-full items-center gap-2 rounded-lg border border-line bg-surface-2 px-2.5 text-left text-sm text-ink hover:border-line-strong focus-visible:outline-2 focus-visible:outline-signal"
						onclick={() => {
							onpick(node.id);
							query = '';
						}}
					>
						<span class="text-ink-3">
							{#if node.targetId === null}<Monitor class="size-3.5" aria-hidden="true" />{:else}<KindIcon kind={node.kind} class="size-3.5" />{/if}
						</span>
						<span class="min-w-0 flex-1 truncate">{node.name}</span>
						{#if node.site}<span class="shrink-0 truncate text-xs text-ink-3">{node.site}</span>{/if}
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</div>
