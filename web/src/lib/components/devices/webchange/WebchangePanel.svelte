<script lang="ts">
	/**
	 * What a `webchange` device has to show beyond charts. The timeline of
	 * detected changes comes first, newest on top: from `lg`, the selected
	 * change (the newest by default) is read in a pane beside it; below, each
	 * row opens the change's own page (`/targets/{id}/changes/{id}`). The
	 * watched pages, which can run to hundreds on a crawled site, sit folded
	 * at the bottom; picking one narrows the timeline to it.
	 *
	 * Checks run on the device's own interval; "Check now" only asks for an
	 * immediate one — the server answers before it is done (202), so the page
	 * refreshes itself once, a few seconds later, on a best effort basis.
	 */
	import { untrack } from 'svelte';
	import {
		checkWebchangeNow,
		getWebchangePages,
		listWebchangeChanges,
		type Target,
		type WebchangeChange,
		type WebchangePage
	} from '#lib/api/index.js';
	import { formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ErrorNotice, Panel, Plate } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { ChevronRight, Eye, RefreshCw, TriangleAlert } from 'lucide-svelte';
	import PagesTable from './PagesTable.svelte';
	import ChangeTimeline from './ChangeTimeline.svelte';
	import ChangePane from './ChangePane.svelte';
	import { groupSameEdits } from './format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let pages = $state<WebchangePage[]>([]);
	let screenshotsAvailable = $state(true);
	/** Pages found when Max pages stopped the last crawl; `null` when the whole site was read. */
	let found = $state<number | null>(null);
	let pagesError = $state<unknown>(null);

	let changes = $state<WebchangeChange[]>([]);
	let loadingChanges = $state(true);
	let changesError = $state<unknown>(null);

	/** Page the change timeline is narrowed to; `null` shows every page. */
	let selectedUrl = $state<string | null>(null);

	/** Two columns from `lg`: the timeline selects in place instead of navigating. */
	let wide = $state(false);
	/** Change picked in the timeline; the newest one shown when unset or gone from the list. */
	let pickedId = $state<number | null>(null);
	const selectedId = $derived(
		changes.some((change) => change.id === pickedId) ? pickedId : (changes[0]?.id ?? null)
	);
	/** The other pages the selected change's edit was also made on. */
	const sameEdit = $derived.by(() => {
		const group = groupSameEdits(changes).find((edit) =>
			[edit.lead, ...edit.others].some((change) => change.id === selectedId)
		);
		return group ? [group.lead, ...group.others].filter((change) => change.id !== selectedId) : [];
	});

	let checking = $state(false);
	let checkError = $state<unknown>(null);
	let justChecked = $state(false);
	let refreshTimer: ReturnType<typeof setTimeout> | null = null;

	const lastChanged = $derived.by(() => {
		const dates = pages.map((p) => p.last_changed).filter((d): d is string => d !== null);
		return dates.length === 0 ? null : dates.sort().at(-1)!;
	});

	async function loadPages(signal?: AbortSignal) {
		pagesError = null;
		try {
			const response = await getWebchangePages(target.id, signal);
			pages = response.pages;
			screenshotsAvailable = response.screenshots_available;
			found = response.found;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			pagesError = cause;
		}
	}

	async function loadChanges(signal?: AbortSignal) {
		changesError = null;
		try {
			changes = await listWebchangeChanges(target.id, { url: selectedUrl ?? undefined }, signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			changesError = cause;
		} finally {
			loadingChanges = false;
		}
	}

	function selectUrl(url: string | null) {
		selectedUrl = url;
		pickedId = null;
		loadingChanges = true;
		void loadChanges();
		if (url) document.getElementById('webchange-timeline')?.scrollIntoView({ block: 'start', behavior: 'smooth' });
	}

	async function checkNow() {
		checking = true;
		checkError = null;
		try {
			await checkWebchangeNow(target.id);
			justChecked = true;
			if (refreshTimer) clearTimeout(refreshTimer);
			refreshTimer = setTimeout(() => {
				justChecked = false;
				void loadPages();
				void loadChanges();
			}, 6000);
		} catch (cause) {
			checkError = cause;
		} finally {
			checking = false;
		}
	}

	$effect(() => {
		void target.id;
		loadingChanges = true;
		pages = [];
		changes = [];
		selectedUrl = null;
		pickedId = null;
		const controller = new AbortController();
		void loadPages(controller.signal);
		void loadChanges(controller.signal);
		const timer = setInterval(() => void untrack(() => loadPages(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
			if (refreshTimer) clearTimeout(refreshTimer);
		};
	});

	$effect(() => {
		const query = window.matchMedia('(min-width: 64rem)');
		wide = query.matches;
		const onchange = () => (wide = query.matches);
		query.addEventListener('change', onchange);
		return () => query.removeEventListener('change', onchange);
	});
</script>

<div class="flex flex-col gap-6">
	<Panel
		title={m.devicesb_webchange_panel_title()}
		description={(pages.length === 1
			? m.devicesb_webchange_panel_summary_one
			: m.devicesb_webchange_panel_summary_other)({
			count: pages.length,
			when: lastChanged ? formatRelative(lastChanged) : m.devicesb_webchange_panel_never()
		})}
		class="rise-in"
	>
		{#snippet aside()}
			{#if changes.length > 0 && selectedUrl === null && !wide}
				<Button variant="primary" href={`/targets/${target.id}/changes/${changes[0].id}`}>
					<Eye class="size-4" aria-hidden="true" />
					{m.devicesb_webchange_panel_latest()}
				</Button>
			{/if}
			{#if auth.isAdmin}
				<Button variant="secondary" onclick={checkNow} loading={checking}>
					<RefreshCw class="size-4" aria-hidden="true" />
					{m.devicesb_webchange_panel_check_now()}
				</Button>
			{/if}
		{/snippet}
		{#if checkError}
			<ErrorNotice error={checkError} title={m.devicesb_webchange_panel_check_error()} onretry={checkNow} class="mb-3" />
		{:else if justChecked}
			<Plate tone="signal" label={m.devicesb_webchange_panel_queued()} size="md" />
		{/if}
		{#if found !== null}
			<p class="mb-2 flex items-start gap-1.5 text-[0.8125rem] text-warning-ink">
				<TriangleAlert class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
				<span>{m.devicesb_webchange_panel_truncated({ read: pages.length, found })}</span>
			</p>
		{/if}
		{#if !screenshotsAvailable}
			<p class="text-[0.8125rem] text-ink-2">
				{m.devicesb_webchange_panel_no_browser()}
				<a
					href="https://dumbmonit.readthedocs.io/en/latest/devices/webchange/"
					target="_blank"
					rel="noopener noreferrer"
					class="text-ink underline"
				>
					{m.devicesb_webchange_panel_setup_guide()}
				</a>
			</p>
		{/if}
	</Panel>

	<Panel
		id="webchange-timeline"
		title={m.devicesb_webchange_panel_timeline()}
		description={selectedUrl
			? m.devicesb_webchange_panel_filtered({ url: selectedUrl })
			: m.devicesb_webchange_panel_every_page()}
		padded={false}
		class="rise-in scroll-mt-20"
	>
		{#snippet aside()}
			{#if selectedUrl}
				<Button variant="ghost" size="sm" class="min-h-10 sm:min-h-0" onclick={() => selectUrl(null)}>{m.devicesb_webchange_panel_clear_filter()}</Button>
			{/if}
		{/snippet}
		{#if changesError}
			<ErrorNotice error={changesError} title={m.devicesb_webchange_panel_changes_error()} onretry={() => void loadChanges()} class="m-4" />
		{:else}
			{@const split = wide && !loadingChanges && selectedId !== null}
			<div class={split ? 'grid grid-cols-[minmax(17rem,2fr)_minmax(0,3fr)]' : ''}>
				<div class={split ? 'sticky top-16 max-h-[calc(100vh-5rem)] self-start overflow-y-auto border-r border-line px-2 pb-2' : 'px-2 pb-2'}>
					<ChangeTimeline
						targetId={target.id}
						{changes}
						loading={loadingChanges}
						pageCount={pages.length}
						filteredUrl={selectedUrl}
						selectedId={split ? selectedId : null}
						onselect={split ? (change) => (pickedId = change.id) : undefined}
					/>
				</div>
				{#if split && selectedId !== null}
					<div class="min-w-0 px-5 py-4">
						<ChangePane targetId={target.id} changeId={selectedId} {sameEdit} onselect={(change) => (pickedId = change.id)} />
					</div>
				{/if}
			</div>
		{/if}
	</Panel>

	<!-- A single page is the device's own address, already in the header -->
	{#if pagesError || pages.length > 1}
		<details class="group rounded-[var(--radius-card)] border border-line bg-surface shadow-lift" open={pagesError !== null}>
			<summary class="flex min-h-11 cursor-pointer items-center gap-2 px-5 py-3 select-none">
				<ChevronRight class="size-4 text-ink-3 transition-transform group-open:rotate-90" aria-hidden="true" />
				<span class="text-base font-semibold tracking-tight text-ink">{m.devicesb_webchange_panel_pages_count({ count: pages.length })}</span>
				{#if selectedUrl}<span class="truncate text-sm text-ink-2">· {m.devicesb_webchange_panel_filtered({ url: selectedUrl })}</span>{/if}
			</summary>
			<div class="border-t border-line px-5 py-4">
				{#if pagesError}
					<ErrorNotice error={pagesError} title={m.devicesb_webchange_panel_pages_error()} onretry={() => void loadPages()} />
				{:else}
					<PagesTable {pages} {selectedUrl} onselect={selectUrl} />
				{/if}
			</div>
		</details>
	{/if}
</div>
