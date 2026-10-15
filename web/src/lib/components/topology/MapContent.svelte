<svelte:options namespace="svg" />

<script lang="ts">
	/**
	 * The drawing of the map, in map coordinates: folder sections, links,
	 * then the device boxes on top. Pure SVG with token colours (`fill-*`,
	 * `stroke-*`), so the same markup serves the interactive canvas, the
	 * print sheet and the SVG/PNG export (which inlines the computed styles).
	 *
	 * A state is never colour only: every box spells it out under the name,
	 * a down box gets a thick outline and an exclamation badge, an impacted
	 * one a dashed outline and the word "Impacted".
	 */
	import { m } from '#lib/paraglide/messages.js';
	import KindIcon from '#lib/components/KindIcon.svelte';
	import { STATE_LABEL, type TargetState } from '#lib/format.js';
	import { Box, Monitor } from 'lucide-svelte';
	import { NODE_HEIGHT, NODE_WIDTH, type MapLayout, type NodeState, type PlacedNode } from './layout';
	import { EDGE_DASH } from './legend';

	interface Props {
		layout: MapLayout;
		/** Links are real anchors (keyboard, middle click); off for the print sheet. */
		interactive?: boolean;
		/** Node whose links are emphasised (hover / focus). */
		focus?: string | null;
		onfocusnode?: (id: string | null) => void;
	}

	let { layout, interactive = true, focus = null, onfocusnode }: Props = $props();

	const MAX_NAME = 24;
	function short(name: string): string {
		return name.length > MAX_NAME ? `${name.slice(0, MAX_NAME - 1)}…` : name;
	}

	function stateText(state: NodeState): string {
		if (state === 'running') return m.topology_state_running();
		if (state === 'stopped') return m.topology_state_stopped();
		return STATE_LABEL[state as TargetState];
	}

	function caption(p: PlacedNode): string {
		const own = stateText(p.node.state);
		const guest =
			p.node.targetId === null
				? p.node.kind === 'container'
					? m.topology_guest_container()
					: m.topology_guest_vm()
				: null;
		const parts = [guest, own, p.impacted ? m.topology_state_impacted() : null].filter(Boolean);
		return parts.join(' · ');
	}

	/** Tone of the status bar, the same roles as the rack's LEDs. */
	function tone(p: PlacedNode): string {
		if (p.down) return 'fill-warning';
		switch (p.node.state) {
			case 'online':
			case 'running':
				return p.impacted ? 'fill-advisory' : 'fill-signal';
			case 'misconfigured':
			case 'pending':
				return 'fill-advisory';
			default:
				return 'fill-ink-3';
		}
	}

	function href(p: PlacedNode): string | null {
		const id = p.node.targetId ?? p.node.hostId;
		return id === null ? null : `/targets/${id}`;
	}

	const linked = $derived.by(() => {
		if (!focus) return null;
		const set = new Set<string>([focus]);
		for (const e of layout.edges) {
			if (e.from === focus) set.add(e.to);
			if (e.to === focus) set.add(e.from);
		}
		return set;
	});
</script>

{#each layout.sections as section (section.group)}
	<g class="topo-section">
		<rect
			x={section.x}
			y={section.y}
			width={section.width}
			height={section.height}
			rx="12"
			class="fill-surface-2 stroke-line"
			stroke-width="1"
		/>
		<text x={section.x + 16} y={section.y + 22} class="fill-ink-2" font-size="12" font-weight="700" letter-spacing="0.04em">
			{(section.group || m.devices_folders_none()).toUpperCase()}
		</text>
		<text x={section.x + section.width - 16} y={section.y + 22} text-anchor="end" class={section.down > 0 ? 'fill-warning-ink' : 'fill-ink-3'} font-size="11" font-weight="600">
			{section.down > 0 ? m.topology_summary_down({ count: section.down }) : m.topology_summary_devices({ count: section.count })}
		</text>
	</g>
{/each}

{#each layout.edges as edge (`${edge.from}>${edge.to}`)}
	{@const dimmed = linked !== null && !(linked.has(edge.from) && linked.has(edge.to))}
	<path
		d={edge.path}
		fill="none"
		class={edge.impacted ? 'stroke-warning' : 'stroke-ink-3'}
		stroke-width={edge.tree ? 1.6 : 1.2}
		stroke-dasharray={EDGE_DASH[edge.source] || undefined}
		stroke-linecap="round"
		opacity={dimmed ? 0.15 : edge.tree ? 0.9 : 0.5}
	/>
{/each}

{#each layout.nodes as p (p.node.id)}
	{@const link = interactive ? href(p) : null}
	{@const dimmed = linked !== null && !linked.has(p.node.id)}
	<svelte:element
		this={link ? 'a' : 'g'}
		href={link}
		class="topo-node"
		role={link ? undefined : 'img'}
		aria-label={`${p.node.name}: ${caption(p)}`}
		opacity={dimmed ? 0.35 : 1}
		onpointerenter={() => onfocusnode?.(p.node.id)}
		onpointerleave={() => onfocusnode?.(null)}
		onfocus={() => onfocusnode?.(p.node.id)}
		onblur={() => onfocusnode?.(null)}
	>
		<title>{p.node.name} — {caption(p)}{p.node.site ? ` — ${p.node.site}` : ''}</title>
		<g transform={`translate(${p.x},${p.y})`}>
			<rect
				width={NODE_WIDTH}
				height={NODE_HEIGHT}
				rx="8"
				class={`topo-box ${p.down ? 'fill-warning-soft stroke-warning' : 'fill-surface'} ${p.impacted ? 'stroke-advisory' : p.down ? '' : 'stroke-line-strong'}`}
				stroke-width={p.down ? 2.5 : p.impacted ? 1.5 : 1}
				stroke-dasharray={p.impacted ? '5 3' : undefined}
			/>
			<rect x="0" y="0" width="5" height={NODE_HEIGHT} rx="2.5" class={tone(p)} />
			<g transform="translate(13,12)" class={p.node.targetId === null ? 'text-ink-3' : 'text-ink-2'}>
				{#if p.node.targetId === null}
					{#if p.node.kind === 'container'}
						<Box class="size-5" size={20} aria-hidden="true" />
					{:else}
						<Monitor class="size-5" size={20} aria-hidden="true" />
					{/if}
				{:else}
					<KindIcon kind={p.node.kind} class="size-5" size={20} />
				{/if}
			</g>
			<text x="42" y="19" class="fill-ink" font-size="13" font-weight="600">{short(p.node.name)}</text>
			<text x="42" y="35" class={p.down ? 'fill-warning-ink' : p.impacted ? 'fill-advisory-ink' : 'fill-ink-2'} font-size="11" font-weight={p.down || p.impacted ? 600 : 400}>
				{caption(p)}
			</text>
			{#if p.down || p.impacted}
				<!-- Badge: "!" for down, "↯" (impact) for behind a down device. -->
				<circle cx={NODE_WIDTH - 14} cy="14" r="8" class={p.down ? 'fill-warning' : 'fill-advisory'} />
				<text x={NODE_WIDTH - 14} y="18" text-anchor="middle" font-size="11" font-weight="800" class="fill-surface">{p.down ? '!' : '↓'}</text>
			{/if}
		</g>
	</svelte:element>
{/each}

<style>
	:global(.topo-node:focus-visible) {
		outline: none;
	}
	:global(.topo-node:focus-visible .topo-box) {
		stroke: var(--c-signal);
		stroke-width: 3;
	}
	:global(a.topo-node:hover .topo-box) {
		stroke: var(--c-signal);
	}
</style>
