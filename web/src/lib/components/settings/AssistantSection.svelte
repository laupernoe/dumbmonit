<script lang="ts">
	/**
	 * Settings → API & assistants: API tokens for MCP clients (Claude, ChatGPT,
	 * Cursor, VS Code…) and for the REST API, with the ready-to-paste snippets.
	 *
	 * A token is shown in clear once, right after creation. The snippets are
	 * always visible so people can see what they will paste before creating
	 * anything; they carry the real token only while it is on screen.
	 */
	import { Bot, FileJson, KeyRound } from 'lucide-svelte';
	import {
		API_DOCS_URL,
		ASSISTANT_DOCS_URL,
		DEFAULT_TOKEN_EXPIRY_DAYS,
		OPENAPI_PATH,
		TOKEN_EXPIRY_CHOICES,
		createApiToken,
		listApiTokens,
		parseNetworks,
		revokeApiToken,
		type ApiToken,
		type ApiTokenScope,
		type CreatedApiToken
	} from '$lib/api/tokens';
	import { formatDateTime, formatRelative, parseServerDate } from '$lib/format';
	import { Button, Confirm, CopyBlock, EmptyState, ErrorNotice, Field, Panel, Plate, Skeleton } from '$lib/ui';

	const EXAMPLE_PROMPTS = ['Is everything fine?', 'Silence the NAS for two hours', 'What happened last night?', 'How full is the backup server?'];

	let tokens = $state<ApiToken[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const list = await listApiTokens(signal);
			tokens = Array.isArray(list) ? list : [];
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	// --- Create ---------------------------------------------------------------

	let name = $state('');
	let scope = $state<ApiTokenScope>('read');
	/** The `<select>` carries strings; "never" stands for `null`. */
	let expiry = $state(String(DEFAULT_TOKEN_EXPIRY_DAYS));
	let networks = $state('');
	let nameError = $state<string | null>(null);
	let creating = $state(false);
	let createError = $state<unknown>(null);
	let created = $state<CreatedApiToken | null>(null);

	async function create(event: SubmitEvent) {
		event.preventDefault();
		createError = null;
		if (!name.trim()) {
			nameError = 'Name the token, for example after the assistant or the machine it runs on.';
			return;
		}
		nameError = null;
		creating = true;
		try {
			created = await createApiToken({
				name: name.trim(),
				scope,
				expires_in_days: expiry === 'never' ? null : Number(expiry),
				allowed_networks: parseNetworks(networks)
			});
			tokens = [created, ...tokens.filter((token) => token.id !== created?.id)];
			name = '';
			networks = '';
		} catch (cause) {
			createError = cause;
		} finally {
			creating = false;
		}
	}

	// --- Revoke ---------------------------------------------------------------

	let revoking = $state<number | null>(null);
	let revokeError = $state<{ id: number; cause: unknown } | null>(null);

	async function revoke(token: ApiToken) {
		revoking = token.id;
		revokeError = null;
		try {
			await revokeApiToken(token.id);
			tokens = await listApiTokens();
			if (created?.id === token.id) created = null;
		} catch (cause) {
			revokeError = { id: token.id, cause };
		} finally {
			revoking = null;
		}
	}

	// --- Token list helpers -----------------------------------------------------

	const DAY = 86_400_000;

	/** "in 3 months", "in 12 d", "in 5 h": how long a token still has. */
	function remaining(value: string): string {
		const date = parseServerDate(value);
		if (!date) return 'at an unknown date';
		const ms = date.getTime() - Date.now();
		if (ms <= 0) return 'now';
		const hours = Math.round(ms / 3_600_000);
		if (hours < 24) return `in ${Math.max(hours, 1)} h`;
		const days = Math.round(ms / DAY);
		if (days < 60) return `in ${days} d`;
		const months = Math.round(days / 30);
		if (months < 24) return `in ${months} months`;
		return `in ${Math.round(days / 365)} years`;
	}

	/** Expiring within a week: worth a glance before it breaks a script. */
	function expiresSoon(token: ApiToken): boolean {
		const date = parseServerDate(token.expires_at);
		return !!date && !token.expired && date.getTime() - Date.now() < 7 * DAY;
	}

	// --- Connection snippets ----------------------------------------------------

	type Client = 'claude-code' | 'claude-desktop' | 'vscode' | 'cursor' | 'chatgpt';
	const CLIENTS: { id: Client; label: string }[] = [
		{ id: 'claude-code', label: 'Claude Code' },
		{ id: 'claude-desktop', label: 'Claude Desktop' },
		{ id: 'vscode', label: 'VS Code' },
		{ id: 'cursor', label: 'Cursor / other' },
		{ id: 'chatgpt', label: 'ChatGPT' }
	];
	let client = $state<Client>('claude-code');

	// The URL assistants will reach this server at: what the browser sees.
	const origin = $derived(typeof location === 'undefined' ? '' : location.origin);
	const url = $derived(`${origin}/api/mcp`);
	const isHttps = $derived(typeof location === 'undefined' ? false : location.protocol === 'https:');
	const token = $derived(created?.secret ?? 'dmt_…paste-your-token…');
	const bearer = $derived(`Bearer ${token}`);
	/** Snippets carrying the real token are blurred until hovered, like the token itself. */
	const live = $derived(created !== null);

	const claudeCommand = $derived(`claude mcp add --transport http dumbmonit ${url} --header "Authorization: ${bearer}"`);
	// Claude Desktop reads local (stdio) servers from its config file: mcp-remote
	// bridges to this HTTP endpoint. The header goes through an environment
	// variable, without a space after the colon, as mcp-remote recommends.
	const claudeDesktop = $derived(
		JSON.stringify(
			{
				mcpServers: {
					dumbmonit: {
						command: 'npx',
						args: ['-y', 'mcp-remote', url, '--header', 'Authorization:${DUMBMONIT_AUTH}', ...(isHttps ? [] : ['--allow-http'])],
						env: { DUMBMONIT_AUTH: bearer }
					}
				}
			},
			null,
			2
		)
	);
	const vscodeJson = $derived(JSON.stringify({ servers: { dumbmonit: { type: 'http', url, headers: { Authorization: bearer } } } }, null, 2));
	const cursorJson = $derived(JSON.stringify({ mcpServers: { dumbmonit: { url, headers: { Authorization: bearer } } } }, null, 2));

	// The same token on the REST API: no cookie, no CSRF header.
	const curlRead = $derived(`curl -H "Authorization: ${bearer}" ${origin}/api/targets`);
	const curlWrite = $derived(`curl -X POST -H "Authorization: ${bearer}" ${origin}/api/targets/1/probe`);

	// Arrow keys move between tabs, as a tablist is expected to behave.
	function onTabKey(event: KeyboardEvent) {
		const index = CLIENTS.findIndex((c) => c.id === client);
		if (event.key === 'ArrowRight') client = CLIENTS[(index + 1) % CLIENTS.length].id;
		else if (event.key === 'ArrowLeft') client = CLIENTS[(index - 1 + CLIENTS.length) % CLIENTS.length].id;
		else return;
		event.preventDefault();
		document.getElementById(`assistant-tab-${client}`)?.focus();
	}

	const linkClass = 'underline decoration-line underline-offset-2 hover:text-ink';
	const codeClass = 'font-mono text-[0.8125rem]';
</script>

<Panel
	id="assistant"
	title="API & assistants"
	description="One kind of token for both: let Claude, ChatGPT, Cursor or any MCP client ask DumbMonit how things are, or call the REST API from a script. A read token can only look; a write token can also change things — silence a device, run a probe, add or edit devices, rules and channels."
	padded={false}
>
	<div class="px-5 py-4">
		<p class="text-sm text-ink-2">Once connected, try asking:</p>
		<ul class="mt-2 flex flex-wrap gap-2" aria-label="Example prompts">
			{#each EXAMPLE_PROMPTS as prompt (prompt)}
				<li class="rounded-lg border border-line bg-canvas-deep px-2.5 py-1 text-sm text-ink">“{prompt}”</li>
			{/each}
		</ul>

		<!-- Create -->
		<form class="mt-5 grid gap-4 sm:grid-cols-2" onsubmit={create} novalidate>
			<Field label="New token" for="api-token-name" error={nameError} help="Only used to recognise the token in this list.">
				<input
					id="api-token-name"
					type="text"
					class="input"
					bind:value={name}
					placeholder="Claude on my laptop"
					autocomplete="off"
					maxlength="80"
					disabled={creating}
					aria-invalid={nameError ? 'true' : undefined}
					oninput={() => (nameError = null)}
				/>
			</Field>
			<fieldset class="grid min-w-0 gap-1.5" disabled={creating}>
				<legend class="mb-1.5 block text-sm font-semibold text-ink">Scope</legend>
				<div class="flex w-fit max-w-full flex-wrap gap-1 rounded-lg border border-line bg-canvas-deep p-1" role="radiogroup" aria-label="Token scope">
					{#each [{ value: 'read', label: 'Read' }, { value: 'write', label: 'Read and write' }] as option (option.value)}
						<label
							class={`cursor-pointer rounded-md px-3 py-1.5 text-sm transition-colors has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-signal ${scope === option.value ? 'bg-surface font-semibold text-ink shadow-lift' : 'text-ink-2 hover:text-ink'}`}
						>
							<input type="radio" class="sr-only" name="api-token-scope" value={option.value} bind:group={scope} />
							{option.label}
						</label>
					{/each}
				</div>
				<p class="text-[0.8125rem] text-ink-2">
					{scope === 'read' ? 'Can never change anything.' : 'Can change everything except accounts, tokens and backups.'}
				</p>
			</fieldset>
			<Field label="Expires" for="api-token-expiry" help="An expired token stops working; create a new one then.">
				<select id="api-token-expiry" class="input" bind:value={expiry} disabled={creating}>
					{#each TOKEN_EXPIRY_CHOICES as choice (choice.label)}
						<option value={choice.days === null ? 'never' : String(choice.days)}>{choice.label}</option>
					{/each}
				</select>
			</Field>
			<Field
				label="Allowed networks"
				for="api-token-networks"
				help="Optional. Addresses or CIDR ranges, separated by commas or spaces. Empty: usable from anywhere."
			>
				<input
					id="api-token-networks"
					type="text"
					class="input font-mono"
					bind:value={networks}
					placeholder="192.168.1.0/24, 10.8.0.5"
					autocomplete="off"
					spellcheck="false"
					disabled={creating}
				/>
			</Field>
			<div class="sm:col-span-2">
				<Button type="submit" variant="secondary" loading={creating}>
					<KeyRound class="size-4" aria-hidden="true" />
					Create token
				</Button>
			</div>
		</form>
		{#if createError}
			<ErrorNotice error={createError} title="Could not create the token" class="mt-3" />
		{/if}

		<div aria-live="polite">
			{#if created}
				<div class="rise-in mt-4 rounded-[var(--radius-card)] border border-advisory/40 bg-surface p-4">
					<div class="flex flex-wrap items-center justify-between gap-2">
						<div class="flex min-w-0 flex-wrap items-center gap-2">
							<p class="font-semibold text-ink">Token “{created.name}” created</p>
							<Plate tone="advisory" label="Shown once — copy it now" />
							<Plate tone={created.scope === 'write' ? 'info' : 'ghost'} label={created.scope === 'write' ? 'Read and write' : 'Read only'} bare />
						</div>
						<Button variant="ghost" size="sm" onclick={() => (created = null)}>I've copied it</Button>
					</div>
					<div class="mt-3">
						<CopyBlock value={created.secret} label="Copy token" secret />
					</div>
					<p class="mt-2 text-sm text-ink-2">
						{created.expires_at ? `Expires ${formatDateTime(created.expires_at)}.` : 'Never expires.'}
						{created.allowed_networks.length > 0 ? `Usable only from ${created.allowed_networks.join(', ')}.` : 'Usable from any address.'}
						The snippets below now carry this token. Treat it like a password.
					</p>
				</div>
			{/if}
		</div>

		<!-- Connection snippets -->
		<div class="mt-6">
			<div class="flex flex-wrap items-center justify-between gap-2">
				<p class="text-sm font-semibold text-ink">Connect an assistant (MCP)</p>
				<div class="flex max-w-full flex-wrap gap-1 rounded-lg border border-line bg-canvas-deep p-1" role="tablist" aria-label="Assistant">
					{#each CLIENTS as option (option.id)}
						<button
							type="button"
							role="tab"
							id={`assistant-tab-${option.id}`}
							aria-selected={client === option.id}
							aria-controls="assistant-snippets"
							tabindex={client === option.id ? 0 : -1}
							class={`rounded-md px-3 py-1.5 text-sm transition-colors ${client === option.id ? 'bg-surface font-semibold text-ink shadow-lift' : 'text-ink-2 hover:text-ink'}`}
							onclick={() => (client = option.id)}
							onkeydown={onTabKey}
						>
							{option.label}
						</button>
					{/each}
				</div>
			</div>

			<div id="assistant-snippets" role="tabpanel" aria-labelledby={`assistant-tab-${client}`} class="mt-3 grid min-w-0 gap-4">
				{#if client === 'claude-code'}
					<div>
						<p class="mb-1.5 text-sm text-ink-2">One command in a terminal:</p>
						<CopyBlock value={claudeCommand} label="Copy command" secret={live} />
					</div>
				{:else if client === 'claude-desktop'}
					<div>
						<p class="mb-1.5 text-sm text-ink-2">
							Add to <code class={codeClass}>claude_desktop_config.json</code> (Settings → Developer → Edit Config), then restart Claude Desktop. It needs Node.js: <code class={codeClass}>mcp-remote</code> bridges the desktop app to this server.
						</p>
						<CopyBlock value={claudeDesktop} label="Copy JSON" secret={live} />
					</div>
					{#if !isHttps}
						<p class="text-sm text-ink-2">This page is served over plain HTTP, so the snippet includes <code class={codeClass}>--allow-http</code>. Keep that to your own network.</p>
					{/if}
				{:else if client === 'vscode'}
					<div>
						<p class="mb-1.5 text-sm text-ink-2">Add to <code class={codeClass}>.vscode/mcp.json</code> in a workspace (or run “MCP: Add Server” and choose HTTP):</p>
						<CopyBlock value={vscodeJson} label="Copy JSON" secret={live} />
					</div>
				{:else if client === 'cursor'}
					<div>
						<p class="mb-1.5 text-sm text-ink-2">Cursor (<code class={codeClass}>.cursor/mcp.json</code>) and most other MCP clients accept this shape — a Streamable HTTP server with a bearer header:</p>
						<CopyBlock value={cursorJson} label="Copy JSON" secret={live} />
					</div>
				{:else}
					<div class="grid min-w-0 gap-3">
						<p class="text-sm text-ink-2">
							In ChatGPT, open Settings → Connectors → Create (developer mode), then fill in the MCP server URL and the authorization header. ChatGPT connects from OpenAI's servers, so the URL must be reachable from the internet over HTTPS.
						</p>
						{#if !isHttps}
							<Plate tone="advisory" label="This page is not served over HTTPS: put DumbMonit behind a reverse proxy with TLS before exposing it." />
						{/if}
						<div>
							<p class="mb-1.5 text-sm text-ink-2">MCP server URL</p>
							<CopyBlock value={url} label="Copy URL" />
						</div>
						<div>
							<p class="mb-1.5 text-sm text-ink-2">Authorization header</p>
							<CopyBlock value={bearer} label="Copy header" secret={live} />
						</div>
					</div>
				{/if}
				<p class="text-sm text-ink-2">
					The server speaks MCP over Streamable HTTP, without sessions. Every client, the tools and the security notes: <a class={linkClass} href={ASSISTANT_DOCS_URL} target="_blank" rel="noreferrer">Connect an assistant</a>.
				</p>
			</div>
		</div>

		<!-- REST API access -->
		<div class="mt-6">
			<p class="text-sm font-semibold text-ink">REST API</p>
			<p class="mt-1 text-sm text-ink-2">
				The same token opens the REST API to scripts and dashboards: send it as a bearer header instead of a session cookie — no anti-CSRF header needed. A read token maps to a viewer, a write token to an administrator, except that no token can manage accounts, sign-in settings, tokens or backups.
			</p>
			<div class="mt-3 grid min-w-0 gap-3">
				<div>
					<p class="mb-1.5 text-sm text-ink-2">Read — every device:</p>
					<CopyBlock value={curlRead} label="Copy command" secret={live} />
				</div>
				<div>
					<p class="mb-1.5 text-sm text-ink-2">Write — probe device 1 now (needs a write token):</p>
					<CopyBlock value={curlWrite} label="Copy command" secret={live} />
				</div>
			</div>
			<div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2 text-sm text-ink-2">
				<a class={`inline-flex items-center gap-1.5 ${linkClass}`} href={OPENAPI_PATH} target="_blank" rel="noreferrer">
					<FileJson class="size-4 shrink-0" aria-hidden="true" />
					OpenAPI description
				</a>
				<a class={linkClass} href={API_DOCS_URL} target="_blank" rel="noreferrer">HTTP API reference</a>
			</div>
			<p class="mt-2 text-sm text-ink-2">
				The OpenAPI file loads in Swagger UI, Postman or Insomnia, and generates clients. Calls from another web origin work with a bearer token only, never with a session.
			</p>
			<p class="mt-2 text-sm text-ink-2">
				A Prometheus or Grafana you already run can read this instance with a read token — <code class={codeClass}>/metrics</code> for the server's own health, <code class={codeClass}>/federate</code> for the measurements, <code class={codeClass}>/prometheus</code> as a Grafana data source: see <a class={linkClass} href="https://dumbmonit.readthedocs.io/en/latest/reference/metrics/#scraping-dumbmonit" target="_blank" rel="noreferrer">Scraping DumbMonit</a>.
			</p>
		</div>

		<!-- Token list -->
		<div class="mt-6">
			<p class="mb-2 text-sm font-semibold text-ink">Tokens</p>
			{#if error}
				<ErrorNotice {error} title="Could not load the tokens" onretry={() => void load()} />
			{:else if loading}
				<Skeleton class="h-14 w-full" />
			{:else if tokens.length === 0}
				<EmptyState icon={Bot} title="No token yet." description="Create one above, then paste the snippet into your assistant or script." />
			{:else}
				<ul class="divide-y divide-line rounded-[var(--radius-card)] border border-line" role="list">
					{#each tokens as item (item.id)}
						{@const revoked = item.revoked_at !== null}
						{@const inactive = revoked || item.expired}
						<li class={`px-4 py-3 ${inactive ? 'ghost-cell' : ''}`}>
							<div class="flex flex-wrap items-center gap-x-3 gap-y-2">
								<div class="min-w-0 flex-1">
									<div class="flex flex-wrap items-center gap-2">
										<span class={`min-w-0 break-words font-semibold ${inactive ? 'text-ink-2' : 'text-ink'}`}>{item.name}</span>
										<code class="rounded-md border border-line bg-canvas-deep px-1.5 py-0.5 font-mono text-[0.75rem] text-ink-2">{item.prefix}…</code>
										<Plate tone={item.scope === 'write' ? 'info' : 'ghost'} label={item.scope === 'write' ? 'Read and write' : 'Read only'} bare />
										{#if revoked}
											<Plate tone="ghost" label="Revoked" />
										{:else if item.expired}
											<Plate tone="ghost" label="Expired" />
										{:else if expiresSoon(item)}
											<Plate tone="advisory" label={`Expires ${remaining(item.expires_at ?? '')}`} />
										{/if}
									</div>
									<p class="mt-1 text-sm text-ink-2">
										Created <time class="tnum" title={formatDateTime(item.created_at)}>{formatRelative(item.created_at)}</time>{#if item.created_by}&nbsp;by {item.created_by}{/if}
										·
										{#if item.expires_at === null}
											Never expires
										{:else if item.expired}
											Expired <time class="tnum" title={formatDateTime(item.expires_at)}>{formatRelative(item.expires_at)}</time>
										{:else}
											Expires <time class="tnum" title={formatDateTime(item.expires_at)}>{remaining(item.expires_at)}</time>
										{/if}
										{#if revoked}
											· Revoked <time class="tnum" title={formatDateTime(item.revoked_at)}>{formatRelative(item.revoked_at)}</time>
										{/if}
									</p>
									<p class="mt-0.5 text-sm text-ink-2">
										Last used <time class="tnum" title={formatDateTime(item.last_used_at)}>{formatRelative(item.last_used_at)}</time>{#if item.last_used_ip}&nbsp;from <span class="font-mono text-[0.8125rem]">{item.last_used_ip}</span>{/if}
										·
										{#if item.allowed_networks.length === 0}
											Any network
										{:else}
											Only from
											{#each item.allowed_networks as network, index (network)}
												<code class="font-mono text-[0.8125rem] break-all">{network}</code>{index < item.allowed_networks.length - 1 ? ', ' : ''}
											{/each}
										{/if}
									</p>
								</div>
								{#if !revoked}
									<Confirm confirmLabel="Revoke for good?" loading={revoking === item.id} onconfirm={() => revoke(item)}>Revoke</Confirm>
								{/if}
							</div>
							{#if revokeError?.id === item.id}
								<ErrorNotice error={revokeError.cause} title="Could not revoke the token" class="mt-3" />
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>
</Panel>
