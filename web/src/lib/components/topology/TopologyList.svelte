<script lang="ts">
	/**
	 * The map as an indented list: the same trees, site by site. A full
	 * equivalent of the map for screen readers and for whoever prefers to read.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import KindIcon from '#lib/components/KindIcon.svelte';
	import { STATE_LABEL, type TargetState } from '#lib/format.js';
	import { Box, Monitor } from 'lucide-svelte';
	import type { ListRow, NodeState, SiteInfo } from './layout';
	import { sourceShort } from './legend';

	interface Props {
		sections: { site: SiteInfo; rows: ListRow[] }[];
	}

	let { sections }: Props = $props();

	function stateText(state: NodeState): string {
		if (state === 'running') return m.topology_state_running();
		if (state === 'stopped') return m.topology_state_stopped();
		return STATE_LABEL[state as TargetState];
	}

	function href(row: ListRow): string | null {
		const id = row.node.targetId ?? row.node.hostId;
		return id === null ? null : `/targets/${id}`;
	}
</script>

<div class="flex flex-col gap-4" aria-label={m.topology_list_label()} role="region">
	{#each sections as section (section.site.key)}
		<section class="overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface">
			<h2 class="border-b border-line bg-surface-2 px-4 py-2 text-xs font-bold tracking-wide text-ink-2 uppercase">
				{section.site.name ?? m.topology_site_main()}
			</h2>
			<ul>
				{#each section.rows as row (row.node.id)}
					{@const link = href(row)}
					<li class="border-b border-line last:border-b-0" style={`padding-left: ${Math.min(row.depth, 6) * 1.1}rem`}>
						<svelte:element
							this={link ? 'a' : 'div'}
							href={link}
							class={`flex min-h-11 items-center gap-2.5 px-3 py-1.5 ${link ? 'hover:bg-surface-2 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-signal' : ''}`}
						>
							{#if row.depth > 0}
								<span class="text-ink-3" aria-hidden="true">└</span>
							{/if}
							<span class={row.down ? 'text-warning' : 'text-ink-2'}>
								{#if row.node.targetId === null}
									{#if row.node.kind === 'container'}<Box class="size-4" aria-hidden="true" />{:else}<Monitor class="size-4" aria-hidden="true" />{/if}
								{:else}
									<KindIcon kind={row.node.kind} class="size-4" />
								{/if}
							</span>
							<span class="min-w-0 flex-1">
								<span class="block truncate text-sm font-semibold text-ink">{row.node.name}</span>
								<span class="block truncate text-xs text-ink-2">
									{#if row.via}{sourceShort(row.via)} · {/if}{row.node.targetId === null ? (row.node.kind === 'container' ? m.topology_guest_container() : m.topology_guest_vm()) + ' · ' : ''}{stateText(row.node.state)}
								</span>
							</span>
							{#if row.down}
								<span class="shrink-0 rounded-full bg-warning-soft px-2 py-0.5 text-xs font-bold text-warning-ink">{m.topology_legend_down()}</span>
							{:else if row.impacted}
								<span class="shrink-0 rounded-full border border-dashed border-advisory px-2 py-0.5 text-xs font-semibold text-advisory-ink">{m.topology_state_suppressed()}</span>
							{/if}
						</svelte:element>
					</li>
				{/each}
			</ul>
		</section>
	{/each}
</div>
