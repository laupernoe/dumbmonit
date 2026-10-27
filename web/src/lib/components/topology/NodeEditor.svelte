<script lang="ts">
	/**
	 * Correction tools for one node, inside its panel while the map is being
	 * corrected: attach it under another node (what a drag does), draw a
	 * link of a chosen type, move it to another site, group it. Ends with the
	 * corrections already made on it, each one removable.
	 */
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { Button, Toggle } from '#lib/ui/index.js';
	import Segmented from '#lib/components/devices/Segmented.svelte';
	import type { TopologyCorrection, TopologySource } from '#lib/api/index.js';
	import NodePicker from './NodePicker.svelte';
	import { LINK_KINDS, canUseForAlerts } from './edits';
	import { correctionBy, correctionText } from './editText';
	import { sourceLabel } from './legend';
	import type { Graph } from './layout';

	interface Props {
		graph: Graph;
		id: string;
		corrections: TopologyCorrection[];
		sites: string[];
		groups: string[];
		/** Parent picked by a drop on the map, waiting for confirmation. */
		pending: string | null;
		busy: boolean;
		onattach: (parent: string, useForAlerts: boolean) => void;
		onlink: (other: string, kind: TopologySource) => void;
		onsite: (site: string) => void;
		ongroup: (group: string) => void;
		onremove: (correction: TopologyCorrection) => void;
		onpending: (parent: string | null) => void;
	}

	let { graph, id, corrections, sites, groups, pending, busy, onattach, onlink, onsite, ongroup, onremove, onpending }: Props = $props();

	type Tool = 'attach' | 'link' | 'site' | 'group';
	let tool = $state<Tool>('attach');
	let alerts = $state(true);
	let linkKind = $state<TopologySource>('manual');
	let other = $state<string | null>(null);

	const node = $derived(graph.nodes.get(id)!);
	let site = $state('');
	let group = $state('');
	// A new node selected: the fields start from what it has (a refresh of the map leaves them alone).
	$effect(() => {
		const current = id;
		untrack(() => {
			const picked = graph.nodes.get(current);
			site = picked?.site ?? '';
			group = picked?.group ?? '';
			other = null;
		});
	});
	$effect(() => {
		if (pending !== null) tool = 'attach';
	});

	const name = (node: string) => graph.nodes.get(node)?.name ?? node;
	const mine = $derived(corrections.filter((c) => c.to === id || c.from === id));
	const forAlerts = $derived(pending !== null && canUseForAlerts(id, pending));

	const TOOLS = $derived([
		{ id: 'attach' as const, label: m.topology_edit_tool_attach() },
		{ id: 'link' as const, label: m.topology_edit_tool_link() },
		{ id: 'site' as const, label: m.topology_edit_tool_site() },
		{ id: 'group' as const, label: m.topology_edit_tool_group() }
	]);
</script>

<section class="flex flex-col gap-3 rounded-lg border border-signal/40 bg-surface-2 p-3" aria-label={m.topology_edit_section()}>
	<h3 class="text-sm font-semibold text-ink">{m.topology_edit_section()}</h3>
	<Segmented options={TOOLS} value={tool} onchange={(value) => (tool = value)} label={m.topology_edit_section()} size="sm" />

	{#if tool === 'attach'}
		{#if pending !== null}
			<div class="flex flex-col gap-2.5" aria-live="polite">
				<p class="text-sm text-ink">{m.topology_edit_attach_confirm({ child: node.name, parent: name(pending) })}</p>
				{#if forAlerts}
					<label class="flex items-start gap-2.5 text-sm text-ink-2">
						<span class="mt-0.5"><Toggle bind:checked={alerts} label={m.topology_edit_alerts()} /></span>
						<span aria-hidden="true">{m.topology_edit_alerts()}</span>
					</label>
				{:else}
					<p class="text-xs text-ink-3">{m.topology_edit_alerts_unavailable()}</p>
				{/if}
				<div class="flex flex-wrap gap-2">
					<Button variant="primary" size="sm" loading={busy} onclick={() => onattach(pending!, forAlerts && alerts)}>{m.topology_edit_attach_button()}</Button>
					<Button variant="ghost" size="sm" onclick={() => onpending(null)}>{m.topology_edit_cancel()}</Button>
				</div>
			</div>
		{:else}
			<p class="text-xs text-ink-3">{m.topology_edit_attach_hint()}</p>
			<NodePicker nodes={graph.nodes.values()} except={id} id={`attach-${id}`} label={m.topology_edit_attach_search()} onpick={(parent) => onpending(parent)} />
		{/if}
	{:else if tool === 'link'}
		<div class="flex flex-col gap-1.5">
			<label for={`link-kind-${id}`} class="text-xs font-semibold text-ink-2">{m.topology_edit_link_kind()}</label>
			<select id={`link-kind-${id}`} class="input" bind:value={linkKind}>
				{#each LINK_KINDS as kind (kind)}
					<option value={kind}>{sourceLabel(kind)}</option>
				{/each}
			</select>
		</div>
		{#if other === null}
			<NodePicker nodes={graph.nodes.values()} except={id} id={`link-${id}`} label={m.topology_edit_link_search()} onpick={(picked) => (other = picked)} />
		{:else}
			<p class="text-sm text-ink">{m.topology_edit_link_confirm({ from: name(other), to: node.name })}</p>
			<div class="flex flex-wrap gap-2">
				<Button variant="primary" size="sm" loading={busy} onclick={() => other && onlink(other, linkKind)}>{m.topology_edit_link_button()}</Button>
				<Button variant="ghost" size="sm" onclick={() => (other = null)}>{m.topology_edit_cancel()}</Button>
			</div>
		{/if}
	{:else if tool === 'site'}
		<form
			class="flex flex-col gap-1.5"
			onsubmit={(event) => {
				event.preventDefault();
				onsite(site.trim());
			}}
		>
			<label for={`site-${id}`} class="text-xs font-semibold text-ink-2">{m.topology_edit_site_label()}</label>
			<input id={`site-${id}`} class="input" list={`sites-${id}`} maxlength="80" bind:value={site} placeholder={m.topology_site_main()} />
			<datalist id={`sites-${id}`}>
				{#each sites as known (known)}<option value={known}></option>{/each}
			</datalist>
			<p class="text-xs text-ink-3">{m.topology_edit_site_hint()}</p>
			<div><Button type="submit" variant="primary" size="sm" loading={busy}>{m.topology_edit_site_save()}</Button></div>
		</form>
	{:else}
		<form
			class="flex flex-col gap-1.5"
			onsubmit={(event) => {
				event.preventDefault();
				ongroup(group.trim());
			}}
		>
			<label for={`group-${id}`} class="text-xs font-semibold text-ink-2">{m.topology_edit_group_label()}</label>
			<input id={`group-${id}`} class="input" list={`groups-${id}`} maxlength="80" bind:value={group} placeholder={m.devices_folders_none()} />
			<datalist id={`groups-${id}`}>
				{#each groups as known (known)}<option value={known}></option>{/each}
			</datalist>
			<p class="text-xs text-ink-3">{m.topology_edit_group_hint()}</p>
			<div><Button type="submit" variant="primary" size="sm" loading={busy}>{m.topology_edit_group_save()}</Button></div>
		</form>
	{/if}

	{#if mine.length > 0}
		<div class="border-t border-line pt-2.5">
			<h4 class="mb-1.5 text-xs font-semibold text-ink-2">{m.topology_edit_here()}</h4>
			<ul class="flex flex-col gap-1.5">
				{#each mine as c (c.id)}
					<li class="flex items-start gap-2">
						<span class="min-w-0 flex-1 text-sm text-ink">
							{correctionText(c, name)}
							<span class="block text-xs text-ink-3">{correctionBy(c)}</span>
						</span>
						<Button variant="ghost" size="sm" disabled={busy} onclick={() => onremove(c)}>{m.topology_edit_remove()}</Button>
					</li>
				{/each}
			</ul>
		</div>
	{/if}
</section>
