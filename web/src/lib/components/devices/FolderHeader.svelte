<script lang="ts">
	/**
	 * Header of one folder section on /targets: collapse toggle, device count,
	 * worst status in the folder (so trouble is visible even collapsed), and —
	 * while a device is being dragged — a drop target that moves it here.
	 */
	import type { TargetState } from '$lib/format';
	import { STATE_LABEL, STATE_TONE } from '$lib/format';
	import { Plate } from '$lib/ui';
	import { ChevronDown, FolderOpen } from 'lucide-svelte';

	interface Props {
		label: string;
		count: number;
		worst: TargetState;
		collapsed: boolean;
		ontoggle: () => void;
		/** A device is being dragged somewhere on the page: show this as a drop target. */
		dropActive?: boolean;
		ondrop?: () => void;
	}

	let { label, count, worst, collapsed, ontoggle, dropActive = false, ondrop }: Props = $props();

	let over = $state(false);
</script>

<div
	class={`flex items-center gap-2 rounded-[var(--radius-card)] border px-3 py-2 transition-colors ${over ? 'border-signal bg-signal-soft' : 'border-line bg-surface-2'}`}
	role={dropActive ? 'group' : undefined}
	aria-label={dropActive ? `Drop here to move into ${label}` : undefined}
	ondragover={dropActive ? (e) => { e.preventDefault(); over = true; } : undefined}
	ondragleave={dropActive ? () => (over = false) : undefined}
	ondrop={
		dropActive
			? (e) => {
					e.preventDefault();
					over = false;
					ondrop?.();
				}
			: undefined
	}
>
	<button
		type="button"
		class="flex min-w-0 flex-1 items-center gap-2 text-left"
		onclick={ontoggle}
		aria-expanded={!collapsed}
	>
		<ChevronDown class={`size-4 shrink-0 text-ink-2 transition-transform duration-200 ${collapsed ? '-rotate-90' : ''}`} aria-hidden="true" />
		<FolderOpen class="size-4 shrink-0 text-ink-3" aria-hidden="true" />
		<span class="truncate font-semibold text-ink">{label}</span>
		<span class="tnum text-[0.8125rem] text-ink-2">{count}</span>
	</button>
	<Plate tone={STATE_TONE[worst]} label={STATE_LABEL[worst]} size="sm" />
</div>
