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
	let panel = $state<HTMLDivElement | null>(null);

	/** Gap between the trigger and the panel, and the panel and the screen edge, in px. */
	const GAP = 4;
	const EDGE = 8;

	function place() {
		if (!root || !panel) return;
		const anchor = root.getBoundingClientRect();
		const vw = document.documentElement.clientWidth;
		const vh = window.innerHeight;
		panel.style.maxHeight = '';
		const { width, height } = panel.getBoundingClientRect();

		const preferred = align === 'right' ? anchor.right - width : anchor.left;
		const left = Math.max(EDGE, Math.min(preferred, vw - width - EDGE));

		const below = vh - anchor.bottom - GAP - EDGE;
		const above = anchor.top - GAP - EDGE;
		const fitsBelow = height <= below;
		const fitsAbove = height <= above;
		// Requested side first; the other one only when it fits better.
		const up =
			placement === 'up'
				? fitsAbove || (!fitsBelow && above >= below)
				: !fitsBelow && (fitsAbove || above > below);
		const room = Math.max(up ? above : below, 0);
		if (height > room) panel.style.maxHeight = `${room}px`;
		const top = up ? anchor.top - GAP - Math.min(height, room) : anchor.bottom + GAP;

		panel.style.left = `${Math.round(left)}px`;
		panel.style.top = `${Math.round(top)}px`;
	}

	$effect(() => {
		const node = panel;
		if (!open || !node) return;
		// Top layer where supported; otherwise `fixed` still escapes `overflow` clipping.
		if (typeof node.showPopover === 'function' && !node.matches(':popover-open')) node.showPopover();
		place();
		const resize = new ResizeObserver(place);
		resize.observe(node);
		window.addEventListener('resize', place);
		window.addEventListener('scroll', place, { capture: true, passive: true });
		return () => {
			resize.disconnect();
			window.removeEventListener('resize', place);
			window.removeEventListener('scroll', place, { capture: true });
		};
	});

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
			bind:this={panel}
			popover="manual"
			role="menu"
			aria-label={label}
			class="fixed inset-auto z-30 m-0 min-w-48 max-w-[calc(100vw-1rem)] overflow-y-auto rounded-[var(--radius-card)] border border-line bg-surface-2 p-1.5 text-left text-ink shadow-float"
		>
			{@render children({ close })}
		</div>
	{/if}
</div>
