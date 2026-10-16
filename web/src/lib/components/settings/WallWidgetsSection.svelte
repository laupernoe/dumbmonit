<script lang="ts">
	/**
	 * Settings → Wall widgets (admins). Off by default. Five independent
	 * widgets, each with its toggle and its few choices, and the seconds each
	 * stays on the wall before the next:
	 *
	 * - coins (CoinGecko): pick among the main ones, or add any CoinGecko id;
	 *   display currency; an optional free Demo key;
	 * - weather (Open-Meteo): find a city, keep up to four; units;
	 * - stocks (Finnhub): ticker symbols and the free key they need;
	 * - exchange rates (Frankfurter): base currency and the ones to show;
	 * - critical vulnerabilities (NVD, CISA KEV): which list, how many.
	 *
	 * The server reads every source and caches it; nothing here talks to them
	 * except the city search, which also goes through the server. API keys are
	 * write-only: the form only learns whether one is saved.
	 */
	import { Plus, Search, X } from 'lucide-svelte';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	import { ApiError } from '#lib/api/index.js';
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
	import { useDirtyTracker } from '#lib/settings/unsaved.svelte.js';

	const MAX_COINS = 10;
	const MAX_PLACES = 4;
	const MAX_CURRENCIES = 10;

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
	/** Keys typed in this visit; `null` keeps the saved one, `''` clears it. */
	let finnhubKey = $state<string | null>(null);
	let coingeckoKey = $state<string | null>(null);
	let symbolsText = $state('');

	const tracker = useDirtyTracker(() => ({ form, symbolsText, finnhubKey, coingeckoKey }));

	let customCoin = $state('');
	let placeQuery = $state('');
	let searching = $state(false);
	let found = $state<WallPlace[] | null>(null);
	let searchError = $state<string | null>(null);

	function adopt(saved: WallWidgetSettings) {
		form = structuredClone({
			rotate_seconds: saved.rotate_seconds,
			crypto: saved.crypto,
			weather: saved.weather,
			stocks: saved.stocks,
			fx: saved.fx,
			cve: saved.cve
		});
		symbolsText = saved.stocks.symbols.join(', ');
		finnhubKey = null;
		coingeckoKey = null;
	}

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const [saved, offered] = await Promise.all([getWidgetSettings(signal), getWidgetCatalog(signal)]);
			adopt(saved);
			hasFinnhub = saved.has_finnhub_key;
			hasCoingecko = saved.has_coingecko_key;
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

	function addPlace(place: WallPlace) {
		if (form.weather.places.length >= MAX_PLACES || form.weather.places.some((p) => samePlace(p, place))) return;
		form.weather.places = [...form.weather.places, place];
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
		const secrets: { finnhub?: string; coingecko?: string } = {};
		if (finnhubKey !== null) secrets.finnhub = finnhubKey.trim();
		if (coingeckoKey !== null) secrets.coingecko = coingeckoKey.trim();
		if (Object.keys(secrets).length) payload.secrets = secrets;
		try {
			const saved = await saveWidgetSettings(payload);
			adopt(saved);
			hasFinnhub = saved.has_finnhub_key;
			hasCoingecko = saved.has_coingecko_key;
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

{#snippet secret(id: string, label: string, help: string, saved: boolean, value: string | null, set: (v: string | null) => void)}
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
	</Field>
{/snippet}

{#if !missing}
	<Panel id="widgets" title={m.settings_widgets_title()} description={m.settings_widgets_description()}>
		{#if loading}
			<Skeleton class="h-40 w-full" />
		{:else if error}
			<ErrorNotice {error} onretry={() => void load()} />
		{:else}
			<div class="space-y-6">
				<Field label={m.settings_widgets_rotate()} for="widgets-rotate" help={m.settings_widgets_rotate_help()}>
					<input id="widgets-rotate" type="number" class="input max-w-32" min="5" max="300" step="1" bind:value={form.rotate_seconds} />
				</Field>

				<!-- Weather -->
				<div class="space-y-4 border-t border-line pt-5">
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
											onclick={() => addPlace(place)}
											disabled={form.weather.places.length >= MAX_PLACES || form.weather.places.some((p) => samePlace(p, place))}
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
						{#if form.weather.places.length >= MAX_PLACES}
							<p class="mt-1 text-[0.8125rem] text-ink-2">{m.settings_widgets_weather_limit({ count: MAX_PLACES })}</p>
						{/if}
					</div>
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
