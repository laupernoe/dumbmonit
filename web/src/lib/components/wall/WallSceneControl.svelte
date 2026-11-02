<script lang="ts">
	/**
	 * "Backdrop" and "Theme" on the wall: two quiet buttons beside Exit, each
	 * opening a small panel. Backdrop chooses the world behind the board (one of
	 * the four cities, auto from the time zone, or a plain board); Theme
	 * chooses Night or Day. Both are remembered per browser and are the same
	 * choices Settings sets as the default; a `?city=` or `?theme=` on a shared
	 * link overrides them for that display without touching what is saved.
	 * Escape or a click elsewhere closes a panel (not the wall).
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { Building2, Moon, Sun } from 'lucide-svelte';
	import { Button } from '#lib/ui/index.js';
	import { wallSceneOptions } from '#lib/components/wall/wallOptions.js';
	import type { WallSceneChoice } from '#lib/components/wall/wallScene.js';
	import type { WallTheme } from '#lib/components/wall/wallTheme.js';

	interface Props {
		scene: WallSceneChoice;
		/** True when a `?city=` link is choosing right now. */
		sceneForced: boolean;
		onscene: (value: WallSceneChoice) => void;
		theme: WallTheme;
		/** True when a `?theme=` link is choosing right now. */
		themeForced: boolean;
		/** False when the city is a night scene: Day has nothing to light. */
		dayAvailable: boolean;
		/** False on a plain board: there is no world to light. */
		showTheme: boolean;
		ontheme: (value: WallTheme) => void;
	}

	let { scene, sceneForced, onscene, theme, themeForced, dayAvailable, showTheme, ontheme }: Props = $props();

	let open = $state<'scene' | 'theme' | null>(null);
	let root = $state<HTMLDivElement | null>(null);

	const SCENE_OPTIONS = $derived(wallSceneOptions());
	const THEME_OPTIONS = $derived<{ value: WallTheme; label: string; icon: typeof Moon }[]>([
		{ value: 'dark', label: m.wall_theme_night(), icon: Moon },
		{ value: 'light', label: m.wall_theme_day(), icon: Sun }
	]);
	const ThemeIcon = $derived(theme === 'light' && dayAvailable ? Sun : Moon);

	$effect(() => {
		if (!open) return;
		const onPointer = (event: PointerEvent) => {
			if (root && !root.contains(event.target as Node)) open = null;
		};
		const onKey = (event: KeyboardEvent) => {
			if (event.key === 'Escape') {
				event.stopPropagation();
				open = null;
			}
		};
		document.addEventListener('pointerdown', onPointer);
		window.addEventListener('keydown', onKey, true);
		return () => {
			document.removeEventListener('pointerdown', onPointer);
			window.removeEventListener('keydown', onKey, true);
		};
	});

	function toggle(panel: 'scene' | 'theme') {
		open = open === panel ? null : panel;
	}

	/** Arrows move the choice, as in any radio group. */
	function arrow(event: KeyboardEvent, index: number, count: number, pick: (index: number) => void) {
		const delta = event.key === 'ArrowRight' || event.key === 'ArrowDown' ? 1 : event.key === 'ArrowLeft' || event.key === 'ArrowUp' ? -1 : 0;
		if (!delta) return;
		event.preventDefault();
		const next = (index + delta + count) % count;
		pick(next);
		const group = (event.currentTarget as HTMLElement).parentElement;
		group?.querySelectorAll<HTMLElement>('[role="radio"]')[next]?.focus();
	}

	const CHIP = 'min-h-10 rounded-[var(--radius-plate)] border px-3 py-1 text-[0.8125rem] transition-colors';
	const CHIP_ON = 'border-signal bg-signal-soft font-semibold text-ink';
	const CHIP_OFF = 'border-line text-ink-2 hover:bg-surface-2';
</script>

<div class="relative flex items-start gap-1" bind:this={root}>
	<Button variant="ghost" size="sm" onclick={() => toggle('scene')} aria-expanded={open === 'scene'} aria-controls="wall-scene-panel" class="min-h-10">
		<Building2 class="size-4" aria-hidden="true" />
		{m.wall_scene_button()}
	</Button>
	{#if showTheme}
		<Button variant="ghost" size="sm" onclick={() => toggle('theme')} aria-expanded={open === 'theme'} aria-controls="wall-theme-panel" class="min-h-10">
			<ThemeIcon class="size-4" aria-hidden="true" />
			{m.wall_theme_button()}
		</Button>
	{/if}

	{#if open === 'scene'}
		<div
			id="wall-scene-panel"
			class="absolute top-full right-0 z-20 mt-2 w-[min(calc(100vw-2rem),22rem)] rounded-[var(--radius-card)] border border-line bg-surface p-4 text-left shadow-float"
			role="dialog"
			aria-label={m.wall_scene_world_title()}
		>
			<p class="m-0 text-sm font-semibold text-ink">{m.wall_scene_world_title()}</p>
			<p class="m-0 mt-1 text-[0.8125rem] text-ink-2">{sceneForced ? m.wall_scene_forced() : m.wall_scene_world_hint()}</p>
			<div class="mt-3 flex flex-wrap gap-1.5" role="radiogroup" aria-label={m.wall_scene_world_title()}>
				{#each SCENE_OPTIONS as option, i (option.value)}
					<button
						type="button"
						role="radio"
						aria-checked={scene === option.value}
						tabindex={scene === option.value ? 0 : -1}
						class={`${CHIP} ${scene === option.value ? CHIP_ON : CHIP_OFF}`}
						onclick={() => onscene(option.value)}
						onkeydown={(event) => arrow(event, i, SCENE_OPTIONS.length, (next) => onscene(SCENE_OPTIONS[next].value))}>{option.label}</button
					>
				{/each}
			</div>
		</div>
	{:else if open === 'theme'}
		<div
			id="wall-theme-panel"
			class="absolute top-full right-0 z-20 mt-2 w-[min(calc(100vw-2rem),20rem)] rounded-[var(--radius-card)] border border-line bg-surface p-4 text-left shadow-float"
			role="dialog"
			aria-label={m.wall_theme_title()}
		>
			<p class="m-0 text-sm font-semibold text-ink">{m.wall_theme_title()}</p>
			{#if themeForced}
				<p class="m-0 mt-1 text-[0.8125rem] text-ink-2">{m.wall_theme_forced()}</p>
			{:else if !dayAvailable}
				<p class="m-0 mt-1 text-[0.8125rem] text-ink-2">{m.wall_theme_night_only()}</p>
			{/if}
			<div class="mt-3 grid grid-cols-2 gap-1.5" role="radiogroup" aria-label={m.wall_theme_title()}>
				{#each THEME_OPTIONS as option, i (option.value)}
					{@const selected = option.value === 'dark' ? theme === 'dark' || !dayAvailable : theme === 'light' && dayAvailable}
					{@const OptionIcon = option.icon}
					<button
						type="button"
						role="radio"
						aria-checked={selected}
						tabindex={selected ? 0 : -1}
						disabled={option.value === 'light' && !dayAvailable}
						class={`${CHIP} flex items-center justify-center gap-1.5 disabled:cursor-not-allowed disabled:opacity-50 ${selected ? CHIP_ON : CHIP_OFF}`}
						onclick={() => ontheme(option.value)}
						onkeydown={(event) => arrow(event, i, THEME_OPTIONS.length, (next) => dayAvailable && ontheme(THEME_OPTIONS[next].value))}
					>
						<OptionIcon class="size-4 shrink-0" aria-hidden="true" />
						{option.label}
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>
