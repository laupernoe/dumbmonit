<script lang="ts">
	/**
	 * "Backdrop" on the wall: a quiet button beside Exit that opens a small
	 * panel choosing the night city drawn behind the verdict and the clock
	 * (none, auto, or one of the four status page scenes). Remembered per
	 * browser; a `?city=` on a shared link overrides it for that display
	 * without touching what is saved. Escape or a click elsewhere closes it.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { Building2 } from 'lucide-svelte';
	import { Button } from '#lib/ui/index.js';
	import { SCENE_CHOICES } from '#lib/components/status/scenes/registry.js';
	import type { WallSceneChoice } from '#lib/components/wall/wallScene.js';

	interface Props {
		value: WallSceneChoice;
		/** True when a `?city=` link is choosing right now. */
		forced: boolean;
		onchange: (value: WallSceneChoice) => void;
	}

	let { value, forced, onchange }: Props = $props();

	let open = $state(false);
	let root = $state<HTMLDivElement | null>(null);

	const OPTIONS = $derived<{ value: WallSceneChoice; label: string }[]>([
		{ value: 'none', label: m.wall_scene_none() },
		{ value: 'auto', label: m.wall_scene_auto() },
		...SCENE_CHOICES.map((c) => ({ value: c.value as WallSceneChoice, label: c.label }))
	]);

	$effect(() => {
		if (!open) return;
		const onPointer = (event: PointerEvent) => {
			if (root && !root.contains(event.target as Node)) open = false;
		};
		const onKey = (event: KeyboardEvent) => {
			if (event.key === 'Escape') {
				// Closes the panel only, not the wall.
				event.stopPropagation();
				open = false;
			}
		};
		document.addEventListener('pointerdown', onPointer);
		window.addEventListener('keydown', onKey, true);
		return () => {
			document.removeEventListener('pointerdown', onPointer);
			window.removeEventListener('keydown', onKey, true);
		};
	});
</script>

<div class="relative" bind:this={root}>
	<Button
		variant="ghost"
		size="sm"
		onclick={() => (open = !open)}
		aria-expanded={open}
		aria-controls="wall-scene-panel"
		class="min-h-10"
	>
		<Building2 class="size-4" aria-hidden="true" />
		{m.wall_scene_button()}
	</Button>

	{#if open}
		<div
			id="wall-scene-panel"
			class="absolute top-full right-0 z-20 mt-2 w-[min(calc(100vw-2rem),20rem)] rounded-[var(--radius-card)] border border-line bg-surface p-4 text-left shadow-float"
			role="dialog"
			aria-label={m.wall_scene_title()}
		>
			<p class="m-0 text-sm font-semibold text-ink">{m.wall_scene_title()}</p>
			<p class="m-0 mt-1 text-[0.8125rem] text-ink-2">{forced ? m.wall_scene_forced() : m.wall_scene_hint()}</p>
			<div class="mt-3 flex flex-wrap gap-1.5" role="radiogroup" aria-label={m.wall_scene_title()}>
				{#each OPTIONS as option (option.value)}
					<button
						type="button"
						role="radio"
						aria-checked={value === option.value}
						class={`min-h-10 rounded-[var(--radius-plate)] border px-3 py-1 text-[0.8125rem] transition-colors ${value === option.value ? 'border-signal bg-signal-soft font-semibold text-ink' : 'border-line text-ink-2 hover:bg-surface-2'}`}
						onclick={() => onchange(option.value)}>{option.label}</button
					>
				{/each}
			</div>
		</div>
	{/if}
</div>
