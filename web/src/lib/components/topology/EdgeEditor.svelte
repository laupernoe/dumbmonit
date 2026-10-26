<script lang="ts">
	/**
	 * The panel of a link picked while correcting the map: its two ends, its
	 * type, who drew it — DumbMonit's deduction, a correction by hand, a
	 * learned rule — and the one thing to do about it: mark it wrong, or take
	 * back the correction or the rule that drew it.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { X } from 'lucide-svelte';
	import { Button } from '#lib/ui/index.js';
	import type { TopologyCorrection, TopologyRule } from '#lib/api/index.js';
	import { correctionBy, correctionText, ruleText } from './editText';
	import { sourceLabel } from './legend';
	import type { MapEdge } from './layout';

	interface Props {
		edge: MapEdge;
		name: (id: string) => string;
		corrections: TopologyCorrection[];
		rules: TopologyRule[];
		busy: boolean;
		onwrong: (edge: MapEdge) => void;
		onremovecorrection: (correction: TopologyCorrection) => void;
		onremoverule: (rule: TopologyRule) => void;
		onselect: (id: string) => void;
		onclose: () => void;
	}

	let { edge, name, corrections, rules, busy, onwrong, onremovecorrection, onremoverule, onselect, onclose }: Props = $props();

	const correction = $derived(edge.correctionId != null ? corrections.find((c) => c.id === edge.correctionId) : undefined);
	const rule = $derived(edge.ruleId != null ? rules.find((r) => r.id === edge.ruleId) : undefined);
	const origin = $derived(edge.origin ?? 'deduced');

	const chip =
		'inline-flex min-h-10 max-w-full items-center rounded-lg border border-line bg-surface-2 px-2.5 text-left text-sm text-ink hover:border-line-strong focus-visible:outline-2 focus-visible:outline-signal';
</script>

<header class="flex items-start gap-3 border-b border-line px-4 py-3">
	<div class="min-w-0 flex-1">
		<h2 class="text-base font-semibold text-ink">{sourceLabel(edge.source)}</h2>
		<p class="text-xs text-ink-2">{m.topology_edit_edge_panel()}</p>
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
	<dl class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-4 gap-y-1.5">
		<dt class="text-ink-3">{m.topology_edit_edge_from()}</dt>
		<dd><button type="button" class={chip} onclick={() => onselect(edge.from)}><span class="truncate">{name(edge.from)}</span></button></dd>
		<dt class="text-ink-3">{m.topology_edit_edge_to()}</dt>
		<dd><button type="button" class={chip} onclick={() => onselect(edge.to)}><span class="truncate">{name(edge.to)}</span></button></dd>
		<dt class="text-ink-3">{m.topology_edit_edge_origin()}</dt>
		<dd class="text-ink">
			{#if origin === 'manual'}
				{m.topology_origin_manual()}
				{#if correction}<span class="block text-xs text-ink-3">{correctionBy(correction)}</span>{/if}
			{:else if origin === 'rule'}
				{m.topology_origin_rule()}
				{#if rule}<span class="block text-xs text-ink-3">{ruleText(rule, name)}</span>{/if}
			{:else}
				{m.topology_origin_deduced()}
			{/if}
		</dd>
	</dl>

	{#if origin === 'manual' && correction}
		<p class="text-ink-2">{correctionText(correction, name)}</p>
		<div><Button variant="secondary" loading={busy} onclick={() => onremovecorrection(correction)}>{m.topology_edit_remove_correction()}</Button></div>
	{:else if origin === 'rule' && rule}
		<p class="text-ink-2">{m.topology_edit_rule_edge_hint()}</p>
		<div><Button variant="secondary" loading={busy} onclick={() => onremoverule(rule)}>{m.topology_edit_remove_rule()}</Button></div>
	{:else if edge.source === 'service'}
		<p class="text-ink-2">{m.topology_edit_service_hint()}</p>
	{:else}
		<p class="text-ink-2">{edge.source === 'parent' ? m.topology_edit_wrong_parent_hint() : m.topology_edit_wrong_hint()}</p>
		<div><Button variant="danger" loading={busy} onclick={() => onwrong(edge)}>{m.topology_edit_wrong()}</Button></div>
	{/if}
</div>
