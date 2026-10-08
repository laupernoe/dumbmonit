<script lang="ts">
	/**
	 * Settings → Appearance: the theme, as three faceplate tiles with a swatch.
	 * The swatches use literal palette values on purpose: each tile previews
	 * its own theme regardless of the one currently applied.
	 */
	import { Check, Moon, Sun, SunMoon } from 'lucide-svelte';
	import type { Icon as LucideIcon } from 'lucide-svelte';
	import { theme, type ThemePreference } from '#lib/stores/theme.svelte.js';
	import { Button, Panel } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale, locales, setLocale, type Locale } from '#lib/paraglide/runtime.js';

	// A name for each shipped locale, in that locale's own language where we
	// can tell (Intl knows), falling back to English. Only locales with a
	// `messages/{locale}.json` file exist here — today, just English — so the
	// picker below hides itself until a second one lands through Weblate.
	function localeName(locale: string): string {
		try {
			return new Intl.DisplayNames([locale, 'en'], { type: 'language' }).of(locale) ?? locale;
		} catch {
			return locale;
		}
	}

	interface Option {
		value: ThemePreference;
		label: () => string;
		hint: () => string;
		icon: typeof LucideIcon;
	}

	const OPTIONS: Option[] = [
		{ value: 'auto', label: () => m.settings_appearance_system(), hint: () => m.settings_appearance_system_hint(), icon: SunMoon },
		{ value: 'light', label: () => m.settings_appearance_day(), hint: () => m.settings_appearance_day_hint(), icon: Sun },
		{ value: 'dark', label: () => m.settings_appearance_night(), hint: () => m.settings_appearance_night_hint(), icon: Moon }
	];

	// Day and night palette, as in app.css.
	const DAY = { canvas: '#f3efe6', surface: '#fbf9f4', ink: '#16213a', signal: '#0f8f86' };
	const NIGHT = { canvas: '#0a1020', surface: '#111a2e', ink: '#e8eef8', signal: '#22d3c5' };

	function onkeydown(event: KeyboardEvent, index: number) {
		const delta = event.key === 'ArrowRight' || event.key === 'ArrowDown' ? 1 : event.key === 'ArrowLeft' || event.key === 'ArrowUp' ? -1 : 0;
		if (!delta) return;
		event.preventDefault();
		const next = OPTIONS[(index + delta + OPTIONS.length) % OPTIONS.length];
		theme.set(next.value);
		(event.currentTarget as HTMLElement).parentElement?.querySelectorAll<HTMLElement>('[role="radio"]')[OPTIONS.indexOf(next)]?.focus();
	}
</script>

{#snippet swatch(value: ThemePreference)}
	<!-- A miniature page: ground, a surface bar, a signal trace. -->
	<span class="relative block h-12 w-full overflow-hidden rounded-lg border border-line" aria-hidden="true">
		{#if value === 'auto'}
			<span class="absolute inset-0" style:background={`linear-gradient(105deg, ${DAY.canvas} 50%, ${NIGHT.canvas} 50%)`}></span>
			<span class="absolute top-2 left-2 h-2 w-[calc(50%-0.75rem)] rounded-sm" style:background={DAY.surface} style:border={`1px solid ${DAY.ink}22`}></span>
			<span class="absolute top-2 right-2 h-2 w-[calc(50%-0.75rem)] rounded-sm" style:background={NIGHT.surface} style:border={`1px solid ${NIGHT.ink}22`}></span>
			<svg class="absolute bottom-1 left-2 h-5 w-[calc(50%-0.75rem)]" viewBox="0 0 40 16" preserveAspectRatio="none"><path d="M1 13l8-6 7 5 8-9 8 7 7-4" fill="none" stroke={DAY.signal} stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></svg>
			<svg class="absolute right-2 bottom-1 h-5 w-[calc(50%-0.75rem)]" viewBox="0 0 40 16" preserveAspectRatio="none"><path d="M1 13l8-6 7 5 8-9 8 7 7-4" fill="none" stroke={NIGHT.signal} stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></svg>
		{:else}
			{@const p = value === 'light' ? DAY : NIGHT}
			<span class="absolute inset-0" style:background={p.canvas}></span>
			<span class="absolute top-2 right-2 left-2 h-2 rounded-sm" style:background={p.surface} style:border={`1px solid ${p.ink}22`}></span>
			<svg class="absolute right-2 bottom-1 left-2 h-5" viewBox="0 0 80 16" preserveAspectRatio="none"><path d="M1 13l14-6 12 5 14-9 14 7 12-4 12 1" fill="none" stroke={p.signal} stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></svg>
		{/if}
	</span>
{/snippet}

<Panel id="appearance" title={m.settings_appearance_title()} description={m.settings_appearance_description()}>
	<div class="grid gap-2 sm:grid-cols-3" role="radiogroup" aria-label={m.settings_appearance_theme()}>
		{#each OPTIONS as option, i (option.value)}
			{@const selected = theme.preference === option.value}
			{@const Icon = option.icon}
			<button
				type="button"
				role="radio"
				aria-checked={selected}
				tabindex={selected ? 0 : -1}
				class={`faceplate flex flex-col gap-3 p-3 text-left ${selected ? '!border-signal ring-2 ring-signal/30' : ''}`}
				data-interactive
				onclick={() => theme.set(option.value)}
				onkeydown={(e) => onkeydown(e, i)}
			>
				{@render swatch(option.value)}
				<span class="flex items-center gap-2">
					<Icon class="size-4 shrink-0 text-ink-2" aria-hidden="true" />
					<span class="min-w-0 flex-1">
						<span class="block text-sm font-semibold text-ink">{option.label()}</span>
						<span class="block text-[0.8125rem] text-ink-2">{option.hint()}</span>
					</span>
					{#if selected}<Check class="size-4 shrink-0 text-signal-ink" aria-hidden="true" />{/if}
				</span>
			</button>
		{/each}
	</div>

	{#if locales.length > 1}
		<div class="mt-5 flex flex-wrap items-center justify-between gap-3 border-t border-line pt-4">
			<label for="settings-language" class="text-sm font-semibold text-ink">{m["settings.appearance.language_label"]()}</label>
			<select
				id="settings-language"
				class="h-9 rounded-lg border border-line-strong bg-surface px-3 text-sm text-ink"
				value={getLocale()}
				onchange={(e) => setLocale((e.currentTarget as HTMLSelectElement).value as Locale)}
			>
				{#each locales as locale (locale)}
					<option value={locale}>{localeName(locale)}</option>
				{/each}
			</select>
		</div>
	{/if}

	<div class="mt-5 flex flex-wrap items-center justify-between gap-3 border-t border-line pt-4">
		<div class="min-w-0">
			<p class="text-sm font-semibold text-ink">{m.settings_appearance_wall_title()}</p>
			<p class="text-[0.8125rem] text-ink-2">
				{m.settings_appearance_wall_description()}
			</p>
		</div>
		<Button href="/wall" variant="secondary" size="sm">{m.settings_appearance_wall_open()}</Button>
	</div>
</Panel>
