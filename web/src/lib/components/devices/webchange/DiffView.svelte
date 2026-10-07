<script lang="ts">
	/**
	 * Visible-text diff of a change: one line per row, a `+`/`−` gutter marker
	 * next to the colour so an inserted or deleted line never reads by colour
	 * alone, and a collapsed run of unchanged lines folds into one notice.
	 */
	import type { WebchangeDiffLine } from '#lib/api/index.js';
	import { skipLabel } from './format';

	interface Props {
		diff: WebchangeDiffLine[];
	}

	let { diff }: Props = $props();

	function gutter(op: WebchangeDiffLine['op']): string {
		if (op === 'insert') return '+';
		if (op === 'delete') return '−';
		return ' ';
	}

	function rowClass(op: WebchangeDiffLine['op']): string {
		if (op === 'insert') return 'block px-3 bg-signal-soft text-signal-ink';
		if (op === 'delete') return 'block px-3 bg-warning-soft text-warning-ink';
		return 'block px-3 text-ink-2';
	}

	/** Spoken cue for a screen reader, since the gutter glyph alone reads poorly. */
	function spoken(op: WebchangeDiffLine['op']): string {
		if (op === 'insert') return 'Added: ';
		if (op === 'delete') return 'Removed: ';
		return '';
	}
</script>

{#if diff.length === 0}
	<p class="px-3 py-3 text-sm text-ink-2">No visible text to compare.</p>
{:else}
	<div class="overflow-x-auto rounded-lg border border-line bg-canvas-deep">
		<!-- A <pre> preserves every whitespace character, including template indentation: the loop stays on one line. -->
		<pre class="min-w-full whitespace-pre-wrap break-all font-mono text-[0.8125rem] leading-relaxed"><code>{#each diff as line, i (i)}{#if line.op === 'skip'}<span class="my-0.5 block px-3 py-1 text-center text-[0.75rem] text-ink-3 select-none">{skipLabel(line.count)}</span>{:else}<span class={rowClass(line.op)}><span class="mr-2 inline-block w-3 shrink-0 text-center font-semibold select-none" aria-hidden="true">{gutter(line.op)}</span><span class="sr-only">{spoken(line.op)}</span>{line.text || ' '}</span>{/if}{/each}</code></pre>
	</div>
{/if}
