<script lang="ts">
	/**
	 * The widgets deck of the wall: one card at a time, each held for the
	 * seconds chosen in Settings, then the next fades in (no cascade, nothing
	 * slides). Coins, stocks, exchange rates and critical vulnerabilities; long
	 * lists split into pages. (The weather is not a card: it sits under the
	 * clock, `WallWeather`.) All cards sit in
	 * the same grid cell, so the deck keeps the height of its tallest card and
	 * nothing around it jumps.
	 *
	 * Each card names its source, discreetly, and how fresh it is. A source that
	 * fails keeps its last figures for a while, marked as not updated; with
	 * nothing to show the card says "Unavailable" — never a broken wall.
	 * Rises and falls carry an arrow and a sign, not only a colour.
	 * Under `prefers-reduced-motion` the cards swap without fading.
	 */
	import { Bitcoin, ChartCandlestick, ArrowLeftRight, ShieldAlert } from 'lucide-svelte';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	import { formatRelative } from '#lib/format.js';
	import type { WallWidgets, WallSection } from '#lib/api/types.js';
	import {
		formatNumber,
		formatPrice,
		fxQuote,
		nextCard,
		widgetCards,
		type WidgetCard
	} from '#lib/wall/widgets.js';
	import Trend from './widgets/Trend.svelte';

	interface Props {
		widgets: WallWidgets;
		/** Re-read every second by the wall, so ages stay current. */
		now: Date;
		paused?: boolean;
	}

	let { widgets, now, paused = false }: Props = $props();

	const locale = $derived(getLocale());
	const cards = $derived(widgetCards(widgets));
	let index = $state(0);
	const active = $derived(cards.length ? Math.min(index, cards.length - 1) : 0);

	$effect(() => {
		const count = cards.length;
		const ms = Math.max(5, widgets.rotate_seconds) * 1000;
		if (paused || count < 2) return;
		const timer = setInterval(() => (index = nextCard(index, count)), ms);
		return () => clearInterval(timer);
	});

	function sectionOf(card: WidgetCard): WallSection<unknown> | null {
		return (widgets[card.kind] as WallSection<unknown> | null) ?? null;
	}

	function title(card: WidgetCard): string {
		switch (card.kind) {
			case 'crypto':
				return m.wall_widgets_crypto();
			case 'stocks':
				return m.wall_widgets_stocks();
			case 'fx':
				return m.wall_widgets_fx();
			case 'cve':
				return m.wall_widgets_cve();
		}
	}

	function age(updated: string | null): string {
		void now;
		return updated ? formatRelative(updated) : '';
	}

	function shortDate(utc: string): string {
		const date = new Date(`${utc.slice(0, 19)}Z`);
		if (Number.isNaN(date.getTime())) return utc.slice(0, 10);
		return date.toLocaleDateString(locale, { day: 'numeric', month: 'short', timeZone: 'UTC' });
	}
</script>

{#if cards.length > 0}
	<section class="deck" aria-label={m.wall_widgets_aria()} aria-roledescription={m.wall_widgets_carousel()}>
		<div class="stack">
			{#each cards as card, i (card.key)}
				{@const section = sectionOf(card)}
				{@const shown = i === active}
				<article
					class="card"
					class:card--shown={shown}
					aria-hidden={shown ? undefined : 'true'}
					inert={!shown}
				>
					<header class="head">
						<span class="eyebrow">
							{#if card.kind === 'crypto'}<Bitcoin class="size-[1em]" aria-hidden="true" />
							{:else if card.kind === 'stocks'}<ChartCandlestick class="size-[1em]" aria-hidden="true" />
							{:else if card.kind === 'fx'}<ArrowLeftRight class="size-[1em]" aria-hidden="true" />
							{:else}<ShieldAlert class="size-[1em]" aria-hidden="true" />{/if}
							{title(card)}
						</span>
						{#if card.pages > 1}
							<span class="page tnum">{m.wall_widgets_page({ page: card.page + 1, pages: card.pages })}</span>
						{/if}
					</header>

					{#if !section || (!section.data && section.needs_key)}
						<p class="unavailable">{m.wall_widgets_needs_key()}</p>
					{:else if !section.data}
						<p class="unavailable">{m.wall_widgets_unavailable()}</p>
					{:else if card.kind === 'crypto' && widgets.crypto?.data}
						{@const data = widgets.crypto.data}
						<ul class="rows">
							{#each data.coins.slice(card.from, card.to) as coin (coin.id)}
								<li class="row">
									<span class="name"><b>{coin.symbol}</b> <span class="sub">{coin.name}</span></span>
									<span class="price tnum">{formatPrice(coin.price, data.currency, locale)}</span>
									<Trend percent={coin.change_24h} />
								</li>
							{/each}
						</ul>
						<p class="note">{m.wall_widgets_crypto_note()}</p>
					{:else if card.kind === 'stocks' && widgets.stocks?.data}
						{@const data = widgets.stocks.data}
						{#if data.quotes.length === 0}
							<p class="unavailable">{m.wall_widgets_unavailable()}</p>
						{:else}
							<ul class="rows">
								{#each data.quotes.slice(card.from, card.to) as quote (quote.symbol)}
									<li class="row">
										<span class="name"><b>{quote.symbol}</b></span>
										<span class="price tnum">{formatNumber(quote.price, locale)}</span>
										<Trend percent={quote.change_pct} />
									</li>
								{/each}
							</ul>
						{/if}
						{#if data.unknown.length > 0 && card.page === card.pages - 1}
							<p class="note">{m.wall_widgets_stocks_unknown({ symbols: data.unknown.join(', ') })}</p>
						{/if}
					{:else if card.kind === 'fx' && widgets.fx?.data}
						{@const data = widgets.fx.data}
						<ul class="rows">
							{#each data.rates.slice(card.from, card.to) as rate (rate.code)}
								{@const quote = fxQuote(rate.rate, rate.previous)}
								<li class="row">
									<span class="name tnum"><b>{quote.per} {rate.code}</b></span>
									<span class="price tnum">{formatPrice(quote.value, data.base, locale)}</span>
									<Trend percent={quote.change} />
								</li>
							{/each}
						</ul>
						<p class="note">{m.wall_widgets_fx_note({ base: data.base, date: shortDate(`${data.date}T12:00:00`) })}</p>
					{:else if card.kind === 'cve' && widgets.cve?.data}
						{@const items = widgets.cve.data}
						{#if items.length === 0}
							<p class="unavailable">{m.wall_widgets_cve_none()}</p>
						{:else}
							<ul class="cves">
								{#each items.slice(card.from, card.to) as cve (cve.id)}
									<li>
										<p class="cve-head">
											<span class="score tnum" class:score--none={cve.score === null}>
												{cve.score === null ? m.wall_widgets_cve_unscored() : cve.score.toFixed(1)}
											</span>
											<b class="tnum">{cve.id}</b>
											{#if cve.exploited}<span class="tag">{m.wall_widgets_cve_exploited()}</span>{/if}
											{#if cve.ransomware}<span class="tag">{m.wall_widgets_cve_ransomware()}</span>{/if}
											<span class="when tnum">{shortDate(cve.date)}</span>
										</p>
										<p class="cve-title">{cve.title}</p>
									</li>
								{/each}
							</ul>
						{/if}
					{/if}

					{#if section}
						<footer class="foot">
							<span>{m.wall_widgets_source({ source: section.source })}</span>
							{#if section.updated}
								<span class="tnum" class:stale={!!section.error}>
									{section.error ? m.wall_widgets_stale({ when: age(section.updated) }) : age(section.updated)}
								</span>
							{/if}
						</footer>
					{/if}
				</article>
			{/each}
		</div>
		{#if cards.length > 1}
			<ol class="dots" aria-hidden="true">
				{#each cards as card, i (card.key)}
					<li class:on={i === active}></li>
				{/each}
			</ol>
		{/if}
	</section>
{/if}

<style>
	.deck {
		width: 100%;
		max-width: 30rem;
		text-align: left;
	}
	.stack {
		display: grid;
	}
	.card {
		grid-area: 1 / 1;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 1rem 1.25rem 0.875rem;
		border-radius: var(--radius-card);
		border: 1px solid var(--c-line);
		background: color-mix(in srgb, var(--c-surface) 92%, transparent);
		box-shadow: var(--shadow-float);
		color: var(--c-ink);
		opacity: 0;
		visibility: hidden;
		transition:
			opacity 700ms ease,
			visibility 0s linear 700ms;
	}
	.card--shown {
		opacity: 1;
		visibility: visible;
		transition:
			opacity 700ms ease,
			visibility 0s;
	}
	.head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 0.75rem;
	}
	.eyebrow {
		display: inline-flex;
		align-items: center;
		gap: 0.45em;
		font-size: 0.875rem;
		font-weight: 700;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--c-ink-2);
	}
	.page {
		font-size: 0.8125rem;
		color: var(--c-ink-2);
	}
	.unavailable {
		font-size: 1.125rem;
		font-weight: 600;
		color: var(--c-ink-2);
		padding-block: 0.75rem;
	}
	.rows {
		display: flex;
		flex-direction: column;
	}
	.row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto 6.5rem;
		align-items: baseline;
		gap: 0.75rem;
		padding-block: 0.4rem;
		border-top: 1px solid var(--c-line);
		font-size: 1.25rem;
	}
	.row:first-child {
		border-top: 0;
	}
	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.name b {
		font-weight: 750;
	}
	.sub {
		font-size: 0.75em;
		color: var(--c-ink-2);
	}
	.price {
		font-weight: 650;
		text-align: right;
	}
	.row :global(.trend) {
		font-size: 0.875em;
	}
	.note {
		font-size: 0.8125rem;
		color: var(--c-ink-2);
	}
	.foot {
		margin-top: auto;
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 0.25rem 1rem;
		font-size: 0.75rem;
		color: var(--c-ink-3, var(--c-ink-2));
	}
	.stale {
		color: var(--c-advisory-ink);
		font-weight: 600;
	}

	/* Vulnerabilities */
	.cves {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}
	.cves li + li {
		padding-top: 0.625rem;
		border-top: 1px solid var(--c-line);
	}
	.cve-head {
		display: flex;
		align-items: baseline;
		flex-wrap: wrap;
		gap: 0.25rem 0.5rem;
		font-size: 1rem;
	}
	.score {
		min-width: 2.75rem;
		padding: 0.05rem 0.4rem;
		border-radius: 0.375rem;
		border: 1.5px solid var(--c-warning);
		color: var(--c-warning-ink);
		font-weight: 800;
		text-align: center;
	}
	.score--none {
		border-color: var(--c-line-strong);
		color: var(--c-ink-2);
		font-weight: 600;
		font-size: 0.8125rem;
	}
	.tag {
		padding: 0 0.4rem;
		border-radius: 0.375rem;
		background: var(--c-warning-soft);
		color: var(--c-warning-ink);
		font-size: 0.75rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.when {
		margin-left: auto;
		font-size: 0.8125rem;
		color: var(--c-ink-2);
	}
	.cve-title {
		font-size: 0.9375rem;
		line-height: 1.3;
		color: var(--c-ink);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	/* Where the deck is in its round. */
	/* On a pill of its own: legible over any scene. */
	.dots {
		display: flex;
		justify-content: center;
		gap: 0.375rem;
		width: fit-content;
		margin: 0.5rem auto 0;
		padding: 0.375rem 0.625rem;
		border-radius: 999px;
		background: color-mix(in srgb, var(--c-surface) 85%, transparent);
	}
	.dots li {
		width: 0.375rem;
		height: 0.375rem;
		border-radius: 999px;
		background: var(--c-line-strong);
		transition: width 400ms ease;
	}
	.dots li.on {
		width: 1.25rem;
		background: var(--c-ink-2);
	}

	/* The wall proper, read from across the room. */
	@media (min-width: 1024px) {
		.row {
			font-size: 1.5rem;
			grid-template-columns: minmax(0, 1fr) auto 7.5rem;
		}
		.cve-head {
			font-size: 1.125rem;
		}
		.cve-title {
			font-size: 1.0625rem;
		}
	}
	@media (min-width: 1024px) and (max-width: 1599px) {
		.row {
			font-size: 1.25rem;
			grid-template-columns: minmax(0, 1fr) auto 6.5rem;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.card,
		.card--shown,
		.dots li {
			transition: none;
		}
	}
</style>
