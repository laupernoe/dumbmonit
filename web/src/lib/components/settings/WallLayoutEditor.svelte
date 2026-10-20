<script lang="ts">
	/**
	 * Where each widget goes on the wall, shown on a scale model of the screen
	 * (16:9): the top bar, the right column under the clock, the bottom bar,
	 * and a tray for hidden widgets. What is dropped where is what `/wall`
	 * shows, in that order.
	 *
	 * Mouse, pen and touch share one drag (pointer events, no library): press
	 * a widget, move a few pixels, drop it on a zone; a marker shows the place
	 * it will take. Without a pointer: Enter or Space selects a widget, the
	 * arrow keys (or the buttons) move it within its zone, and the "move to"
	 * buttons change its zone. On a phone the model is too small to drag on:
	 * the zones become an ordered list (`WallLayoutList`).
	 *
	 * Placing a widget does not turn it on; one that is off, or waits for its
	 * API key (it stays off the wall until then), says so on its chip.
	 */
	import { tick } from 'svelte';
	import { ArrowLeft, ArrowRight, RotateCcw } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { Button } from '#lib/ui/index.js';
	import {
		DEFAULT_LAYOUT,
		dropIndex,
		moveToZone,
		placeAt,
		sameLayout,
		shift,
		WALL_ZONES,
		zoneOf,
		type WallLayout,
		type WallWidgetId,
		type WallZone
	} from '#lib/wall/layout.js';
	import { WIDGET_NAMES, ZONE_NAMES } from '#lib/wall/names.js';
	import WallLayoutList from './WallLayoutList.svelte';

	interface Props {
		layout: WallLayout;
		/** Each widget's state: off, or on but waiting for its API key; unknown shows nothing. */
		states?: Partial<Record<WallWidgetId, 'on' | 'off' | 'key'>>;
		disabled?: boolean;
	}

	let { layout = $bindable(), states = {}, disabled = false }: Props = $props();

	/** Movement (px) before a press becomes a drag. */
	const THRESHOLD = 5;
	/** Zones in the order of the "move to" buttons. */
	const TARGETS: readonly WallZone[] = ['top', 'right', 'bottom', 'hidden'];

	let selected = $state<WallWidgetId | null>(null);
	let live = $state('');
	let drag = $state<{ id: WallWidgetId; x: number; y: number; over: WallZone | null; index: number } | null>(null);
	/** A drag just ended: the click that follows it must not select. */
	let swallowClick = false;
	let stop: (() => void) | null = null;

	const isDefault = $derived(sameLayout(layout, DEFAULT_LAYOUT));
	const selectedZone = $derived(selected ? zoneOf(layout, selected) : null);
	const selectedIndex = $derived(selected && selectedZone ? layout[selectedZone].indexOf(selected) : -1);

	$effect(() => () => stop?.());

	function announce(id: WallWidgetId) {
		const zone = zoneOf(layout, id);
		live =
			zone === 'hidden'
				? m.settings_widgets_layout_moved_hidden({ name: WIDGET_NAMES[id]() })
				: m.settings_widgets_layout_moved({
						name: WIDGET_NAMES[id](),
						zone: ZONE_NAMES[zone](),
						position: layout[zone].indexOf(id) + 1,
						count: layout[zone].length
					});
	}

	async function keepFocus(id: WallWidgetId) {
		await tick();
		document.getElementById(`chip-${id}`)?.focus();
	}

	function apply(next: WallLayout, id: WallWidgetId) {
		layout = next;
		announce(id);
		void keepFocus(id);
	}

	/** The zone under the pointer and the place the widget would take in it. */
	function hit(x: number, y: number, id: WallWidgetId): { over: WallZone | null; index: number } {
		const zoneEl = document.elementsFromPoint(x, y).find((el) => el instanceof HTMLElement && el.dataset.dropZone) as HTMLElement | undefined;
		if (!zoneEl) return { over: null, index: 0 };
		const over = zoneEl.dataset.dropZone as WallZone;
		const vertical = over === 'right';
		const centers = Array.from(zoneEl.querySelectorAll<HTMLElement>('[data-chip]'))
			.filter((chip) => chip.dataset.chip !== id)
			.map((chip) => {
				const box = chip.getBoundingClientRect();
				return vertical ? box.top + box.height / 2 : box.left + box.width / 2;
			});
		return { over, index: dropIndex(centers, vertical ? y : x) };
	}

	function press(event: PointerEvent, id: WallWidgetId) {
		if (disabled || event.button !== 0) return;
		const startX = event.clientX;
		const startY = event.clientY;
		let moving = false;

		const move = (e: PointerEvent) => {
			if (!moving && Math.hypot(e.clientX - startX, e.clientY - startY) < THRESHOLD) return;
			moving = true;
			drag = { id, x: e.clientX, y: e.clientY, ...hit(e.clientX, e.clientY, id) };
		};
		const end = (e: PointerEvent) => {
			cleanup();
			if (!moving) return;
			swallowClick = true;
			setTimeout(() => (swallowClick = false), 0);
			const target = hit(e.clientX, e.clientY, id);
			drag = null;
			if (e.type === 'pointerup' && target.over) {
				selected = id;
				apply(placeAt(layout, id, target.over, target.index), id);
			}
		};
		const cleanup = () => {
			window.removeEventListener('pointermove', move);
			window.removeEventListener('pointerup', end);
			window.removeEventListener('pointercancel', end);
			stop = null;
		};
		window.addEventListener('pointermove', move);
		window.addEventListener('pointerup', end);
		window.addEventListener('pointercancel', end);
		stop = cleanup;
	}

	function pick(id: WallWidgetId) {
		if (swallowClick) return;
		selected = selected === id ? null : id;
	}

	function key(event: KeyboardEvent, id: WallWidgetId) {
		if (disabled) return;
		if (event.key === 'Escape') {
			selected = null;
			return;
		}
		if (selected !== id) return;
		const delta = event.key === 'ArrowLeft' || event.key === 'ArrowUp' ? -1 : event.key === 'ArrowRight' || event.key === 'ArrowDown' ? 1 : 0;
		if (!delta) return;
		event.preventDefault();
		apply(shift(layout, id, delta), id);
	}

	function reset() {
		layout = structuredClone(DEFAULT_LAYOUT);
		selected = null;
		live = m.settings_widgets_layout_reset_done();
	}

	/** Whether the drop marker sits just before this chip (or at the zone's end when `id` is null). */
	function markerBefore(zone: WallZone, id: WallWidgetId | null): boolean {
		if (!drag || drag.over !== zone) return false;
		const others = layout[zone].filter((other) => other !== drag!.id);
		return (others[drag.index] ?? null) === id;
	}
</script>

{#snippet chip(id: WallWidgetId, zone: WallZone)}
	{@const state = states[id]}
	<button
		id="chip-{id}"
		type="button"
		data-chip={id}
		class="chip"
		class:chip--selected={selected === id}
		class:chip--lifted={drag?.id === id}
		aria-pressed={selected === id}
		aria-label={WIDGET_NAMES[id]()}
		aria-describedby="layout-help"
		{disabled}
		onpointerdown={(event) => press(event, id)}
		onclick={() => pick(id)}
		onkeydown={(event) => key(event, id)}
	>
		<span class="truncate">{WIDGET_NAMES[id]()}</span>
		{#if state === 'off'}
			<span class="chip-tag">{m.settings_widgets_layout_off()}</span>
		{:else if state === 'key'}
			<span class="chip-tag chip-tag--warn">{m.settings_widgets_layout_key()}</span>
		{/if}
	</button>
{/snippet}

{#snippet dropzone(zone: WallZone, cls: string)}
	<div class="zone {cls}" class:zone--over={drag?.over === zone} data-drop-zone={zone} role="group" aria-label={ZONE_NAMES[zone]()}>
		<span class="zone-label">{ZONE_NAMES[zone]()}</span>
		<div class="zone-chips" class:zone-chips--column={zone === 'right'}>
			{#each layout[zone] as id (id)}
				{#if markerBefore(zone, id)}<span class="marker" aria-hidden="true"></span>{/if}
				{@render chip(id, zone)}
			{/each}
			{#if markerBefore(zone, null)}<span class="marker" aria-hidden="true"></span>{/if}
			{#if layout[zone].length === 0 && drag?.over !== zone}
				<span class="zone-empty">{m.settings_widgets_zone_empty()}</span>
			{/if}
		</div>
	</div>
{/snippet}

<div class="space-y-3">
	<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
		<p id="layout-help" class="min-w-0 flex-1 basis-64 text-[0.8125rem] text-ink-2">
			<span class="hidden md:inline">{m.settings_widgets_layout_help()}</span>
			<span class="md:hidden">{m.settings_widgets_layout_help_list()}</span>
		</p>
		<Button variant="secondary" size="sm" class="min-h-10" onclick={reset} disabled={disabled || isDefault}>
			<RotateCcw class="size-4" aria-hidden="true" />
			{m.settings_widgets_layout_reset()}
		</Button>
	</div>

	<div class="md:hidden">
		<WallLayoutList bind:layout {states} {disabled} />
	</div>

	<div class="hidden md:block">
		<div class="screen" role="group" aria-label={m.settings_widgets_layout_screen()}>
			{@render dropzone('top', 'screen-top')}
			<div class="screen-left" aria-hidden="true">
				<span class="zone-label">{m.settings_widgets_layout_monitors()}</span>
				<span class="ghost ghost--title"></span>
				<span class="ghost ghost--line"></span>
				<span class="ghost ghost--line ghost--short"></span>
				<span class="ghost ghost--block"></span>
			</div>
			<div class="screen-right">
				<div class="screen-clock" aria-hidden="true">
					<span class="tnum">12:34</span>
					<span class="ghost ghost--line ghost--short"></span>
				</div>
				{@render dropzone('right', 'screen-column')}
			</div>
			{@render dropzone('bottom', 'screen-bottom')}
		</div>

		<div class="mt-3">
			{@render dropzone('hidden', 'tray')}
		</div>

		<div class="mt-3 min-h-10 rounded-lg border border-line bg-surface-2/50 p-2" aria-live="off">
			{#if selected && selectedZone}
				{@const name = WIDGET_NAMES[selected]()}
				<div class="flex flex-wrap items-center gap-2">
					<span class="text-sm font-medium text-ink">{m.settings_widgets_layout_selected({ name })}</span>
					<Button
						variant="secondary"
						size="sm"
						class="min-h-10"
						aria-label={m.settings_widgets_layout_earlier({ name })}
						disabled={disabled || selectedIndex <= 0}
						onclick={() => selected && apply(shift(layout, selected, -1), selected)}
					>
						<ArrowLeft class="size-4" aria-hidden="true" />
					</Button>
					<Button
						variant="secondary"
						size="sm"
						class="min-h-10"
						aria-label={m.settings_widgets_layout_later({ name })}
						disabled={disabled || selectedIndex >= layout[selectedZone].length - 1}
						onclick={() => selected && apply(shift(layout, selected, 1), selected)}
					>
						<ArrowRight class="size-4" aria-hidden="true" />
					</Button>
					<span class="ml-2 text-sm text-ink-2">{m.settings_widgets_layout_move_to()}</span>
					{#each TARGETS as target (target)}
						<Button
							variant="secondary"
							size="sm"
							class="min-h-10"
							aria-pressed={target === selectedZone}
							disabled={disabled || target === selectedZone}
							onclick={() => selected && apply(moveToZone(layout, selected, target), selected)}
						>
							{ZONE_NAMES[target]()}
						</Button>
					{/each}
				</div>
			{:else}
				<p class="px-1 py-2 text-[0.8125rem] text-ink-3">{m.settings_widgets_layout_none_selected()}</p>
			{/if}
		</div>
	</div>

	<p class="sr-only" role="status" aria-live="polite">{live}</p>
</div>

{#if drag}
	<div class="drag-ghost" style:left="{drag.x}px" style:top="{drag.y}px" aria-hidden="true">{WIDGET_NAMES[drag.id]()}</div>
{/if}

<style>
	/* The scale model: 16:9, growing only if its zones need more room than that. */
	.screen {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 30%;
		grid-template-rows: auto minmax(0, 1fr) auto;
		grid-template-areas:
			'top top'
			'left right'
			'bottom bottom';
		gap: 0.5rem;
		aspect-ratio: 16 / 9;
		padding: 0.5rem;
		border: 2px solid var(--c-line-strong, var(--c-line));
		border-radius: 0.75rem;
		background: var(--c-canvas);
	}
	:global(.screen-top) {
		grid-area: top;
	}
	:global(.screen-bottom) {
		grid-area: bottom;
	}
	.screen-left {
		grid-area: left;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		min-width: 0;
		padding: 0.5rem;
		border-radius: 0.5rem;
		background: var(--c-surface-2);
	}
	.screen-right {
		grid-area: right;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		min-width: 0;
		min-height: 0;
	}
	.screen-clock {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 0.25rem;
		font-size: 1.25rem;
		font-weight: 600;
		color: var(--c-ink-3);
	}
	:global(.screen-column) {
		flex: 1;
	}

	.ghost {
		display: block;
		height: 0.5rem;
		border-radius: 999px;
		background: var(--c-line);
	}
	.ghost--title {
		height: 1rem;
		width: 60%;
	}
	.ghost--line {
		width: 85%;
	}
	.ghost--short {
		width: 40%;
	}
	.ghost--block {
		flex: 1;
		min-height: 1.5rem;
		border-radius: 0.5rem;
	}

	.zone {
		position: relative;
		min-width: 0;
		min-height: 2.75rem;
		padding: 1.25rem 0.375rem 0.375rem;
		border: 1px dashed var(--c-line-strong, var(--c-line));
		border-radius: 0.5rem;
		background: color-mix(in srgb, var(--c-surface) 70%, transparent);
	}
	.zone--over {
		border-style: solid;
		border-color: var(--c-signal);
		background: var(--c-signal-soft);
	}
	.zone-label {
		position: absolute;
		top: 0.1875rem;
		left: 0.5rem;
		font-size: 0.6875rem;
		font-weight: 600;
		letter-spacing: 0.02em;
		text-transform: uppercase;
		color: var(--c-ink-3);
	}
	.screen-left > .zone-label {
		position: static;
	}
	.zone-chips {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.375rem;
		min-height: 2.5rem;
	}
	.zone-chips--column {
		flex-direction: column;
		align-items: stretch;
		flex-wrap: nowrap;
	}
	.zone-empty {
		font-size: 0.75rem;
		color: var(--c-ink-3);
		padding-inline: 0.25rem;
	}

	.chip {
		display: inline-flex;
		min-width: 0;
		min-height: 2.5rem;
		align-items: center;
		gap: 0.375rem;
		padding: 0 0.625rem;
		border: 1px solid var(--c-line-strong, var(--c-line));
		border-radius: 0.5rem;
		background: var(--c-surface);
		color: var(--c-ink);
		font-size: 0.8125rem;
		cursor: grab;
		/* The drag is ours: no scroll or zoom starts from a widget. */
		touch-action: none;
		user-select: none;
	}
	.chip:hover:not(:disabled) {
		border-color: var(--c-ink-3);
	}
	.chip--selected {
		border-color: var(--c-signal);
		background: var(--c-signal-soft);
		color: var(--c-signal-ink);
		box-shadow: 0 0 0 1px var(--c-signal);
	}
	.chip--lifted {
		opacity: 0.4;
	}
	.chip:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.chip-tag {
		flex-shrink: 0;
		font-size: 0.6875rem;
		color: var(--c-ink-3);
	}
	.chip-tag--warn {
		color: var(--c-warning-ink);
	}

	/* Where the dragged widget will land. */
	.marker {
		align-self: stretch;
		flex-shrink: 0;
		width: 3px;
		min-height: 2.5rem;
		border-radius: 2px;
		background: var(--c-signal);
	}
	.zone-chips--column > .marker {
		width: auto;
		min-height: 3px;
		height: 3px;
	}

	.drag-ghost {
		position: fixed;
		z-index: 80;
		pointer-events: none;
		transform: translate(-50%, -120%);
		padding: 0.375rem 0.75rem;
		border: 1px solid var(--c-signal);
		border-radius: 0.5rem;
		background: var(--c-surface);
		color: var(--c-ink);
		font-size: 0.8125rem;
		box-shadow: var(--shadow-lift, 0 4px 12px rgb(0 0 0 / 0.2));
	}
</style>
