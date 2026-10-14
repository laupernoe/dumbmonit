<script lang="ts">
	/**
	 * The watched pages of a `webchange` device, in one list. Pages that
	 * changed come first, the most recently changed on top; each row carries
	 * its latest change (kind, lines moved, when), a camera link to its
	 * before/after screenshots when the headless browser captured some, a
	 * "+N pages" fold when the same edit was found on other pages, and an
	 * "earlier changes" fold. The pages that never changed are folded below.
	 *
	 * Every change row is a link to the change's own page. Given `onselect`
	 * (the device panel's two-column layout), a plain click selects the change
	 * in place instead and the arrow keys walk the rows; a modified click still
	 * opens the page, in a new tab if asked.
	 */
	import { untrack } from 'svelte';
	import {
		listWebchangeChanges,
		type WebchangeChange,
		type WebchangeChangeKind,
		type WebchangePage
	} from '#lib/api/index.js';
	import { formatDateTime, formatRelative, parseServerDate } from '#lib/format.js';
	import { EmptyState, ErrorNotice, Plate, Skeleton, type Tone } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Camera, ChevronDown, ExternalLink, FileDiff, FileMinus, FilePlus, History, Search } from 'lucide-svelte';
	import { groupSameEdits, pagePath, shortenPath, stepIndex } from './format';

	interface Props {
		targetId: number;
		pages: WebchangePage[];
		changes: WebchangeChange[];
		loading: boolean;
		/** The server has a headless browser: screenshots can exist. */
		screenshotsAvailable: boolean;
		/** Change shown next to the list, highlighted; `null` when nothing is. */
		selectedId?: number | null;
		/** Selects a change in place instead of following its link. */
		onselect?: (change: WebchangeChange, screenshots: boolean) => void;
	}

	let { targetId, pages, changes, loading, screenshotsAvailable, selectedId = null, onselect }: Props = $props();

	/** Below this, a filter box is more clutter than it's worth. */
	const FILTER_THRESHOLD = 8;

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
	/** Ink tint of the icon — informational, never the only cue (the word says it too). */
	const KIND_TINT: Record<WebchangeChangeKind, string> = {
		changed: 'text-info-ink',
		new_page: 'text-info-ink',
		removed_page: 'text-warning-ink'
	};

	let query = $state('');
	const needle = $derived(query.trim().toLowerCase());
	const pageByUrl = $derived(new Map(pages.map((page) => [page.url, page])));
	const matches = (url: string): boolean => {
		if (!needle) return true;
		return url.toLowerCase().includes(needle) || (pageByUrl.get(url)?.title ?? '').toLowerCase().includes(needle);
	};

	/** One row per page that changed, by its latest change, newest first. */
	const changedRows = $derived.by(() => {
		const seen = new Set<string>();
		const rows: { url: string; latest: WebchangeChange; signature: string | null }[] = [];
		for (const change of changes) {
			if (seen.has(change.url)) continue;
			seen.add(change.url);
			if (matches(change.url)) rows.push({ url: change.url, latest: change, signature: change.signature });
		}
		return rows;
	});
	const edits = $derived(groupSameEdits(changedRows));

	const quiet = $derived.by(() => {
		const changed = new Set(changes.map((change) => change.url));
		return pages.filter((page) => !changed.has(page.url) && matches(page.url));
	});

	/** Leads of the same-edit groups unfolded onto their other pages. */
	let unfolded = $state<Set<number>>(new Set());
	/** Urls whose earlier changes are shown. */
	let showHistory = $state<Set<string>>(new Set());
	function flip<T>(set: Set<T>, value: T): Set<T> {
		const next = new Set(set);
		if (next.has(value)) next.delete(value);
		else next.add(value);
		return next;
	}

	// A change picked elsewhere (the detail pane's list of other pages) unfolds
	// its group, so the highlighted row is on screen.
	$effect(() => {
		const id = selectedId;
		const owner = edits.find((edit) => edit.others.some((row) => row.latest.id === id));
		if (owner && !untrack(() => unfolded).has(owner.lead.latest.id)) unfolded = flip(unfolded, owner.lead.latest.id);
	});

	// --- Earlier changes of a page, fetched when its fold opens ---------------
	let fetched = $state<Record<string, WebchangeChange[]>>({});
	let historyLoading = $state<Set<string>>(new Set());
	let historyError = $state<Record<string, unknown>>({});
	let controller: AbortController | null = null;
	/** Latest request per url: an older answer arriving late is dropped. */
	const tickets = new Map<string, number>();
	let ticketSeq = 0;

	$effect(() => {
		void targetId;
		fetched = {};
		historyError = {};
		historyLoading = new Set();
		showHistory = new Set();
		unfolded = new Set();
		tickets.clear();
		controller?.abort();
		controller = new AbortController();
		return () => controller?.abort();
	});

	async function loadHistory(url: string) {
		const ticket = ++ticketSeq;
		tickets.set(url, ticket);
		historyLoading = new Set(historyLoading).add(url);
		historyError = { ...historyError, [url]: null };
		try {
			const result = await listWebchangeChanges(targetId, { url }, controller?.signal);
			if (tickets.get(url) === ticket) fetched = { ...fetched, [url]: result };
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			if (tickets.get(url) === ticket) historyError = { ...historyError, [url]: cause };
		} finally {
			if (tickets.get(url) === ticket) {
				const next = new Set(historyLoading);
				next.delete(url);
				historyLoading = next;
			}
		}
	}

	function toggleHistory(url: string) {
		const opening = !showHistory.has(url);
		showHistory = flip(showHistory, url);
		if (opening) void loadHistory(url);
	}

	/** Changes of a page other than its latest, newest first, deduplicated by id. */
	function earlier(row: { url: string; latest: WebchangeChange }): WebchangeChange[] {
		const byId = new Map<number, WebchangeChange>();
		for (const change of [...changes.filter((c) => c.url === row.url), ...(fetched[row.url] ?? [])]) byId.set(change.id, change);
		byId.delete(row.latest.id);
		return [...byId.values()].sort((a, b) => b.id - a.id);
	}
	/** How many earlier changes a row advertises before the fold is opened. */
	const earlierCount = (row: { url: string; latest: WebchangeChange }): number =>
		Math.max((pageByUrl.get(row.url)?.changes ?? 1) - 1, earlier(row).length);

	// --- Helpers -------------------------------------------------------------
	const hrefOf = (change: WebchangeChange) => `/targets/${targetId}/changes/${change.id}`;
	const isoOf = (value: string): string | undefined => parseServerDate(value)?.toISOString();
	/** Screenshots exist for a page when the server has a browser and took one. */
	const hasShots = (url: string): boolean => screenshotsAvailable && (pageByUrl.get(url)?.latest_snapshot ?? null) !== null;

	function statusOf(page: WebchangePage): { tone: Tone; label: string } | null {
		if (page.last_checked === null) return { tone: 'ghost', label: m.devicesb_webchange_pages_not_checked() };
		if (page.status === 404) return { tone: 'muted', label: m.devicesb_webchange_pages_removed() };
		if (page.error) return { tone: 'warning', label: m.devicesb_webchange_pages_error({ error: page.error }) };
		return null;
	}

	let list = $state<HTMLElement | null>(null);

	function click(event: MouseEvent, change: WebchangeChange, screenshots: boolean) {
		if (!onselect || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
		event.preventDefault();
		onselect(change, screenshots);
	}

	function keydown(event: KeyboardEvent) {
		if (!onselect || event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
		// Only a focused change row walks the list: a button, a link out or the
		// filter box keep their own keys.
		const target = event.target;
		if (!(target instanceof HTMLElement) || !target.matches('a[data-change]')) return;
		const rows = [...(list?.querySelectorAll<HTMLElement>('a[data-change]') ?? [])];
		const current = rows.indexOf(target);
		const next = stepIndex(current, event.key, rows.length);
		if (next === current || next < 0) return;
		event.preventDefault();
		rows[next].focus();
		const id = Number(rows[next].dataset.change);
		const change = [...changes, ...Object.values(fetched).flat()].find((c) => c.id === id);
		if (change) onselect(change, false);
	}

	const actionClass =
		'inline-flex min-h-10 min-w-10 shrink-0 items-center justify-center gap-1 rounded-lg px-2 text-[0.75rem] font-semibold text-ink-2 hover:bg-surface-2 hover:text-ink';
</script>

{#snippet counts(change: WebchangeChange)}
	<span class="tnum inline-flex items-center gap-1.5">
		{#if change.added > 0}<span class="text-signal-ink">+{change.added}</span>{/if}
		{#if change.removed > 0}<span class="text-warning-ink">−{change.removed}</span>{/if}
	</span>
{/snippet}

{#snippet row(change: WebchangeChange, page: WebchangePage | undefined, withTitle: boolean)}
	{@const Icon = KIND_ICON[change.kind]}
	{@const selected = change.id === selectedId}
	{@const status = page ? statusOf(page) : null}
	<a
		href={hrefOf(change)}
		data-change={change.id}
		onclick={(event) => click(event, change, false)}
		aria-current={selected ? 'true' : undefined}
		class={`flex min-h-12 min-w-0 flex-1 items-start gap-2.5 rounded-lg px-2 py-1.5 ${selected ? 'bg-surface-2 ring-1 ring-line-strong' : 'hover:bg-surface-2'}`}
	>
		<Icon class={`mt-0.5 size-4 shrink-0 ${KIND_TINT[change.kind]}`} aria-hidden="true" />
		<span class="min-w-0 flex-1">
			<span class="block truncate text-sm font-semibold text-ink" title={change.url}>{shortenPath(pagePath(change.url), 48)}</span>
			<span class="mt-0.5 flex min-w-0 flex-wrap items-center gap-x-2 text-[0.75rem] text-ink-2">
				<span>{kindWord(change.kind)}</span>
				{@render counts(change)}
				<time class="tnum" datetime={isoOf(change.detected_at)} title={formatDateTime(change.detected_at)}>{formatRelative(change.detected_at)}</time>
				{#if withTitle && page?.title}<span class="max-w-full min-w-0 truncate text-ink-3">{page.title}</span>{/if}
				{#if status}<Plate tone={status.tone} label={status.label} />{/if}
			</span>
		</span>
	</a>
{/snippet}

{#snippet quietRow(page: WebchangePage)}
	{@const status = statusOf(page)}
	<li class="flex min-h-10 items-center gap-2 px-2 py-1">
		<span class="min-w-0 flex-1">
			<span class="block truncate text-sm text-ink" title={page.url}>{shortenPath(pagePath(page.url), 48)}</span>
			{#if page.title}<span class="block truncate text-[0.75rem] text-ink-3">{page.title}</span>{/if}
		</span>
		{#if status}<Plate tone={status.tone} label={status.label} />{/if}
		<a
			href={page.url}
			target="_blank"
			rel="noopener noreferrer"
			class={actionClass}
			title={m.devicesb_webchange_pages_open({ url: page.url })}
		>
			<ExternalLink class="size-3.5" aria-hidden="true" />
			<span class="sr-only">{m.devicesb_webchange_pages_open_tab()}</span>
		</a>
	</li>
{/snippet}

{#snippet changeRow(item: { url: string; latest: WebchangeChange }, grouped: number)}
	{@const page = pageByUrl.get(item.url)}
	{@const open = showHistory.has(item.url)}
	{@const count = earlierCount(item)}
	<div class="flex items-center gap-0.5">
		{@render row(item.latest, page, true)}
		{#if grouped > 0}
			<button
				type="button"
				class={`tnum ${actionClass}`}
				aria-expanded={unfolded.has(item.latest.id)}
				title={m.devicesb_webchange_timeline_same_title({ count: grouped + 1 })}
				onclick={() => (unfolded = flip(unfolded, item.latest.id))}
			>
				{(grouped === 1 ? m.devicesb_webchange_timeline_more_one : m.devicesb_webchange_timeline_more_other)({ count: grouped })}
				<ChevronDown class={`size-3.5 transition-transform ${unfolded.has(item.latest.id) ? 'rotate-180' : ''}`} aria-hidden="true" />
			</button>
		{/if}
		{#if hasShots(item.url)}
			<a
				href={`${hrefOf(item.latest)}#screenshots`}
				class={actionClass}
				title={m.devicesb_webchange_list_screenshots({ path: pagePath(item.url) })}
				onclick={(event) => click(event, item.latest, true)}
			>
				<Camera class="size-4" aria-hidden="true" />
				<span class="sr-only">{m.devicesb_webchange_list_screenshots({ path: pagePath(item.url) })}</span>
			</a>
		{/if}
		{#if count > 0}
			<button
				type="button"
				class={`tnum ${actionClass}`}
				aria-expanded={open}
				title={m.devicesb_webchange_list_earlier({ count })}
				onclick={() => toggleHistory(item.url)}
			>
				<History class="size-4" aria-hidden="true" />
				{count}
				<span class="sr-only">{m.devicesb_webchange_list_earlier({ count })}</span>
			</button>
		{/if}
	</div>
	{#if open}
		<div class="ml-4 border-l border-line pl-2">
			{#if historyError[item.url]}
				<ErrorNotice error={historyError[item.url]} title={m.devicesb_webchange_panel_changes_error()} onretry={() => void loadHistory(item.url)} class="my-1" />
			{:else if historyLoading.has(item.url) && earlier(item).length < count}
				<Skeleton class="my-1 h-10 w-full" />
			{/if}
			<ol>
				{#each earlier(item) as change (change.id)}
					<li class="flex items-center">{@render row(change, undefined, false)}</li>
				{/each}
			</ol>
		</div>
	{/if}
{/snippet}

{#if loading}
	<div class="space-y-2 p-3" aria-busy="true" aria-label={m.devicesb_webchange_timeline_loading()}>
		<Skeleton class="h-12 w-full" />
		<Skeleton class="h-12 w-full" />
		<Skeleton class="h-12 w-3/4" />
	</div>
{:else if pages.length === 0 && changes.length === 0}
	<EmptyState icon={FileDiff} title={m.devicesb_webchange_pages_empty()} description={m.devicesb_webchange_pages_empty_hint()} />
{:else}
	<!-- svelte-ignore a11y_no_static_element_interactions: the arrow keys are a shortcut over links that work on their own -->
	<div bind:this={list} onkeydown={keydown} class="p-2">
		{#if pages.length > FILTER_THRESHOLD}
			<label class="relative mb-2 block w-full sm:w-72">
				<span class="sr-only">{m.devicesb_webchange_pages_filter()}</span>
				<Search class="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-ink-3" aria-hidden="true" />
				<input class="input h-10 w-full text-sm" style="padding-left: 2rem" type="search" placeholder={m.devicesb_webchange_pages_filter()} bind:value={query} />
			</label>
		{/if}

		{#if changes.length === 0}
			<p class="px-2 py-2 text-[0.8125rem] text-ink-2">
				{pages.length === 1
					? m.devicesb_webchange_timeline_watching_one()
					: m.devicesb_webchange_timeline_watching_other({ count: pages.length })}
				{m.devicesb_webchange_timeline_baseline_hint()}
			</p>
		{/if}

		{#if changedRows.length > 0}
			<ol>
				{#each edits as edit (edit.lead.latest.id)}
					<li>
						{@render changeRow(edit.lead, edit.others.length)}
						{#if unfolded.has(edit.lead.latest.id)}
							<ol class="ml-4 border-l border-line pl-2">
								{#each edit.others as other (other.latest.id)}
									<li>{@render changeRow(other, 0)}</li>
								{/each}
							</ol>
						{/if}
					</li>
				{/each}
			</ol>
		{/if}

		{#if needle && changedRows.length === 0 && quiet.length === 0}
			<p class="px-2 py-3 text-sm text-ink-2">{m.devicesb_webchange_pages_no_match({ query })}</p>
		{/if}

		<!-- A lone quiet page is the device's own address, already in the header -->
		{#if quiet.length > 0 && !(pages.length === 1 && changes.length === 0)}
			<details class={`group ${changedRows.length > 0 ? 'mt-1 border-t border-line pt-1' : ''}`} open={changedRows.length === 0 || needle !== ''}>
				<summary class="flex min-h-10 cursor-pointer items-center gap-1.5 rounded-lg px-2 text-[0.8125rem] font-semibold text-ink-2 select-none hover:bg-surface-2">
					<ChevronDown class="size-3.5 -rotate-90 transition-transform group-open:rotate-0" aria-hidden="true" />
					{m.devicesb_webchange_list_quiet({ count: quiet.length })}
				</summary>
				<ul class="divide-y divide-line/60">
					{#each quiet as page (page.url)}
						{@render quietRow(page)}
					{/each}
				</ul>
			</details>
		{/if}
	</div>
{/if}
