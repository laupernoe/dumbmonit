<script lang="ts">
	/**
	 * The panel of the node clicked on the map: its state, where it sits, what
	 * it depends on and what depends on it — each a button that moves the
	 * selection along the chain — and the way to its page. A side panel on a
	 * wide screen, a bottom sheet on a phone.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import KindIcon from '#lib/components/KindIcon.svelte';
	import { kindLabel } from '#lib/kind-label.js';
	import { STATE_LABEL, type TargetState } from '#lib/format.js';
	import { Box, Monitor, X } from 'lucide-svelte';
	import { Button } from '#lib/ui/index.js';
	import { ancestors, blockers, descendants, type Graph, type NodeState } from './layout';
	import { sourceShort } from './legend';

	interface Props {
		graph: Graph;
		id: string;
		onselect: (id: string) => void;
		onclose: () => void;
	}

	let { graph, id, onselect, onclose }: Props = $props();

	const node = $derived(graph.nodes.get(id)!);
	const down = $derived(graph.down.has(id));
	const impacted = $derived(graph.impacted.has(id));
	const upstream = $derived(ancestors(graph, id));
	const dependents = $derived(descendants(graph, id));
	const behind = $derived(impacted ? blockers(graph, id) : []);
	const cabled = $derived(graph.cabled.get(id) ?? []);
	const via = $derived(graph.treeSource.get(id) ?? null);
	const href = $derived(node.targetId !== null ? `/targets/${node.targetId}` : node.hostId !== null ? `/targets/${node.hostId}` : null);

	const SHOWN = 8;

	function stateText(state: NodeState): string {
		if (state === 'running') return m.topology_state_running();
		if (state === 'stopped') return m.topology_state_stopped();
		return STATE_LABEL[state as TargetState];
	}

	function name(other: string): string {
		return graph.nodes.get(other)?.name ?? other;
	}

	const chip =
		'inline-flex min-h-10 max-w-full items-center gap-1.5 rounded-lg border border-line bg-surface-2 px-2.5 text-left text-sm text-ink hover:border-line-strong focus-visible:outline-2 focus-visible:outline-signal';
</script>

{#snippet nodeChip(other: string)}
	{@const n = graph.nodes.get(other)}
	{#if n}
		<li class="max-w-full">
			<button type="button" class={chip} onclick={() => onselect(other)}>
				<span class={graph.down.has(other) ? 'text-warning' : 'text-ink-3'}>
					{#if n.targetId === null}<Monitor class="size-3.5" aria-hidden="true" />{:else}<KindIcon kind={n.kind} class="size-3.5" />{/if}
				</span>
				<span class="truncate">{n.name}</span>
				{#if graph.down.has(other)}
					<span class="shrink-0 text-xs font-bold text-warning-ink">· {stateText(n.state)}</span>
				{/if}
			</button>
		</li>
	{/if}
{/snippet}

<section
	data-topo-panel
	class="absolute inset-x-2 bottom-2 z-10 flex max-h-[58%] flex-col overflow-hidden rounded-[var(--radius-card)] border border-line-strong bg-surface shadow-lift sm:inset-x-auto sm:top-3 sm:right-3 sm:bottom-3 sm:max-h-none sm:w-80"
	aria-label={m.topology_panel_label({ name: node.name })}
>
	<header class="flex items-start gap-3 border-b border-line px-4 py-3">
		<span class={`mt-0.5 inline-flex size-9 shrink-0 items-center justify-center rounded-full border-2 ${down ? 'border-warning bg-warning-soft text-warning-ink' : impacted ? 'border-dashed border-advisory text-advisory-ink' : 'border-line-strong text-ink-2'}`}>
			{#if node.targetId === null}
				{#if node.kind === 'container'}<Box class="size-4" aria-hidden="true" />{:else}<Monitor class="size-4" aria-hidden="true" />{/if}
			{:else}
				<KindIcon kind={node.kind} class="size-4" />
			{/if}
		</span>
		<div class="min-w-0 flex-1">
			<h2 class="truncate text-base font-semibold text-ink">{node.name}</h2>
			<p class="truncate text-xs text-ink-2">
				{node.targetId === null ? (node.kind === 'container' ? m.topology_guest_container() : m.topology_guest_vm()) : kindLabel(node.kind)}
			</p>
		</div>
		<button
			type="button"
			class="-mt-1 -mr-2 inline-flex size-10 shrink-0 items-center justify-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-ink focus-visible:outline-2 focus-visible:outline-signal"
			aria-label={m.topology_panel_close()}
			title={m.topology_panel_close()}
			onclick={onclose}
		>
			<X class="size-4" aria-hidden="true" />
		</button>
	</header>

	<div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 py-3 text-sm">
		<p
			class={`rounded-lg px-3 py-2 font-semibold ${down ? 'bg-warning-soft text-warning-ink' : impacted ? 'border border-dashed border-advisory text-advisory-ink' : 'bg-surface-2 text-ink'}`}
		>
			{#if down}
				{m.topology_panel_down({ count: dependents.length })}
			{:else if impacted}
				{m.topology_panel_unreachable({ names: behind.map(name).join(', ') })}
			{:else}
				{stateText(node.state)}
			{/if}
		</p>

		<dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-1.5">
			<dt class="text-ink-3">{m.topology_panel_site()}</dt>
			<dd class="truncate text-ink">{node.site ?? m.topology_site_main()}</dd>
			<dt class="text-ink-3">{m.topology_panel_folder()}</dt>
			<dd class="truncate text-ink">{node.group || m.devices_folders_none()}</dd>
			{#if via}
				<dt class="text-ink-3">{m.topology_panel_placed_by()}</dt>
				<dd class="truncate text-ink">{sourceShort(via)}</dd>
			{/if}
		</dl>

		<div>
			<h3 class="mb-1.5 text-xs font-semibold tracking-wide text-ink-3 uppercase">{m.topology_panel_depends_on()}</h3>
			{#if upstream.length === 0}
				<p class="text-ink-2">{m.topology_panel_none()}</p>
			{:else}
				<ul class="flex flex-wrap gap-1.5">
					{#each upstream.slice(0, SHOWN) as other (other)}{@render nodeChip(other)}{/each}
				</ul>
				{#if upstream.length > SHOWN}<p class="mt-1 text-xs text-ink-3">{m.topology_panel_more({ count: upstream.length - SHOWN })}</p>{/if}
			{/if}
		</div>

		<div>
			<h3 class="mb-1.5 text-xs font-semibold tracking-wide text-ink-3 uppercase">{m.topology_panel_dependents({ count: dependents.length })}</h3>
			{#if dependents.length === 0}
				<p class="text-ink-2">{m.topology_panel_none()}</p>
			{:else}
				<ul class="flex flex-wrap gap-1.5">
					{#each dependents.slice(0, SHOWN) as other (other)}{@render nodeChip(other)}{/each}
				</ul>
				{#if dependents.length > SHOWN}<p class="mt-1 text-xs text-ink-3">{m.topology_panel_more({ count: dependents.length - SHOWN })}</p>{/if}
			{/if}
		</div>

		{#if cabled.length > 0}
			<div>
				<h3 class="mb-1.5 text-xs font-semibold tracking-wide text-ink-3 uppercase">{m.topology_panel_cabled()}</h3>
				<ul class="flex flex-wrap gap-1.5">
					{#each cabled.slice(0, SHOWN) as other (other)}{@render nodeChip(other)}{/each}
				</ul>
			</div>
		{/if}
	</div>

	{#if href}
		<footer class="border-t border-line px-4 py-3">
			<Button variant="primary" {href} class="w-full justify-center">
				{node.targetId === null ? m.topology_panel_open_host() : m.topology_panel_open()}
			</Button>
		</footer>
	{/if}
</section>
