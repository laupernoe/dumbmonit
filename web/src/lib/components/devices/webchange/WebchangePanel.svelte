<script lang="ts">
	/**
	 * What a `webchange` device has to show beyond charts: its watched pages
	 * and their changes, in one list (`WatchedPages`). From `lg`, the selected
	 * change (the newest by default) is read in a pane beside the list; below,
	 * each row opens the change's own page (`/targets/{id}/changes/{id}`).
	 *
	 * Checks run on the device's own interval; "Check now" only asks for an
	 * immediate one — the server answers before it is done (202), so the page
	 * refreshes itself once, a few seconds later, on a best effort basis. A
	 * refresh swaps the data in place: nothing the reader opened is reset.
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
	import { RefreshCw, TriangleAlert } from 'lucide-svelte';
	import WatchedPages from './WatchedPages.svelte';
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
	let loading = $state(true);
	let changesError = $state<unknown>(null);

	/** Two columns from `lg`: the list selects in place instead of navigating. */
	let wide = $state(false);
	/** Change picked in the list; the newest one shown when unset. */
	let pickedId = $state<number | null>(null);
	/** The picked change is read from its screenshots (camera button) rather than its text. */
	let screenshotsFirst = $state(false);
	const selectedId = $derived(pickedId ?? changes[0]?.id ?? null);
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

	/** Bumped per load: an answer from an older one (slower, or from the previous device) is dropped. */
	let generation = 0;
	let controller: AbortController | null = null;

	async function refresh() {
		const mine = ++generation;
		controller?.abort();
		const signal = (controller = new AbortController()).signal;
		const [pagesResult, changesResult] = await Promise.allSettled([
			getWebchangePages(target.id, signal),
			listWebchangeChanges(target.id, {}, signal)
		]);
		if (mine !== generation) return;
		if (pagesResult.status === 'fulfilled') {
			pages = pagesResult.value.pages;
			screenshotsAvailable = pagesResult.value.screenshots_available;
			found = pagesResult.value.found;
			pagesError = null;
		} else pagesError = pagesResult.reason;
		if (changesResult.status === 'fulfilled') {
			changes = changesResult.value;
			changesError = null;
		} else changesError = changesResult.reason;
		loading = false;
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
				void refresh();
			}, 6000);
		} catch (cause) {
			checkError = cause;
		} finally {
			checking = false;
		}
	}

	function select(change: WebchangeChange, screenshots: boolean) {
		pickedId = change.id;
		screenshotsFirst = screenshots;
	}

	$effect(() => {
		void target.id;
		untrack(() => {
			loading = true;
			pages = [];
			changes = [];
			found = null;
			pagesError = null;
			changesError = null;
			pickedId = null;
			screenshotsFirst = false;
			justChecked = false;
		});
		void refresh();
		const timer = setInterval(() => void refresh(), 60_000);
		return () => {
			generation++;
			controller?.abort();
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

<Panel
	title={m.devicesb_webchange_panel_title()}
	description={(pages.length === 1
		? m.devicesb_webchange_panel_summary_one
		: m.devicesb_webchange_panel_summary_other)({
		count: pages.length,
		when: lastChanged ? formatRelative(lastChanged) : m.devicesb_webchange_panel_never()
	})}
	padded={false}
	class="rise-in"
>
	{#snippet aside()}
		{#if auth.isAdmin}
			<Button variant="secondary" onclick={checkNow} loading={checking}>
				<RefreshCw class="size-4" aria-hidden="true" />
				{m.devicesb_webchange_panel_check_now()}
			</Button>
		{/if}
	{/snippet}

	{#if checkError || justChecked || found !== null || !screenshotsAvailable}
		<div class="flex flex-col gap-1.5 border-b border-line px-4 py-2.5">
			{#if checkError}
				<ErrorNotice error={checkError} title={m.devicesb_webchange_panel_check_error()} onretry={checkNow} />
			{:else if justChecked}
				<Plate tone="signal" label={m.devicesb_webchange_panel_queued()} size="md" />
			{/if}
			{#if found !== null}
				<p class="flex items-start gap-1.5 text-[0.8125rem] text-warning-ink">
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
		</div>
	{/if}

	{#if changesError && changes.length === 0}
		<ErrorNotice error={changesError} title={m.devicesb_webchange_panel_changes_error()} onretry={() => void refresh()} class="m-4" />
	{:else}
		{#if pagesError}
			<ErrorNotice error={pagesError} title={m.devicesb_webchange_panel_pages_error()} onretry={() => void refresh()} class="m-3" />
		{/if}
		{@const split = wide && !loading && selectedId !== null}
		<div class={split ? 'grid grid-cols-[minmax(18rem,2fr)_minmax(0,3fr)]' : ''}>
			<div class={split ? 'max-h-[calc(100vh-5rem)] self-start overflow-y-auto border-r border-line lg:sticky lg:top-16' : ''}>
				<WatchedPages
					targetId={target.id}
					{pages}
					{changes}
					{loading}
					{screenshotsAvailable}
					selectedId={split ? selectedId : null}
					onselect={split ? select : undefined}
				/>
			</div>
			{#if split && selectedId !== null}
				<div class="min-w-0 px-5 py-4">
					<ChangePane
						targetId={target.id}
						changeId={selectedId}
						{sameEdit}
						{screenshotsFirst}
						onselect={(change) => select(change, false)}
					/>
				</div>
			{/if}
		</div>
	{/if}
</Panel>
