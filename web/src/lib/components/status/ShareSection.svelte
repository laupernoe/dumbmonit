<script lang="ts">
	/**
	 * "Show it elsewhere": the three ways to put a status page on another site,
	 * one at a time behind tabs: a README badge, the iframe embed, and the
	 * website banner (HTML, PHP, WordPress). Each tab says when to pick it,
	 * offers its few options on the left and shows one block of code (and a
	 * preview when there is one) on the right. Everything here reads the public
	 * document, so it says no more than the page does.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { Download, ExternalLink } from 'lucide-svelte';
	import type { StatusPage } from '#lib/api/index.js';
	import { Button, CopyBlock, Field, Panel, Plate, Toggle } from '#lib/ui/index.js';
	import { rovingIndex } from '#lib/ui/roving.js';
	import { websiteBannerDocsUrl } from '#lib/docs.js';
	import {
		badgeHtml,
		badgeMarkdown,
		badgeUrl,
		bannerHtml,
		bannerPhp,
		embedIframe,
		embedUrl,
		publicApi,
		serviceKeys,
		type BadgeKind,
		type EmbedTheme
	} from './share';

	interface Props {
		page: StatusPage;
	}

	let { page }: Props = $props();

	type Tab = 'badge' | 'embed' | 'banner';
	const TABS = (): { id: Tab; label: string }[] => [
		{ id: 'badge', label: m.status_share_badge() },
		{ id: 'embed', label: m.status_share_embed() },
		{ id: 'banner', label: m.status_elsewhere_tab_banner() }
	];
	let tab = $state<Tab>('badge');

	function onTabKey(event: KeyboardEvent) {
		const tabs = TABS();
		const next = rovingIndex(event.key, tabs.findIndex((t) => t.id === tab), tabs.length);
		if (next === null) return;
		tab = tabs[next].id;
		event.preventDefault();
		document.getElementById(`elsewhere-tab-${tab}`)?.focus();
	}

	// The local address, for the banner and the previews.
	const here = $derived(typeof window === 'undefined' ? '' : window.location.origin);
	// A page with its own public domain is shared from there; otherwise from here.
	const origin = $derived(page.domain ? `https://${page.domain}` : here);
	const pageUrl = $derived(page.domain ? `${origin}/` : `${origin}/s/${page.slug}`);

	// Badge
	const KINDS = (): { value: BadgeKind; label: string }[] => [
		{ value: 'badge', label: m.status_share_kind_status() },
		{ value: 'uptime', label: m.status_share_kind_uptime() },
		{ value: 'response', label: m.status_share_kind_response() }
	];
	const WINDOWS = [1, 7, 30, 90];
	let kind = $state<BadgeKind>('badge');
	let days = $state(30);
	/** `''` is the whole page; otherwise a service key. */
	let component = $state('');
	let format = $state<'markdown' | 'html'>('markdown');
	const components = $derived(serviceKeys(page.items.map((item) => item.label)));
	const windows = $derived(WINDOWS.filter((d) => d <= page.show_uptime_days));
	const badgeImage = $derived(badgeUrl(origin, page.slug, kind, component, days));
	const badgePreview = $derived(badgeUrl(here, page.slug, kind, component, days));
	const alt = $derived(
		m.status_share_alt({
			name: component ? (components.find((c) => c.key === component)?.label ?? m.status_share_service()) : page.title,
			kind: (KINDS().find((k) => k.value === kind)?.label ?? '').toLowerCase()
		})
	);
	const badgeCode = $derived(format === 'markdown' ? badgeMarkdown(alt, badgeImage, pageUrl) : badgeHtml(alt, badgeImage, pageUrl));

	// Embed
	let embedTheme = $state<EmbedTheme>('page');
	let embedHistory = $state(true);
	const iframeTitle = $derived(m.status_share_iframe_title({ title: page.title }));
	const embedCode = $derived(embedIframe(embedUrl(origin, page.slug, embedTheme, embedHistory), iframeTitle));
	const embedPreview = $derived(embedUrl(here, page.slug, embedTheme, embedHistory));

	// Website banner
	type Platform = 'html' | 'php' | 'wordpress';
	let platform = $state<Platform>('html');
	const PLATFORMS = (): { value: Platform; label: string; hint: string }[] => [
		{ value: 'html', label: m.status_banner_html(), hint: m.status_banner_html_hint() },
		{ value: 'php', label: m.status_banner_php(), hint: m.status_banner_php_hint() },
		{ value: 'wordpress', label: m.status_banner_wp(), hint: m.status_banner_wp_hint() }
	];
	const bannerApi = $derived(publicApi(here, page.slug));
	const bannerCode = $derived(platform === 'html' ? bannerHtml(bannerApi, getLocale()) : platform === 'php' ? bannerPhp(bannerApi) : bannerApi);
	const bannerCopyLabel = $derived(
		platform === 'html' ? m.status_banner_copy_html() : platform === 'php' ? m.status_banner_copy_php() : m.status_banner_copy_address()
	);

	const LEAD = 'text-sm text-ink-2';
	const SUBHEAD = 'text-sm font-semibold text-ink';
	const SEGMENT =
		'inline-flex min-h-10 cursor-pointer items-center rounded-md px-3 py-1.5 text-sm focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ink';
</script>

<!-- A two-or-three way choice drawn as a segmented control (radio buttons underneath). -->
{#snippet segmented(name: string, label: string, options: { value: string; label: string }[], value: string, onpick: (v: string) => void)}
	<div class="grid gap-1.5">
		<span class={SUBHEAD} id="{name}-label">{label}</span>
		<div class="flex max-w-full flex-wrap gap-1 justify-self-start rounded-lg border border-line bg-canvas-deep p-1" role="radiogroup" aria-labelledby="{name}-label">
			{#each options as option (option.value)}
				<label class={`${SEGMENT} ${value === option.value ? 'bg-surface font-semibold text-ink shadow-lift' : 'text-ink-2 hover:text-ink'}`}>
					<input type="radio" class="sr-only" {name} value={option.value} checked={value === option.value} onchange={() => onpick(option.value)} />
					{option.label}
				</label>
			{/each}
		</div>
	</div>
{/snippet}

<Panel title={m.status_elsewhere_title()} description={m.status_elsewhere_description()}>
	{#snippet aside()}
		{#if !page.published}
			<Plate tone="ghost" label={m.status_share_draft()} />
		{/if}
	{/snippet}

	<div class="grid gap-5">
		<div class="flex max-w-full flex-wrap gap-1 justify-self-start rounded-lg border border-line bg-canvas-deep p-1" role="tablist" aria-label={m.status_elsewhere_title()}>
			{#each TABS() as option (option.id)}
				<button
					type="button"
					role="tab"
					id="elsewhere-tab-{option.id}"
					aria-selected={tab === option.id}
					aria-controls="elsewhere-panel"
					tabindex={tab === option.id ? 0 : -1}
					class={`min-h-10 rounded-md px-4 py-1.5 text-sm transition-colors ${tab === option.id ? 'bg-surface font-semibold text-ink shadow-lift' : 'text-ink-2 hover:text-ink'}`}
					onclick={() => (tab = option.id)}
					onkeydown={onTabKey}
				>
					{option.label}
				</button>
			{/each}
		</div>

		{#if !page.published}
			<p class="text-[0.8125rem] text-advisory-ink">{m.status_elsewhere_draft()}</p>
		{/if}

		<div id="elsewhere-panel" role="tabpanel" aria-labelledby="elsewhere-tab-{tab}" class="grid min-w-0 items-start gap-x-8 gap-y-5 lg:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
			{#if tab === 'badge'}
				<div class="grid gap-4">
					<p class={LEAD}>{m.status_elsewhere_badge_when()}</p>
					<div class="grid gap-3 sm:grid-cols-2">
						<Field label={m.status_elsewhere_badge_shows()} for="share-kind">
							<select id="share-kind" class="input" bind:value={kind}>
								{#each KINDS() as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
							</select>
						</Field>
						<Field label={m.status_share_for()} for="share-component">
							<select id="share-component" class="input" bind:value={component}>
								<option value="">{m.status_share_whole_page()}</option>
								{#each components as option (option.key)}<option value={option.key}>{option.label}</option>{/each}
							</select>
						</Field>
						{#if kind === 'uptime'}
							<Field label={m.status_share_window()} for="share-days">
								<select id="share-days" class="input" bind:value={days}>
									{#each windows as option (option)}<option value={option}>{option === 1 ? m.status_share_24_hours() : m.status_share_days({ days: option })}</option>{/each}
								</select>
							</Field>
						{/if}
					</div>
					{@render segmented('share-format', m.status_elsewhere_format(), [{ value: 'markdown', label: 'Markdown' }, { value: 'html', label: 'HTML' }], format, (v) => (format = v as 'markdown' | 'html'))}
					<p class="text-[0.8125rem] text-ink-2">{format === 'markdown' ? m.status_elsewhere_format_markdown() : m.status_elsewhere_format_html()}</p>
				</div>
				<div class="grid min-w-0 gap-3">
					{#if page.published}
						<div class="flex min-h-10 items-center gap-3 rounded-lg border border-dashed border-line px-3 py-2">
							<span class="text-[0.8125rem] text-ink-2">{m.status_share_preview()}</span>
							<img src={badgePreview} {alt} class="h-5" />
						</div>
					{/if}
					<CopyBlock value={badgeCode} label={format === 'markdown' ? m.status_share_copy_markdown() : m.status_share_copy_html()} />
				</div>
			{:else if tab === 'embed'}
				<div class="grid gap-4">
					<p class={LEAD}>{m.status_elsewhere_embed_when()}</p>
					<Field label={m.status_elsewhere_embed_theme()} for="share-embed-theme">
						<select id="share-embed-theme" class="input" bind:value={embedTheme}>
							<option value="page">{m.status_elsewhere_embed_theme_page()}</option>
							<option value="light">{m.status_form_theme_day()}</option>
							<option value="dark">{m.status_form_theme_night()}</option>
						</select>
					</Field>
					<div class="flex min-h-12 items-center justify-between gap-4 rounded-lg border border-line px-3 py-2">
						<div class="min-w-0">
							<label for="share-embed-history" class="block text-sm text-ink">{m.status_elsewhere_embed_history()}</label>
							<p class="text-[0.8125rem] text-ink-2">{m.status_elsewhere_embed_history_help()}</p>
						</div>
						<Toggle id="share-embed-history" bind:checked={embedHistory} label={m.status_elsewhere_embed_history()} />
					</div>
				</div>
				<div class="grid min-w-0 gap-3">
					<CopyBlock value={embedCode} label={m.status_share_copy_iframe()} />
					{#if page.published}
						<div class="grid gap-1.5">
							<span class="text-[0.8125rem] text-ink-2">{m.status_share_preview()}</span>
							{#key embedPreview}
								<iframe src={embedPreview} title={iframeTitle} class="h-80 w-full rounded-lg border border-line bg-canvas" loading="lazy"></iframe>
							{/key}
						</div>
					{/if}
				</div>
			{:else}
				<div class="grid gap-4">
					<p class={LEAD}>{m.status_elsewhere_banner_when()}</p>
					{@render segmented('share-platform', m.status_elsewhere_banner_platform(), PLATFORMS(), platform, (v) => (platform = v as Platform))}
					<p class="text-[0.8125rem] text-ink-2">{PLATFORMS().find((p) => p.value === platform)?.hint}</p>
					<div class="flex flex-wrap items-center gap-x-4 gap-y-2">
						{#if platform === 'php'}
							<Button href="/api/integrations/banner.php" download="banner.php" variant="secondary">
								<Download class="size-4" aria-hidden="true" />
								{m.status_banner_php_download()}
							</Button>
						{:else if platform === 'wordpress'}
							<Button href="/api/integrations/wordpress.zip" download="dumbmonit-status-banner.zip" variant="secondary">
								<Download class="size-4" aria-hidden="true" />
								{m.status_banner_wp_download()}
							</Button>
						{/if}
						<a
							href={websiteBannerDocsUrl()}
							target="_blank"
							rel="noopener"
							class="inline-flex min-h-10 w-fit items-center gap-1.5 text-sm text-signal-ink underline-offset-2 hover:underline"
						>
							{m.status_banner_docs()}
							<ExternalLink class="size-3.5" aria-hidden="true" />
						</a>
					</div>
				</div>
				<div class="grid min-w-0 gap-3">
					<CopyBlock value={bannerCode} label={bannerCopyLabel} />
				</div>
			{/if}
		</div>
	</div>
</Panel>
