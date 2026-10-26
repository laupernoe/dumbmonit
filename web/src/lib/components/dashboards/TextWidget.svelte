<script lang="ts">
	/** A note: small Markdown, rendered with elements — never as HTML. */
	import { parseMarkdown, type Inline } from '#lib/dashboards/markdown.js';

	let { markdown }: { markdown: string } = $props();
	const blocks = $derived(parseMarkdown(markdown));
</script>

{#snippet inline(parts: Inline[])}
	{#each parts as part, i (i)}
		{#if part.t === 'b'}<strong>{part.v}</strong>
		{:else if part.t === 'i'}<em>{part.v}</em>
		{:else if part.t === 'code'}<code class="rounded bg-surface-2 px-1 py-0.5 text-[0.85em]">{part.v}</code>
		{:else if part.t === 'a'}<a href={part.href} rel="noopener noreferrer" target={part.href.startsWith('/') ? undefined : '_blank'} class="text-signal-ink underline">{part.v}</a>
		{:else}{part.v}{/if}
	{/each}
{/snippet}

<div class="flex max-w-prose flex-col gap-2 text-sm leading-relaxed text-ink">
	{#each blocks as block, i (i)}
		{#if block.t === 'h'}
			<p class={block.level === 1 ? 'text-lg font-bold' : block.level === 2 ? 'text-base font-bold' : 'text-sm font-bold'}>{@render inline(block.inline)}</p>
		{:else if block.t === 'p'}
			<p>{@render inline(block.inline)}</p>
		{:else if block.t === 'ul'}
			<ul class="list-disc pl-5">{#each block.items as item, j (j)}<li>{@render inline(item)}</li>{/each}</ul>
		{:else if block.t === 'ol'}
			<ol class="list-decimal pl-5">{#each block.items as item, j (j)}<li>{@render inline(item)}</li>{/each}</ol>
		{:else}
			<pre class="overflow-x-auto rounded-lg bg-surface-2 p-2 text-xs">{block.text}</pre>
		{/if}
	{/each}
</div>
