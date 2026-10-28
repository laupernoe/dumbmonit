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
	 *
	 * Weathermap (`traffic`): a link with port counters is drawn as a pipe whose
	 * width and tint follow its load, dashed when the port loses frames, with
	 * dots moving along it in the direction of the busiest flow (still under
	 * reduced motion). Its `412 Mb/s · 41 %` label shows on hover or focus, and
	 * always from 50 %: the load is written, never only coloured. A link behind
	 * a down device is greyed out, like the dependants it cascades to.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import KindIcon from '#lib/components/KindIcon.svelte';
	import { STATE_LABEL, type TargetState } from '#lib/format.js';
	import { Box, Monitor, Server } from 'lucide-svelte';
	import type { Frame, MapLayout, NodeState, PlacedEdge, PlacedNode } from './layout';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { EDGE_DASH, guestLabel, portsText, sourceShort, zoneLabel } from './legend';
	import { linkLoad, loadText, type Band } from './traffic';

	interface Props {
		layout: MapLayout;
		/** Nodes are focusable buttons; off for the print sheet. */
		interactive?: boolean;
		/** Nodes to light up (a dependency chain); everything else dims. */
		highlight?: Set<string> | null;
		selected?: string | null;
		onhover?: (id: string | null) => void;
		onselect?: (id: string) => void;
		/** Correcting the map: links become buttons too. */
		editing?: boolean;
		/** Key of the link picked while correcting. */
		selectedEdge?: string | null;
		onedge?: (key: string) => void;
		/** Node a dragged node would be attached under. */
		dropTarget?: string | null;
		/** Node being dragged: drawn faded where it was. */
		dragging?: string | null;
		/** Weathermap: draw the load of the links that have traffic. */
		traffic?: boolean;
		/** Key of the link whose traffic panel is open. */
		selectedLink?: string | null;
		onlink?: (key: string) => void;
	}

	let {
		layout,
		interactive = true,
		highlight = null,
		selected = null,
		onhover,
		onselect,
		editing = false,
		selectedEdge = null,
		onedge,
		dropTarget = null,
		dragging = null,
		traffic = false,
		selectedLink = null,
		onlink
	}: Props = $props();

	/** The link under the pointer or focus: its label shows. */
	let hoverLink = $state<string | null>(null);

	const names = $derived(new Map(layout.nodes.map((p) => [p.node.id, p.node.name])));

	function edgeLabel(edge: PlacedEdge): string {
		const kind = edge.source === 'wan' ? '' : sourceShort(edge.source);
		const label = m.topology_edit_edge_label({ from: names.get(edge.from) ?? edge.from, to: names.get(edge.to) ?? edge.to, kind });
		const ports = portsText(edge.fromPort, edge.toPort);
		return ports ? m.topology_edit_edge_ports_label({ label, ports }) : label;
	}

	function onedgekey(event: KeyboardEvent, key: string) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			onedge?.(key);
		}
	}

	function onlinkkey(event: KeyboardEvent, key: string) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			onlink?.(key);
		}
	}

	/** Whole class names, so Tailwind sees them: the tint of each load band. */
	const BAND_STROKE: Record<Band, string> = {
		idle: 'stroke-ink-3',
		low: 'stroke-signal',
		mid: 'stroke-signal',
		high: 'stroke-advisory',
		full: 'stroke-warning',
		unknown: 'stroke-ink-2'
	};

	/** The links that show their traffic: those that have some, when the weathermap is on. */
	function weather(edge: PlacedEdge) {
		return traffic && edge.traffic ? { traffic: edge.traffic, load: linkLoad(edge.traffic, edge.from) } : null;
	}

	function weatherText(edge: PlacedEdge): string {
		const w = weather(edge);
		if (!w) return '';
		const text = loadText(w.load, getLocale());
		return w.load.errors ? `${text} · ${m.topology_traffic_errors_short()}` : text;
	}

	function weatherLabel(edge: PlacedEdge): string {
		return m.topology_traffic_edge_label({ from: names.get(edge.from) ?? edge.from, to: names.get(edge.to) ?? edge.to, load: weatherText(edge) });
	}

	/** The label shows on hover, focus or selection, and always on a link past half its capacity. */
	function labelled(edge: PlacedEdge): boolean {
		const w = weather(edge);
		if (!w || edge.stale) return false;
		return hoverLink === edge.key || selectedLink === edge.key || (w.load.pct ?? 0) >= 50;
	}

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
		return [p.node.name, guestLabel(p.node), l.caption ?? stateText(p.node.state), p.node.ips?.join(', '), p.node.subnet, p.node.site].filter(Boolean).join(' · ');
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
			case 'location':
				return m.topology_site_via_location();
			case 'manual':
				return m.topology_site_via_manual();
			default:
				return null;
		}
	}

	function summary(frame: Frame): string {
		return frame.down > 0 ? m.topology_summary_down({ count: frame.down }) : m.topology_summary_devices({ count: frame.count });
	}

	/** How many characters of `em` pixels each fit in `room` pixels (at least a few). */
	function fit(room: number, em: number): number {
		return Math.max(6, Math.floor(room / em));
	}

	function frameTitle(frame: Frame): string {
		if (frame.kind === 'main') return frame.label ?? m.topology_site_main();
		if (frame.kind === 'site') return frame.label ?? '';
		if (frame.kind === 'zone') return zoneLabel(frame.label, frame.vlan, frame.vlanName);
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
		{#if frame.kind === 'zone'}
			<rect x={frame.x} y={frame.y} width={frame.width} height={frame.height} rx="12" fill="none" class="stroke-line-strong" stroke-width="1" stroke-dasharray="3 4" />
			<text x={frame.x + 16} y={frame.y + 25} class="fill-ink-2" font-size="11.5" font-weight="600">{short(frameTitle(frame), fit(frame.width - 32 - summary(frame).length * 6.8, 6.8))}</text>
		{:else if frame.kind === 'group'}
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
			<text x={frame.x + 22} y={frame.y + 27} class="fill-ink" font-size="14" font-weight="700">{short(frameTitle(frame), fit(frame.width - 22 - 18 - summary(frame).length * 6.8 - 12, 8.4))}</text>
			{#if source}
				<text x={frame.x + 22} y={frame.y + 45} class="fill-ink-3" font-size="11.5" font-weight="500">{short(source, fit(frame.width - 22 - 18, 6.4))}</text>
			{/if}
		{/if}
		<text
			x={frame.x + frame.width - 18}
			y={frame.y + (frame.kind === 'group' || frame.kind === 'zone' ? 25 : 27)}
			text-anchor="end"
			class={frame.down > 0 ? 'fill-warning-ink' : 'fill-ink-3'}
			font-size="11.5"
			font-weight="600"
		>
			{summary(frame)}
		</text>
	</g>
{/each}

{#each layout.edges as edge (edge.key)}
	{@const lit = highlight !== null && highlight.has(edge.from) && highlight.has(edge.to)}
	{@const dimmed = highlight !== null && !lit}
	{@const dependency = edge.style === 'dependency'}
	{@const tone = edgeTone(edge.impacted, lit, dependency)}
	{@const w = weather(edge)}
	<g opacity={dimmed ? 0.1 : w ? (edge.stale ? 0.4 : 0.95) : edge.impacted || lit ? 1 : dependency ? 0.8 : 0.6}>
		{#if edge.wan}
			<!-- A WAN link: a wide soft band under a dashed line, a tunnel between two sites. -->
			<path d={edge.path} fill="none" class={STROKE[tone]} stroke-width="7" stroke-linecap="round" opacity="0.14" />
		{/if}
		{#if w}
			<!-- The load: a pipe as wide as the link is busy, dashed when its port loses frames. A greyed link has stale figures. -->
			<path
				d={edge.path}
				fill="none"
				class={edge.stale ? 'stroke-ink-3' : BAND_STROKE[w.load.band]}
				stroke-width={w.load.width}
				stroke-dasharray={w.load.errors ? '5 3' : edge.stale ? '2 3' : undefined}
				stroke-linecap={w.load.errors ? 'butt' : 'round'}
			/>
			{#if w.load.forward !== null && !edge.stale}
				<path
					d={edge.path}
					fill="none"
					class={`stroke-surface ${w.load.forward ? 'topo-flow' : 'topo-flow topo-flow-back'}`}
					stroke-width={Math.max(1.6, w.load.width * 0.5)}
					stroke-dasharray="0.1 10"
					stroke-linecap="round"
					opacity="0.9"
				/>
			{/if}
		{:else}
			<path
				d={edge.path}
				fill="none"
				class={STROKE[tone]}
				stroke-width={edge.wan ? 2.2 : dependency ? (lit || edge.impacted ? 2.2 : 1.6) : lit || edge.impacted ? 1.6 : 1.1}
				stroke-dasharray={EDGE_DASH[edge.wan && edge.source === 'wan' ? 'wan' : edge.source] || undefined}
				stroke-linecap="round"
			/>
		{/if}
		{#if edge.arrow}
			<polygon points={edge.arrow} class={FILL[tone]} />
		{/if}
		{#if edge.origin !== 'deduced'}
			<!-- Corrected by hand: a hollow diamond; set by a learned rule: a filled one. Never colour alone. -->
			<path
				d={`M${edge.mid.x},${edge.mid.y - 5.5} L${edge.mid.x + 5.5},${edge.mid.y} L${edge.mid.x},${edge.mid.y + 5.5} L${edge.mid.x - 5.5},${edge.mid.y} Z`}
				class={`${edge.origin === 'rule' ? FILL[tone] : 'fill-canvas'} ${STROKE[tone]}`}
				stroke-width="1.6"
			/>
		{/if}
		{#if selectedEdge === edge.key || (w && selectedLink === edge.key)}
			<path d={edge.path} fill="none" class="stroke-signal" stroke-width={(w?.load.width ?? 1) + 4} stroke-linecap="round" opacity="0.35" />
		{/if}
	</g>
{/each}

{#if editing && interactive}
	<!-- Wide invisible strokes: a link is easy to hit with a finger, and reachable with Tab. -->
	{#each layout.edges as edge (edge.key)}
		{#if edge.source !== 'wan' && edge.from && edge.to}
			<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
			<path
				d={edge.path}
				fill="none"
				stroke="transparent"
				stroke-width="16"
				class="topo-edge"
				pointer-events="stroke"
				role="button"
				tabindex="0"
				aria-label={edgeLabel(edge)}
				aria-pressed={selectedEdge === edge.key}
				onclick={() => onedge?.(edge.key)}
				onkeydown={(event: KeyboardEvent) => onedgekey(event, edge.key)}
			><title>{edgeLabel(edge)}</title></path>
		{/if}
	{/each}
{/if}

{#if traffic && !editing && interactive}
	<!-- Wide invisible strokes: a link with traffic opens its panel, by touch or with Tab. -->
	{#each layout.edges as edge (edge.key)}
		{#if edge.traffic}
			<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
			<path
				d={edge.path}
				fill="none"
				stroke="transparent"
				stroke-width="18"
				class="topo-edge"
				pointer-events="stroke"
				role="button"
				tabindex="0"
				aria-label={weatherLabel(edge)}
				aria-pressed={selectedLink === edge.key}
				onclick={() => onlink?.(edge.key)}
				onkeydown={(event: KeyboardEvent) => onlinkkey(event, edge.key)}
				onpointerenter={() => (hoverLink = edge.key)}
				onpointerleave={() => (hoverLink = null)}
				onfocus={() => (hoverLink = edge.key)}
				onblur={() => (hoverLink = null)}
			><title>{weatherLabel(edge)}</title></path>
		{/if}
	{/each}
	{#each layout.edges as edge (edge.key)}
		{#if labelled(edge)}
			{@const text = weatherText(edge)}
			{@const width = text.length * 6.3 + 14}
			<g transform={`translate(${edge.mid.x},${edge.mid.y})`} pointer-events="none">
				<rect x={-width / 2} y="-10" {width} height="20" rx="10" class="fill-surface stroke-line-strong" stroke-width="1" />
				<text y="4" text-anchor="middle" class="fill-ink" font-size="11" font-weight="600">{text}</text>
			</g>
		{/if}
	{/each}
{/if}

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
		data-node-id={interactive ? p.node.id : undefined}
		transform={`translate(${p.x},${p.y})`}
		opacity={dragging === p.node.id ? 0.35 : dimmed ? 0.22 : 1}
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
		{#if dropTarget === p.node.id}
			<circle r={p.r + 10} fill="none" class="stroke-signal" stroke-width="2.5" stroke-dasharray="5 4" />
		{/if}
		<!-- A wider invisible disc: an easier target for a finger. -->
		<circle r={p.r + 8} fill="transparent" />
		<circle r={p.r} class={`topo-disc ${l.fill} ${l.ring}`} stroke-width={l.width} stroke-dasharray={l.dash} />
		<g transform={`translate(${-icon / 2},${-icon / 2})`} class={p.down ? 'text-warning-ink' : p.node.targetId === null ? 'text-ink-3' : 'text-ink-2'}>
			{#if p.node.targetId === null}
				{#if p.node.kind === 'container'}
					<Box class="size-4" size={icon} aria-hidden="true" />
				{:else if p.node.kind === 'pve_node'}
					<Server class="size-4" size={icon} aria-hidden="true" />
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
	:global(.topo-edge) {
		cursor: pointer;
		outline: none;
	}
	:global(.topo-edge:hover),
	:global(.topo-edge:focus-visible) {
		stroke: var(--c-signal);
		stroke-opacity: 0.3;
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
	:global(.topo-flow) {
		animation: topo-flow 1.1s linear infinite;
	}
	:global(.topo-flow-back) {
		animation-direction: reverse;
	}
	@keyframes -global-topo-flow {
		to {
			stroke-dashoffset: -10.1;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		:global(.topo-pulse),
		:global(.topo-flow) {
			animation: none;
		}
	}
</style>
