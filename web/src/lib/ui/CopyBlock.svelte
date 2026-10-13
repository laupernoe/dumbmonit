<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/** A command or token to copy: monospace block with a copy button that confirms. */
	import { Check, Copy, TextCursorInput } from 'lucide-svelte';
	import { copyText, selectContents } from '#lib/clipboard.js';
	import { applySudo, hasSudo, loadSudoPreference, saveSudoPreference } from '#lib/sudo.js';
	import { onMount, untrack } from 'svelte';

	interface Props {
		value: string;
		label?: string;
		/** Mask the value (tokens) until hovered/focused. */
		secret?: boolean;
		/** Unix shell command: offers a "Run with sudo" checkbox that edits what is shown and copied. */
		sudo?: boolean;
		class?: string;
	}
	let { value: source, label = m.app_ui_copy(), secret = false, sudo = false, class: className = '' }: Props = $props();

	/* A command already containing sudo starts checked; otherwise the remembered choice (read after mount). */
	let useSudo = $state(untrack(() => sudo && hasSudo(source)));
	onMount(() => {
		if (sudo && !hasSudo(source)) useSudo = loadSudoPreference() ?? false;
	});
	const value = $derived(sudo ? applySudo(source, useSudo) : source);

	let code: HTMLElement | undefined = $state();
	let status = $state<'idle' | 'copied' | 'failed'>('idle');
	let timer: ReturnType<typeof setTimeout> | undefined;

	async function copy() {
		const ok = await copyText(value);
		if (ok) {
			status = 'copied';
		} else {
			/* Clipboard unavailable: hand the text over selected so Ctrl+C works. */
			status = 'failed';
			selectContents(code);
		}
		clearTimeout(timer);
		timer = setTimeout(() => (status = 'idle'), ok ? 1600 : 4000);
	}

	const title = $derived(status === 'copied' ? m.app_ui_copied() : status === 'failed' ? m.app_ui_select_copy() : label);
</script>

<div class={`group relative min-w-0 max-w-full rounded-lg border border-line bg-canvas-deep ${className}`}>
	<pre class={`overflow-x-auto whitespace-pre-wrap break-all px-3 py-2.5 pr-12 font-mono text-[0.8125rem] leading-relaxed text-ink ${secret ? 'blur-[3px] transition group-focus-within:blur-0 group-hover:blur-0' : ''}`}><code bind:this={code}>{value}</code></pre>
	<button
		type="button"
		class="absolute top-1.5 right-1.5 inline-flex size-10 items-center justify-center rounded-md border sm:size-8 border-line bg-surface text-ink-2 transition hover:text-ink"
		onclick={copy}
		aria-label={title}
		{title}
	>
		{#if status === 'copied'}<Check class="check-draw size-4 text-signal-ink" aria-hidden="true" />{:else if status === 'failed'}<TextCursorInput class="size-4 text-warning-ink" aria-hidden="true" />{:else}<Copy class="size-4" aria-hidden="true" />{/if}
	</button>
	{#if status === 'failed'}
		<span class="pointer-events-none absolute top-full right-1.5 mt-1 rounded-md border border-line bg-surface px-2 py-1 text-xs text-ink-2 shadow-sm" role="status">{m.app_ui_select_copy()}</span>
	{/if}
	{#if sudo}
		<label class="flex min-h-10 cursor-pointer items-center gap-2 border-t border-line px-3 text-sm text-ink-2">
			<input
				type="checkbox"
				class="size-4 accent-signal"
				checked={useSudo}
				onchange={(event) => {
					useSudo = event.currentTarget.checked;
					saveSudoPreference(useSudo);
				}}
			/>
			{m.app_ui_run_with_sudo()}
		</label>
	{/if}
</div>
