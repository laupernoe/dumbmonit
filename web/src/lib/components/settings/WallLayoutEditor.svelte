<script lang="ts">
	/**
	 * Where each widget goes on the wall: four zones (top bar, bottom bar,
	 * right column, hidden), each listing its widgets in order. A widget
	 * changes zone with its select and moves within it with the up and down
	 * buttons — all reachable with the keyboard, nothing to drag. Zones and an
	 * order, not a free grid: the wall fits each zone to the room it has.
	 *
	 * Placing a widget does not turn it on; one that is off, or waits for its
	 * API key (it stays off the wall until then), says so here.
	 */
	import { ArrowDown, ArrowUp } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { moveToZone, shift, WALL_ZONES, type WallLayout, type WallWidgetId, type WallZone } from '#lib/wall/layout.js';

	interface Props {
		layout: WallLayout;
		/** Each widget's state: off, or on but waiting for its API key; unknown shows nothing. */
		states?: Partial<Record<WallWidgetId, 'on' | 'off' | 'key'>>;
		disabled?: boolean;
	}

	let { layout = $bindable(), states = {}, disabled = false }: Props = $props();

	const NAMES: Record<WallWidgetId, () => string> = {
		weather: m.wall_widgets_weather,
		news: m.wall_widgets_news,
		cve: m.wall_widgets_cve,
		crypto: m.wall_widgets_crypto,
		stocks: m.wall_widgets_stocks,
		fx: m.wall_widgets_fx,
		claude: m.wall_widgets_claude,
		openai: m.wall_widgets_openai
	};
	const ZONES: Record<WallZone, () => string> = {
		top: m.settings_widgets_zone_top,
		bottom: m.settings_widgets_zone_bottom,
		right: m.settings_widgets_zone_right,
		hidden: m.settings_widgets_zone_hidden
	};

	/** After a move, the focus follows the widget's button so the keyboard can keep going. */
	async function move(id: WallWidgetId, delta: -1 | 1) {
		layout = shift(layout, id, delta);
		await Promise.resolve();
		requestAnimationFrame(() => {
			const same = document.getElementById(`layout-${id}-${delta < 0 ? 'up' : 'down'}`) as HTMLButtonElement | null;
			const other = document.getElementById(`layout-${id}-${delta < 0 ? 'down' : 'up'}`) as HTMLButtonElement | null;
			(same && !same.disabled ? same : other)?.focus();
		});
	}
</script>

<div class="grid gap-4 md:grid-cols-2">
	{#each WALL_ZONES as zone (zone)}
		<section class="min-w-0 rounded-xl border border-line bg-surface-2/50 p-3" aria-labelledby="layout-zone-{zone}">
			<h4 id="layout-zone-{zone}" class="text-sm font-semibold text-ink">{ZONES[zone]()}</h4>
			{#if layout[zone].length === 0}
				<p class="mt-2 text-[0.8125rem] text-ink-2">{m.settings_widgets_zone_empty()}</p>
			{:else}
				<ol class="mt-2 space-y-1.5">
					{#each layout[zone] as id, i (id)}
						{@const name = NAMES[id]()}
						<li class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 rounded-lg border border-line bg-surface px-2 py-1">
							<span class="min-w-0 flex-1 basis-32 truncate text-sm text-ink">{name}</span>
							{#if states[id] === 'off'}
								<span class="shrink-0 text-xs text-ink-3">{m.settings_widgets_layout_off()}</span>
							{:else if states[id] === 'key'}
								<span class="shrink-0 text-xs text-warning-ink">{m.settings_widgets_layout_key()}</span>
							{/if}
							<select
								class="input layout-select h-10 py-0 text-sm"
								aria-label={m.settings_widgets_layout_place({ name })}
								value={zone}
								{disabled}
								onchange={(event) => (layout = moveToZone(layout, id, event.currentTarget.value as WallZone))}
							>
								{#each WALL_ZONES as other (other)}
									<option value={other}>{ZONES[other]()}</option>
								{/each}
							</select>
							<button
								id="layout-{id}-up"
								type="button"
								class="grid size-10 shrink-0 place-items-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-ink disabled:opacity-40"
								aria-label={m.settings_widgets_layout_up({ name })}
								disabled={disabled || i === 0}
								onclick={() => void move(id, -1)}
							>
								<ArrowUp class="size-4" aria-hidden="true" />
							</button>
							<button
								id="layout-{id}-down"
								type="button"
								class="grid size-10 shrink-0 place-items-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-ink disabled:opacity-40"
								aria-label={m.settings_widgets_layout_down({ name })}
								disabled={disabled || i === layout[zone].length - 1}
								onclick={() => void move(id, 1)}
							>
								<ArrowDown class="size-4" aria-hidden="true" />
							</button>
						</li>
					{/each}
				</ol>
			{/if}
		</section>
	{/each}
</div>

<style>
	/* The zone select keeps to its words, so the name keeps its room (it wraps under it on a phone). */
	.layout-select {
		width: 10.5rem;
		max-width: 100%;
		flex-shrink: 1;
	}
</style>
