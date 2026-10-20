<script lang="ts">
	/**
	 * Settings → Wall widgets (admins). Off by default. Independent widgets,
	 * each with its toggle and its few choices, and the wall's layout: a scale
	 * model of the screen where each widget is dropped in the zone and place it
	 * should have (`WallLayoutEditor`). Every widget turned on shows at once:
	 *
	 * - news (RSS/Atom feeds, `WallNewsFields`): their own endpoint, saved
	 *   together with the rest by the one Save button;
	 * - a free message (plain text, one short line);
	 * - coins (CoinGecko): pick among the main ones, or add any CoinGecko id;
	 *   display currency; an optional free Demo key;
	 * - weather (Open-Meteo): find a city; the wall shows its icon and
	 *   temperature under the clock; units;
	 * - stocks (Finnhub): ticker symbols and the free key they need;
	 * - exchange rates (Frankfurter): base currency and the ones to show;
	 * - critical vulnerabilities (NVD, CISA KEV): which list, how many;
	 * - AI usage (Claude, OpenAI organization admin APIs): admin key of each,
	 *   period, optional "biggest users" with names for the ids the APIs return.
	 *
	 * The server reads every source and caches it; nothing here talks to them
	 * except the city search, which also goes through the server. API keys are
	 * write-only: the form only learns whether one is saved.
	 */
	import { Plus, Search, X } from 'lucide-svelte';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	import { ApiError } from '#lib/api/index.js';
	import { wallDocsUrl } from '#lib/docs.js';
	import type { WallPlace, WallWidgetCatalog, WallWidgetSettings, WallWidgetSettingsPayload } from '#lib/api/types.js';
	import {
		DEFAULT_WIDGET_SETTINGS,
		getWidgetCatalog,
		getWidgetSettings,
		saveWidgetSettings,
		searchPlaces
	} from '#lib/api/wallWidgets.js';
	import { Button, ErrorNotice, Field, Panel, Skeleton, Toggle } from '#lib/ui/index.js';
	import UnsavedBadge from './UnsavedBadge.svelte';
	import WallLayoutEditor from './WallLayoutEditor.svelte';
	import WallNewsFields from './WallNewsFields.svelte';
	import { DEFAULT_NEWS_SETTINGS, getNewsSettings, saveNewsSettings, type NewsSettings } from '#lib/api/wallNews.js';
	import { resolveLayout } from '#lib/wall/layout.js';
	import { useDirtyTracker } from '#lib/settings/unsaved.svelte.js';

	const MAX_COINS = 10;
	const MAX_CURRENCIES = 10;
	const MAX_PEOPLE = 30;
	const MAX_TEXT = 200;

	let loading = $state(true);
	let error = $state<unknown>(null);
	let missing = $state(false);
	let saving = $state(false);
	let notice = $state<string | null>(null);
	let problem = $state<string | null>(null);

	let form = $state<WallWidgetSettings>(structuredClone(DEFAULT_WIDGET_SETTINGS));
	let catalog = $state<WallWidgetCatalog>({ coins: [], coin_currencies: [], fx_currencies: [] });
	let hasFinnhub = $state(false);
	let hasCoingecko = $state(false);
	let hasAnthropic = $state(false);
	let hasOpenai = $state(false);
	/** Keys typed in this visit; `null` keeps the saved one, `''` clears it. */
	let finnhubKey = $state<string | null>(null);
	let coingeckoKey = $state<string | null>(null);
	let anthropicKey = $state<string | null>(null);
	let openaiKey = $state<string | null>(null);
	let symbolsText = $state('');
	/** The News widget's feeds live behind their own endpoint (older servers: none, and the widget hides). */
	let news = $state<NewsSettings>({ ...DEFAULT_NEWS_SETTINGS, feeds: [] });
	let newsAvailable = $state(true);

	const tracker = useDirtyTracker(() => ({ form, news, symbolsText, finnhubKey, coingeckoKey, anthropicKey, openaiKey }));

	let customCoin = $state('');
	let placeQuery = $state('');
	let searching = $state(false);
	let found = $state<WallPlace[] | null>(null);
	let searchError = $state<string | null>(null);

	/** On, off, or on but without the API key it needs: then it stays off the wall. */
	function keyed(on: boolean, saved: boolean, typed: string | null): 'on' | 'off' | 'key' {
		if (!on) return 'off';
		const hasKey = typed === null ? saved : typed.trim() !== '';
		return hasKey ? 'on' : 'key';
	}

	function adopt(saved: WallWidgetSettings) {
		form = structuredClone({
			crypto: saved.crypto,
			weather: saved.weather,
			stocks: saved.stocks,
			fx: saved.fx,
			cve: saved.cve,
			ai: saved.ai,
			text: saved.text ?? { enabled: false, content: '' },
			layout: resolveLayout(saved.layout)
		});
		symbolsText = saved.stocks.symbols.join(', ');
		finnhubKey = null;
		coingeckoKey = null;
		anthropicKey = null;
		openaiKey = null;
	}

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const [saved, offered, savedNews] = await Promise.all([
				getWidgetSettings(signal),
				getWidgetCatalog(signal),
				getNewsSettings(signal).catch((cause) => {
					if (cause instanceof DOMException && cause.name === 'AbortError') throw cause;
					return null;
				})
			]);
			adopt(saved);
			newsAvailable = savedNews !== null;
			news = savedNews ? { ticker: savedNews.ticker, feeds: savedNews.feeds.map((feed) => ({ ...feed })) } : { ...DEFAULT_NEWS_SETTINGS, feeds: [] };
			hasFinnhub = saved.has_finnhub_key;
			hasCoingecko = saved.has_coingecko_key;
			hasAnthropic = saved.has_anthropic_key;
			hasOpenai = saved.has_openai_key;
			catalog = offered;
			tracker.markClean();
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			if (cause instanceof ApiError && cause.missing) missing = true;
			else error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	function currencyName(code: string): string {
		try {
			return new Intl.DisplayNames([getLocale()], { type: 'currency' }).of(code.toUpperCase()) ?? code.toUpperCase();
		} catch {
			return code.toUpperCase();
		}
	}

	// --- Coins

	const coinsFull = $derived(form.crypto.coins.length >= MAX_COINS);
	const presetIds = $derived(new Set(catalog.coins.map((coin) => coin.id)));
	const customCoins = $derived(form.crypto.coins.filter((id) => !presetIds.has(id)));

	function toggleCoin(id: string) {
		notice = null;
		form.crypto.coins = form.crypto.coins.includes(id)
			? form.crypto.coins.filter((other) => other !== id)
			: [...form.crypto.coins, id];
	}

	function addCustomCoin() {
		const id = customCoin.trim().toLowerCase();
		if (!/^[a-z0-9-]{1,60}$/.test(id) || form.crypto.coins.includes(id) || coinsFull) return;
		form.crypto.coins = [...form.crypto.coins, id];
		customCoin = '';
	}

	// --- Weather

	async function runSearch() {
		const q = placeQuery.trim();
		if (q.length < 2) return;
		searching = true;
		found = null;
		searchError = null;
		try {
			found = await searchPlaces(q, getLocale());
		} catch (cause) {
			searchError = cause instanceof Error ? cause.message : String(cause);
		} finally {
			searching = false;
		}
	}

	const samePlace = (a: WallPlace, b: WallPlace) =>
		Math.abs(a.latitude - b.latitude) < 0.01 && Math.abs(a.longitude - b.longitude) < 0.01;

	/** One place: choosing another replaces it. */
	function choosePlace(place: WallPlace) {
		form.weather.places = [place];
		found = null;
		placeQuery = '';
		notice = null;
	}

	function removePlace(place: WallPlace) {
		form.weather.places = form.weather.places.filter((p) => !samePlace(p, place));
	}

	// --- Rates

	const fxFull = $derived(form.fx.currencies.length >= MAX_CURRENCIES);
	function toggleCurrency(code: string) {
		notice = null;
		form.fx.currencies = form.fx.currencies.includes(code)
			? form.fx.currencies.filter((other) => other !== code)
			: [...form.fx.currencies, code];
	}

	// --- AI usage

	function addPerson(kind: 'claude' | 'openai') {
		const list = form.ai[kind].people;
		if (list.length >= MAX_PEOPLE) return;
		form.ai[kind].people = [...list, { id: '', name: '' }];
	}

	function removePerson(kind: 'claude' | 'openai', index: number) {
		form.ai[kind].people = form.ai[kind].people.filter((_, i) => i !== index);
	}

	// --- Save

	async function save() {
		saving = true;
		problem = null;
		notice = null;
		const payload: WallWidgetSettingsPayload = {
			...form,
			stocks: {
				...form.stocks,
				symbols: symbolsText
					.split(/[\s,;]+/)
					.map((s) => s.trim().toUpperCase())
					.filter(Boolean)
			}
		};
		const secrets: { finnhub?: string; coingecko?: string; anthropic?: string; openai?: string } = {};
		if (finnhubKey !== null) secrets.finnhub = finnhubKey.trim();
		if (coingeckoKey !== null) secrets.coingecko = coingeckoKey.trim();
		if (anthropicKey !== null) secrets.anthropic = anthropicKey.trim();
		if (openaiKey !== null) secrets.openai = openaiKey.trim();
		if (Object.keys(secrets).length) payload.secrets = secrets;
		try {
			if (newsAvailable) {
				const savedNews = await saveNewsSettings(news);
				news = { ticker: savedNews.ticker, feeds: savedNews.feeds.map((feed) => ({ ...feed })) };
			}
			const saved = await saveWidgetSettings(payload);
			adopt(saved);
			hasFinnhub = saved.has_finnhub_key;
			hasCoingecko = saved.has_coingecko_key;
			hasAnthropic = saved.has_anthropic_key;
			hasOpenai = saved.has_openai_key;
			tracker.markClean();
			notice = m.settings_widgets_saved();
		} catch (cause) {
			problem = cause instanceof Error ? cause.message : String(cause);
		} finally {
			saving = false;
		}
	}
</script>

{#snippet header(id: string, label: string, hint: string, value: { get: () => boolean; set: (v: boolean) => void })}
	<div class="flex items-start justify-between gap-4">
		<div class="min-w-0">
			<label for={id} class="text-sm font-semibold text-ink">{label}</label>
			<p class="text-[0.8125rem] text-ink-2">{hint}</p>
		</div>
		<Toggle {id} checked={value.get()} onchange={(next) => value.set(next)} disabled={saving} />
	</div>
{/snippet}

{#snippet chip(on: boolean, disabled: boolean, label: string, onchange: () => void)}
	<li>
		<label
			class="flex min-h-10 cursor-pointer items-center gap-2 rounded-lg border px-3 text-sm {on
				? 'border-signal bg-signal-soft text-signal-ink'
				: 'border-line bg-surface text-ink'} {disabled ? 'opacity-50' : ''}"
		>
			<input type="checkbox" class="size-4 accent-[var(--c-signal)]" checked={on} {disabled} {onchange} />
			{label}
		</label>
	</li>
{/snippet}

{#snippet secret(id: string, label: string, help: string, saved: boolean, value: string | null, set: (v: string | null) => void, docAnchor?: string)}
	<Field {label} for={id} {help}>
		<div class="flex flex-wrap items-center gap-2">
			<input
				{id}
				type="password"
				class="input min-w-0 flex-1 font-mono"
				autocomplete="off"
				spellcheck="false"
				maxlength="200"
				placeholder={saved ? m.settings_widgets_key_keep() : ''}
				value={value ?? ''}
				oninput={(e) => set(e.currentTarget.value === '' ? null : e.currentTarget.value)}
			/>
			{#if saved && value !== ''}
				<Button variant="ghost" size="sm" onclick={() => set('')} disabled={saving}>{m.settings_widgets_key_remove()}</Button>
			{/if}
		</div>
		<p class="mt-1 text-[0.8125rem] text-ink-2" aria-live="polite">
			{#if value === ''}
				{m.settings_widgets_key_removing()}
			{:else if saved}
				{m.settings_widgets_key_saved()}
			{/if}
		</p>
		{#if docAnchor}
			<a
				class="mt-1 inline-flex min-h-10 items-center text-sm text-ink-2 underline decoration-line underline-offset-2 hover:text-ink"
				href={wallDocsUrl(docAnchor)}
				target="_blank"
				rel="noreferrer">{m.settings_widgets_ai_key_howto()}</a
			>
		{/if}
	</Field>
{/snippet}

{#snippet missingKey(state: 'on' | 'off' | 'key')}
	{#if state === 'key'}
		<p class="text-[0.8125rem] text-warning-ink" role="status">{m.settings_widgets_key_missing()}</p>
	{/if}
{/snippet}

{#snippet people(kind: 'claude' | 'openai')}
	<div class="space-y-2">
		<p class="text-sm font-medium text-ink">{m.settings_widgets_ai_people()}</p>
		<p class="text-[0.8125rem] text-ink-2">{m.settings_widgets_ai_people_help()}</p>
		<ul class="space-y-2">
			{#each form.ai[kind].people as person, i (i)}
				<li class="flex flex-wrap items-center gap-2">
					<input
						type="text"
						class="input min-w-0 flex-[2] font-mono"
						autocomplete="off"
						spellcheck="false"
						maxlength="100"
						aria-label={m.settings_widgets_ai_person_id()}
						placeholder={m.settings_widgets_ai_person_id()}
						bind:value={person.id}
					/>
					<input
						type="text"
						class="input min-w-0 flex-1"
						autocomplete="off"
						maxlength="40"
						aria-label={m.settings_widgets_ai_person_name()}
						placeholder={m.settings_widgets_ai_person_name()}
						bind:value={person.name}
					/>
					<button
						type="button"
						class="grid size-10 shrink-0 place-items-center rounded-lg text-ink-2 hover:text-ink"
						aria-label={m.settings_widgets_remove({ name: person.name || person.id })}
						onclick={() => removePerson(kind, i)}
						disabled={saving}
					>
						<X class="size-4" aria-hidden="true" />
					</button>
				</li>
			{/each}
		</ul>
		<Button variant="secondary" size="sm" onclick={() => addPerson(kind)} disabled={saving || form.ai[kind].people.length >= MAX_PEOPLE}>
			<Plus class="size-4" aria-hidden="true" />
			{m.settings_widgets_ai_person_add()}
		</Button>
	</div>
{/snippet}

{#if !missing}
	<Panel id="widgets" title={m.settings_widgets_title()} description={m.settings_widgets_description()}>
		{#if loading}
			<Skeleton class="h-40 w-full" />
		{:else if error}
			<ErrorNotice {error} onretry={() => void load()} />
		{:else}
			<div class="space-y-6">
				<!-- Weather -->
				<div class="space-y-4">
					{@render header('widgets-weather', m.settings_widgets_weather(), m.settings_widgets_weather_hint(), {
						get: () => form.weather.enabled,
						set: (v) => (form.weather.enabled = v)
					})}
					<div class="grid gap-4 sm:grid-cols-[1fr_14rem] items-start">
						<Field label={m.settings_widgets_weather_search()} for="widgets-place">
							<div class="flex gap-2">
								<input
									id="widgets-place"
									type="search"
									class="input min-w-0 flex-1"
									maxlength="80"
									autocomplete="off"
									bind:value={placeQuery}
									onkeydown={(e) => {
										if (e.key === 'Enter') {
											e.preventDefault();
											void runSearch();
										}
									}}
								/>
								<Button variant="secondary" onclick={runSearch} disabled={placeQuery.trim().length < 2 || searching}>
									<Search class="size-4" aria-hidden="true" />
									{m.settings_widgets_weather_find()}
								</Button>
							</div>
						</Field>
						<Field label={m.settings_widgets_weather_units()} for="widgets-units">
							<select id="widgets-units" class="input" bind:value={form.weather.units}>
								<option value="metric">{m.settings_widgets_weather_metric()}</option>
								<option value="imperial">{m.settings_widgets_weather_imperial()}</option>
							</select>
						</Field>
					</div>
					<div role="status" aria-live="polite">
						{#if searching}
							<p class="text-[0.8125rem] text-ink-2">{m.settings_widgets_weather_searching()}</p>
						{:else if searchError}
							<p class="text-[0.8125rem] text-warning-ink">{searchError}</p>
						{:else if found && found.length === 0}
							<p class="text-[0.8125rem] text-ink-2">{m.settings_widgets_weather_none_found()}</p>
						{:else if found}
							<ul class="flex flex-wrap gap-2">
								{#each found as place (`${place.latitude},${place.longitude}`)}
									{@const label = place.region ? `${place.name} · ${place.region}` : place.name}
									<li>
										<Button
											variant="secondary"
											size="sm"
											class="min-h-10"
											onclick={() => choosePlace(place)}
											disabled={form.weather.places.some((p) => samePlace(p, place))}
											aria-label={m.settings_widgets_weather_add({ place: label })}
										>
											<Plus class="size-4" aria-hidden="true" />
											{label}
										</Button>
									</li>
								{/each}
							</ul>
						{/if}
					</div>
					<div>
						<p class="text-sm font-medium text-ink">{m.settings_widgets_weather_places()}</p>
						{#if form.weather.places.length === 0}
							<p class="mt-1 text-[0.8125rem] text-ink-2">{m.settings_widgets_weather_no_place()}</p>
						{:else}
							<ul class="mt-2 flex flex-wrap gap-2">
								{#each form.weather.places as place (`${place.latitude},${place.longitude}`)}
									<li class="flex min-h-10 min-w-0 max-w-full items-center gap-1 rounded-lg border border-line bg-surface pl-3 text-sm text-ink">
										<span class="min-w-0 truncate" title={place.region}>{place.name}</span>
										<button
											type="button"
											class="grid size-10 shrink-0 place-items-center rounded-lg text-ink-2 hover:text-ink"
											aria-label={m.settings_widgets_remove({ name: place.name })}
											onclick={() => removePlace(place)}
											disabled={saving}
										>
											<X class="size-4" aria-hidden="true" />
										</button>
									</li>
								{/each}
							</ul>
						{/if}
					</div>
				</div>

				<!-- News -->
				{#if newsAvailable}
					<div class="space-y-4 border-t border-line pt-5">
						{@render header('widgets-news', m.settings_news_ticker(), m.settings_news_ticker_hint(), {
							get: () => news.ticker,
							set: (v) => (news.ticker = v)
						})}
						<WallNewsFields bind:form={news} {saving} />
					</div>
				{/if}

				<!-- Free message -->
				<div class="space-y-4 border-t border-line pt-5">
					{@render header('widgets-text', m.settings_widgets_text(), m.settings_widgets_text_hint(), {
						get: () => form.text.enabled,
						set: (v) => (form.text.enabled = v)
					})}
					<Field label={m.settings_widgets_text_content()} for="widgets-text-content" help={m.settings_widgets_text_help({ count: form.text.content.length, max: MAX_TEXT })}>
						<input
							id="widgets-text-content"
							type="text"
							class="input"
							maxlength={MAX_TEXT}
							autocomplete="off"
							placeholder={m.settings_widgets_text_placeholder()}
							bind:value={form.text.content}
						/>
					</Field>
				</div>

				<!-- Coins -->
				<div class="space-y-4 border-t border-line pt-5">
					{@render header('widgets-crypto', m.settings_widgets_crypto(), m.settings_widgets_crypto_hint(), {
						get: () => form.crypto.enabled,
						set: (v) => (form.crypto.enabled = v)
					})}
					<fieldset class="min-w-0">
						<legend class="mb-1.5 text-sm font-medium text-ink">{m.settings_widgets_crypto_coins()}</legend>
						<ul class="flex flex-wrap gap-2">
							{#each catalog.coins as coin (coin.id)}
								{@const on = form.crypto.coins.includes(coin.id)}
								{@render chip(on, saving || (!on && coinsFull), coin.symbol === coin.name ? coin.symbol : `${coin.symbol} · ${coin.name}`, () => toggleCoin(coin.id))}
							{/each}
							{#each customCoins as id (id)}
								{@render chip(true, saving, id, () => toggleCoin(id))}
							{/each}
						</ul>
					</fieldset>
					<div class="grid gap-4 sm:grid-cols-2 items-start">
						<Field label={m.settings_widgets_crypto_custom()} for="widgets-coin" help={m.settings_widgets_crypto_custom_help()}>
							<div class="flex gap-2">
								<input id="widgets-coin" type="text" class="input min-w-0 flex-1 font-mono" maxlength="60" autocomplete="off" placeholder="avalanche-2" bind:value={customCoin} />
								<Button variant="secondary" onclick={addCustomCoin} disabled={!customCoin.trim() || coinsFull}>
									<Plus class="size-4" aria-hidden="true" />
									{m.settings_widgets_add()}
								</Button>
							</div>
						</Field>
						<Field label={m.settings_widgets_crypto_currency()} for="widgets-coin-currency">
							<select id="widgets-coin-currency" class="input" bind:value={form.crypto.currency}>
								{#each catalog.coin_currencies as code (code)}
									<option value={code}>{code.toUpperCase()} · {currencyName(code)}</option>
								{/each}
							</select>
						</Field>
					</div>
					{@render secret(
						'widgets-coingecko',
						m.settings_widgets_crypto_key(),
						m.settings_widgets_crypto_key_help(),
						hasCoingecko,
						coingeckoKey,
						(v) => (coingeckoKey = v)
					)}
				</div>

				<!-- Stocks -->
				<div class="space-y-4 border-t border-line pt-5">
					{@render header('widgets-stocks', m.settings_widgets_stocks(), m.settings_widgets_stocks_hint(), {
						get: () => form.stocks.enabled,
						set: (v) => (form.stocks.enabled = v)
					})}
					<Field label={m.settings_widgets_stocks_symbols()} for="widgets-symbols" help={m.settings_widgets_stocks_symbols_help()}>
						<input id="widgets-symbols" type="text" class="input font-mono" autocomplete="off" spellcheck="false" placeholder="AAPL, MSFT, NVDA" bind:value={symbolsText} />
					</Field>
					{@render secret(
						'widgets-finnhub',
						m.settings_widgets_stocks_key(),
						m.settings_widgets_stocks_key_help(),
						hasFinnhub,
						finnhubKey,
						(v) => (finnhubKey = v)
					)}
					{@render missingKey(keyed(form.stocks.enabled, hasFinnhub, finnhubKey))}
				</div>

				<!-- Exchange rates -->
				<div class="space-y-4 border-t border-line pt-5">
					{@render header('widgets-fx', m.settings_widgets_fx(), m.settings_widgets_fx_hint(), {
						get: () => form.fx.enabled,
						set: (v) => (form.fx.enabled = v)
					})}
					<div class="grid gap-4 sm:grid-cols-[minmax(0,14rem)_1fr] items-start">
						<Field label={m.settings_widgets_fx_base()} for="widgets-fx-base">
							<select id="widgets-fx-base" class="input" bind:value={form.fx.base}>
								{#each catalog.fx_currencies as code (code)}
									<option value={code}>{code} · {currencyName(code)}</option>
								{/each}
							</select>
						</Field>
						<fieldset class="min-w-0">
							<legend class="mb-1.5 text-sm font-medium text-ink">{m.settings_widgets_fx_currencies()}</legend>
							<ul class="flex flex-wrap gap-2">
								{#each catalog.fx_currencies.filter((code) => code !== form.fx.base) as code (code)}
									{@const on = form.fx.currencies.includes(code)}
									{@render chip(on, saving || (!on && fxFull), code, () => toggleCurrency(code))}
								{/each}
							</ul>
						</fieldset>
					</div>
				</div>

				<!-- Vulnerabilities -->
				<div class="space-y-4 border-t border-line pt-5">
					{@render header('widgets-cve', m.settings_widgets_cve(), m.settings_widgets_cve_hint(), {
						get: () => form.cve.enabled,
						set: (v) => (form.cve.enabled = v)
					})}
					<div class="grid gap-4 sm:grid-cols-[1fr_10rem] items-start">
						<Field label={m.settings_widgets_cve_source()} for="widgets-cve-source">
							<select id="widgets-cve-source" class="input" bind:value={form.cve.source}>
								<option value="both">{m.settings_widgets_cve_source_both()}</option>
								<option value="nvd">{m.settings_widgets_cve_source_nvd()}</option>
								<option value="kev">{m.settings_widgets_cve_source_kev()}</option>
							</select>
						</Field>
						<Field label={m.settings_widgets_cve_count()} for="widgets-cve-count">
							<input id="widgets-cve-count" type="number" class="input" min="1" max="10" step="1" bind:value={form.cve.count} />
						</Field>
					</div>
				</div>

				<!-- AI usage -->
				<div class="space-y-4 border-t border-line pt-5">
					<div class="min-w-0">
						<h3 class="text-sm font-semibold text-ink">{m.settings_widgets_ai()}</h3>
						<p class="text-[0.8125rem] text-ink-2">{m.settings_widgets_ai_hint()}</p>
					</div>
					<Field label={m.settings_widgets_ai_period()} for="widgets-ai-period">
						<select id="widgets-ai-period" class="input max-w-64" bind:value={form.ai.period}>
							<option value="today">{m.wall_widgets_ai_today()}</option>
							<option value="week">{m.wall_widgets_ai_week()}</option>
							<option value="month">{m.wall_widgets_ai_month()}</option>
						</select>
					</Field>
					{@render header('widgets-ai-by-person', m.settings_widgets_ai_by_person(), m.settings_widgets_ai_by_person_help(), {
						get: () => form.ai.by_person,
						set: (v) => (form.ai.by_person = v)
					})}

					<div class="space-y-4 rounded-lg border border-line p-4">
						{@render header('widgets-ai-claude', m.settings_widgets_ai_claude(), m.settings_widgets_ai_claude_hint(), {
							get: () => form.ai.claude.enabled,
							set: (v) => (form.ai.claude.enabled = v)
						})}
						{@render secret(
							'widgets-anthropic',
							m.settings_widgets_ai_claude_key(),
							m.settings_widgets_ai_claude_key_help(),
							hasAnthropic,
							anthropicKey,
							(v) => (anthropicKey = v),
			'ai-admin-key-claude'
						)}
						{@render missingKey(keyed(form.ai.claude.enabled, hasAnthropic, anthropicKey))}
						{#if form.ai.by_person}{@render people('claude')}{/if}
					</div>

					<div class="space-y-4 rounded-lg border border-line p-4">
						{@render header('widgets-ai-openai', m.settings_widgets_ai_openai(), m.settings_widgets_ai_openai_hint(), {
							get: () => form.ai.openai.enabled,
							set: (v) => (form.ai.openai.enabled = v)
						})}
						{@render secret(
							'widgets-openai',
							m.settings_widgets_ai_openai_key(),
							m.settings_widgets_ai_openai_key_help(),
							hasOpenai,
							openaiKey,
							(v) => (openaiKey = v),
			'ai-admin-key-openai'
						)}
						{@render missingKey(keyed(form.ai.openai.enabled, hasOpenai, openaiKey))}
						{#if form.ai.by_person}{@render people('openai')}{/if}
					</div>
				</div>

				<!-- Layout -->
				<div class="space-y-3 border-t border-line pt-5">
					<div class="min-w-0">
						<h3 class="text-sm font-semibold text-ink">{m.settings_widgets_layout()}</h3>
						<p class="text-[0.8125rem] text-ink-2">{m.settings_widgets_layout_hint()}</p>
					</div>
					<WallLayoutEditor
						bind:layout={form.layout}
						disabled={saving}
						states={{
							weather: form.weather.enabled ? 'on' : 'off',
							news: news.ticker ? 'on' : 'off',
							text: form.text.enabled ? 'on' : 'off',
							crypto: form.crypto.enabled ? 'on' : 'off',
							stocks: keyed(form.stocks.enabled, hasFinnhub, finnhubKey),
							fx: form.fx.enabled ? 'on' : 'off',
							cve: form.cve.enabled ? 'on' : 'off',
							claude: keyed(form.ai.claude.enabled, hasAnthropic, anthropicKey),
							openai: keyed(form.ai.openai.enabled, hasOpenai, openaiKey)
						}}
					/>
				</div>

				<div class="flex flex-wrap items-center gap-3 border-t border-line pt-4">
					<Button onclick={save} disabled={saving}>{m.settings_widgets_save()}</Button>
					<UnsavedBadge dirty={tracker.dirty} />
					<p class="min-h-5 text-sm" role="status" aria-live="polite">
						{#if problem}<span class="text-warning-ink">{problem}</span>{:else if notice}<span class="text-signal-ink">{notice}</span>{/if}
					</p>
				</div>
			</div>
		{/if}
	</Panel>
{/if}
