<script lang="ts">
	/**
	 * Settings footer of a status page: the banner you can put on your own
	 * website (HTML, PHP, WordPress). The snippets carry the server address and
	 * the page slug, nothing else: the banner reads the public document.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { Download, ExternalLink } from 'lucide-svelte';
	import { Button, CopyBlock } from '#lib/ui/index.js';
	import { websiteBannerDocsUrl } from '#lib/docs.js';

	interface Props {
		slug: string;
		published: boolean;
	}

	let { slug, published }: Props = $props();

	// A script tag cannot be written whole inside this file's own script block.
	const CLOSE_SCRIPT = '</' + 'script>';

	const origin = $derived(typeof window === 'undefined' ? '' : window.location.origin);
	const pageApi = $derived(`${origin}/api/public/status/${encodeURIComponent(slug || 'my-page')}`);
	const html = $derived(
		`<script async\n  src="${pageApi}/banner.js"\n  data-position="top" data-show="issues" data-lang="${getLocale()}">${CLOSE_SCRIPT}`
	);
	const php = $derived(
		`<?php\nrequire __DIR__ . '/banner.php';\ndumbmonit_banner('${pageApi}',\n    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);\n?>`
	);
</script>

<section class="grid gap-4 border-t border-line pt-5" aria-labelledby="status-banner-title">
	<div class="grid gap-1">
		<h3 id="status-banner-title" class="text-sm font-semibold text-ink">{m.status_banner_title()}</h3>
		<p class="text-[0.8125rem] text-ink-2">{m.status_banner_intro()}</p>
		<a
			href={websiteBannerDocsUrl()}
			target="_blank"
			rel="noopener"
			class="inline-flex min-h-10 w-fit items-center gap-1.5 text-sm text-signal underline-offset-2 hover:underline"
		>
			{m.status_banner_docs()}
			<ExternalLink class="size-3.5" aria-hidden="true" />
		</a>
		{#if !published}
			<p class="text-[0.8125rem] text-advisory">{m.status_banner_draft()}</p>
		{/if}
	</div>

	<div class="grid gap-2">
		<h4 class="text-sm text-ink">{m.status_banner_html()}</h4>
		<p class="text-[0.8125rem] text-ink-2">{m.status_banner_html_hint()}</p>
		<CopyBlock value={html} label={m.status_banner_copy_html()} />
	</div>

	<div class="grid gap-2">
		<h4 class="text-sm text-ink">{m.status_banner_php()}</h4>
		<p class="text-[0.8125rem] text-ink-2">{m.status_banner_php_hint()}</p>
		<div>
			<Button href="/api/integrations/banner.php" download="banner.php" variant="secondary">
				<Download class="size-4" aria-hidden="true" />
				{m.status_banner_php_download()}
			</Button>
		</div>
		<CopyBlock value={php} label={m.status_banner_copy_php()} />
	</div>

	<div class="grid gap-2">
		<h4 class="text-sm text-ink">{m.status_banner_wp()}</h4>
		<p class="text-[0.8125rem] text-ink-2">{m.status_banner_wp_hint()}</p>
		<div>
			<Button href="/api/integrations/wordpress.zip" download="dumbmonit-status-banner.zip" variant="secondary">
				<Download class="size-4" aria-hidden="true" />
				{m.status_banner_wp_download()}
			</Button>
		</div>
		<CopyBlock value={pageApi} label={m.status_banner_copy_address()} />
	</div>
</section>
