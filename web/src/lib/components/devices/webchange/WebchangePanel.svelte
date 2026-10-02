<script lang="ts">
	/**
	 * What a `webchange` device has to show beyond charts: every watched page
	 * with its last outcome, and the timeline of detected changes, each one
	 * opening onto its before/after comparison (`/targets/{id}/changes/{id}`).
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
	} from '$lib/api';
	import { formatRelative } from '$lib/format';
	import { auth } from '$lib/stores/auth.svelte';
	import { Button, ErrorNotice, Panel, Plate, Skeleton } from '$lib/ui';
	import { RefreshCw } from 'lucide-svelte';
	import PagesTable from './PagesTable.svelte';
	import ChangeTimeline from './ChangeTimeline.svelte';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let pages = $state<WebchangePage[]>([]);
	let screenshotsAvailable = $state(true);
	let loadingPages = $state(true);
	let pagesError = $state<unknown>(null);

	let changes = $state<WebchangeChange[]>([]);
	let loadingChanges = $state(true);
	let changesError = $state<unknown>(null);

	/** Page the change timeline is narrowed to; `null` shows every page. */
	let selectedUrl = $state<string | null>(null);

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
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			pagesError = cause;
		} finally {
			loadingPages = false;
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
		loadingChanges = true;
		void loadChanges();
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
		loadingPages = true;
		loadingChanges = true;
		pages = [];
		changes = [];
		selectedUrl = null;
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
</script>

<div class="flex flex-col gap-6">
	<Panel
		title="Website changes"
		description={`${pages.length} ${pages.length === 1 ? 'page' : 'pages'} watched · last change ${lastChanged ? formatRelative(lastChanged) : 'never'}`}
		class="rise-in"
	>
		{#snippet aside()}
			{#if auth.isAdmin}
				<Button variant="secondary" onclick={checkNow} loading={checking}>
					<RefreshCw class="size-4" aria-hidden="true" />
					Check now
				</Button>
			{/if}
		{/snippet}
		{#if checkError}
			<ErrorNotice error={checkError} title="Could not start the check" onretry={checkNow} class="mb-3" />
		{:else if justChecked}
			<Plate tone="signal" label="Check queued · refreshing shortly" size="md" />
		{/if}
		{#if !screenshotsAvailable}
			<p class="text-[0.8125rem] text-ink-2">
				No headless browser is configured: screenshots are skipped, text changes are still detected. See the
				<a
					href="https://dumbmonit.readthedocs.io/en/latest/devices/webchange/"
					target="_blank"
					rel="noopener noreferrer"
					class="text-ink underline"
				>
					setup guide
				</a>.
			</p>
		{/if}
	</Panel>

	<Panel title="Pages" padded={false} class="rise-in">
		<div class="px-5 py-4">
			{#if pagesError}
				<ErrorNotice error={pagesError} title="Could not load the watched pages" onretry={() => void loadPages()} />
			{:else if loadingPages}
				<div class="flex flex-col gap-3" aria-busy="true" aria-label="Loading pages">
					<Skeleton class="h-10 w-full" rows={3} />
				</div>
			{:else}
				<PagesTable {pages} {selectedUrl} onselect={selectUrl} />
			{/if}
		</div>
	</Panel>

	<Panel
		title="Change timeline"
		description={selectedUrl ? `Filtered to ${selectedUrl}` : 'Every page, newest first.'}
		padded={false}
		class="rise-in"
	>
		{#snippet aside()}
			{#if selectedUrl}
				<Button variant="ghost" size="sm" onclick={() => selectUrl(null)}>Clear filter</Button>
			{/if}
		{/snippet}
		<div class="px-3 py-2">
			{#if changesError}
				<ErrorNotice error={changesError} title="Could not load the changes" onretry={() => void loadChanges()} class="m-2" />
			{:else}
				<ChangeTimeline targetId={target.id} {changes} loading={loadingChanges} pageCount={pages.length} filteredUrl={selectedUrl} />
			{/if}
		</div>
	</Panel>
</div>
