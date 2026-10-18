<script lang="ts">
	/**
	 * The wall's right rail below the clock: the cards of the widgets that are
	 * turned on and have something to show (critical vulnerabilities, Claude and
	 * OpenAI usage, stocks). Never a "key required" card: a widget that is not
	 * ready is simply absent. Coins and exchange rates live in the market bar,
	 * the weather under the clock.
	 *
	 * The room is shared out by measuring. Every card is drawn once, invisibly,
	 * at its natural size; the cards that fit together in the rail's height form
	 * a page. One page: all the cards, grown to fill the room when there is
	 * plenty (a few widgets get bigger cards). Several pages: they take turns,
	 * each for the seconds chosen in Settings, cross-fading (no slide).
	 *
	 * Each card names its source, discreetly, and how fresh it is. Rises and
	 * falls carry an arrow and a sign, not only a colour. In a Claude or OpenAI
	 * card listing several people, each has a round badge with their initials
	 * beside their name. Under `prefers-reduced-motion` pages swap without fading.
	 */
	import { ShieldAlert, ChartCandlestick, Sparkles, Bot } from 'lucide-svelte';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	import { formatRelative } from '#lib/format.js';
	import type { WallAiPeriod, WallAiUsage, WallWidgets, WallSection } from '#lib/api/types.js';
	import {
		fitScale,
		formatNumber,
		formatTokens,
		formatUsd,
		nextCard,
		packColumn,
		widgetCards,
		type WidgetCard
	} from '#lib/wall/widgets.js';
	import Trend from './widgets/Trend.svelte';
	import PersonBadge from './widgets/PersonBadge.svelte';

	interface Props {
		widgets: WallWidgets;
		/** Re-read every second by the wall, so ages stay current. */
		now: Date;
		paused?: boolean;
	}

	let { widgets, now, paused = false }: Props = $props();

	/** Space between two cards, in pixels at scale 1 (0.75rem). */
	const GAP = 12;
	/** Room kept for the page dots when there are several pages. */
	const DOTS = 26;

	const locale = $derived(getLocale());
	const cards = $derived(widgetCards(widgets));

	let room = $state(0);
	let heights = $state<number[]>([]);
	/** Little height to share (a 720p screen): the cards drop their small print. */
	const tight = $derived(room > 0 && room < 170);
	const measured = $derived(room > 0 && cards.length > 0 && cards.every((_, i) => (heights[i] ?? 0) > 0));

	const pages = $derived.by(() => {
		if (!measured) return [cards.map((_, i) => i)];
		const all = packColumn(heights.slice(0, cards.length), room, GAP);
		return all.length > 1 ? packColumn(heights.slice(0, cards.length), room - DOTS, GAP) : all;
	});
	const roomForCards = $derived(room - (pages.length > 1 ? DOTS : 0));
	const scales = $derived(
		pages.map((page) => {
			if (!measured) return 1;
			const content = page.reduce((sum, i) => sum + heights[i], 0) + GAP * (page.length - 1);
			return fitScale(content, roomForCards);
		})
	);

	let index = $state(0);
	const active = $derived(pages.length ? index % pages.length : 0);
	$effect(() => {
		const count = pages.length;
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
			case 'stocks':
				return m.wall_widgets_stocks();
			case 'cve':
				return m.wall_widgets_cve();
			case 'claude':
				return m.wall_widgets_claude();
			case 'openai':
				return m.wall_widgets_openai();
		}
	}

	function periodLabel(period: WallAiPeriod): string {
		switch (period) {
			case 'today':
				return m.wall_widgets_ai_today();
			case 'month':
				return m.wall_widgets_ai_month();
			default:
				return m.wall_widgets_ai_week();
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

{#snippet cardBody(card: WidgetCard)}
	{@const section = sectionOf(card)}
	<header class="head">
		<span class="eyebrow">
			{#if card.kind === 'stocks'}<ChartCandlestick class="size-[1em]" aria-hidden="true" />
			{:else if card.kind === 'claude'}<Sparkles class="size-[1em]" aria-hidden="true" />
			{:else if card.kind === 'openai'}<Bot class="size-[1em]" aria-hidden="true" />
			{:else}<ShieldAlert class="size-[1em]" aria-hidden="true" />{/if}
			{title(card)}
		</span>
		{#if card.kind === 'claude' || card.kind === 'openai'}
			<span class="page">{periodLabel((section?.data as WallAiUsage).period)}</span>
		{:else if card.pages > 1}
			<span class="page tnum">{m.wall_widgets_page({ page: card.page + 1, pages: card.pages })}</span>
		{/if}
	</header>

	{#if section?.data}
		{#if card.kind === 'stocks'}
			{@const data = widgets.stocks!.data!}
			<ul class="rows">
				{#each data.quotes.slice(card.from, card.to) as quote (quote.symbol)}
					<li class="row">
						<span class="name"><b>{quote.symbol}</b></span>
						<span class="price tnum">{formatNumber(quote.price, locale)}</span>
						<Trend percent={quote.change_pct} />
					</li>
				{/each}
			</ul>
		{:else if card.kind === 'claude' || card.kind === 'openai'}
			{@const usage = section.data as WallAiUsage}
			{@const people = usage.people ?? []}
			{#if card.page === 0}
				<div class="ai-total">
					<span class="ai-cost tnum">{formatUsd(usage.cost_usd, locale)}</span>
					<dl class="ai-tokens tnum">
						<div>
							<dt>{m.wall_widgets_ai_input()}</dt>
							<dd>{formatTokens(usage.input_tokens, locale)}</dd>
						</div>
						<div>
							<dt>{m.wall_widgets_ai_output()}</dt>
							<dd>{formatTokens(usage.output_tokens, locale)}</dd>
						</div>
					</dl>
				</div>
			{/if}
			{#if people.length > 0}
				<!-- A badge only where there are several people to tell apart. -->
				<ul class="rows">
					{#each people.slice(card.from, card.to) as person, i (`${card.from + i}`)}
						<li class="row row--ai">
							<span class="who">
								{#if people.length > 1}<PersonBadge name={person.name} />{/if}
								<b class="name">{person.name ?? m.wall_widgets_ai_unattributed()}</b>
							</span>
							<span
								class="price tnum"
								title={`${m.wall_widgets_ai_input()} ${formatTokens(person.input_tokens, locale)} · ${m.wall_widgets_ai_output()} ${formatTokens(person.output_tokens, locale)}`}
							>
								{m.wall_widgets_ai_tokens({ count: formatTokens(person.input_tokens + person.output_tokens, locale) })}
							</span>
						</li>
					{/each}
				</ul>
			{/if}
		{:else if card.kind === 'cve'}
			{@const items = widgets.cve!.data!}
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
{/snippet}

{#if cards.length > 0}
	<section class="deck" aria-label={m.wall_widgets_aria()} aria-roledescription={m.wall_widgets_carousel()}>
		<div class="room" bind:clientHeight={room}>
			<!-- Each card at its natural size, unseen: what the pages are cut from. -->
			<div class="probe" class:tight aria-hidden="true" inert>
				{#each cards as card, i (card.key)}
					<article class="card" bind:clientHeight={heights[i]}>{@render cardBody(card)}</article>
				{/each}
			</div>

			<div class="stack" class:stack--ready={measured}>
				{#each pages as page, p (page.join(','))}
					{@const shown = p === active}
					<div
						class="page-cards"
						class:page-cards--shown={shown}
						class:tight
						style:--k={scales[p]}
						aria-hidden={shown ? undefined : 'true'}
						inert={!shown}
					>
						{#each page as i (cards[i].key)}
							<article class="card">{@render cardBody(cards[i])}</article>
						{/each}
					</div>
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
	.deck {
		display: flex;
		flex-direction: column;
		flex: 1 1 0;
		min-height: 0;
		width: 100%;
		text-align: left;
		/* One size for the whole rail: cards grow or shrink from it. */
		--wbase: 1rem;
	}
	/* The room the cards share: what the rail leaves under the photo. */
	.room {
		position: relative;
		flex: 1 1 0;
		min-height: 0;
		overflow: hidden;
	}
	.probe {
		position: absolute;
		inset-inline: 0;
		top: 0;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		font-size: var(--wbase);
		visibility: hidden;
		pointer-events: none;
	}
	.stack {
		display: grid;
		opacity: 0;
	}
	.stack--ready {
		opacity: 1;
	}
	.page-cards {
		grid-area: 1 / 1;
		display: flex;
		flex-direction: column;
		align-content: start;
		gap: 0.75em;
		font-size: calc(var(--wbase) * var(--k, 1));
		opacity: 0;
		visibility: hidden;
		transition:
			opacity 700ms ease,
			visibility 0s linear 700ms;
	}
	.page-cards--shown {
		opacity: 1;
		visibility: visible;
		transition:
			opacity 700ms ease,
			visibility 0s;
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: 0.5em;
		padding: 0.8em 1.1em 0.7em;
		border-radius: var(--radius-card);
		border: 1px solid var(--c-line);
		background: color-mix(in srgb, var(--c-surface) 92%, transparent);
		box-shadow: var(--shadow-float);
		color: var(--c-ink);
	}
	.probe .card {
		font-size: inherit;
	}
	.head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 0.75em;
	}
	.eyebrow {
		display: inline-flex;
		align-items: center;
		gap: 0.45em;
		font-size: 0.8em;
		font-weight: 700;
		letter-spacing: 0.07em;
		text-transform: uppercase;
		color: var(--c-ink-2);
	}
	.page {
		font-size: 0.8em;
		font-weight: 600;
		color: var(--c-ink-2);
	}
	.rows {
		display: flex;
		flex-direction: column;
	}
	.row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto 5.5em;
		align-items: baseline;
		gap: 0.6em;
		padding-block: 0.28em;
		border-top: 1px solid var(--c-line);
		font-size: 1.15em;
	}
	.row:first-child {
		border-top: 0;
	}
	.row--ai {
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
	}
	.who {
		display: flex;
		align-items: center;
		gap: 0.55em;
		min-width: 0;
	}
	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.name,
	.row b {
		font-weight: 750;
	}
	.price {
		font-weight: 650;
		text-align: right;
	}
	.row--ai .price {
		font-size: 0.85em;
		font-weight: 600;
		color: var(--c-ink-2);
	}
	.row :global(.trend) {
		font-size: 0.8em;
	}
	.foot {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 0.2em 1em;
		font-size: 0.7em;
		color: var(--c-ink-3, var(--c-ink-2));
	}
	.stale {
		color: var(--c-advisory-ink);
		font-weight: 600;
	}

	/* AI usage */
	.ai-total {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		justify-content: space-between;
		gap: 0.2em 1em;
	}
	.ai-cost {
		font-size: 2em;
		font-weight: 750;
		line-height: 1.1;
	}
	.ai-tokens {
		display: flex;
		gap: 1em;
	}
	.ai-tokens dt {
		font-size: 0.7em;
		color: var(--c-ink-2);
	}
	.ai-tokens dd {
		font-size: 1.1em;
		font-weight: 650;
	}

	/* Vulnerabilities */
	.cves {
		display: flex;
		flex-direction: column;
		gap: 0.4em;
	}
	.cves li + li {
		padding-top: 0.4em;
		border-top: 1px solid var(--c-line);
	}
	.cve-head {
		display: flex;
		align-items: baseline;
		flex-wrap: wrap;
		gap: 0.15em 0.5em;
		font-size: 1em;
	}
	.score {
		min-width: 2.6em;
		padding: 0.02em 0.35em;
		border-radius: 0.375em;
		border: 1.5px solid var(--c-warning);
		color: var(--c-warning-ink);
		font-weight: 800;
		text-align: center;
	}
	.score--none {
		border-color: var(--c-line-strong);
		color: var(--c-ink-2);
		font-weight: 600;
		font-size: 0.8em;
	}
	.tag {
		padding: 0 0.4em;
		border-radius: 0.375em;
		background: var(--c-warning-soft);
		color: var(--c-warning-ink);
		font-size: 0.7em;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.when {
		margin-left: auto;
		font-size: 0.8em;
		color: var(--c-ink-2);
	}
	.cve-title {
		font-size: 0.85em;
		line-height: 1.3;
		color: var(--c-ink-2);
		display: -webkit-box;
		-webkit-line-clamp: 1;
		line-clamp: 1;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.tight .foot,
	.tight .cve-title {
		display: none;
	}
	.tight .card {
		gap: 0.3em;
		padding: 0.55em 0.9em 0.5em;
	}

	/* Where the rail is in its round. */
	.dots {
		display: flex;
		justify-content: center;
		gap: 0.375rem;
		width: fit-content;
		margin: 0.4rem auto 0;
		padding: 0.3rem 0.625rem;
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

	/* The wall proper, read from across the room: a bigger base on a bigger board. */
	@media (min-width: 1600px) {
		.deck {
			--wbase: 1.15rem;
		}
	}
	@media (max-width: 1023px) {
		/* Phones and tablets: the page scrolls, so the rail has no height to share. */
		.deck {
			flex: none;
		}
		.room {
			flex: none;
			overflow: visible;
		}
		.probe {
			display: none;
		}
		.stack {
			opacity: 1;
		}
		.page-cards {
			grid-area: auto;
			display: none;
		}
		.page-cards--shown {
			display: flex;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.page-cards,
		.page-cards--shown,
		.dots li {
			transition: none;
		}
	}
</style>
