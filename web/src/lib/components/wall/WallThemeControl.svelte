<script lang="ts">
	/**
	 * "Theme" on the wall: a quiet button next to Music that opens a small
	 * panel choosing Auto / Day / Night / OLED for this wall. Remembered per
	 * browser, like the Spotify-speaker switch; a `?theme=` on a shared link
	 * overrides it for that display without touching what is saved.
	 *
	 * OLED trades the day/night palette for true black with dimmed text, so a
	 * display that never turns off does not burn its panel: the wall itself
	 * (not this panel) nudges the whole layout a few pixels every few minutes
	 * and can dim further at night.
	 */
	import { tick } from 'svelte';
	import { Sun, Moon, SunMoon, Contrast } from 'lucide-svelte';
	import type { Icon as LucideIcon } from 'lucide-svelte';
	import { Button, Toggle } from '#lib/ui/index.js';
	import type { WallThemeChoice } from '#lib/components/wall/wallTheme.js';

	interface Props {
		value: WallThemeChoice;
		/** The choice a `?theme=` link is forcing right now, if any. */
		forced: WallThemeChoice | null;
		nightDim: boolean;
		open: boolean;
		onchange: (value: WallThemeChoice) => void;
		onnightdim: (value: boolean) => void;
	}

	let { value, forced, nightDim, open = $bindable(), onchange, onnightdim }: Props = $props();

	let root = $state<HTMLDivElement | null>(null);

	const OPTIONS: { value: WallThemeChoice; label: string; hint: string; icon: typeof LucideIcon }[] = [
		{ value: 'auto', label: 'Auto', hint: 'Follows the rest of DumbMonit.', icon: SunMoon },
		{ value: 'light', label: 'Day', hint: 'Chart paper, navy ink.', icon: Sun },
		{ value: 'dark', label: 'Night', hint: 'Radar composite, cyan signal.', icon: Moon },
		{ value: 'oled', label: 'OLED', hint: 'True black, dimmed text, burn-in care.', icon: Contrast }
	];

	const effective = $derived(forced ?? value);
	const Icon = $derived(OPTIONS.find((o) => o.value === effective)?.icon ?? SunMoon);

	async function toggle() {
		open = !open;
		if (open) await tick();
	}

	// A click anywhere else closes the panel (Escape is handled by the wall).
	$effect(() => {
		if (!open) return;
		const onPointer = (event: PointerEvent) => {
			if (root && !root.contains(event.target as Node)) open = false;
		};
		document.addEventListener('pointerdown', onPointer);
		return () => document.removeEventListener('pointerdown', onPointer);
	});

	function pick(next: WallThemeChoice) {
		onchange(next);
	}

	function onkeydown(event: KeyboardEvent, index: number) {
		const delta =
			event.key === 'ArrowRight' || event.key === 'ArrowDown'
				? 1
				: event.key === 'ArrowLeft' || event.key === 'ArrowUp'
					? -1
					: 0;
		if (!delta) return;
		event.preventDefault();
		const next = OPTIONS[(index + delta + OPTIONS.length) % OPTIONS.length];
		pick(next.value);
		root?.querySelectorAll<HTMLElement>('[role="radio"]')[OPTIONS.indexOf(next)]?.focus();
	}
</script>

<div bind:this={root}>
	<Button
		variant="ghost"
		size="sm"
		onclick={toggle}
		aria-expanded={open}
		aria-controls="wall-theme-panel"
		class="theme-toggle {open ? 'is-open' : ''}"
	>
		<Icon class="size-4" aria-hidden="true" />
		Theme
	</Button>

	{#if open}
		<div
			id="wall-theme-panel"
			class="absolute top-full right-0 z-20 mt-2 w-[min(calc(100vw-2rem),22rem)] rounded-[var(--radius-card)] border border-line bg-surface p-4 text-left shadow-float"
			role="dialog"
			aria-label="Theme for this wall"
		>
			<p class="text-sm font-semibold text-ink">Theme on this display</p>
			{#if forced}
				<p class="mt-1 text-[0.8125rem] text-ink-2">
					A link set this one for right now; it is not saved here.
				</p>
			{/if}
			<div class="mt-3 grid grid-cols-2 gap-2" role="radiogroup" aria-label="Theme">
				{#each OPTIONS as option, i (option.value)}
					{@const selected = value === option.value}
					{@const OptionIcon = option.icon}
					<button
						type="button"
						role="radio"
						aria-checked={selected}
						tabindex={selected ? 0 : -1}
						class={`flex flex-col gap-1 rounded-[var(--radius-plate)] border p-2.5 text-left transition-colors ${selected ? 'border-signal bg-signal-soft' : 'border-line hover:bg-surface-2'}`}
						onclick={() => pick(option.value)}
						onkeydown={(e) => onkeydown(e, i)}
					>
						<span class="flex items-center gap-1.5 text-[0.8125rem] font-semibold text-ink">
							<OptionIcon class="size-3.5 shrink-0 text-ink-2" aria-hidden="true" />
							{option.label}
						</span>
						<span class="text-[0.75rem] text-ink-2">{option.hint}</span>
					</button>
				{/each}
			</div>

			{#if effective === 'oled'}
				<div class="mt-3 flex items-start justify-between gap-3 border-t border-line pt-3">
					<div class="min-w-0">
						<p class="text-sm font-semibold text-ink">Dim at night</p>
						<p class="mt-0.5 text-[0.75rem] text-ink-2">Darkens further from 22:00 to 07:00.</p>
					</div>
					<Toggle checked={nightDim} onchange={onnightdim} label="Dim this display at night" />
				</div>
			{/if}
		</div>
	{/if}
</div>
