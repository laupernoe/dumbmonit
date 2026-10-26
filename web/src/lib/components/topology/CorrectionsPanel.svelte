<script lang="ts">
	/**
	 * Under the map while it is being corrected: what DumbMonit has learned
	 * from the corrections (suggestions to apply or decline, never applied on
	 * their own), the rules already decided, and every correction with who
	 * made it, each one removable.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { Lightbulb } from 'lucide-svelte';
	import { Button } from '#lib/ui/index.js';
	import type { TopologyCorrection, TopologyRule, TopologySuggestion } from '#lib/api/index.js';
	import { correctionBy, correctionText, ruleText } from './editText';

	interface Props {
		corrections: TopologyCorrection[];
		rules: TopologyRule[];
		suggestions: TopologySuggestion[];
		name: (id: string) => string;
		busy: boolean;
		onaccept: (suggestion: TopologySuggestion) => void;
		ondecline: (suggestion: TopologySuggestion) => void;
		onremoverule: (rule: TopologyRule) => void;
		onremove: (correction: TopologyCorrection) => void;
		class?: string;
	}

	let { corrections, rules, suggestions, name, busy, onaccept, ondecline, onremoverule, onremove, class: className = '' }: Props = $props();

	const newestFirst = $derived([...corrections].sort((a, b) => b.id - a.id));
</script>

<div class={`grid gap-4 lg:grid-cols-2 ${className}`}>
	<section class="flex flex-col gap-3" aria-labelledby="topo-learned">
		<h2 id="topo-learned" class="text-base font-semibold text-ink">{m.topology_edit_learned_title()}</h2>
		{#if suggestions.length === 0 && rules.length === 0}
			<p class="text-sm text-ink-2">{m.topology_edit_learned_empty()}</p>
		{/if}
		{#each suggestions as s (s.key)}
			<article class="flex flex-col gap-2.5 rounded-[var(--radius-card)] border border-advisory/50 bg-advisory-soft p-3.5">
				<p class="flex items-start gap-2 font-semibold text-ink">
					<Lightbulb class="mt-0.5 size-4 shrink-0 text-advisory-ink" aria-hidden="true" />
					<span>{ruleText(s, name)}</span>
				</p>
				<p class="text-sm text-ink-2">{m.topology_edit_suggestion_why({ count: s.evidence, matches: s.matches })}</p>
				<div class="flex flex-wrap gap-2">
					<Button variant="primary" size="sm" disabled={busy} onclick={() => onaccept(s)}>{m.topology_edit_accept()}</Button>
					<Button variant="ghost" size="sm" disabled={busy} onclick={() => ondecline(s)}>{m.topology_edit_decline()}</Button>
				</div>
			</article>
		{/each}
		{#if rules.length > 0}
			<ul class="flex flex-col divide-y divide-line rounded-[var(--radius-card)] border border-line bg-surface">
				{#each rules as rule (rule.id)}
					<li class="flex items-start gap-3 px-3.5 py-2.5">
						<div class="min-w-0 flex-1">
							<p class={`text-sm ${rule.status === 'active' ? 'text-ink' : 'text-ink-3 line-through decoration-ink-3/60'}`}>{ruleText(rule, name)}</p>
							<p class="text-xs text-ink-3">
								{rule.status === 'active' ? m.topology_edit_rule_matches({ matches: rule.matches }) : m.topology_edit_rule_declined()}
							</p>
						</div>
						<Button variant="ghost" size="sm" disabled={busy} onclick={() => onremoverule(rule)}>{m.topology_edit_remove()}</Button>
					</li>
				{/each}
			</ul>
		{/if}
	</section>

	<section class="flex flex-col gap-3" aria-labelledby="topo-corrections">
		<h2 id="topo-corrections" class="text-base font-semibold text-ink">{m.topology_edit_list_title({ count: corrections.length })}</h2>
		{#if corrections.length === 0}
			<p class="text-sm text-ink-2">{m.topology_edit_list_empty()}</p>
		{:else}
			<ul class="flex max-h-[28rem] flex-col divide-y divide-line overflow-y-auto rounded-[var(--radius-card)] border border-line bg-surface">
				{#each newestFirst as c (c.id)}
					<li class="flex items-start gap-3 px-3.5 py-2.5">
						<div class="min-w-0 flex-1">
							<p class="text-sm break-words text-ink">{correctionText(c, name)}</p>
							<p class="text-xs text-ink-3">{correctionBy(c)}</p>
						</div>
						<Button variant="ghost" size="sm" disabled={busy} onclick={() => onremove(c)}>{m.topology_edit_remove()}</Button>
					</li>
				{/each}
			</ul>
		{/if}
	</section>
</div>
