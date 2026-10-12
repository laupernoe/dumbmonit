<script lang="ts">
	/**
	 * Rows / tiles / compact for /targets: three toggle buttons in one strip,
	 * the same machined look as `Segmented`. Each button carries its name
	 * (visible from `sm`, read out everywhere), `aria-pressed` says which one
	 * is on, and the strip is 40 px tall on phones.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { LayoutGrid, List, Rows3 } from 'lucide-svelte';
	import type { RackView } from './view';

	interface Props {
		value: RackView;
		onchange: (value: RackView) => void;
	}

	let { value, onchange }: Props = $props();

	const OPTIONS = $derived([
		{ id: 'rows' as const, label: m.devices_targets_view_rows(), icon: Rows3 },
		{ id: 'tiles' as const, label: m.devices_targets_view_tiles(), icon: LayoutGrid },
		{ id: 'compact' as const, label: m.devices_targets_view_compact(), icon: List }
	]);
</script>

<div
	role="group"
	aria-label={m.devices_targets_view_label()}
	class="inline-flex shrink-0 items-center gap-0.5 rounded-lg border border-line-strong bg-surface-2 p-0.5 shadow-[inset_0_1px_2px_rgb(0_0_0/0.04)]"
>
	{#each OPTIONS as option (option.id)}
		{@const active = option.id === value}
		{@const Icon = option.icon}
		<button
			type="button"
			aria-pressed={active}
			title={option.label}
			onclick={() => onchange(option.id)}
			class={`inline-flex h-8 min-w-10 items-center justify-center gap-1.5 rounded-[6px] px-2.5 text-sm font-semibold whitespace-nowrap transition-[background-color,color,box-shadow] duration-200 ease-out-expo focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-signal max-sm:h-10 ${active ? 'bg-surface text-ink shadow-lift' : 'text-ink-2 hover:text-ink'}`}
		>
			<Icon class="size-4" aria-hidden="true" />
			<span class="max-xl:sr-only">{option.label}</span>
		</button>
	{/each}
</div>
