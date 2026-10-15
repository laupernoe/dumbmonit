<script lang="ts">
	/**
	 * A small floating menu anchored under its trigger: open/close state,
	 * outside click and Escape close it. Floating (not in-flow) so it never
	 * pushes the row it lives in — unlike `AckControl`'s menu, this one sits
	 * next to short, fixed-height rows where that push would read as a jump.
	 *
	 * `children` is free-form content, not just a list of menu items: a
	 * "move to folder" list, a folder's rename/delete actions, or an inline
	 * "new folder" form all render the same way.
	 */
	import type { Snippet } from 'svelte';

	interface Props {
		trigger: Snippet<[{ toggle: () => void; open: boolean }]>;
		children: Snippet<[{ close: () => void }]>;
		label: string;
		align?: 'left' | 'right';
		/** Open above the trigger (in a bar pinned to the bottom of the screen). */
		placement?: 'down' | 'up';
		open?: boolean;
		class?: string;
	}

	let {
		trigger,
		children,
		label,
		align = 'right',
		placement = 'down',
		open = $bindable(false),
		class: className = ''
	}: Props = $props();

	let root = $state<HTMLDivElement | null>(null);

	function toggle() {
		open = !open;
	}
	function close() {
		open = false;
	}

	$effect(() => {
		if (!open) return;
		const onpointerdown = (event: PointerEvent) => {
			if (root && !root.contains(event.target as Node)) close();
		};
		const onkeydown = (event: KeyboardEvent) => {
			if (event.key === 'Escape') {
				event.stopPropagation();
				close();
			}
		};
		document.addEventListener('pointerdown', onpointerdown);
		document.addEventListener('keydown', onkeydown);
		return () => {
			document.removeEventListener('pointerdown', onpointerdown);
			document.removeEventListener('keydown', onkeydown);
		};
	});
</script>

<div class={`relative inline-block ${className}`} bind:this={root}>
	{@render trigger({ toggle, open })}
	{#if open}
		<div
			role="menu"
			aria-label={label}
			class={`absolute z-30 min-w-48 ${placement === 'up' ? 'bottom-full mb-1' : 'top-full mt-1'} rounded-[var(--radius-card)] border border-line bg-surface-2 p-1.5 text-left shadow-float ${align === 'right' ? 'right-0' : 'left-0'}`}
		>
			{@render children({ close })}
		</div>
	{/if}
</div>
