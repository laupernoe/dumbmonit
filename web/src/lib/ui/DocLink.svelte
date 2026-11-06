<script lang="ts">
	import { withBase } from '#lib/paths.js';
	/**
	 * The only help a screen carries: a discreet link to the matching page of the
	 * documentation site (in the language of the UI), in a new tab.
	 * `page` is the path after `/docs/`, for example `devices/agent/`.
	 */
	import { BookOpen } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { docsUrl } from '#lib/docs.js';

	let { page, label = undefined, class: className = '' }: { page: string; label?: string; class?: string } = $props();
</script>

<a
	href={withBase(docsUrl(page))}
	target="_blank"
	rel="noopener"
	class={`inline-flex min-h-10 items-center gap-1.5 justify-self-start text-[0.8125rem] font-medium text-ink-2 transition-colors hover:text-signal-ink hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal ${className}`}
>
	<BookOpen class="size-4 shrink-0" aria-hidden="true" />
	{label ?? m.ui_doclink_label()}
</a>
