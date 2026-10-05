<script lang="ts">
	/**
	 * Header of one folder section on /targets: collapse toggle, device count,
	 * worst status in the folder (so trouble is visible even collapsed), and —
	 * while a device is being dragged — a drop target that moves it here.
	 *
	 * Folder actions are entirely in-app: double-click (or the "⋯" menu)
	 * renames in place, and delete asks with the same two-step inline
	 * `Confirm` used elsewhere — no browser prompt or confirm dialog.
	 */
	import type { TargetState } from '$lib/format';
	import { STATE_LABEL, STATE_TONE } from '$lib/format';
	import { Confirm, Menu, Plate } from '$lib/ui';
	import { ChevronDown, EllipsisVertical, FolderOpen, GripVertical } from 'lucide-svelte';

	interface Props {
		label: string;
		/** Empty: the "no folder" section — it has no rename/delete, only a drop zone. */
		folderKey: string;
		count: number;
		worst: TargetState;
		collapsed: boolean;
		ontoggle: () => void;
		/** Admin, unfiltered, and this is a named folder: rename/delete are offered. */
		canManage?: boolean;
		busy?: boolean;
		onrename?: (next: string) => void | Promise<void>;
		ondelete?: () => void | Promise<void>;
		/** A device is being dragged and the pointer is currently over this header. */
		dropHover?: boolean;
		/** A drag is active somewhere on the page: this header is a valid drop target. */
		dropActive?: boolean;
		/** Reports the header's own element, so the page can hit-test it while dragging. */
		onheaderref?: (el: HTMLElement | null) => void;
	}

	let {
		label,
		folderKey,
		count,
		worst,
		collapsed,
		ontoggle,
		canManage = false,
		busy = false,
		onrename,
		ondelete,
		dropHover = false,
		dropActive = false,
		onheaderref
	}: Props = $props();

	let renaming = $state(false);
	let draft = $state('');
	let input = $state<HTMLInputElement | null>(null);

	function startRename() {
		if (!canManage) return;
		draft = folderKey;
		renaming = true;
		queueMicrotask(() => {
			input?.focus();
			input?.select();
		});
	}

	function cancelRename() {
		renaming = false;
	}

	function confirmRename() {
		const next = draft.trim();
		renaming = false;
		if (next && next !== folderKey) void onrename?.(next);
	}

	let el = $state<HTMLDivElement | null>(null);
	$effect(() => {
		onheaderref?.(el);
		return () => onheaderref?.(null);
	});

	// The actions menu overflows below the header, into the folder's own row
	// list — which otherwise paints (and catches pointer events) above it,
	// being later in the document. Lift the header while it is open.
	let actionsOpen = $state(false);
</script>

<div
	bind:this={el}
	class={`relative flex items-center gap-2 rounded-[var(--radius-card)] border px-3 py-2 transition-colors ${dropHover ? 'border-signal bg-signal-soft' : dropActive ? 'border-dashed border-line-strong' : 'border-line bg-surface-2'}`}
	style={actionsOpen ? 'z-index: 40;' : undefined}
>
	{#if dropActive}
		<GripVertical class="size-3.5 shrink-0 text-ink-3" aria-hidden="true" />
	{/if}
	<button
		type="button"
		class="flex min-w-0 flex-1 items-center gap-2 text-left"
		onclick={ontoggle}
		ondblclick={() => {
			if (canManage) startRename();
		}}
		aria-expanded={!collapsed}
	>
		<ChevronDown class={`size-4 shrink-0 text-ink-2 transition-transform duration-200 ${collapsed ? '-rotate-90' : ''}`} aria-hidden="true" />
		<FolderOpen class="size-4 shrink-0 text-ink-3" aria-hidden="true" />
		{#if renaming}
			<span class="sr-only">Renaming folder</span>
		{:else}
			<span class="truncate font-semibold text-ink">{label}</span>
		{/if}
		<span class="tnum text-[0.8125rem] text-ink-2">{count}</span>
	</button>

	{#if renaming}
		<input
			bind:this={input}
			bind:value={draft}
			class="input !h-7 w-40 text-[0.8125rem]"
			aria-label={`Rename folder "${folderKey}"`}
			disabled={busy}
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => {
				if (e.key === 'Enter') {
					e.preventDefault();
					confirmRename();
				} else if (e.key === 'Escape') {
					e.preventDefault();
					cancelRename();
				}
			}}
			onblur={confirmRename}
		/>
	{/if}

	<Plate tone={STATE_TONE[worst]} label={count > 0 ? STATE_LABEL[worst] : 'Empty'} size="sm" />

	{#if canManage && !renaming}
		<Menu label={`Actions for "${label}"`} align="right" bind:open={actionsOpen}>
			{#snippet trigger({ toggle, open })}
				<button
					type="button"
					class="rounded-md p-1 text-ink-3 hover:bg-surface-2 hover:text-ink"
					onclick={toggle}
					aria-haspopup="menu"
					aria-expanded={open}
					aria-label={`Actions for "${label}"`}
				>
					<EllipsisVertical class="size-4" aria-hidden="true" />
				</button>
			{/snippet}
			{#snippet children({ close })}
				<button
					type="button"
					role="menuitem"
					class="block w-full rounded-lg px-2 py-1.5 text-left text-[0.8125rem] font-medium text-ink hover:bg-surface"
					onclick={() => {
						close();
						startRename();
					}}
				>
					Rename folder
				</button>
				<Confirm
					size="sm"
					variant="danger"
					class="mt-1 w-full justify-start !px-2"
					confirmLabel="Delete for good?"
					loading={busy}
					onconfirm={() => {
						close();
						void ondelete?.();
					}}
				>
					Delete folder
				</Confirm>
			{/snippet}
		</Menu>
	{/if}
</div>
