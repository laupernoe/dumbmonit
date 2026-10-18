<script lang="ts">
	/**
	 * The market bar of the wall: a still band, in the headline band's style,
	 * with the coins (short symbol, price, change) and the world's currencies
	 * against the base currency side by side. Rises and falls carry an arrow and
	 * a sign, not only a colour. When everything does not fit the width, the
	 * figures split in pages that take turns, cross-fading (no sliding); under
	 * `prefers-reduced-motion` they swap without fading. Nothing to show: no bar.
	 */
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	import type { WallWidgets } from '#lib/api/types.js';
	import { formatPrice, formatRate, marketItems, nextCard, packRow } from '#lib/wall/widgets.js';
	import Trend from './widgets/Trend.svelte';

	interface Props {
		widgets: WallWidgets;
		paused?: boolean;
		/** Where the bar sits, only for its border. */
		edge?: 'top' | 'bottom';
	}

	let { widgets, paused = false, edge = 'bottom' }: Props = $props();

	const STEP_MS = 9000;
	/** Space between two figures, in pixels (1.75rem). */
	const GAP = 28;

	const locale = $derived(getLocale());
	const items = $derived(marketItems(widgets));
	const fxBase = $derived(widgets.fx?.data?.base ?? '');

	let width = $state(0);
	let widths = $state<Record<string, number>>({});
	let labelWidths = $state<Record<string, number>>({});
	const measured = $derived(
		width > 0 && items.length > 0 && items.every((item) => (widths[item.key] ?? 0) > 0) && Object.values(labelWidths).every((w) => w > 0)
	);

	const pages = $derived.by(() => {
		if (!measured) return [items.map((_, i) => i)];
		// A page keeps a little room for the page dots.
		const room = width - 8;
		const all = packRow(items.map((item) => widths[item.key]), items.map((item) => item.group), labelWidths, room, GAP);
		return all.length > 1 ? packRow(items.map((item) => widths[item.key]), items.map((item) => item.group), labelWidths, room - 44, GAP) : all;
	});

	let index = $state(0);
	const active = $derived(pages.length ? index % pages.length : 0);
	$effect(() => {
		const count = pages.length;
		if (paused || count < 2) return;
		const timer = setInterval(() => (index = nextCard(index, count)), STEP_MS);
		return () => clearInterval(timer);
	});

	function label(group: string): string {
		return group === 'crypto' ? m.wall_widgets_crypto() : m.wall_market_vs({ base: fxBase });
	}
</script>

{#snippet figure(i: number)}
	{@const item = items[i]}
	<li class="fig">
		<b class="sym">{item.symbol}</b>
		<span class="val tnum">{item.group === 'fx' ? formatRate(item.price, locale) : formatPrice(item.price, item.currency, locale)}</span>
		<Trend percent={item.change} />
	</li>
{/snippet}

{#if items.length > 0}
	<section class="market market--{edge}" aria-label={m.wall_market_aria()}>
		<div class="room" bind:clientWidth={width}>
			<!-- Every figure at its natural width, unseen: what the pages are cut from. -->
			<div class="probe" aria-hidden="true" inert>
				{#each ['crypto', 'fx'] as group (group)}
					{#if items.some((item) => item.group === group)}
						<span class="tag" bind:clientWidth={labelWidths[group]}>{label(group)}</span>
					{/if}
				{/each}
				{#each items as item, i (item.key)}
					<span class="probe-fig" bind:clientWidth={widths[item.key]}>
						<ul>{@render figure(i)}</ul>
					</span>
				{/each}
			</div>

			<div class="stack" class:stack--ready={measured}>
				{#each pages as page, p (page.join(','))}
					{@const shown = p === active}
					<ul class="page" class:page--shown={shown} aria-hidden={shown ? undefined : 'true'} inert={!shown}>
						{#each page as i, n (items[i].key)}
							{#if n === 0 || items[page[n - 1]].group !== items[i].group}
								<li class="tag">{label(items[i].group)}</li>
							{/if}
							{@render figure(i)}
						{/each}
					</ul>
				{/each}
			</div>
		</div>
		{#if pages.length > 1}
			<ol class="dots" aria-hidden="true">
				{#each pages as page, p (page.join(','))}
					<li class:on={p === active}></li>
				{/each}
			</ol>
		{/if}
	</section>
{/if}

<style>
	.market {
		display: flex;
		align-items: center;
		gap: 1rem;
		height: var(--market-h, 3.25rem);
		padding-inline: 1.5rem;
		background: color-mix(in srgb, var(--c-surface) 94%, transparent);
		color: var(--c-ink);
		font-size: 1.125rem;
		white-space: nowrap;
	}
	.market--bottom {
		border-top: 1px solid var(--c-line-strong);
	}
	.market--top {
		border-block: 1px solid var(--c-line-strong);
	}
	.room {
		position: relative;
		flex: 1 1 0;
		min-width: 0;
		height: 100%;
		overflow: hidden;
	}
	.probe {
		position: absolute;
		left: 0;
		top: 0;
		display: flex;
		width: max-content;
		visibility: hidden;
		pointer-events: none;
	}
	.probe-fig {
		display: inline-flex;
	}
	.probe ul {
		display: flex;
	}
	.stack {
		display: grid;
		height: 100%;
		opacity: 0;
	}
	.stack--ready {
		opacity: 1;
	}
	.page {
		grid-area: 1 / 1;
		display: flex;
		align-items: center;
		gap: 1.75rem;
		min-width: 0;
		opacity: 0;
		visibility: hidden;
		transition:
			opacity 700ms ease,
			visibility 0s linear 700ms;
	}
	.page--shown {
		opacity: 1;
		visibility: visible;
		transition:
			opacity 700ms ease,
			visibility 0s;
	}
	.fig {
		display: inline-flex;
		align-items: baseline;
		gap: 0.5rem;
	}
	.sym {
		font-weight: 750;
	}
	.val {
		font-weight: 600;
	}
	.fig :global(.trend) {
		font-size: 0.8em;
	}
	/* A group's name, quiet and uppercase, with a short rule that is not a colour. */
	.tag {
		font-size: 0.72em;
		font-weight: 700;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--c-ink-2);
		padding-right: 1.25rem;
		border-right: 1px solid var(--c-line-strong);
	}
	.dots {
		display: flex;
		gap: 0.375rem;
		flex-shrink: 0;
	}
	.dots li {
		width: 0.375rem;
		height: 0.375rem;
		border-radius: 999px;
		background: var(--c-line-strong);
		transition: width 400ms ease;
	}
	.dots li.on {
		width: 1.1rem;
		background: var(--c-ink-2);
	}
	@media (max-width: 1023px) {
		.market {
			padding-inline: 1rem;
			font-size: 1rem;
		}
	}
	@media (min-width: 2400px) {
		/* The board and bars are zoomed there already. */
		.market {
			font-size: 1.125rem;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.page,
		.page--shown,
		.dots li {
			transition: none;
		}
	}
</style>
