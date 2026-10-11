<script lang="ts">
	/**
	 * Website health & SEO: the last reading (status, final URL, redirect
	 * chain, platform), the tracked SEO metadata with "changed" badges,
	 * robots.txt / sitemap state and the recent confirmed changes. Read from
	 * what the probe stored; opening the page never fetches the site.
	 */
	import { untrack } from 'svelte';
	import { getWebsiteOverview, listWebsiteChanges } from '#lib/api/website.js';
	import type { Target, WebsiteChange, WebsiteOverview } from '#lib/api/index.js';
	import { formatRelative } from '#lib/format.js';
	import { ErrorNotice, Panel, Plate, Skeleton, type Tone } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		target: Target;
	}
	let { target }: Props = $props();

	let overview = $state<WebsiteOverview | null>(null);
	let changes = $state<WebsiteChange[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	$effect(() => {
		const id = target.id;
		const controller = new AbortController();
		untrack(() => {
			loading = true;
			error = null;
		});
		Promise.all([getWebsiteOverview(id, controller.signal), listWebsiteChanges(id, 20, controller.signal)])
			.then(([o, c]) => {
				overview = o;
				changes = c;
			})
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				error = cause;
			})
			.finally(() => {
				if (!controller.signal.aborted) loading = false;
			});
		return () => controller.abort();
	});

	const probe = $derived(overview?.probe ?? null);

	const FIELD_LABEL: Record<string, () => string> = {
		final_url: () => m.website_field_final_url(),
		title: () => m.website_field_title(),
		description: () => m.website_field_description(),
		canonical: () => m.website_field_canonical(),
		robots: () => m.website_field_robots(),
		og_title: () => m.website_field_og_title(),
		og_description: () => m.website_field_og_description(),
		lang: () => m.website_field_lang(),
		h1_count: () => m.website_field_h1_count(),
		hreflang_count: () => m.website_field_hreflang_count()
	};
	const fieldLabel = (f: string) => FIELD_LABEL[f]?.() ?? f;

	function statusTone(status: number): Tone {
		if (status >= 200 && status < 300) return 'signal';
		if (status >= 300 && status < 400) return 'info';
		return 'warning';
	}

	/** An http -> https hop on the same host is the normal way in. */
	function isUpgrade(from: string, to: string): boolean {
		try {
			const a = new URL(from);
			const b = new URL(to);
			return a.protocol === 'http:' && b.protocol === 'https:' && a.host === b.host;
		} catch {
			return false;
		}
	}

	/** Each hop: the URL that answered with a redirect and where it led. */
	const chain = $derived.by(() => {
		if (!probe) return [];
		const targets = [...probe.redirects.slice(1).map((h) => h.url), probe.final_url];
		return probe.redirects.map((hop, i) => ({
			key: i,
			from: hop.url,
			to: targets[i],
			status: hop.status,
			upgrade: isUpgrade(hop.url, targets[i])
		}));
	});

	function robotsState(o: WebsiteOverview): { tone: Tone; label: string } {
		if (o.robots_blocks_all === null) return { tone: 'ghost', label: m.website_unknown() };
		return o.robots_blocks_all
			? { tone: 'warning', label: m.website_robots_blocks_all() }
			: { tone: 'signal', label: m.website_robots_open() };
	}
	function sitemapState(o: WebsiteOverview): { tone: Tone; label: string } {
		if (o.sitemap_ok === null) return { tone: 'ghost', label: m.website_unknown() };
		return o.sitemap_ok
			? { tone: 'signal', label: m.website_sitemap_ok() }
			: { tone: 'advisory', label: m.website_sitemap_missing() };
	}
</script>

<Panel title={m.website_panel_title()} description={m.website_panel_desc()} class="mb-4">
	{#if loading}
		<Skeleton class="h-24 w-full" />
	{:else if error}
		<ErrorNotice {error} title={m.website_panel_error()} />
	{:else if !overview || (!probe && !overview.error)}
		<p class="text-sm text-ink-2">{m.website_panel_empty()}</p>
	{:else}
		{#if overview.error}
			<p class="mb-4 text-sm text-warning-ink">{overview.error}</p>
		{/if}

		{#if probe}
			<dl class="grid grid-cols-1 gap-x-6 gap-y-3 text-sm sm:grid-cols-2">
				<div class="min-w-0">
					<dt class="text-ink-3">{m.website_status()}</dt>
					<dd class="mt-0.5 flex flex-wrap items-center gap-2">
						<Plate tone={statusTone(probe.status)} label={`HTTP ${probe.status}`} />
						<span class="tnum text-ink-2">{m.website_response_ms({ ms: probe.response_ms })}</span>
					</dd>
				</div>
				<div class="min-w-0">
					<dt class="text-ink-3">{m.website_platform()}</dt>
					<dd class="mt-0.5 font-medium">{overview.platform_label ?? probe.platform}</dd>
				</div>
				<div class="min-w-0 sm:col-span-2">
					<dt class="text-ink-3">{m.website_final_url()}</dt>
					<dd class="mt-0.5 min-w-0 font-medium break-all">{probe.final_url}</dd>
				</div>
				{#if overview.checked_at}
					<div class="min-w-0">
						<dt class="text-ink-3">{m.website_checked()}</dt>
						<dd class="mt-0.5">{formatRelative(overview.checked_at)}</dd>
					</div>
				{/if}
				{#if probe.blank !== null}
					<div class="min-w-0">
						<dt class="text-ink-3">{m.website_content()}</dt>
						<dd class="mt-0.5">
							{probe.blank
								? m.website_content_blank({ chars: probe.text_chars ?? 0 })
								: m.website_content_ok({ chars: probe.text_chars ?? 0 })}
						</dd>
					</div>
				{/if}
			</dl>

			<h3 class="mt-5 mb-2 text-sm font-semibold">{m.website_redirects()}</h3>
			{#if chain.length === 0}
				<p class="text-sm text-ink-2">{m.website_redirects_none()}</p>
			{:else}
				<ol class="space-y-2 text-sm">
					{#each chain as hop (hop.key)}
						<li class="min-w-0 rounded-md border border-line px-3 py-2">
							<div class="flex flex-wrap items-center gap-2">
								<Plate tone="info" label={String(hop.status)} size="sm" />
								{#if hop.upgrade}<Plate tone="signal" label={m.website_redirect_https()} size="sm" />{/if}
							</div>
							<p class="mt-1 text-ink-2 break-all">{hop.from}</p>
							<p class="break-all">→ {hop.to}</p>
						</li>
					{/each}
				</ol>
			{/if}
		{/if}

		<h3 class="mt-5 mb-2 text-sm font-semibold">{m.website_seo()}</h3>
		{#if overview.fields.length === 0}
			<p class="text-sm text-ink-2">{m.website_seo_none()}</p>
		{:else}
			<div class="rounded-lg border border-line">
				<table class="w-full table-fixed text-left text-[0.8125rem]">
					<tbody>
						{#each overview.fields as f (f.field)}
							<tr class="border-t border-line align-top first:border-t-0">
								<th scope="row" class="w-2/5 px-3 py-2 font-medium break-words text-ink-2 sm:w-1/3">{fieldLabel(f.field)}</th>
								<td class="px-3 py-2 break-words">
									{#if f.value}{f.value}{:else}<span class="text-ink-3">{m.website_value_empty()}</span>{/if}
									{#if f.recently_changed}
										<Plate tone="advisory" label={m.website_changed()} size="sm" class="ml-1" />
									{/if}
									{#if f.pending !== null}
										<p class="mt-1 text-ink-3">{m.website_pending({ value: f.pending || m.website_value_empty() })}</p>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}

		<h3 class="mt-5 mb-2 text-sm font-semibold">{m.website_crawl()}</h3>
		<div class="flex flex-wrap gap-x-6 gap-y-2 text-sm">
			<span class="flex flex-wrap items-center gap-2">
				<span class="text-ink-3">robots.txt</span>
				<Plate tone={robotsState(overview).tone} label={robotsState(overview).label} size="sm" />
			</span>
			<span class="flex flex-wrap items-center gap-2">
				<span class="text-ink-3">{m.website_sitemap()}</span>
				<Plate tone={sitemapState(overview).tone} label={sitemapState(overview).label} size="sm" />
			</span>
		</div>

		<h3 class="mt-5 mb-2 text-sm font-semibold">{m.website_changes()}</h3>
		{#if changes.length === 0}
			<p class="text-sm text-ink-2">{m.website_changes_none()}</p>
		{:else}
			<ul class="space-y-2 text-sm">
				{#each changes as c (c.id)}
					<li class="min-w-0 rounded-md border border-line px-3 py-2">
						<p class="flex flex-wrap items-baseline justify-between gap-x-3">
							<span class="font-medium">{fieldLabel(c.field)}</span>
							<span class="tnum text-ink-3">{formatRelative(c.detected_at)}</span>
						</p>
						<p class="mt-1 break-words text-ink-2">
							<span class="text-ink-3">{m.website_before()}</span>
							{c.before || m.website_value_empty()}
						</p>
						<p class="break-words">
							<span class="text-ink-3">{m.website_after()}</span>
							{c.after || m.website_value_empty()}
						</p>
					</li>
				{/each}
			</ul>
		{/if}
	{/if}
</Panel>
