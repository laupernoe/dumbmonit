<script lang="ts">
	/**
	 * Detected changes, newest first: what kind, how much text moved, when.
	 * Each row opens the before/after comparison. The icon carries the kind on
	 * its own (never colour alone) — a diff glyph, a page appearing, a page
	 * disappearing.
	 */
	import { goto } from '$app/navigation';
	import type { WebchangeChange, WebchangeChangeKind } from '$lib/api';
	import { formatDateTime, formatRelative } from '$lib/format';
	import { EmptyState, Skeleton } from '$lib/ui';
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
	const KIND_WORD: Record<WebchangeChangeKind, string> = {
		changed: 'Changed',
		new_page: 'New page',
		removed_page: 'Removed page'
	};
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
	<div class="space-y-2" aria-busy="true" aria-label="Loading changes">
		<Skeleton class="h-10 w-full" />
		<Skeleton class="h-10 w-full" />
		<Skeleton class="h-10 w-3/4" />
	</div>
{:else if changes.length === 0}
	<EmptyState
		icon={FileDiff}
		tone="signal"
		title={filteredUrl ? 'No change on this page yet.' : `Watching ${pageCount} ${pageCount === 1 ? 'page' : 'pages'}.`}
		description={filteredUrl
			? 'Nothing has moved on this page since it was first checked.'
			: 'The first check is the baseline; changes will appear here.'}
	/>
{:else}
	<ol class="divide-y divide-line">
		{#each changes as change, i (change.id)}
			{@const Icon = KIND_ICON[change.kind]}
			<li class="rise-in" style={`--rise-delay: ${Math.min(i, 8) * 30}ms`}>
				<button
					type="button"
					onclick={() => open(change)}
					class="flex w-full flex-wrap items-center gap-x-3 gap-y-1 px-1 py-2.5 text-left hover:bg-surface-2"
				>
					<span class={`flex shrink-0 items-center gap-1.5 font-semibold ${KIND_TINT[change.kind]}`}>
						<Icon class="size-4" aria-hidden="true" />
						{KIND_WORD[change.kind]}
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
						View
					</span>
				</button>
			</li>
		{/each}
	</ol>
{/if}
