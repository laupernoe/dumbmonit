<script lang="ts">
	/**
	 * Detected changes, newest first, under one heading per day: which page,
	 * what kind, how much text moved, at what time. The icon carries the kind
	 * on its own (never colour alone) — a diff glyph, a page appearing, a page
	 * disappearing — with the word for screen readers and as a tooltip.
	 *
	 * Every row is a link to the change's own page. Given `onselect` (the
	 * device panel's two-column layout), a plain click selects the change
	 * instead and the arrow keys walk the list; a modified click still opens
	 * the page, in a new tab if asked.
	 */
	import type { WebchangeChange, WebchangeChangeKind } from '#lib/api/index.js';
	import { formatDateTime, parseServerDate } from '#lib/format.js';
	import { EmptyState, Skeleton } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { FileDiff, FileMinus, FilePlus } from 'lucide-svelte';
	import { daysBetween, groupByDay, pagePath, shortenPath, stepIndex } from './format';

	interface Props {
		targetId: number;
		changes: WebchangeChange[];
		loading: boolean;
		/** How many pages are watched in total, for the empty state's wording. */
		pageCount: number;
		/** Page the list is filtered to, for the empty state's wording. */
		filteredUrl: string | null;
		/** Change shown next to the list, highlighted; `null` when nothing is. */
		selectedId?: number | null;
		/** Selects a change in place instead of following its link. */
		onselect?: (change: WebchangeChange) => void;
	}

	let { targetId, changes, loading, pageCount, filteredUrl, selectedId = null, onselect }: Props = $props();

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

	const groups = $derived(groupByDay(changes, (change) => parseServerDate(change.detected_at)));

	function dayLabel(day: Date | null): string {
		if (!day) return m.misc_never();
		const ago = daysBetween(day, new Date());
		if (ago === 0) return m.devicesb_webchange_timeline_today();
		if (ago === 1) return m.devicesb_webchange_timeline_yesterday();
		return day.toLocaleDateString(getLocale(), {
			weekday: 'long',
			day: 'numeric',
			month: 'long',
			year: ago > 300 ? 'numeric' : undefined
		});
	}

	function timeOf(value: string): string {
		const date = parseServerDate(value);
		return date ? date.toLocaleTimeString(getLocale(), { hour: '2-digit', minute: '2-digit', hourCycle: 'h23' }) : '';
	}

	const hrefOf = (change: WebchangeChange) => `/targets/${targetId}/changes/${change.id}`;

	let list = $state<HTMLElement | null>(null);

	function click(event: MouseEvent, change: WebchangeChange) {
		if (!onselect || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
		event.preventDefault();
		onselect(change);
	}

	function keydown(event: KeyboardEvent) {
		if (!onselect || changes.length === 0) return;
		const current = changes.findIndex((change) => change.id === selectedId);
		const next = stepIndex(current, event.key, changes.length);
		if (next === current || next < 0) return;
		event.preventDefault();
		onselect(changes[next]);
		list?.querySelector<HTMLElement>(`[data-change="${changes[next].id}"]`)?.focus();
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
	<!-- svelte-ignore a11y_no_static_element_interactions: the arrow keys are a shortcut over links that work on their own -->
	<div bind:this={list} onkeydown={keydown}>
		{#each groups as group (group.key)}
			<section aria-label={dayLabel(group.day)}>
				<h3 class={`${onselect ? 'sticky top-0 z-10' : ''} bg-surface px-2 pt-3 pb-1 text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase`}>
					{dayLabel(group.day)}
				</h3>
				<ol>
					{#each group.items as change (change.id)}
						{@const Icon = KIND_ICON[change.kind]}
						{@const selected = change.id === selectedId}
						<li>
							<a
								href={hrefOf(change)}
								data-change={change.id}
								onclick={(event) => click(event, change)}
								aria-current={selected ? 'true' : undefined}
								title={kindWord(change.kind)}
								class={`flex min-h-11 items-center gap-3 rounded-lg px-2 py-1.5 ${selected ? 'bg-surface-2 ring-1 ring-line-strong' : 'hover:bg-surface-2'}`}
							>
								<Icon class={`size-4 shrink-0 ${KIND_TINT[change.kind]}`} aria-hidden="true" />
								<span class="sr-only">{kindWord(change.kind)}</span>
								<span class="min-w-0 flex-1 truncate text-sm font-semibold text-ink" title={change.url}>{shortenPath(pagePath(change.url), 48)}</span>
								<span class="tnum flex shrink-0 items-center gap-1.5 text-[0.75rem]">
									{#if change.added > 0}<span class="text-signal-ink">+{change.added}</span>{/if}
									{#if change.removed > 0}<span class="text-warning-ink">−{change.removed}</span>{/if}
								</span>
								<time class="tnum w-11 shrink-0 text-right text-[0.8125rem] text-ink-2" datetime={change.detected_at} title={formatDateTime(change.detected_at)}>
									{timeOf(change.detected_at)}
								</time>
							</a>
						</li>
					{/each}
				</ol>
			</section>
		{/each}
	</div>
{/if}
