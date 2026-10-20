<script lang="ts">
	/**
	 * One wall widget, said small and still: the monitors are what the wall is
	 * for, the widgets sit beside them in a quieter voice. Two shapes:
	 *
	 * - `bar`: one line in the top or bottom bar, every entry side by side
	 *   (`BTC 91 234 $ ▲1.2 %  ETH …`);
	 * - `column`: a light card in the right column, one entry per line.
	 *
	 * Every entry shows at once; nothing turns, nothing scrolls. When the zone
	 * runs out of room it asks for less (`compact`: no percentages, no tags;
	 * `cap`: fewer entries, the rest counted as "+N"), never for a rotation.
	 * Each entry is a `data-entry`, which is what the zone counts.
	 *
	 * Rises and falls carry an arrow and a sign, not only a colour. Text only,
	 * no link: a kiosk never navigates away.
	 */
	import { Bot, ChartCandlestick, Coins, Banknote, MessageSquareText, Newspaper, ShieldAlert, Sparkles } from 'lucide-svelte';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	import { formatRelative } from '#lib/format.js';
	import type { WallAiPeriod, WallAiUsage, WallSection, WallWidgets } from '#lib/api/types.js';
	import type { NewsItem } from '#lib/api/wallNews.js';
	import type { WallWidgetId } from '#lib/wall/layout.js';
	import { formatNumber, formatPrice, formatTokens, formatUsd, fxQuote, roundTemperature, weatherKind, type WeatherKind } from '#lib/wall/widgets.js';
	import Trend from './widgets/Trend.svelte';
	import WeatherIcon from './widgets/WeatherIcon.svelte';

	interface Props {
		id: WallWidgetId;
		variant: 'bar' | 'column';
		widgets: WallWidgets | null;
		news: NewsItem[];
		/** Re-read every second by the wall, so ages stay current. */
		now: Date;
		/** At most this many entries; the rest are counted. */
		cap?: number;
		/** Short forms: no percentages, no tags, no source. */
		compact?: boolean;
	}

	let { id, variant, widgets, news, now, cap = Infinity, compact = false }: Props = $props();

	const locale = $derived(getLocale());
	const bar = $derived(variant === 'bar');

	const section = $derived(
		id === 'news' || id === 'text' ? null : ((widgets?.[id] as WallSection<unknown> | null | undefined) ?? null)
	);
	/**
	 * Off, waiting for a key, or nothing to say: the widget takes no room at
	 * all (a missing key is said in Settings, not on the wall).
	 */
	const visible = $derived(id === 'news' ? news.length > 0 : id === 'text' ? !!widgets?.text : !!section?.data);

	const TITLES: Record<WallWidgetId, () => string> = {
		weather: m.wall_widgets_weather,
		news: m.wall_widgets_news,
		cve: m.wall_widgets_cve,
		crypto: m.wall_widgets_crypto,
		stocks: m.wall_widgets_stocks,
		fx: m.wall_widgets_fx,
		claude: m.wall_widgets_claude,
		openai: m.wall_widgets_openai,
		text: m.wall_widgets_text
	};
	const ICONS = { text: MessageSquareText, news: Newspaper, cve: ShieldAlert, crypto: Coins, stocks: ChartCandlestick, fx: Banknote, claude: Sparkles, openai: Bot };

	const SKY: Record<WeatherKind, () => string> = {
		clear: m.wall_widgets_sky_clear,
		partly: m.wall_widgets_sky_partly,
		cloudy: m.wall_widgets_sky_cloudy,
		fog: m.wall_widgets_sky_fog,
		drizzle: m.wall_widgets_sky_drizzle,
		rain: m.wall_widgets_sky_rain,
		snow: m.wall_widgets_sky_snow,
		storm: m.wall_widgets_sky_storm
	};

	function periodLabel(period: WallAiPeriod): string {
		if (period === 'today') return m.wall_widgets_ai_today();
		if (period === 'month') return m.wall_widgets_ai_month();
		return m.wall_widgets_ai_week();
	}

	function age(at: string | null): string {
		void now;
		return at ? formatRelative(at) : '';
	}

	function take<T>(list: readonly T[]): { shown: T[]; more: number } {
		const shown = list.slice(0, Math.max(1, cap));
		return { shown, more: list.length - shown.length };
	}

	const weather = $derived(id === 'weather' ? widgets?.weather?.data ?? null : null);
	const usage = $derived(id === 'claude' || id === 'openai' ? ((section?.data as WallAiUsage | null) ?? null) : null);
</script>

{#snippet more(count: number)}
	{#if count > 0}<li class="more tnum" data-entry>{m.wall_widgets_more({ count })}</li>{/if}
{/snippet}

{#if visible}
	<svelte:element
		this={bar ? 'div' : 'section'}
		class="w"
		class:w--bar={bar}
		class:w--column={!bar}
		data-wall-info
		data-widget={id}
		aria-label={TITLES[id]()}
		title={section ? m.wall_widgets_source({ source: section.source }) : undefined}
	>
		{#if id !== 'weather' && id !== 'text'}
			{@const Icon = ICONS[id]}
			<span class="w-head">
				<Icon class="w-icon" aria-hidden="true" />
				{#if !bar}<span class="w-title">{TITLES[id]()}</span>{/if}
				{#if !bar && usage}<span class="w-aside">{periodLabel(usage.period)}</span>{/if}
				{#if !bar && id === 'fx' && widgets?.fx?.data}<span class="w-aside">{m.wall_market_vs({ base: widgets.fx.data.base })}</span>{/if}
			</span>
		{/if}

		{#if id === 'weather' && weather}
			<span class="entry" data-entry role="img" aria-label={m.wall_weather_now({ place: weather.name, sky: SKY[weatherKind(weather.code)](), temperature: `${Math.round(weather.temperature)}${weather.temperature_unit}` })}>
				<WeatherIcon code={weather.code} isDay={weather.is_day} class="w-icon w-icon--weather" />
				<b class="tnum">{roundTemperature(weather.temperature)}</b>
				{#if !compact}<span class="dim">{weather.name}</span>{/if}
			</span>
		{:else if id === 'text' && widgets?.text}
			<span class="entry text" data-entry>{widgets.text}</span>
		{:else if id === 'news'}
			{@const list = take(news)}
			<ul class="entries" class:entries--news={!bar}>
				{#each list.shown as item (item.link)}
					<li class="entry entry--news" data-entry>
						<span class="news-text">
							{#if !compact || !bar}<span class="dim source">{item.source}</span>{/if}
							<span class="headline" title={item.title}>{item.title}</span>
						</span>
					</li>
				{/each}
			</ul>
		{:else if id === 'crypto' && widgets?.crypto?.data}
			{@const data = widgets.crypto.data}
			{@const list = take(data.coins)}
			<ul class="entries">
				{#each list.shown as coin (coin.id)}
					<li class="entry entry--quote" data-entry>
						<b>{coin.symbol.toUpperCase()}</b>
						<span class="tnum">{formatPrice(coin.price, data.currency, locale)}</span>
						{#if !compact}<Trend percent={coin.change_24h} />{/if}
					</li>
				{/each}
				{@render more(list.more)}
			</ul>
		{:else if id === 'stocks' && widgets?.stocks?.data}
			{@const list = take(widgets.stocks.data.quotes)}
			<ul class="entries">
				{#each list.shown as quote (quote.symbol)}
					<li class="entry entry--quote" data-entry>
						<b>{quote.symbol}</b>
						<span class="tnum">{formatNumber(quote.price, locale)}</span>
						{#if !compact}<Trend percent={quote.change_pct} />{/if}
					</li>
				{/each}
				{@render more(list.more)}
			</ul>
		{:else if id === 'fx' && widgets?.fx?.data}
			{@const data = widgets.fx.data}
			{@const list = take(data.rates)}
			<ul class="entries">
				{#each list.shown as rate (rate.code)}
					{@const quote = fxQuote(rate.rate, rate.previous)}
					<li class="entry entry--quote" data-entry>
						<b>{quote.per === 1 ? rate.code : `${quote.per} ${rate.code}`}</b>
						<span class="tnum">{formatPrice(quote.value, data.base, locale)}</span>
						{#if !compact}<Trend percent={quote.change} />{/if}
					</li>
				{/each}
				{@render more(list.more)}
			</ul>
		{:else if id === 'cve' && widgets?.cve?.data}
			{@const items = widgets.cve.data}
			{#if items.length === 0}
				<span class="entry muted" data-entry>{m.wall_widgets_cve_none()}</span>
			{:else}
				{@const list = take(items)}
				<ul class="entries" class:entries--cve={!bar}>
					{#each list.shown as cve (cve.id)}
						<li class="entry entry--cve" data-entry>
							<span class="score tnum">{cve.score === null ? m.wall_widgets_cve_unscored() : cve.score.toFixed(1)}</span>
							<b class="tnum">{cve.id}</b>
							{#if !compact && cve.exploited}<span class="tag">{m.wall_widgets_cve_exploited()}</span>{/if}
							{#if !compact && cve.ransomware}<span class="tag">{m.wall_widgets_cve_ransomware()}</span>{/if}
							{#if !bar && !compact}<span class="cve-title" title={cve.title}>{cve.title}</span>{/if}
						</li>
					{/each}
					{@render more(list.more)}
				</ul>
			{/if}
		{:else if usage}
			{@const people = bar ? [] : (usage.people ?? [])}
			{@const list = take([null, ...people])}
			<ul class="entries">
				{#each list.shown as person, i (i)}
					{#if person === null}
						<li class="entry entry--quote" data-entry title={periodLabel(usage.period)}>
							{#if bar}<b>{TITLES[id]()}</b>{/if}
							<span class="tnum">{formatUsd(usage.cost_usd, locale)}</span>
							{#if !compact}<span class="dim tnum">{m.wall_widgets_ai_tokens({ count: formatTokens(usage.input_tokens + usage.output_tokens, locale) })}</span>{/if}
						</li>
					{:else}
						<li class="entry entry--quote" data-entry>
							<b>{person.name ?? m.wall_widgets_ai_unattributed()}</b>
							<span class="dim tnum">{m.wall_widgets_ai_tokens({ count: formatTokens(person.input_tokens + person.output_tokens, locale) })}</span>
						</li>
					{/if}
				{/each}
				{@render more(list.more)}
			</ul>
		{/if}

		{#if !bar && !compact && section?.error && section.updated}
			<span class="stale tnum">{m.wall_widgets_stale({ when: age(section.updated) })}</span>
		{/if}
	</svelte:element>
{/if}

<style>
	/* A quieter voice than the monitors': smaller type, second ink, no colour but the trends. */
	.w {
		min-width: 0;
		color: var(--c-ink-2);
		font-size: var(--w-size, 1rem);
		line-height: 1.3;
	}
	.w :global(.w-icon) {
		width: 1em;
		height: 1em;
		flex-shrink: 0;
		color: var(--c-ink-3, var(--c-ink-2));
	}
	.w b {
		font-weight: 650;
		color: var(--c-ink);
	}
	.dim {
		color: var(--c-ink-3, var(--c-ink-2));
	}
	.muted {
		color: var(--c-ink-3, var(--c-ink-2));
		font-style: italic;
	}
	.w :global(.trend) {
		font-size: 0.85em;
	}

	/* ---- In a bar: one line, entries side by side. */
	.w--bar {
		display: inline-flex;
		align-items: center;
		gap: 0.625em;
		white-space: nowrap;
		flex-shrink: 0;
	}
	.w--bar .w-head {
		display: inline-flex;
	}
	.w--bar .entries {
		display: inline-flex;
		align-items: center;
		gap: 1em;
	}
	.w--bar .entry {
		display: inline-flex;
		align-items: baseline;
		gap: 0.4em;
	}
	.w--bar .entry--news {
		max-width: 32em;
	}
	.w--bar .news-text {
		display: inline-flex;
		gap: 0.5em;
		min-width: 0;
	}
	.w--bar .headline {
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.w--bar :global(.w-icon--weather) {
		align-self: center;
	}

	/* ---- In the right column: a light card, one entry per line. */
	.w--column {
		display: flex;
		flex-direction: column;
		gap: 0.375em;
		width: 100%;
		padding: 0.625em 0.875em;
		border-radius: 12px;
		background: color-mix(in srgb, var(--c-surface) 72%, transparent);
		text-align: left;
	}
	.w--column .w-head {
		display: flex;
		align-items: center;
		gap: 0.5em;
		font-size: 0.8125em;
		font-weight: 600;
		letter-spacing: 0.02em;
		text-transform: uppercase;
		color: var(--c-ink-3, var(--c-ink-2));
	}
	.w-aside {
		margin-left: auto;
		text-transform: none;
		letter-spacing: 0;
		font-weight: 500;
	}
	.w--column .entries {
		display: flex;
		flex-direction: column;
		gap: 0.25em;
	}
	.w--column .entry--quote {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		align-items: baseline;
		gap: 0.75em;
	}
	.w--column .entry--quote > b {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.w--column .entry--quote > :global(:last-child:not(b)) {
		justify-self: end;
	}
	/* Headlines and vulnerabilities: text only, airy, two lines at most each. */
	.entries--news,
	.entries--cve {
		gap: 0.875em !important;
	}
	.w--column .news-text {
		display: flex;
		flex-direction: column;
		gap: 0.125em;
		min-width: 0;
	}
	.w--column .source {
		font-size: 0.75em;
		letter-spacing: 0.02em;
	}
	.w--column .headline,
	.cve-title {
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		overflow: hidden;
		overflow-wrap: anywhere;
		font-size: 1.0625em;
		line-height: 1.4;
		color: var(--c-ink);
	}
	.w--column .entry--cve {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0.125em 0.5em;
	}
	.w--column .entry--cve > b {
		font-size: 0.875em;
		font-weight: 600;
		color: var(--c-ink-2);
	}
	.cve-title {
		flex-basis: 100%;
		font-size: 1em;
	}
	/* The free message: plain text, a few lines in the column, one line in a bar. */
	.text {
		color: var(--c-ink);
		overflow-wrap: anywhere;
	}
	.w--bar .text {
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 40em;
	}
	.w--column .text {
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 4;
		line-clamp: 4;
		overflow: hidden;
		font-size: 1.0625em;
		line-height: 1.4;
	}
	.score {
		font-weight: 700;
		color: var(--c-warning-ink);
	}
	.tag {
		border: 1px solid var(--c-line-strong);
		border-radius: 999px;
		padding: 0 0.4em;
		font-size: 0.75em;
	}
	.more {
		font-size: 0.875em;
		color: var(--c-ink-3, var(--c-ink-2));
	}
	.stale {
		font-size: 0.75em;
		color: var(--c-advisory-ink);
	}
	/* The weather is one line: it keeps to its size, at the column's edge. */
	.w--column[data-widget='weather'] {
		width: auto;
		align-self: flex-end;
	}
	.w--column .entry:has(:global(.w-icon--weather)) {
		display: flex;
		align-items: center;
		gap: 0.5em;
	}
</style>
