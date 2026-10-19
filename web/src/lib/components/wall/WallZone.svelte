<script lang="ts">
	/**
	 * A zone of the wall (top bar, bottom bar or right column) and the widgets
	 * the admin put there, in their order (Settings → Wall widgets → Layout).
	 * Every widget shows at once. When they do not fit the zone's room, the
	 * zone asks for less, step by step, measured after each render: one entry
	 * fewer for the widget that shows the most (`trimCaps`) down to two each,
	 * then the short forms, then down to one entry each, and only then do the
	 * last widgets give way. Nothing turns, nothing scrolls, nothing is clipped.
	 *
	 * `fit` is off on phones and tablets: the page scrolls there, so the zone
	 * just wraps.
	 */
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import type { WallWidgets } from '#lib/api/types.js';
	import type { NewsItem } from '#lib/api/wallNews.js';
	import { trimCaps, type WallWidgetId } from '#lib/wall/layout.js';
	import WallWidget from './WallWidget.svelte';

	interface Props {
		ids: readonly WallWidgetId[];
		variant: 'bar' | 'column';
		widgets: WallWidgets | null;
		news: NewsItem[];
		now: Date;
		fit?: boolean;
		class?: string;
	}

	let { ids, variant, widgets, news, now, fit = true, class: className = '' }: Props = $props();

	/** Entries a widget starts with, before fitting: a wall is glanced at, not read through. */
	const START: Partial<Record<WallWidgetId, number>> = { news: variant === 'bar' ? 3 : 5 };

	let compact = $state(false);
	let caps = $state<Record<string, number>>({});
	/** Widgets left out at the end of the zone, as a last resort. */
	let dropped = $state(0);
	let box = $state<HTMLElement | null>(null);
	let width = $state(0);
	let height = $state(0);

	const shownIds = $derived(dropped > 0 ? ids.slice(0, Math.max(0, ids.length - dropped)) : ids);

	/** What the zone is made of; values (prices) changing do not start the fit over. */
	const shape = $derived(
		[
			ids.join(','),
			news.length,
			widgets?.crypto?.data?.coins.length,
			widgets?.stocks?.data?.quotes.length,
			widgets?.fx?.data?.rates.length,
			widgets?.cve?.data?.length,
			widgets?.claude?.data?.people?.length,
			widgets?.openai?.data?.people?.length,
			[widgets?.weather, widgets?.crypto, widgets?.stocks, widgets?.fx, widgets?.cve, widgets?.claude, widgets?.openai]
				.map((s) => (s ? (s.data ? 'd' : 'e') : '-'))
				.join(''),
			Math.round(width),
			// A bar's height follows its content: only a column's room counts.
			variant === 'column' ? Math.round(height) : 0
		].join('|')
	);
	let lastShape = '';

	$effect(() => {
		const current = shape;
		void widgets;
		void compact;
		void caps;
		void dropped;
		untrack(() => {
			if (current !== lastShape) {
				lastShape = current;
				compact = false;
				caps = { ...START };
				dropped = 0;
			}
		});
		if (!fit) return;
		const frame = requestAnimationFrame(() => {
			const el = box;
			if (!el) return;
			const over = variant === 'bar' ? el.scrollWidth > el.clientWidth + 1 : el.scrollHeight > el.clientHeight + 1;
			if (!over) return;
			const items = Array.from(el.querySelectorAll<HTMLElement>('[data-widget]'));
			const shown = items.map((item) => item.querySelectorAll('[data-entry]:not(.more)').length);
			const next = trimCaps(shown, 2) ?? (compact ? trimCaps(shown, 1) : null);
			if (next) {
				const updated = { ...caps };
				items.forEach((item, i) => (updated[item.dataset.widget!] = next[i]));
				caps = updated;
			} else if (!compact) {
				compact = true;
			} else if (dropped < ids.length) {
				dropped += 1;
			}
		});
		return () => cancelAnimationFrame(frame);
	});
</script>

{#if ids.length > 0}
	<div
		class="zone zone--{variant} {className}"
		class:zone--fit={fit}
		aria-label={m.wall_widgets_aria()}
		bind:this={box}
		bind:clientWidth={width}
		bind:clientHeight={height}
	>
		{#each shownIds as id (id)}
			<WallWidget {id} {variant} {widgets} {news} {now} {compact} cap={caps[id] ?? Infinity} />
		{/each}
	</div>
{/if}

<style>
	.zone {
		min-width: 0;
		min-height: 0;
	}
	.zone--bar {
		display: flex;
		align-items: center;
		gap: 0.75rem 2rem;
		flex-wrap: wrap;
	}
	/* On the wall proper: one line, the widgets separated by a thin rule. */
	.zone--bar.zone--fit {
		flex-wrap: nowrap;
		overflow: hidden;
	}
	.zone--bar.zone--fit > :global(* + *) {
		border-left: 1px solid var(--c-line);
		padding-left: 2rem;
	}
	.zone--column {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 0.625rem;
	}
	.zone--column.zone--fit {
		overflow: hidden;
	}
</style>
