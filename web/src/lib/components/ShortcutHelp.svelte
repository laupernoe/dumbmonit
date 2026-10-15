<script lang="ts">
	/** Keyboard shortcut help dialog, opened with `?` or from the command palette. */
	import { tick } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { palette } from '#lib/stores/palette.svelte.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { SHORTCUTS, type ShortcutId } from '#lib/palette/shortcuts.js';
	import { shortcutKeys } from '#lib/palette/logic.js';

	const LABEL: Record<ShortcutId, () => string> = {
		overview: () => m.misc_shortcuts_go({ page: m.misc_nav_overview() }),
		devices: () => m.misc_shortcuts_go({ page: m.misc_nav_devices() }),
		alerts: () => m.misc_shortcuts_go({ page: m.misc_nav_alerts() }),
		status: () => m.misc_shortcuts_go({ page: m.misc_palette_status_pages() }),
		add: () => m.misc_palette_add_device(),
		theme: () => m.misc_palette_theme(),
		help: () => m.misc_shortcuts_title()
	};

	const rows = $derived([
		{ label: m.misc_palette_label(), keys: [palette.isMac ? '⌘' : 'Ctrl', 'K'], sequence: false },
		...SHORTCUTS.filter((s) => !s.admin || auth.isAdmin).map((s) => ({
			label: LABEL[s.id](),
			keys: shortcutKeys(s.keys),
			sequence: s.keys.includes(' ')
		}))
	]);

	let closeButton = $state<HTMLButtonElement | null>(null);
	$effect(() => {
		if (palette.helpOpen) void tick().then(() => closeButton?.focus());
	});

	// Focus stays on the only control; Escape closes.
	function onWindowKeydown(event: KeyboardEvent) {
		if (!palette.helpOpen) return;
		if (event.key === 'Escape') {
			event.preventDefault();
			palette.closeHelp();
		} else if (event.key === 'Tab') {
			event.preventDefault();
			closeButton?.focus();
		}
	}
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#if palette.helpOpen}
	<div
		class="shortcut-backdrop fixed inset-0 z-50 flex items-start justify-center bg-canvas/70 px-4 pt-[12vh] backdrop-blur-sm"
		onclick={(event) => {
			if (event.target === event.currentTarget) palette.closeHelp();
		}}
		role="presentation"
	>
		<div
			class="shortcut-panel w-full max-w-md overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface shadow-float ring-1 ring-signal/40"
			role="dialog"
			aria-modal="true"
			aria-labelledby="shortcut-help-title"
		>
			<div class="flex items-center justify-between border-b border-line px-4 py-3">
				<h2 id="shortcut-help-title" class="text-sm font-semibold text-ink">{m.misc_shortcuts_title()}</h2>
				<button
					bind:this={closeButton}
					type="button"
					class="rounded-lg border border-line px-2.5 py-1 text-[0.8125rem] font-semibold text-ink-2 hover:bg-surface-2 hover:text-ink"
					onclick={() => palette.closeHelp()}
				>
					{m.misc_shortcuts_close()}
				</button>
			</div>
			<p class="px-4 pt-3 text-[0.8125rem] text-ink-2">{m.misc_shortcuts_intro()}</p>
			<ul class="px-4 py-3">
				{#each rows as row (row.label + row.keys.join())}
					<li class="flex items-center justify-between gap-4 py-1.5 text-sm text-ink">
						<span class="min-w-0 truncate">{row.label}</span>
						<span class="flex shrink-0 items-center gap-1">
							{#each row.keys as key, i (i)}
								<kbd class="shortcut-key">{key}</kbd>
							{/each}
						</span>
					</li>
				{/each}
			</ul>
		</div>
	</div>
{/if}

<style>
	.shortcut-key {
		display: inline-block;
		min-width: 1.5rem;
		border: 1px solid var(--c-line);
		border-radius: 0.375rem;
		background: var(--c-surface-2);
		padding: 0.05rem 0.4rem;
		text-align: center;
		font-family: inherit;
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--c-ink-2);
	}
	@keyframes shortcut-in {
		from {
			opacity: 0;
			transform: scale(0.97) translateY(-6px);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}
	.shortcut-backdrop,
	.shortcut-panel {
		animation: shortcut-in 160ms var(--ease-out-expo) both;
	}
	@media (prefers-reduced-motion: reduce) {
		.shortcut-backdrop,
		.shortcut-panel {
			animation: none;
		}
	}
</style>
