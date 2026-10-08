<script lang="ts">
	/**
	 * Settings → Wall music. Two ways to play music on the wall display:
	 *
	 * - Spotify Connect: the wall becomes a speaker ("DumbMonit Wall") you pick
	 *   in the Spotify app on your phone, and it shows what plays on the
	 *   account wherever it plays. Needs a Spotify Developer app of your own
	 *   (its Client ID; Authorization Code + PKCE, no secret) and Premium.
	 * - A link (Spotify, Deezer, YouTube) sent to every wall, played in the
	 *   service's own player — for Deezer and YouTube, which have no
	 *   Connect-style remote on the web, and for accounts without Premium.
	 *
	 * Connecting goes back to this page by itself when DumbMonit is served over
	 * HTTPS; over plain HTTP Spotify only accepts a 127.0.0.1 redirect, so the
	 * address the browser lands on is pasted back here. Tokens never reach the
	 * browser; admins only for every change.
	 */
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { ExternalLink, Link2, Music2 } from 'lucide-svelte';
	import {
		completeSpotifyConnection,
		disconnectSpotify,
		getSpotifyAccount,
		getWallMusic,
		playOnWall,
		startSpotifyConnection,
		stopWallLink,
		type SpotifyAccount,
		type SpotifyAuthorization,
		type WallMusic
	} from '#lib/api/music.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, CopyBlock, ErrorNotice, Field, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { formatRelative, parseServerDate } from '#lib/format.js';
	import { parseMusicLink } from '#lib/wall/music.js';
	import { musicErrorText } from '#lib/wall/messages.js';
	import SpeakerStatus from './SpeakerStatus.svelte';
	import { m } from '#lib/paraglide/messages.js';

	let account = $state<SpotifyAccount | null>(null);
	let wall = $state<WallMusic | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let missing = $state(false);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const [nextAccount, nextWall] = await Promise.all([getSpotifyAccount(signal), getWallMusic(signal)]);
			account = nextAccount;
			wall = nextWall;
			if (!clientId && nextAccount.client_id) clientId = nextAccount.client_id;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			if (cause && typeof cause === 'object' && 'missing' in cause && cause.missing) missing = true;
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

	// --- Coming back from Spotify (`/settings?spotify=<outcome>#music`) ---------

	const OUTCOME: Record<string, () => string> = {
		connected: () => m.settings_music_outcome_connected(),
		denied: () => m.settings_music_outcome_denied(),
		state: () => m.settings_music_outcome_state(),
		session: () => m.settings_music_outcome_session(),
		exchange: () => m.settings_music_outcome_exchange(),
		internal: () => m.settings_music_outcome_internal()
	};
	let outcome = $state<string | null>(null);
	$effect(() => {
		const value = page.url.searchParams.get('spotify');
		if (!value) return;
		outcome = value;
		// Forget it in the address bar: a reload should not say it again.
		const url = new URL(page.url.href);
		url.searchParams.delete('spotify');
		void goto(url, { shallow: true, replace: true, state: page.state });
	});

	// --- Connecting ----------------------------------------------------------

	let clientId = $state('');
	let clientIdError = $state<string | null>(null);
	let starting = $state(false);
	let startError = $state<unknown>(null);
	let authorization = $state<SpotifyAuthorization | null>(null);
	let landing = $state('');
	let finishing = $state(false);
	let finishError = $state<unknown>(null);

	const secure = typeof window !== 'undefined' && window.location.protocol === 'https:';
	/** Over HTTPS Spotify comes straight back here; over HTTP only 127.0.0.1 is allowed. */
	const redirectUri = $derived(
		account ? (secure ? `${window.location.origin}${account.callback_path}` : account.loopback_redirect_uri) : ''
	);

	async function connect(event: SubmitEvent) {
		event.preventDefault();
		startError = null;
		const id = clientId.trim();
		if (!/^[A-Za-z0-9]{16,64}$/.test(id)) {
			clientIdError = m.settings_music_err_client_id();
			return;
		}
		clientIdError = null;
		starting = true;
		try {
			const next = await startSpotifyConnection(id, redirectUri);
			if (secure) {
				window.location.assign(next.authorize_url);
				return;
			}
			authorization = next;
			landing = '';
		} catch (cause) {
			startError = cause;
		} finally {
			starting = false;
		}
	}

	async function finish(event: SubmitEvent) {
		event.preventDefault();
		finishError = null;
		finishing = true;
		try {
			account = await completeSpotifyConnection(landing.trim());
			authorization = null;
			outcome = 'connected';
			wall = await getWallMusic().catch(() => wall);
		} catch (cause) {
			finishError = cause;
		} finally {
			finishing = false;
		}
	}

	let disconnecting = $state(false);
	let disconnectError = $state<unknown>(null);
	async function disconnect() {
		disconnecting = true;
		disconnectError = null;
		try {
			await disconnectSpotify();
			outcome = null;
			await load();
		} catch (cause) {
			disconnectError = cause;
		} finally {
			disconnecting = false;
		}
	}

	// --- A link for every wall -------------------------------------------------

	let link = $state('');
	let linkError = $state<string | null>(null);
	let sending = $state(false);
	let sendError = $state<unknown>(null);

	async function send(event: SubmitEvent) {
		event.preventDefault();
		sendError = null;
		const parsed = parseMusicLink(link);
		if (!parsed.ok) {
			linkError = musicErrorText(parsed.code);
			return;
		}
		linkError = null;
		sending = true;
		try {
			const saved = await playOnWall(link.trim());
			if (wall) wall = { ...wall, link: saved };
			link = '';
		} catch (cause) {
			sendError = cause;
		} finally {
			sending = false;
		}
	}

	let stopping = $state(false);
	async function stop() {
		stopping = true;
		sendError = null;
		try {
			await stopWallLink();
			if (wall) wall = { ...wall, link: null };
		} catch (cause) {
			sendError = cause;
		} finally {
			stopping = false;
		}
	}

	const current = $derived.by(() => {
		const value = wall?.link?.link;
		if (!value) return null;
		const parsed = parseMusicLink(value);
		return parsed.ok ? parsed.embed : null;
	});
	const playingNow = $derived(wall?.spotify.now_playing ?? null);

	function serverDay(value: string | null): string {
		const date = parseServerDate(value);
		return date ? date.toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' }) : '';
	}
</script>

<Panel
	id="music"
	title={m.settings_music_title()}
	description={m.settings_music_description()}
>
	{#snippet aside()}
		{#if account?.status === 'connected'}
			<Plate tone="signal" label={m.settings_music_plate_connected()} />
		{:else if account?.status === 'expired'}
			<Plate tone="advisory" label={m.settings_music_plate_reconnect()} />
		{:else if account}
			<Plate tone="ghost" label={m.settings_music_plate_off()} />
		{/if}
	{/snippet}

	{#if missing}
		<p class="text-sm text-ink-2">{m.settings_music_missing()}</p>
	{:else if error}
		<ErrorNotice {error} title={m.settings_music_load_error()} onretry={() => void load()} />
	{:else if loading || !account}
		<Skeleton class="h-10 w-full" rows={5} />
	{:else}
		<div class="grid gap-6">
			{#if outcome}
				<div aria-live="polite">
					{#if outcome === 'connected'}
						<Plate tone="signal" label={OUTCOME.connected()} size="md" />
					{:else}
						<ErrorNotice
							title={m.settings_music_not_connected()}
							error={new Error(OUTCOME[outcome]?.() ?? m.settings_music_outcome_unknown())}
							hint={m.settings_music_outcome_hint()}
						/>
					{/if}
				</div>
			{/if}

			<!-- Spotify Connect -->
			<section aria-labelledby="music-spotify">
				<h3 id="music-spotify" class="text-sm font-semibold text-ink">{m.settings_music_connect_heading()}</h3>
				<p class="mt-1 max-w-prose text-sm text-ink-2">
					{m.settings_music_connect_intro({ name: account.speaker_name })}
				</p>

				{#if account.status === 'connected'}
					<dl class="mt-3 grid gap-x-4 gap-y-1.5 text-sm sm:grid-cols-[auto_minmax(0,1fr)]">
						<dt class="text-ink-2">{m.settings_music_account()}</dt>
						<dd class="text-ink">{account.account_name ?? m.settings_music_connected_default()}</dd>
						<dt class="text-ink-2">{m.settings_music_app()}</dt>
						<dd class="min-w-0 truncate font-mono text-[0.8125rem] text-ink">{account.client_id}</dd>
						{#if account.connected_at}
							<dt class="text-ink-2">{m.settings_music_connected_at()}</dt>
							<dd class="text-ink">{formatRelative(account.connected_at)}</dd>
						{/if}
						{#if account.reconnect_by}
							<dt class="text-ink-2">{m.settings_music_reconnect_by()}</dt>
							<dd class="text-ink">{serverDay(account.reconnect_by)} <span class="text-ink-2">— {m.settings_music_reconnect_note()}</span></dd>
						{/if}
						<dt class="text-ink-2">{m.settings_music_right_now()}</dt>
						<dd class="min-w-0 text-ink">
							{#if playingNow}
								{m.settings_music_now_line({
									status: playingNow.playing ? m.settings_music_status_playing() : m.settings_music_status_paused(),
									title: playingNow.title,
									artists: playingNow.artists.length ? m.settings_music_now_artists({ artists: playingNow.artists.join(', ') }) : '',
									device: playingNow.device_name ? m.settings_music_now_device({ device: playingNow.device_name }) : ''
								})}
							{:else if wall?.spotify.error}
								<span class="text-advisory-ink">{wall.spotify.error}</span>
							{:else}
								{m.settings_music_nothing_playing()}
							{/if}
						</dd>
					</dl>
					{#if account.missing_scopes.length > 0}
						<p class="mt-3 text-sm text-advisory-ink">
							{m.settings_music_missing_scopes({ scopes: account.missing_scopes.join(', ') })}
						</p>
					{/if}
					<SpeakerStatus onrenamed={(name) => account && (account = { ...account, speaker_name: name })} />
					{#if auth.isAdmin}
						<div class="mt-4 flex flex-wrap items-center gap-2">
							<Confirm variant="secondary" size="md" confirmLabel={m.settings_music_disconnect_confirm()} loading={disconnecting} onconfirm={disconnect}>
								{m.settings_music_disconnect()}
							</Confirm>
						</div>
						{#if disconnectError}
							<ErrorNotice class="mt-3" error={disconnectError} title={m.settings_music_disconnect_error()} />
						{/if}
					{/if}
				{:else if account.status === 'expired'}
					<p class="mt-3 text-sm text-advisory-ink">
						{account.last_error ? m.settings_music_expired_reason({ reason: account.last_error }) : m.settings_music_expired()}
					</p>
				{/if}

				{#if account.status !== 'connected'}
					{#if !auth.isAdmin}
						<p class="mt-3 text-sm text-ink-2">{m.settings_music_admin_only()}</p>
					{:else}
						<ol class="mt-4 grid gap-4 text-sm text-ink">
							<li class="grid gap-1.5">
								<p>
									<span class="font-semibold">{m.settings_music_step1_before()}</span>
									<a class="inline-flex items-center gap-1 font-semibold underline-offset-2 hover:underline" href="https://developer.spotify.com/dashboard" target="_blank" rel="noopener noreferrer">
										{m.settings_music_step1_link()}<ExternalLink class="size-3.5" aria-hidden="true" />
									</a>
									{m.settings_music_step1_after()}
								</p>
								<CopyBlock value={redirectUri} label={m.settings_music_copy_redirect()} />
								<p class="text-[0.8125rem] text-ink-2">
									{#if secure}
										{m.settings_music_redirect_secure()}
									{:else}
										{m.settings_music_redirect_insecure()}
									{/if}
								</p>
							</li>
							<li>
								<form class="grid gap-2" onsubmit={connect} novalidate>
									<Field label={m.settings_music_step2_label()} for="music-client-id" error={clientIdError} help={m.settings_music_step2_help()}>
										<input
											id="music-client-id"
											class="input font-mono"
											bind:value={clientId}
											autocomplete="off"
											spellcheck="false"
											placeholder="0123456789abcdef0123456789abcdef"
											disabled={starting}
											aria-invalid={clientIdError ? 'true' : undefined}
											oninput={() => (clientIdError = null)}
										/>
									</Field>
									<div>
										<Button type="submit" variant="primary" loading={starting}>
											<Music2 class="size-4" aria-hidden="true" />
											{m.settings_music_connect_button()}
										</Button>
									</div>
									{#if startError}
										<ErrorNotice error={startError} title={m.settings_music_start_error()} />
									{/if}
								</form>
							</li>
							{#if authorization}
								<li class="grid gap-2 rounded-[var(--radius-card)] border border-line bg-canvas-deep p-4">
									<p>
										<span class="font-semibold">{m.settings_music_step3()}</span>
									</p>
									<div>
										<Button variant="secondary" href={authorization.authorize_url} target="_blank" rel="noopener noreferrer">
											{m.settings_music_open_spotify()}
											<ExternalLink class="size-4" aria-hidden="true" />
										</Button>
									</div>
									<form class="grid gap-2" onsubmit={finish} novalidate>
										<Field
											label={m.settings_music_step4_label()}
											for="music-landing"
											help={m.settings_music_step4_help({ uri: authorization.redirect_uri })}
										>
											<input
												id="music-landing"
												class="input font-mono text-[0.8125rem]"
												bind:value={landing}
												autocomplete="off"
												spellcheck="false"
												placeholder="{authorization.redirect_uri}?code=…&state=…"
												disabled={finishing}
											/>
										</Field>
										<div>
											<Button type="submit" variant="secondary" loading={finishing} disabled={!landing.trim()}>{m.settings_music_finish()}</Button>
										</div>
										{#if finishError}
											<ErrorNotice error={finishError} title={m.settings_music_finish_error()} />
										{/if}
									</form>
								</li>
							{/if}
						</ol>
					{/if}
				{/if}

				<details class="mt-4 rounded-[var(--radius-card)] border border-line">
					<summary class="cursor-pointer px-4 py-3 text-sm font-semibold text-ink select-none">{m.settings_music_needs_heading()}</summary>
					<ul class="grid list-disc gap-1.5 border-t border-line py-3 pr-4 pl-8 text-[0.8125rem] text-ink-2">
						<li>{m.settings_music_needs_1()}</li>
						<li>{m.settings_music_needs_2()}</li>
						<li>{m.settings_music_needs_3()}</li>
						<li>{m.settings_music_needs_4()}</li>
						<li>{m.settings_music_needs_5()}</li>
						<li>{m.settings_music_needs_6()}</li>
					</ul>
				</details>
			</section>

			<!-- A link for every wall -->
			<section aria-labelledby="music-link" class="border-t border-line pt-5">
				<h3 id="music-link" class="text-sm font-semibold text-ink">{m.settings_music_link_heading()}</h3>
				<p class="mt-1 max-w-prose text-sm text-ink-2">
					{m.settings_music_link_intro()}
				</p>
				{#if current && wall?.link}
					<div class="mt-3 flex flex-wrap items-center gap-2 text-sm">
						<Plate tone="signal" label={m.settings_music_on_walls()} />
						<span class="min-w-0 truncate text-ink">{current.label} {current.kind}</span>
						<span class="text-ink-2">· {m.settings_music_sent_by({ name: wall.link.set_by })}</span>
						{#if auth.isAdmin}
							<Button variant="ghost" size="sm" onclick={() => void stop()} loading={stopping}>{m.settings_music_stop()}</Button>
						{/if}
					</div>
				{/if}
				{#if auth.isAdmin}
					<form class="mt-3 flex flex-col gap-2 sm:flex-row sm:items-start" onsubmit={send} novalidate>
						<div class="min-w-0 flex-1">
							<label for="music-wall-link" class="sr-only">{m.settings_music_link_label()}</label>
							<input
								id="music-wall-link"
								class="input"
								type="url"
								inputmode="url"
								bind:value={link}
								autocomplete="off"
								spellcheck="false"
								placeholder="https://open.spotify.com/playlist/…, deezer.com/…, youtube.com/…"
								disabled={sending}
								aria-invalid={linkError ? 'true' : undefined}
								aria-describedby={linkError ? 'music-wall-link-error' : undefined}
								oninput={() => (linkError = null)}
							/>
							{#if linkError}
								<p id="music-wall-link-error" class="mt-1.5 text-[0.8125rem] font-medium text-warning-ink" role="alert">{linkError}</p>
							{/if}
						</div>
						<Button type="submit" variant="secondary" loading={sending}>
							<Link2 class="size-4" aria-hidden="true" />
							{m.settings_music_play_on_wall()}
						</Button>
					</form>
					{#if sendError}
						<ErrorNotice class="mt-3" error={sendError} title={m.settings_music_send_error()} />
					{/if}
				{:else if !current}
					<p class="mt-3 text-sm text-ink-2">{m.settings_music_nothing_sent()}</p>
				{/if}
			</section>
		</div>
	{/if}
</Panel>
