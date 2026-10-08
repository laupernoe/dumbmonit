<script lang="ts">
	/**
	 * Detected changes, newest first: what kind, how much text moved, when.
	 * Each row opens the before/after comparison. The icon carries the kind on
	 * its own (never colour alone) — a diff glyph, a page appearing, a page
	 * disappearing.
	 */
	import { goto } from '$app/navigation';
	import type { WebchangeChange, WebchangeChangeKind } from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { EmptyState, Skeleton } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Eye, FileDiff, FileMinus, FilePlus } from 'lucide-svelte';
	import { pagePath, shortenPath } from './format';

	interface Props {
		targetId: number;
		changes: WebchangeChange[];
		loading: boolean;
		/** How many pages are watched in total, for the empty state's wording. */
		pageCount: number;
		/** Page the list is filtered to, for the empty state's wording. */
		filteredUrl: string | null;
	}

	let { targetId, changes, loading, pageCount, filteredUrl }: Props = $props();

	const KIND_ICON: Record<WebchangeChangeKind, typeof FileDiff> = {
		changed: FileDiff,
		new_page: FilePlus,
		removed_page: FileMinus
	};
	const kindWord = (kind: WebchangeChangeKind): string =>
		kind === 'changed'
			? m.devicesb_webchange_timeline_changed()
			: kind === 'new_page'
				? m.devicesb_webchange_timeline_new_page()
				: m.devicesb_webchange_timeline_removed_page();
	/** Ink tint of the icon and word — informational, never the only cue (the icon and word already say what happened). */
	const KIND_TINT: Record<WebchangeChangeKind, string> = {
		changed: 'text-info-ink',
		new_page: 'text-info-ink',
		removed_page: 'text-warning-ink'
	};

	function open(change: WebchangeChange) {
		void goto(`/targets/${targetId}/changes/${change.id}`);
	}
</script>

{#if loading}
	<div class="space-y-2" aria-busy="true" aria-label={m.devicesb_webchange_timeline_loading()}>
		<Skeleton class="h-10 w-full" />
		<Skeleton class="h-10 w-full" />
		<Skeleton class="h-10 w-3/4" />
	</div>
{:else if changes.length === 0}
	<EmptyState
		icon={FileDiff}
		tone="signal"
		title={filteredUrl
			? m.devicesb_webchange_timeline_empty_page()
			: pageCount === 1
				? m.devicesb_webchange_timeline_watching_one()
				: m.devicesb_webchange_timeline_watching_other({ count: pageCount })}
		description={filteredUrl
			? m.devicesb_webchange_timeline_empty_page_hint()
			: m.devicesb_webchange_timeline_baseline_hint()}
	/>
{:else}
	<ol class="divide-y divide-line">
		{#each changes as change, i (change.id)}
			{@const Icon = KIND_ICON[change.kind]}
			<li class="rise-in" style={`--rise-delay: ${Math.min(i, 8) * 30}ms`}>
				<button
					type="button"
					onclick={() => open(change)}
					class="flex min-h-11 w-full flex-wrap items-center gap-x-3 gap-y-1 px-1 py-2.5 text-left hover:bg-surface-2"
				>
					<span class={`flex shrink-0 items-center gap-1.5 font-semibold ${KIND_TINT[change.kind]}`}>
						<Icon class="size-4" aria-hidden="true" />
						{kindWord(change.kind)}
					</span>
					<span class="min-w-0 truncate text-sm text-ink" title={change.url}>{shortenPath(pagePath(change.url))}</span>
					<span class="tnum ml-auto flex shrink-0 items-center gap-2 text-[0.8125rem]">
						<span class="text-signal-ink">+{change.added}</span>
						<span class="text-warning-ink">−{change.removed}</span>
					</span>
					<time class="tnum shrink-0 text-[0.8125rem] text-ink-2" datetime={change.detected_at} title={formatDateTime(change.detected_at)}>
						{formatRelative(change.detected_at)}
					</time>
					<span class="inline-flex shrink-0 items-center gap-1 text-[0.8125rem] font-semibold text-ink">
						<Eye class="size-3.5" aria-hidden="true" />
						{m.devicesb_webchange_timeline_view()}
					</span>
				</button>
			</li>
		{/each}
	</ol>
{/if}
