<script lang="ts">
	/**
	 * A runbook's text. `renderMarkdown` escapes every character before it
	 * writes its own few tags, so `{@html}` here never carries user markup.
	 */
	import { renderMarkdown } from '#lib/markdown.js';

	interface Props {
		source: string;
		class?: string;
	}

	let { source, class: className = '' }: Props = $props();
	const html = $derived(renderMarkdown(source));
</script>

<div
	class={`runbook-text min-w-0 break-words text-sm leading-relaxed text-ink [&_a]:font-medium [&_a]:underline [&_a]:underline-offset-2 [&_code]:rounded [&_code]:bg-surface-2 [&_code]:px-1 [&_code]:font-mono [&_code]:text-[0.8125rem] [&_h3]:mt-3 [&_h3]:font-semibold [&_h4]:mt-3 [&_h4]:font-semibold [&_h5]:mt-2 [&_h5]:font-medium [&_li]:mt-0.5 [&_ol]:mt-2 [&_ol]:list-decimal [&_ol]:pl-5 [&_p]:mt-2 [&_pre]:mt-2 [&_pre]:overflow-x-auto [&_pre]:rounded-md [&_pre]:border [&_pre]:border-line [&_pre]:bg-canvas-deep [&_pre]:p-3 [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_ul]:mt-2 [&_ul]:list-disc [&_ul]:pl-5 ${className}`}
>
	<!-- eslint-disable-next-line svelte/no-at-html-tags -- escaped by renderMarkdown -->
	{@html html}
</div>
