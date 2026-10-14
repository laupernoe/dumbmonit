<script lang="ts">
	/**
	 * Settings → Wall news (admins). Off by default. Two independent parts:
	 *
	 * - the headline ticker: pick a country, tick its sources (the presets are
	 *   served by the server, each feed checked by hand), and/or add any RSS or
	 *   Atom address — "Test" reads it for real and shows the first headline;
	 * - the live tile: a country's official channel, or any YouTube,
	 *   Dailymotion or Twitch link (the server turns a normal address into the
	 *   embed). Position and size of the tile.
	 *
	 * The server reads the feeds (CORS, address guard, size and time limits,
	 * five-minute cache); nothing here talks to a news site.
	 */
	import { Plus, X } from 'lucide-svelte';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	import { ApiError } from '#lib/api/index.js';
	import {
		DEFAULT_NEWS_SETTINGS,
		getNewsPresets,
		getNewsSettings,
		saveNewsSettings,
		testNewsFeed,
		type FeedTest,
		type NewsCountry,
		type NewsFeed,
		type NewsSettings
	} from '#lib/api/wallNews.js';
	import { Button, ErrorNotice, Field, Panel, Skeleton, Toggle } from '#lib/ui/index.js';

	const MAX_FEEDS = 8;
	const CUSTOM = '__custom';

	let loading = $state(true);
	let error = $state<unknown>(null);
	let missing = $state(false);
	let saving = $state(false);
	let notice = $state<string | null>(null);
	let problem = $state<string | null>(null);

	let form = $state<NewsSettings>({ ...DEFAULT_NEWS_SETTINGS, feeds: [] });
	let countries = $state<NewsCountry[]>([]);
	let country = $state('INT');

	let customUrl = $state('');
	let customName = $state('');
	let testing = $state(false);
	let test = $state<FeedTest | null>(null);
	let testError = $state<string | null>(null);

	/** The channel select: a preset's address, `CUSTOM`, or empty. */
	let channel = $state('');

	/** Locale → likely country, when the browser gives no region. */
	const LOCALE_COUNTRY: Record<string, string> = {
		fr: 'FR',
		de: 'DE',
		es: 'ES',
		it: 'IT',
		pt: 'PT',
		'pt-BR': 'BR',
		ru: 'RU',
		'zh-Hans': 'CN',
		en: 'GB'
	};

	function guessCountry(known: Set<string>): string {
		for (const tag of [...(navigator.languages ?? []), navigator.language]) {
			const region = tag?.split('-').find((part, i) => i > 0 && /^[A-Za-z]{2}$/.test(part))?.toUpperCase();
			if (region && known.has(region)) return region;
		}
		const fromLocale = LOCALE_COUNTRY[getLocale()];
		return fromLocale && known.has(fromLocale) ? fromLocale : 'INT';
	}

	function countryName(code: string): string {
		if (code === 'INT') return m.settings_news_country_international();
		try {
			return new Intl.DisplayNames([getLocale()], { type: 'region' }).of(code) ?? code;
		} catch {
			return code;
		}
	}

	const sortedCountries = $derived(
		[...countries].sort((a, b) => {
			if (a.code === 'INT') return 1;
			if (b.code === 'INT') return -1;
			return countryName(a.code).localeCompare(countryName(b.code), getLocale());
		})
	);
	const current = $derived(countries.find((c) => c.code === country) ?? null);
	const allVideos = $derived(countries.flatMap((c) => c.videos));

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const [saved, presets] = await Promise.all([getNewsSettings(signal), getNewsPresets(signal)]);
			form = { ...saved, feeds: saved.feeds.map((feed) => ({ ...feed })) };
			countries = presets;
			country = guessCountry(new Set(presets.map((c) => c.code)));
			channel = !saved.video_url ? '' : allVideos.some((v) => v.url === saved.video_url) ? saved.video_url : CUSTOM;
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

	const hasFeed = (url: string) => form.feeds.some((feed) => feed.url === url);
	const full = $derived(form.feeds.length >= MAX_FEEDS);

	function togglePreset(preset: { label: string; url: string }) {
		notice = null;
		form.feeds = hasFeed(preset.url)
			? form.feeds.filter((feed) => feed.url !== preset.url)
			: [...form.feeds, { url: preset.url, label: preset.label }];
	}

	function remove(feed: NewsFeed) {
		notice = null;
		form.feeds = form.feeds.filter((other) => other.url !== feed.url);
	}

	async function runTest() {
		const url = customUrl.trim();
		if (!url) return;
		testing = true;
		test = null;
		testError = null;
		try {
			test = await testNewsFeed(url);
		} catch (cause) {
			testError = cause instanceof Error ? cause.message : String(cause);
		} finally {
			testing = false;
		}
	}

	function addCustom() {
		const url = customUrl.trim();
		if (!url || full || hasFeed(url)) return;
		form.feeds = [...form.feeds, { url, label: customName.trim() }];
		customUrl = '';
		customName = '';
		test = null;
		testError = null;
		notice = null;
	}

	function chooseChannel(value: string) {
		channel = value;
		notice = null;
		if (value !== CUSTOM) form.video_url = value;
		else if (allVideos.some((v) => v.url === form.video_url)) form.video_url = '';
	}

	async function save() {
		saving = true;
		problem = null;
		notice = null;
		try {
			const saved = await saveNewsSettings(form);
			form = { ...saved, feeds: saved.feeds.map((feed) => ({ ...feed })) };
			notice = m.settings_news_saved();
		} catch (cause) {
			problem = cause instanceof Error ? cause.message : String(cause);
		} finally {
			saving = false;
		}
	}

	function hostOf(url: string): string {
		try {
			return new URL(url).hostname.replace(/^www\./, '');
		} catch {
			return url;
		}
	}
</script>

{#if !missing}
	<Panel id="news" title={m.settings_news_title()} description={m.settings_news_description()}>
		{#if loading}
			<Skeleton class="h-40 w-full" />
		{:else if error}
			<ErrorNotice {error} onretry={() => void load()} />
		{:else}
			<div class="space-y-6">
				<!-- Headline ticker -->
				<div class="space-y-4">
					<div class="flex items-start justify-between gap-4">
						<div class="min-w-0">
							<label for="news-ticker" class="text-sm font-semibold text-ink">{m.settings_news_ticker()}</label>
							<p class="text-[0.8125rem] text-ink-2">{m.settings_news_ticker_hint()}</p>
						</div>
						<Toggle id="news-ticker" bind:checked={form.ticker} disabled={saving} />
					</div>

					<div class="grid gap-4 sm:grid-cols-[minmax(0,14rem)_1fr]">
						<Field label={m.settings_news_country()} for="news-country">
							<select id="news-country" class="input" bind:value={country}>
								{#each sortedCountries as option (option.code)}
									<option value={option.code}>{countryName(option.code)}</option>
								{/each}
							</select>
						</Field>
						<fieldset class="min-w-0">
							<legend class="mb-1.5 text-sm font-medium text-ink">{m.settings_news_sources()}</legend>
							<ul class="flex flex-wrap gap-2">
								{#each current?.feeds ?? [] as preset (preset.url)}
									{@const on = hasFeed(preset.url)}
									<li>
										<label
											class="flex min-h-10 cursor-pointer items-center gap-2 rounded-lg border px-3 text-sm {on
												? 'border-signal bg-signal-soft text-signal-ink'
												: 'border-line bg-surface text-ink'} {!on && full ? 'opacity-50' : ''}"
										>
											<input
												type="checkbox"
												class="size-4 accent-[var(--c-signal)]"
												checked={on}
												disabled={saving || (!on && full)}
												onchange={() => togglePreset(preset)}
											/>
											{preset.label}
										</label>
									</li>
								{/each}
							</ul>
						</fieldset>
					</div>

					<div class="rounded-xl border border-line bg-surface-2/50 p-4">
						<p class="text-sm font-semibold text-ink">{m.settings_news_custom_title()}</p>
						<div class="mt-3 grid gap-3 sm:grid-cols-[1fr_12rem]">
							<Field label={m.settings_news_custom_url()} for="news-custom-url">
								<input
									id="news-custom-url"
									type="url"
									class="input font-mono"
									placeholder="https://…/rss.xml"
									bind:value={customUrl}
									maxlength="500"
									autocomplete="off"
									oninput={() => {
										test = null;
										testError = null;
									}}
								/>
							</Field>
							<Field label={m.settings_news_custom_name()} for="news-custom-name">
								<input id="news-custom-name" type="text" class="input" bind:value={customName} maxlength="40" autocomplete="off" />
							</Field>
						</div>
						<div class="mt-3 flex flex-wrap items-center gap-2">
							<Button variant="secondary" size="sm" onclick={runTest} disabled={!customUrl.trim() || testing}>
								{m.settings_news_test()}
							</Button>
							<Button size="sm" onclick={addCustom} disabled={!customUrl.trim() || full || hasFeed(customUrl.trim())}>
								<Plus class="size-4" aria-hidden="true" />
								{m.settings_news_custom_add()}
							</Button>
							{#if full}<span class="text-[0.8125rem] text-ink-2">{m.settings_news_limit({ count: MAX_FEEDS })}</span>{/if}
						</div>
						<p class="mt-2 min-h-5 text-[0.8125rem] text-ink-2" role="status" aria-live="polite">
							{#if testing}
								{m.settings_news_testing()}
							{:else if test?.ok && test.first}
								{m.settings_news_test_ok({ title: test.first.title })}
							{:else if test}
								<span class="text-warning-ink">{m.settings_news_test_fail({ error: test.error ?? '' })}</span>
							{:else if testError}
								<span class="text-warning-ink">{m.settings_news_test_fail({ error: testError })}</span>
							{/if}
						</p>
					</div>

					<div>
						<p class="text-sm font-medium text-ink">{m.settings_news_selected()}</p>
						{#if form.feeds.length === 0}
							<p class="mt-1 text-[0.8125rem] text-ink-2">{m.settings_news_none()}</p>
						{:else}
							<ul class="mt-2 flex flex-wrap gap-2">
								{#each form.feeds as feed (feed.url)}
									{@const name = feed.label || hostOf(feed.url)}
									<li class="flex min-h-10 min-w-0 max-w-full items-center gap-1 rounded-lg border border-line bg-surface pl-3 text-sm text-ink">
										<span class="min-w-0 truncate" title={feed.url}>{name}</span>
										<button
											type="button"
											class="grid size-10 shrink-0 place-items-center rounded-lg text-ink-2 hover:text-ink"
											aria-label={m.settings_news_remove({ name })}
											onclick={() => remove(feed)}
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

				<!-- Live video -->
				<div class="space-y-4 border-t border-line pt-5">
					<div class="flex items-start justify-between gap-4">
						<div class="min-w-0">
							<label for="news-video" class="text-sm font-semibold text-ink">{m.settings_news_video()}</label>
							<p class="text-[0.8125rem] text-ink-2">{m.settings_news_video_hint()}</p>
						</div>
						<Toggle id="news-video" bind:checked={form.video} disabled={saving} />
					</div>

					<div class="grid gap-4 sm:grid-cols-2">
						<Field label={m.settings_news_video_channel()} for="news-channel">
							<select id="news-channel" class="input" value={channel} onchange={(e) => chooseChannel(e.currentTarget.value)}>
								<option value="">{m.settings_news_video_choose()}</option>
								{#each current?.videos ?? [] as video (video.url)}
									<option value={video.url}>{video.label}</option>
								{/each}
								{#if channel && channel !== CUSTOM && !(current?.videos ?? []).some((v) => v.url === channel)}
									<option value={channel}>{allVideos.find((v) => v.url === channel)?.label ?? channel}</option>
								{/if}
								<option value={CUSTOM}>{m.settings_news_video_custom()}</option>
							</select>
						</Field>
						{#if channel === CUSTOM}
							<Field label={m.settings_news_video_url()} for="news-video-url" help={m.settings_news_video_url_help()}>
								<input
									id="news-video-url"
									type="url"
									class="input font-mono"
									placeholder="https://www.youtube.com/watch?v=…"
									bind:value={form.video_url}
									maxlength="500"
									autocomplete="off"
								/>
							</Field>
						{/if}
					</div>

					<div class="grid gap-4 sm:grid-cols-2">
						<Field label={m.settings_news_position()} for="news-position">
							<select id="news-position" class="input" bind:value={form.video_position}>
								<option value="right">{m.settings_news_position_right()}</option>
								<option value="center">{m.settings_news_position_center()}</option>
							</select>
						</Field>
						<Field label={m.settings_news_size()} for="news-size">
							<select id="news-size" class="input" bind:value={form.video_size}>
								<option value="small">{m.settings_news_size_small()}</option>
								<option value="medium">{m.settings_news_size_medium()}</option>
								<option value="large">{m.settings_news_size_large()}</option>
							</select>
						</Field>
					</div>
				</div>

				<div class="flex flex-wrap items-center gap-3 border-t border-line pt-4">
					<Button onclick={save} disabled={saving}>{m.settings_news_save()}</Button>
					<p class="min-h-5 text-sm" role="status" aria-live="polite">
						{#if problem}<span class="text-warning-ink">{problem}</span>{:else if notice}<span class="text-signal-ink">{notice}</span>{/if}
					</p>
				</div>
			</div>
		{/if}
	</Panel>
{/if}
