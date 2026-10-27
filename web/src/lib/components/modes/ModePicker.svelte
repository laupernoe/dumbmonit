<script lang="ts">
	/**
	 * The two product modes side by side, each with a miniature and what it
	 * changes. A radio group: arrow keys move between the two, the choice is
	 * the checked card. Used by the setup wizard and Settings → General.
	 */
	import { Check } from 'lucide-svelte';
	import type { ProductMode } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import ModePreview from './ModePreview.svelte';

	interface Props {
		value: ProductMode;
		onchange?: (mode: ProductMode) => void;
		disabled?: boolean;
		/** Radio group name, unique per page. */
		name?: string;
	}
	let { value = $bindable(), onchange, disabled = false, name = 'product-mode' }: Props = $props();

	const MODES: { id: ProductMode; title: () => string; text: () => string; points: (() => string)[] }[] = [
		{
			id: 'homelab',
			title: () => m.mode_homelab_title(),
			text: () => m.mode_homelab_text(),
			points: [() => m.mode_homelab_point_nav(), () => m.mode_homelab_point_forms(), () => m.mode_homelab_point_theme()]
		},
		{
			id: 'enterprise',
			title: () => m.mode_enterprise_title(),
			text: () => m.mode_enterprise_text(),
			points: [() => m.mode_enterprise_point_nav(), () => m.mode_enterprise_point_table(), () => m.mode_enterprise_point_team()]
		}
	];

	function pick(mode: ProductMode) {
		if (disabled || mode === value) return;
		value = mode;
		onchange?.(mode);
	}
</script>

<div role="radiogroup" aria-label={m.mode_picker_label()} class="grid gap-3 sm:grid-cols-2">
	{#each MODES as option (option.id)}
		{@const checked = value === option.id}
		<label
			class={`group relative flex min-w-0 cursor-pointer flex-col gap-3 rounded-[var(--radius-card)] border p-3 transition-colors has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-focus ${checked ? 'border-ink bg-surface-2' : 'border-line bg-surface hover:border-line-strong'} ${disabled ? 'cursor-not-allowed opacity-60' : ''}`}
		>
			<input type="radio" {name} value={option.id} {checked} {disabled} class="sr-only" onchange={() => pick(option.id)} />
			<ModePreview mode={option.id} class="h-32" />
			<span class="flex items-center gap-2">
				<span class="text-base font-semibold text-ink">{option.title()}</span>
				{#if checked}
					<span class="ml-auto inline-flex items-center gap-1 rounded-full bg-ink px-2 py-0.5 text-[0.6875rem] font-semibold text-canvas">
						<Check class="size-3" aria-hidden="true" />{m.mode_picker_selected()}
					</span>
				{/if}
			</span>
			<span class="text-sm text-ink-2">{option.text()}</span>
			<ul class="grid gap-1 text-[0.8125rem] text-ink">
				{#each option.points as point, i (i)}
					<li class="flex gap-2"><span aria-hidden="true" class="mt-[0.45rem] size-1 shrink-0 rounded-full bg-ink-3"></span>{point()}</li>
				{/each}
			</ul>
		</label>
	{/each}
</div>
