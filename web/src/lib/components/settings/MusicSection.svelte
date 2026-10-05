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
	import { replaceState } from '$app/navigation';
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
	} from '$lib/api/music';
	import { auth } from '$lib/stores/auth.svelte';
	import { Button, Confirm, CopyBlock, ErrorNotice, Field, Panel, Plate, Skeleton } from '$lib/ui';
	import { formatRelative, parseServerDate } from '$lib/format';
	import { parseMusicLink } from '$lib/wall/music';
	import SpeakerStatus from './SpeakerStatus.svelte';

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

	const OUTCOME: Record<string, string> = {
		connected: 'Spotify is connected.',
		denied: 'Spotify did not grant access. Start again and choose “Agree”.',
		state: 'This approval was unknown, expired or already used. Start again from “Connect Spotify”.',
		session: 'Your DumbMonit session was not the one that started the connection. Sign in, then start again.',
		exchange: 'Spotify refused to complete the connection. Check that the redirect URI below is registered in your app, exactly.',
		internal: 'The connection could not be saved. Check the server logs.'
	};
	let outcome = $state<string | null>(null);
	$effect(() => {
		const value = page.url.searchParams.get('spotify');
		if (!value) return;
		outcome = value;
		// Forget it in the address bar: a reload should not say it again.
		const url = new URL(page.url);
		url.searchParams.delete('spotify');
		replaceState(url, page.state);
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
			clientIdError = 'Paste the Client ID from your app’s page in the Spotify Developer Dashboard (32 letters and digits).';
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
			linkError = parsed.message;
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
	title="Wall music"
	description="Start music from your phone and hear it on the wall display, or send it a Spotify, Deezer or YouTube link."
>
	{#snippet aside()}
		{#if account?.status === 'connected'}
			<Plate tone="signal" label="Spotify connected" />
		{:else if account?.status === 'expired'}
			<Plate tone="advisory" label="Reconnect Spotify" />
		{:else if account}
			<Plate tone="ghost" label="Spotify off" />
		{/if}
	{/snippet}

	{#if missing}
		<p class="text-sm text-ink-2">This server does not offer wall music yet: update DumbMonit.</p>
	{:else if error}
		<ErrorNotice {error} title="Could not load the music settings" onretry={() => void load()} />
	{:else if loading || !account}
		<Skeleton class="h-10 w-full" rows={5} />
	{:else}
		<div class="grid gap-6">
			{#if outcome}
				<div aria-live="polite">
					{#if outcome === 'connected'}
						<Plate tone="signal" label={OUTCOME.connected} size="md" />
					{:else}
						<ErrorNotice
							title="Spotify is not connected"
							error={new Error(OUTCOME[outcome] ?? 'The connection did not complete.')}
							hint="Start again below; nothing was changed."
						/>
					{/if}
				</div>
			{/if}

			<!-- Spotify Connect -->
			<section aria-labelledby="music-spotify">
				<h3 id="music-spotify" class="text-sm font-semibold text-ink">Spotify Connect</h3>
				<p class="mt-1 max-w-prose text-sm text-ink-2">
					The wall becomes a speaker named <strong class="font-semibold text-ink">{account.speaker_name}</strong>: in the
					Spotify app on your phone, open <em>Devices</em> and pick it, the sound comes out of the display. The wall also
					shows what plays on the account wherever it plays — the TV's app, a cast speaker, your laptop.
				</p>

				{#if account.status === 'connected'}
					<dl class="mt-3 grid gap-x-4 gap-y-1.5 text-sm sm:grid-cols-[auto_minmax(0,1fr)]">
						<dt class="text-ink-2">Account</dt>
						<dd class="text-ink">{account.account_name ?? 'Connected'}</dd>
						<dt class="text-ink-2">App (Client ID)</dt>
						<dd class="min-w-0 truncate font-mono text-[0.8125rem] text-ink">{account.client_id}</dd>
						{#if account.connected_at}
							<dt class="text-ink-2">Connected</dt>
							<dd class="text-ink">{formatRelative(account.connected_at)}</dd>
						{/if}
						{#if account.reconnect_by}
							<dt class="text-ink-2">Reconnect by</dt>
							<dd class="text-ink">{serverDay(account.reconnect_by)} <span class="text-ink-2">— Spotify ends connections after six months.</span></dd>
						{/if}
						<dt class="text-ink-2">Right now</dt>
						<dd class="min-w-0 text-ink">
							{#if playingNow}
								{playingNow.playing ? 'Playing' : 'Paused'}: {playingNow.title}{playingNow.artists.length ? ` — ${playingNow.artists.join(', ')}` : ''}{playingNow.device_name ? `, on ${playingNow.device_name}` : ''}
							{:else if wall?.spotify.error}
								<span class="text-advisory-ink">{wall.spotify.error}</span>
							{:else}
								Nothing is playing on this account.
							{/if}
						</dd>
					</dl>
					{#if account.missing_scopes.length > 0}
						<p class="mt-3 text-sm text-advisory-ink">
							Spotify did not grant {account.missing_scopes.join(', ')}: reconnect and accept every permission.
						</p>
					{/if}
					<SpeakerStatus onrenamed={(name) => account && (account = { ...account, speaker_name: name })} />
					{#if auth.isAdmin}
						<div class="mt-4 flex flex-wrap items-center gap-2">
							<Confirm variant="secondary" size="md" confirmLabel="Disconnect Spotify?" loading={disconnecting} onconfirm={disconnect}>
								Disconnect
							</Confirm>
						</div>
						{#if disconnectError}
							<ErrorNotice class="mt-3" error={disconnectError} title="Could not disconnect" />
						{/if}
					{/if}
				{:else if account.status === 'expired'}
					<p class="mt-3 text-sm text-advisory-ink">
						Spotify ended the connection{account.last_error ? ` (${account.last_error})` : ''}. Connect again below: the
						Client ID is kept.
					</p>
				{/if}

				{#if account.status !== 'connected'}
					{#if !auth.isAdmin}
						<p class="mt-3 text-sm text-ink-2">An admin can connect a Spotify account here.</p>
					{:else}
						<ol class="mt-4 grid gap-4 text-sm text-ink">
							<li class="grid gap-1.5">
								<p>
									<span class="font-semibold">1. Create an app</span> in the
									<a class="inline-flex items-center gap-1 font-semibold underline-offset-2 hover:underline" href="https://developer.spotify.com/dashboard" target="_blank" rel="noopener noreferrer">
										Spotify Developer Dashboard<ExternalLink class="size-3.5" aria-hidden="true" />
									</a>
									(any name). Tick <em>Web API</em> and <em>Web Playback SDK</em>, and add this redirect URI:
								</p>
								<CopyBlock value={redirectUri} label="Copy redirect URI" />
								<p class="text-[0.8125rem] text-ink-2">
									{#if secure}
										Spotify comes straight back to this page.
									{:else}
										DumbMonit is served over plain HTTP, and Spotify only accepts https:// or this 127.0.0.1 address. Nothing
										answers there: after approving, your browser shows an error page — copy its address, it carries the
										code. Served over HTTPS, the redirect would be direct.
									{/if}
								</p>
							</li>
							<li>
								<form class="grid gap-2" onsubmit={connect} novalidate>
									<Field label="2. Paste its Client ID" for="music-client-id" error={clientIdError} help="On the app's page, under Basic Information. There is no secret to copy: DumbMonit uses PKCE.">
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
											Connect Spotify
										</Button>
									</div>
									{#if startError}
										<ErrorNotice error={startError} title="Could not start the connection" />
									{/if}
								</form>
							</li>
							{#if authorization}
								<li class="grid gap-2 rounded-[var(--radius-card)] border border-line bg-canvas-deep p-4">
									<p>
										<span class="font-semibold">3. Approve on Spotify</span>, then come back here.
									</p>
									<div>
										<Button variant="secondary" href={authorization.authorize_url} target="_blank" rel="noopener noreferrer">
											Open Spotify
											<ExternalLink class="size-4" aria-hidden="true" />
										</Button>
									</div>
									<form class="grid gap-2" onsubmit={finish} novalidate>
										<Field
											label="4. Paste the address of the page you landed on"
											for="music-landing"
											help="It starts with {authorization.redirect_uri}?code= — valid ten minutes, once."
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
											<Button type="submit" variant="secondary" loading={finishing} disabled={!landing.trim()}>Finish connecting</Button>
										</div>
										{#if finishError}
											<ErrorNotice error={finishError} title="Could not finish the connection" />
										{/if}
									</form>
								</li>
							{/if}
						</ol>
					{/if}
				{/if}

				<details class="mt-4 rounded-[var(--radius-card)] border border-line">
					<summary class="cursor-pointer px-4 py-3 text-sm font-semibold text-ink select-none">What it needs</summary>
					<ul class="grid list-disc gap-1.5 border-t border-line py-3 pr-4 pl-8 text-[0.8125rem] text-ink-2">
						<li>Spotify Premium, on the account that owns the app. A personal app (development mode) admits up to five accounts: add others under <em>User Management</em>.</li>
						<li>The phone must be signed in to the <strong class="font-semibold text-ink">same Spotify account</strong> as the one connected here: a browser speaker is only listed for its own account — not for other members of a Family plan, and not found on the network like a smart speaker.</li>
						<li>For the wall to be a speaker, its browser must open DumbMonit over HTTPS (or as localhost on the display itself) and have working DRM (Widevine): Chrome, Edge or Firefox on a computer, Chromium with Widevine on a Raspberry Pi. Most smart-TV and kiosk browsers cannot — the wall says so on screen, and the walls below show it here.</li>
						<li>Browsers keep a page silent until it is touched: tap or press a key on the wall once after it loads (or start the kiosk browser with <code class="font-mono">--autoplay-policy=no-user-gesture-required</code>).</li>
						<li>“Now playing” works without any of that: the server asks Spotify what plays, every few seconds, even over plain HTTP.</li>
						<li>Spotify ends a connection six months after it was approved: connect again before the date shown.</li>
					</ul>
				</details>
			</section>

			<!-- A link for every wall -->
			<section aria-labelledby="music-link" class="border-t border-line pt-5">
				<h3 id="music-link" class="text-sm font-semibold text-ink">Play a link on the walls</h3>
				<p class="mt-1 max-w-prose text-sm text-ink-2">
					Without Premium, or for Deezer and YouTube — which have no Connect-style remote on the web — send a link from
					any screen, your phone included: every wall plays it in the service's own player. A browser not signed in to
					Spotify or Deezer on the wall plays 30-second previews.
				</p>
				{#if current && wall?.link}
					<div class="mt-3 flex flex-wrap items-center gap-2 text-sm">
						<Plate tone="signal" label="On the walls" />
						<span class="min-w-0 truncate text-ink">{current.label} {current.kind}</span>
						<span class="text-ink-2">· sent by {wall.link.set_by}</span>
						{#if auth.isAdmin}
							<Button variant="ghost" size="sm" onclick={() => void stop()} loading={stopping}>Stop</Button>
						{/if}
					</div>
				{/if}
				{#if auth.isAdmin}
					<form class="mt-3 flex flex-col gap-2 sm:flex-row sm:items-start" onsubmit={send} novalidate>
						<div class="min-w-0 flex-1">
							<label for="music-wall-link" class="sr-only">Link to play on the walls</label>
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
							Play on the wall
						</Button>
					</form>
					{#if sendError}
						<ErrorNotice class="mt-3" error={sendError} title="Could not send the link" />
					{/if}
				{:else if !current}
					<p class="mt-3 text-sm text-ink-2">Nothing is sent to the walls. An admin can send a link.</p>
				{/if}
			</section>
		</div>
	{/if}
</Panel>
