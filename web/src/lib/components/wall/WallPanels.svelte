<script lang="ts">
	/**
	 * The wall's information panels, under the clock: one at a time, still for
	 * the seconds chosen in Settings, then the next (a short fade in; none under
	 * `prefers-reduced-motion`). A page of headlines, an article's picture, or a
	 * widget's card (coins, exchange rates, stocks, critical vulnerabilities,
	 * Claude or OpenAI usage); the round is built by `buildPanels`. Nothing
	 * scrolls and nothing runs between two panels: a display that stays on for
	 * days draws one panel, then rests.
	 *
	 * The room is the height the rail leaves. A list card measures itself once
	 * (two hidden probes, one row and two rows high) and cuts its list into
	 * pages of the rows that fit, so no row is ever half shown. A picture fills
	 * the column's width and never outgrows the room; its words sit on a dark
	 * veil at the bottom, visible before the picture loads too. A picture that
	 * does not load sends its article back to the headlines.
	 *
	 * Pictures come from the server (`/api/wall/news/image/…`), never from the
	 * news site. Text only, no link: a kiosk never navigates away. Rises and
	 * falls carry an arrow and a sign, not only a colour.
	 */
	import { Bot, ChartCandlestick, Coins, Banknote, Newspaper, ShieldAlert, Sparkles } from 'lucide-svelte';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	import { formatRelative } from '#lib/format.js';
	import type { WallAiPeriod, WallAiUsage, WallWidgets, WallSection } from '#lib/api/types.js';
	import type { NewsItem } from '#lib/api/wallNews.js';
	import { formatNumber, formatPrice, formatTokens, formatUsd, fxQuote } from '#lib/wall/widgets.js';
	import {
		buildPanels,
		cardMetrics,
		DEFAULT_ROWS,
		LIST_KINDS,
		nextPanel,
		rowsThatFit,
		widgetRows,
		type ListKind,
		type Panel,
		type Rows,
		type WidgetKind
	} from '#lib/wall/panels.js';
	import Trend from './widgets/Trend.svelte';
	import PersonBadge from './widgets/PersonBadge.svelte';

	interface Props {
		/** The news, most recent first; empty when the news is off. */
		news: NewsItem[];
		/** Articles with a picture get a panel of their own. */
		pictures: boolean;
		widgets: WallWidgets | null;
		/** Seconds each panel stays. */
		dwell: number;
		/** Re-read every second by the wall, so ages stay current. */
		now: Date;
		paused?: boolean;
	}

	let { news, pictures, widgets, dwell, now, paused = false }: Props = $props();

	const locale = $derived(getLocale());
	let room = $state(0);
	let failed = $state(new Set<string>());

	// --- Measuring: two hidden probes per list kind, one row and two rows high.
	let probes = $state<Record<string, number>>({});
	/** Kinds with at least two rows to measure: with one row there is nothing to page. */
	const probed = $derived(
		LIST_KINDS.filter((kind) => (kind === 'headlines' ? news.length : widgetRows(widgets, kind)) >= 2)
	);
	const rows = $derived.by(() => {
		const out: Rows = {};
		for (const kind of probed) {
			const metrics = cardMetrics(probes[`${kind}:1`] ?? 0, probes[`${kind}:2`] ?? 0);
			if (metrics) out[kind] = rowsThatFit(room, metrics.chrome, metrics.row, DEFAULT_ROWS[kind]);
		}
		return out;
	});

	const panels = $derived(buildPanels({ news, pictures, failed, widgets, rows }));
	let index = $state(0);
	const active = $derived(panels.length ? index % panels.length : 0);
	const panel = $derived(panels[active] ?? null);

	// One timer, one step: the panel changes, nothing moves in between.
	$effect(() => {
		const count = panels.length;
		const ms = Math.max(5, dwell) * 1000;
		if (paused || count < 2) return;
		const timer = setInterval(() => (index = nextPanel(index, count)), ms);
		return () => clearInterval(timer);
	});

	// The next picture is fetched while the current panel shows, so it appears whole.
	$effect(() => {
		const count = panels.length;
		for (let step = 1; step < count; step += 1) {
			const next = panels[(active + step) % count];
			if (next.kind === 'picture' && next.item.image) {
				const image = new Image();
				image.src = next.item.image;
				return;
			}
		}
	});

	function drop(image: string) {
		failed = new Set([...failed, image]);
	}

	function sectionOf(kind: WidgetKind): WallSection<unknown> | null {
		return (widgets?.[kind] as WallSection<unknown> | null) ?? null;
	}

	function title(kind: ListKind): string {
		switch (kind) {
			case 'headlines':
				return m.wall_panels_headlines();
			case 'crypto':
				return m.wall_widgets_crypto();
			case 'fx':
				return m.wall_widgets_fx();
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

	/** A probe card: the first `to` rows of a kind, at its natural height. */
	function probePanel(kind: ListKind, to: number): Panel {
		if (kind === 'headlines') return { key: `probe:${kind}:${to}`, kind, items: news.slice(0, to), page: 0, pages: 1 };
		return { key: `probe:${kind}:${to}`, kind, from: 0, to, page: 0, pages: 1 };
	}
</script>

{#snippet head(kind: ListKind, right: string)}
	<header class="head">
		<span class="eyebrow">
			{#if kind === 'headlines'}<Newspaper class="size-[1em]" aria-hidden="true" />
			{:else if kind === 'crypto'}<Coins class="size-[1em]" aria-hidden="true" />
			{:else if kind === 'fx'}<Banknote class="size-[1em]" aria-hidden="true" />
			{:else if kind === 'stocks'}<ChartCandlestick class="size-[1em]" aria-hidden="true" />
			{:else if kind === 'claude'}<Sparkles class="size-[1em]" aria-hidden="true" />
			{:else if kind === 'openai'}<Bot class="size-[1em]" aria-hidden="true" />
			{:else}<ShieldAlert class="size-[1em]" aria-hidden="true" />{/if}
			{title(kind)}
		</span>
		{#if right}<span class="page tnum">{right}</span>{/if}
	</header>
{/snippet}

{#snippet foot(section: WallSection<unknown> | null)}
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

{#snippet card(panel: Panel)}
	{#if panel.kind === 'headlines'}
		<article class="card">
			{@render head('headlines', panel.pages > 1 ? m.wall_widgets_page({ page: panel.page + 1, pages: panel.pages }) : '')}
			<ol class="headlines">
				{#each panel.items as item (item.link)}
					<li>
						<p class="hl-meta">
							<span class="source">{item.source}</span>
							{#if item.published}<span class="age tnum">{age(item.published)}</span>{/if}
						</p>
						<p class="hl-title">{item.title}</p>
					</li>
				{/each}
			</ol>
		</article>
	{:else if panel.kind === 'picture'}
		<figure class="picture">
			<img src={panel.item.image} alt="" decoding="async" onerror={() => panel.item.image && drop(panel.item.image)} />
			<figcaption>
				<span class="meta">
					<span class="source">{panel.item.source}</span>
					{#if panel.item.published}<span class="age tnum">{age(panel.item.published)}</span>{/if}
				</span>
				<span class="title">{panel.item.title}</span>
			</figcaption>
		</figure>
	{:else}
		{@const section = sectionOf(panel.kind)}
		<article class="card">
			{#if panel.kind === 'claude' || panel.kind === 'openai'}
				{@render head(panel.kind, periodLabel((section?.data as WallAiUsage).period))}
			{:else if panel.kind === 'fx' && panel.pages === 1}
				{@render head(panel.kind, m.wall_market_vs({ base: widgets!.fx!.data!.base }))}
			{:else}
				{@render head(panel.kind, panel.pages > 1 ? m.wall_widgets_page({ page: panel.page + 1, pages: panel.pages }) : '')}
			{/if}

			{#if panel.kind === 'crypto'}
				{@const data = widgets!.crypto!.data!}
				<ul class="rows">
					{#each data.coins.slice(panel.from, panel.to) as coin (coin.id)}
						<li class="row">
							<span class="name"><b>{coin.symbol.toUpperCase()}</b></span>
							<span class="price tnum">{formatPrice(coin.price, data.currency, locale)}</span>
							<Trend percent={coin.change_24h} />
						</li>
					{/each}
				</ul>
			{:else if panel.kind === 'fx'}
				{@const data = widgets!.fx!.data!}
				<ul class="rows">
					{#each data.rates.slice(panel.from, panel.to) as rate (rate.code)}
						{@const quote = fxQuote(rate.rate, rate.previous)}
						<li class="row">
							<span class="name"><b>{quote.per === 1 ? rate.code : `${quote.per} ${rate.code}`}</b></span>
							<span class="price tnum">{formatPrice(quote.value, data.base, locale)}</span>
							<Trend percent={quote.change} />
						</li>
					{/each}
				</ul>
			{:else if panel.kind === 'stocks'}
				{@const data = widgets!.stocks!.data!}
				<ul class="rows">
					{#each data.quotes.slice(panel.from, panel.to) as quote (quote.symbol)}
						<li class="row">
							<span class="name"><b>{quote.symbol}</b></span>
							<span class="price tnum">{formatNumber(quote.price, locale)}</span>
							<Trend percent={quote.change_pct} />
						</li>
					{/each}
				</ul>
			{:else if panel.kind === 'cve'}
				{@const items = widgets!.cve!.data!}
				<ul class="cves">
					{#each items.slice(panel.from, panel.to) as cve (cve.id)}
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
			{:else}
				{@const usage = section!.data as WallAiUsage}
				{@const people = usage.people ?? []}
				{#if panel.page === 0}
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
						{#each people.slice(panel.from, panel.to) as person, i (`${panel.from + i}`)}
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
			{/if}
			{@render foot(section)}
		</article>
	{/if}
{/snippet}

{#if panels.length > 0}
	<section class="panels" aria-label={m.wall_panels_aria()} aria-roledescription={m.wall_widgets_carousel()}>
		<div class="room" bind:clientHeight={room}>
			<!-- Each list kind at its natural height with one row, then two: what the pages are cut from. -->
			<div class="probe" aria-hidden="true" inert>
				{#each probed as kind (kind)}
					{#each [1, 2] as to (to)}
						<div class="probe-card" bind:clientHeight={probes[`${kind}:${to}`]}>{@render card(probePanel(kind, to))}</div>
					{/each}
				{/each}
			</div>

			{#if panel}
				{#key panel.key}
					<div class="panel" aria-live="off">{@render card(panel)}</div>
				{/key}
			{/if}
		</div>
	</section>
{/if}

<style>
	.panels {
		display: flex;
		flex-direction: column;
		flex: 1 1 0;
		min-height: 0;
		width: 100%;
		text-align: left;
		/* One size for the whole rail. */
		font-size: var(--wbase, 1rem);
	}
	/* The room the panels take turns in: what the rail leaves under the clock. */
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
		visibility: hidden;
		pointer-events: none;
	}
	.probe-card {
		position: absolute;
		inset-inline: 0;
		top: 0;
	}
	/* As tall as the room, so a picture can be capped to it; a card keeps its own height. */
	.panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		animation: panel-in 320ms ease both;
	}
	@keyframes panel-in {
		from {
			opacity: 0;
		}
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
		white-space: nowrap;
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
		white-space: nowrap;
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

	/* Headlines: source and age, then the title on two lines at most (always the room for two). */
	.headlines {
		display: flex;
		flex-direction: column;
	}
	.headlines li {
		padding-block: 0.45em;
		border-top: 1px solid var(--c-line);
	}
	.headlines li:first-child {
		border-top: 0;
		padding-top: 0.1em;
	}
	.hl-meta {
		display: flex;
		gap: 0.6em;
		align-items: baseline;
		font-size: 0.75em;
		color: var(--c-ink-2);
	}
	.source {
		font-weight: 700;
		letter-spacing: 0.06em;
		text-transform: uppercase;
	}
	.hl-title {
		margin-top: 0.15em;
		font-size: 1.05em;
		font-weight: 650;
		line-height: 1.25;
		min-height: 2.5em;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	/* A picture: the column's width, never taller than the room, words on a dark veil. */
	.picture {
		position: relative;
		width: 100%;
		aspect-ratio: 16 / 10;
		max-height: 100%;
		margin: 0;
		overflow: hidden;
		border-radius: var(--radius-card);
		border: 2px solid var(--c-line-strong);
		background: var(--c-surface-2);
		box-shadow: var(--shadow-float);
	}
	.picture img {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.picture figcaption {
		position: absolute;
		inset-inline: 0;
		bottom: 0;
		display: flex;
		flex-direction: column;
		gap: 0.375em;
		padding: 2.5em 1em 0.9em;
		background: linear-gradient(to top, rgb(0 0 0 / 0.82), rgb(0 0 0 / 0.55) 55%, transparent);
		color: #fff;
	}
	.picture .meta {
		display: flex;
		gap: 0.75em;
		align-items: baseline;
		font-size: 0.8125em;
	}
	.picture .age {
		opacity: 0.85;
	}
	.picture .title {
		font-size: 1.2em;
		font-weight: 700;
		line-height: 1.15;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
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

	/* The wall proper, read from across the room: a bigger base on a bigger board. */
	@media (min-width: 1600px) {
		.panels {
			--wbase: 1.15rem;
		}
	}
	@media (max-width: 1023px) {
		/* Phones and tablets: the page scrolls, so the rail has no height to share. */
		.panels,
		.room {
			flex: none;
		}
		.room {
			overflow: visible;
		}
		.probe {
			display: none;
		}
		.panel,
		.picture {
			height: auto;
			max-height: none;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.panel {
			animation: none;
		}
	}
</style>
