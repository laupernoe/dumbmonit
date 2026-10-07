<script lang="ts">
	/**
	 * One detected website change: the before/after screenshot comparison (when
	 * the server took any) above its visible-text diff. Deep-linkable on its
	 * own — someone can land here straight from a notification — so everything
	 * it needs comes from a single `GET .../webchange/changes/{id}`; nothing is
	 * assumed from the device panel that usually links here.
	 */
	import { page } from '$app/state';
	import {
		getWebchangeChange,
		webchangeScreenshotUrl,
		type WebchangeChangeDetail,
		type WebchangeChangeKind
	} from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { Button, ErrorNotice, PageHeader, Panel, Skeleton } from '#lib/ui/index.js';
	import { FileDiff, FileMinus, FilePlus } from 'lucide-svelte';
	import CompareSlider from '#lib/components/devices/webchange/CompareSlider.svelte';
	import DiffView from '#lib/components/devices/webchange/DiffView.svelte';
	import { pagePath } from '#lib/components/devices/webchange/format.js';

	const targetId = $derived(Number(page.params.id));
	const changeId = $derived(Number(page.params.changeId));

	let detail = $state<WebchangeChangeDetail | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let sideBySide = $state(false);

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

	const hasBefore = $derived(detail?.before_snapshot?.has_screenshot ?? false);
	const hasAfter = $derived(detail?.after_snapshot?.has_screenshot ?? false);
	const bothShots = $derived(hasBefore && hasAfter);
	const anyShot = $derived(hasBefore || hasAfter);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			detail = await getWebchangeChange(targetId, changeId, signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void targetId;
		void changeId;
		loading = true;
		detail = null;
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});
</script>

<svelte:head>
	<title>{detail ? `${KIND_WORD[detail.kind]} · ${pagePath(detail.url)}` : 'Website change'} — DumbMonit</title>
</svelte:head>

{#if error}
	<ErrorNotice {error} title="Could not load this change" onretry={() => void load()} />
{:else if loading || !detail}
	<div class="flex flex-col gap-4" aria-busy="true" aria-label="Loading the change">
		<Skeleton class="h-8 w-64" />
		<Skeleton class="h-72 w-full rounded-[var(--radius-card)]" />
		<Skeleton class="h-40 w-full rounded-[var(--radius-card)]" />
	</div>
{:else}
	{@const Icon = KIND_ICON[detail.kind]}
	{@const changeUrl = detail.url}
	<PageHeader
		title={pagePath(detail.url)}
		description={`${KIND_WORD[detail.kind]} · detected ${formatRelative(detail.detected_at)} · +${detail.added} / −${detail.removed}`}
		back={{ href: `/targets/${targetId}`, label: 'Back to device' }}
	>
		{#snippet actions()}
			<span class="inline-flex items-center gap-1.5 text-sm text-ink-2">
				<Icon class="size-4" aria-hidden="true" />
				{changeUrl}
			</span>
		{/snippet}
	</PageHeader>

	<Panel title="Screenshots" class="rise-in">
		{#snippet aside()}
			{#if bothShots}
				<Button variant="ghost" size="sm" onclick={() => (sideBySide = !sideBySide)} aria-pressed={sideBySide}>
					{sideBySide ? 'Slider' : 'Side by side'}
				</Button>
			{/if}
		{/snippet}

		{#if !anyShot}
			<p class="text-sm text-ink-2">
				No headless browser is configured on this server, so no screenshot was taken for this change — the text diff
				below still shows exactly what moved. Screenshots are an optional extra: see the
				<a
					href="https://dumbmonit.readthedocs.io/en/latest/devices/webchange/"
					target="_blank"
					rel="noopener noreferrer"
					class="text-ink underline"
				>
					setup guide
				</a>
				to turn them on.
			</p>
		{:else if bothShots && !sideBySide}
			<CompareSlider
				beforeSrc={webchangeScreenshotUrl(targetId, detail.before_snapshot!.id)}
				afterSrc={webchangeScreenshotUrl(targetId, detail.after_snapshot!.id)}
				beforeLabel={`Before, ${formatDateTime(detail.before_snapshot!.fetched_at)}`}
				afterLabel={`After, ${formatDateTime(detail.after_snapshot!.fetched_at)}`}
			/>
		{:else if bothShots && sideBySide}
			<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
				<figure class="flex flex-col gap-1.5">
					<figcaption class="text-[0.8125rem] font-semibold text-ink-2">Before · {formatDateTime(detail.before_snapshot!.fetched_at)}</figcaption>
					<img
						src={webchangeScreenshotUrl(targetId, detail.before_snapshot!.id)}
						alt={`Before, ${formatDateTime(detail.before_snapshot!.fetched_at)}`}
						class="w-full rounded-[var(--radius-card)] border border-line bg-canvas-deep object-contain"
					/>
				</figure>
				<figure class="flex flex-col gap-1.5">
					<figcaption class="text-[0.8125rem] font-semibold text-ink-2">After · {formatDateTime(detail.after_snapshot!.fetched_at)}</figcaption>
					<img
						src={webchangeScreenshotUrl(targetId, detail.after_snapshot!.id)}
						alt={`After, ${formatDateTime(detail.after_snapshot!.fetched_at)}`}
						class="w-full rounded-[var(--radius-card)] border border-line bg-canvas-deep object-contain"
					/>
				</figure>
			</div>
		{:else}
			{@const only = hasBefore ? detail.before_snapshot! : detail.after_snapshot!}
			<figure class="flex flex-col gap-1.5">
				<figcaption class="text-[0.8125rem] font-semibold text-ink-2">
					{hasBefore ? 'Last capture before removal' : 'First capture'} · {formatDateTime(only.fetched_at)}
				</figcaption>
				<img
					src={webchangeScreenshotUrl(targetId, only.id)}
					alt={hasBefore ? 'Last capture before the page was removed' : 'First capture of the new page'}
					class="w-full max-w-2xl rounded-[var(--radius-card)] border border-line bg-canvas-deep object-contain"
				/>
			</figure>
		{/if}
	</Panel>

	<Panel title="Text changes" description="Visible text only; markup and scripts are ignored." padded={false} class="mt-6 rise-in">
		<div class="p-3">
			<DiffView diff={detail.diff} />
		</div>
	</Panel>
{/if}
