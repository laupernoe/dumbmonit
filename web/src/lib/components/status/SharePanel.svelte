<script lang="ts">
	/**
	 * Share a page: README badges (status, uptime, response time — for the page
	 * or one service) and the iframe snippet of the compact embed. Everything
	 * here reads the public document, so it says no more than the page does.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import type { StatusPage } from '#lib/api/index.js';
	import { statusBadgeBase } from '#lib/api/index.js';
	import { CopyBlock, Field, Panel, Plate } from '#lib/ui/index.js';
	import { slugify } from './words';

	interface Props {
		page: StatusPage;
	}

	let { page }: Props = $props();

	type Kind = 'badge' | 'uptime' | 'response';
	const KINDS = (): { value: Kind; label: string }[] => [
		{ value: 'badge', label: m.status_share_kind_status() },
		{ value: 'uptime', label: m.status_share_kind_uptime() },
		{ value: 'response', label: m.status_share_kind_response() }
	];
	const WINDOWS = [1, 7, 30, 90];

	let kind = $state<Kind>('badge');
	let days = $state(30);
	/** `''` is the whole page; otherwise a service key. */
	let component = $state('');

	// Service keys, derived from the public labels the same way the server does.
	const components = $derived.by(() => {
		const taken = new Set<string>();
		return page.items.map((item) => {
			const base = slugify(item.label) || 'service';
			let key = base;
			for (let n = 2; taken.has(key); n++) key = `${base}-${n}`;
			taken.add(key);
			return { key, label: item.label };
		});
	});

	// A page with its own public domain is shared from there; otherwise from here.
	const origin = $derived(
		page.domain ? `https://${page.domain}` : typeof window === 'undefined' ? '' : window.location.origin
	);
	const windows = $derived(WINDOWS.filter((d) => d <= page.show_uptime_days));
	const badgeUrl = $derived.by(() => {
		const base = `${origin}${statusBadgeBase(page.slug, component || undefined)}`;
		if (kind === 'uptime') return `${base}/uptime.svg?days=${days}`;
		return `${base}/${kind}.svg`;
	});
	const pageUrl = $derived(page.domain ? `${origin}/` : `${origin}/s/${page.slug}`);
	const alt = $derived(
		m.status_share_alt({ name: component ? (components.find((c) => c.key === component)?.label ?? m.status_share_service()) : page.title, kind: (KINDS().find((k) => k.value === kind)?.label ?? '').toLowerCase() })
	);
	const markdown = $derived(`[![${alt}](${badgeUrl})](${pageUrl})`);
	const html = $derived(`<a href="${pageUrl}"><img src="${badgeUrl}" alt="${alt}"></a>`);
	const iframe = $derived(
		`<iframe src="${origin}/s/${page.slug}/embed" title="${m.status_share_iframe_title({ title: page.title })}" width="100%" height="320" style="border:0"></iframe>`
	);
</script>

<Panel title={m.status_share_title()} description={m.status_share_description()}>
	{#snippet aside()}
		{#if !page.published}
			<Plate tone="ghost" label={m.status_share_draft()} />
		{/if}
	{/snippet}
	<div class="grid gap-4">
		<div class="grid gap-3 sm:grid-cols-3">
			<Field label={m.status_share_badge()} for="share-kind">
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
		{#if page.published}
			<div class="flex items-center gap-3">
				<span class="text-[0.8125rem] text-ink-2">{m.status_share_preview()}</span>
				<img src={badgeUrl} alt={alt} class="h-5" />
			</div>
		{/if}
		<div class="grid gap-2">
			<span class="text-sm text-ink">Markdown</span>
			<CopyBlock value={markdown} label={m.status_share_copy_markdown()} />
			<span class="text-sm text-ink">HTML</span>
			<CopyBlock value={html} label={m.status_share_copy_html()} />
		</div>
		<div class="grid gap-2">
			<span class="text-sm text-ink">{m.status_share_embed()}</span>
			<CopyBlock value={iframe} label={m.status_share_copy_iframe()} />
			<p class="text-[0.8125rem] text-ink-2">{m.status_share_embed_hint({ light: "?theme=light", dark: "?theme=dark", history: "?history=0" })}</p>
		</div>
	</div>
</Panel>
