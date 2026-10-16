<svelte:options namespace="svg" />

<script lang="ts">
	/**
	 * The drawing of the map, in map coordinates: site and folder frames,
	 * links, then the nodes on top. Pure SVG with token colours (`fill-*`,
	 * `stroke-*`), so the same markup serves the interactive canvas, the
	 * print sheet and the SVG/PNG export (which inlines the computed styles).
	 *
	 * A state is never colour only: every node carries a glyph badge (check,
	 * exclamation, down arrow, dots, pause) and anything that needs attention
	 * spells its state under the name. A down node gets a thick ring and a
	 * halo; what it takes down with it a dashed ring and the word "Unreachable".
	 */
	import { m } from '#lib/paraglide/messages.js';
	import KindIcon from '#lib/components/KindIcon.svelte';
	import { STATE_LABEL, type TargetState } from '#lib/format.js';
	import { Box, Monitor } from 'lucide-svelte';
	import type { Frame, MapLayout, NodeState, PlacedNode } from './layout';
	import { EDGE_DASH } from './legend';

	interface Props {
		layout: MapLayout;
		/** Nodes are focusable buttons; off for the print sheet. */
		interactive?: boolean;
		/** Nodes to light up (a dependency chain); everything else dims. */
		highlight?: Set<string> | null;
		selected?: string | null;
		onhover?: (id: string | null) => void;
		onselect?: (id: string) => void;
	}

	let { layout, interactive = true, highlight = null, selected = null, onhover, onselect }: Props = $props();

	type Badge = 'ok' | 'down' | 'impacted' | 'warn' | 'wait' | 'idle';

	interface Look {
		fill: string;
		ring: string;
		width: number;
		dash: string | undefined;
		badge: Badge;
		badgeFill: string;
		/** Spelled under the name; `null` when all is well. */
		caption: string | null;
		captionClass: string;
	}

	function stateText(state: NodeState): string {
		if (state === 'running') return m.topology_state_running();
		if (state === 'stopped') return m.topology_state_stopped();
		return STATE_LABEL[state as TargetState];
	}

	function look(p: PlacedNode): Look {
		if (p.down)
			return { fill: 'fill-warning-soft', ring: 'stroke-warning', width: 3, dash: undefined, badge: 'down', badgeFill: 'fill-warning', caption: stateText(p.node.state), captionClass: 'fill-warning-ink' };
		if (p.impacted)
			return { fill: 'fill-surface', ring: 'stroke-advisory', width: 2, dash: '4 3', badge: 'impacted', badgeFill: 'fill-advisory', caption: m.topology_state_suppressed(), captionClass: 'fill-advisory-ink' };
		switch (p.node.state) {
			case 'online':
			case 'running':
				return { fill: 'fill-surface', ring: 'stroke-signal', width: 2, dash: undefined, badge: 'ok', badgeFill: 'fill-signal', caption: null, captionClass: '' };
			case 'misconfigured':
				return { fill: 'fill-surface', ring: 'stroke-advisory', width: 2, dash: undefined, badge: 'warn', badgeFill: 'fill-advisory', caption: stateText(p.node.state), captionClass: 'fill-advisory-ink' };
			case 'pending':
			case 'unknown':
				return { fill: 'fill-surface', ring: 'stroke-ink-3', width: 1.5, dash: undefined, badge: 'wait', badgeFill: 'fill-ink-3', caption: stateText(p.node.state), captionClass: 'fill-ink-2' };
			default:
				return { fill: 'fill-surface-2', ring: 'stroke-ink-3', width: 1.5, dash: '2 3', badge: 'idle', badgeFill: 'fill-ink-3', caption: stateText(p.node.state), captionClass: 'fill-ink-3' };
		}
	}

	type Tone = 'warning' | 'signal' | 'ink-2' | 'ink-3';
	/** Whole class names, so Tailwind sees them. */
	const STROKE: Record<Tone, string> = { warning: 'stroke-warning', signal: 'stroke-signal', 'ink-2': 'stroke-ink-2', 'ink-3': 'stroke-ink-3' };
	const FILL: Record<Tone, string> = { warning: 'fill-warning', signal: 'fill-signal', 'ink-2': 'fill-ink-2', 'ink-3': 'fill-ink-3' };

	function edgeTone(impacted: boolean, lit: boolean, dependency: boolean): Tone {
		return impacted ? 'warning' : lit ? 'signal' : dependency ? 'ink-2' : 'ink-3';
	}

	function short(name: string, max: number): string {
		return name.length > max ? `${name.slice(0, max - 1)}…` : name;
	}

	function label(p: PlacedNode, l: Look): string {
		const guest = p.node.targetId === null ? (p.node.kind === 'container' ? m.topology_guest_container() : m.topology_guest_vm()) : null;
		return [p.node.name, guest, l.caption ?? stateText(p.node.state), p.node.site].filter(Boolean).join(' · ');
	}

	function frameSource(frame: Frame): string | null {
		if (frame.kind === 'main') return null;
		switch (frame.source) {
			case 'relay':
				return m.topology_site_via_relay();
			case 'agent':
				return m.topology_site_via_agent();
			case 'tag':
				return m.topology_site_via_tag();
			default:
				return null;
		}
	}

	function frameTitle(frame: Frame): string {
		if (frame.kind === 'main') return frame.label ?? m.topology_site_main();
		if (frame.kind === 'site') return frame.label ?? '';
		return (frame.label ?? m.devices_folders_none()).toUpperCase();
	}

	function onkey(event: KeyboardEvent, id: string) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			onselect?.(id);
		}
	}
</script>

{#each layout.frames as frame (frame.key)}
	{@const source = frameSource(frame)}
	<g class="topo-frame" opacity={highlight ? 0.6 : 1}>
		{#if frame.kind === 'group'}
			<rect x={frame.x} y={frame.y} width={frame.width} height={frame.height} rx="14" class="fill-surface stroke-line" stroke-width="1" />
			<text x={frame.x + 18} y={frame.y + 25} class="fill-ink-2" font-size="11.5" font-weight="700" letter-spacing="0.05em">{short(frameTitle(frame), 40)}</text>
		{:else}
			<rect
				x={frame.x}
				y={frame.y}
				width={frame.width}
				height={frame.height}
				rx="22"
				class={frame.kind === 'site' ? 'fill-surface-2 stroke-line-strong' : 'stroke-line-strong'}
				fill={frame.kind === 'main' ? 'none' : undefined}
				stroke-width="1.5"
				stroke-dasharray={frame.kind === 'site' ? '7 5' : '2 5'}
			/>
			<text x={frame.x + 22} y={frame.y + 27} class="fill-ink" font-size="14" font-weight="700">
				{short(frameTitle(frame), 34)}{#if source}<tspan class="fill-ink-3" font-size="11.5" font-weight="500" dx="8">{source}</tspan>{/if}
			</text>
		{/if}
		<text
			x={frame.x + frame.width - 18}
			y={frame.y + (frame.kind === 'group' ? 25 : 27)}
			text-anchor="end"
			class={frame.down > 0 ? 'fill-warning-ink' : 'fill-ink-3'}
			font-size="11.5"
			font-weight="600"
		>
			{frame.down > 0 ? m.topology_summary_down({ count: frame.down }) : m.topology_summary_devices({ count: frame.count })}
		</text>
	</g>
{/each}

{#each layout.edges as edge (edge.key)}
	{@const lit = highlight !== null && highlight.has(edge.from) && highlight.has(edge.to)}
	{@const dimmed = highlight !== null && !lit}
	{@const dependency = edge.style === 'dependency'}
	{@const tone = edgeTone(edge.impacted, lit, dependency)}
	<g opacity={dimmed ? 0.1 : edge.impacted || lit ? 1 : dependency ? 0.8 : 0.6}>
		{#if edge.wan}
			<!-- A WAN link: a wide soft band under a dashed line, a tunnel between two sites. -->
			<path d={edge.path} fill="none" class={STROKE[tone]} stroke-width="7" stroke-linecap="round" opacity="0.14" />
		{/if}
		<path
			d={edge.path}
			fill="none"
			class={STROKE[tone]}
			stroke-width={edge.wan ? 2.2 : dependency ? (lit || edge.impacted ? 2.2 : 1.6) : lit || edge.impacted ? 1.6 : 1.1}
			stroke-dasharray={EDGE_DASH[edge.wan && edge.source === 'wan' ? 'wan' : edge.source] || undefined}
			stroke-linecap="round"
		/>
		{#if edge.arrow}
			<polygon points={edge.arrow} class={FILL[tone]} />
		{/if}
	</g>
{/each}

{#each layout.ports as port, i (i)}
	<circle cx={port.x} cy={port.y} r="5" class={`fill-surface ${port.down ? 'stroke-warning' : 'stroke-ink-3'}`} stroke-width="2" opacity={highlight ? 0.4 : 1} />
{/each}

{#each layout.nodes as p (p.node.id)}
	{@const l = look(p)}
	{@const dimmed = highlight !== null && !highlight.has(p.node.id)}
	{@const icon = p.r >= 22 ? 20 : 16}
	{@const bx = p.r * 0.72}
	{@const by = -p.r * 0.72}
	<!-- A button on the canvas, a plain picture on the sheet: the role follows `interactive`. -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<g
		class={interactive ? 'topo-node' : undefined}
		transform={`translate(${p.x},${p.y})`}
		opacity={dimmed ? 0.22 : 1}
		role={interactive ? 'button' : 'img'}
		tabindex={interactive ? 0 : undefined}
		aria-label={label(p, l)}
		aria-pressed={interactive ? selected === p.node.id : undefined}
		onpointerenter={interactive ? () => onhover?.(p.node.id) : undefined}
		onpointerleave={interactive ? () => onhover?.(null) : undefined}
		onfocus={interactive ? () => onhover?.(p.node.id) : undefined}
		onblur={interactive ? () => onhover?.(null) : undefined}
		onclick={interactive ? () => onselect?.(p.node.id) : undefined}
		onkeydown={interactive ? (event: KeyboardEvent) => onkey(event, p.node.id) : undefined}
	>
		<title>{label(p, l)}</title>
		{#if p.down}
			<circle r={p.r + 11} class={`fill-warning ${interactive ? 'topo-pulse' : ''}`} opacity="0.16" />
		{/if}
		{#if selected === p.node.id}
			<circle r={p.r + 6} fill="none" class="stroke-signal" stroke-width="2.5" />
		{/if}
		<!-- A wider invisible disc: an easier target for a finger. -->
		<circle r={p.r + 8} fill="transparent" />
		<circle r={p.r} class={`topo-disc ${l.fill} ${l.ring}`} stroke-width={l.width} stroke-dasharray={l.dash} />
		<g transform={`translate(${-icon / 2},${-icon / 2})`} class={p.down ? 'text-warning-ink' : p.node.targetId === null ? 'text-ink-3' : 'text-ink-2'}>
			{#if p.node.targetId === null}
				{#if p.node.kind === 'container'}
					<Box class="size-4" size={icon} aria-hidden="true" />
				{:else}
					<Monitor class="size-4" size={icon} aria-hidden="true" />
				{/if}
			{:else}
				<KindIcon kind={p.node.kind} class="size-4" size={icon} />
			{/if}
		</g>
		<g transform={`translate(${bx},${by})`}>
			<circle r="7.5" class={`${l.badgeFill} stroke-canvas`} stroke-width="2" />
			<g class="stroke-surface" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none">
				{#if l.badge === 'ok'}
					<path d="M-3,0.2 L-1,2.2 L3,-2" />
				{:else if l.badge === 'down' || l.badge === 'warn'}
					<path d="M0,-3.4 L0,0.6 M0,3.2 L0,3.3" />
				{:else if l.badge === 'impacted'}
					<path d="M0,-3.4 L0,3 M-2.6,0.6 L0,3.2 L2.6,0.6" />
				{:else if l.badge === 'wait'}
					<path d="M-3,0 L-2.9,0 M0,0 L0.1,0 M3,0 L3.1,0" />
				{:else}
					<path d="M-1.6,-2.6 L-1.6,2.6 M1.6,-2.6 L1.6,2.6" />
				{/if}
			</g>
		</g>
		<text
			y={p.r + 15}
			text-anchor="middle"
			class="topo-label fill-ink stroke-canvas"
			stroke-width="4"
			paint-order="stroke"
			stroke-linejoin="round"
			font-size={p.r >= 22 ? 12 : 11.5}
			font-weight="600"
		>
			{short(p.node.name, p.r >= 22 ? 18 : 16)}
		</text>
		{#if l.caption}
			<text
				y={p.r + 28}
				text-anchor="middle"
				class={`${l.captionClass} stroke-canvas`}
				stroke-width="4"
				paint-order="stroke"
				stroke-linejoin="round"
				font-size="10.5"
				font-weight={p.down || p.impacted ? 700 : 500}
			>
				{short(l.caption, 20)}
			</text>
		{/if}
	</g>
{/each}

<style>
	:global(.topo-node) {
		cursor: pointer;
		outline: none;
	}
	:global(.topo-node:focus-visible .topo-disc),
	:global(.topo-node:hover .topo-disc) {
		stroke: var(--c-signal);
		stroke-width: 3;
	}
	:global(.topo-pulse) {
		transform-box: fill-box;
		transform-origin: center;
		animation: topo-pulse 2.4s ease-in-out infinite;
	}
	@keyframes -global-topo-pulse {
		0%,
		100% {
			transform: scale(0.85);
			opacity: 0.28;
		}
		50% {
			transform: scale(1.12);
			opacity: 0.1;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		:global(.topo-pulse) {
			animation: none;
		}
	}
</style>
