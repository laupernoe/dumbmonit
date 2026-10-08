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
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
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
	} from '#lib/api/tokens.js';
	import { formatDateTime, formatRelative, parseServerDate } from '#lib/format.js';
	import { Button, Confirm, CopyBlock, EmptyState, ErrorNotice, Field, Panel, Plate, Skeleton } from '#lib/ui/index.js';

	const EXAMPLE_PROMPTS = [m.settings_assistant_prompt_fine(), m.settings_assistant_prompt_silence(), m.settings_assistant_prompt_night(), m.settings_assistant_prompt_backup()];

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
			nameError = m.settings_assistant_name_required();
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
		if (!date) return m.settings_assistant_remaining_unknown();
		const rtf = new Intl.RelativeTimeFormat(getLocale(), { numeric: 'auto' });
		const ms = date.getTime() - Date.now();
		if (ms <= 0) return rtf.format(0, 'second');
		const hours = Math.round(ms / 3_600_000);
		if (hours < 24) return rtf.format(Math.max(hours, 1), 'hour');
		const days = Math.round(ms / DAY);
		if (days < 60) return rtf.format(days, 'day');
		const months = Math.round(days / 30);
		if (months < 24) return rtf.format(months, 'month');
		return rtf.format(Math.round(days / 365), 'year');
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
		{ id: 'cursor', label: m.settings_assistant_client_other() },
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
	title={m.settings_assistant_title()}
	description={m.settings_assistant_description()}
	padded={false}
>
	<div class="px-5 py-4">
		<p class="text-sm text-ink-2">{m.settings_assistant_try_asking()}</p>
		<ul class="mt-2 flex flex-wrap gap-2" aria-label={m.settings_assistant_prompts_aria()}>
			{#each EXAMPLE_PROMPTS as prompt (prompt)}
				<li class="rounded-lg border border-line bg-canvas-deep px-2.5 py-1 text-sm text-ink">“{prompt}”</li>
			{/each}
		</ul>

		<!-- Create -->
		<form class="mt-5 grid gap-4 sm:grid-cols-2" onsubmit={create} novalidate>
			<Field label={m.settings_assistant_new_token()} for="api-token-name" error={nameError} help={m.settings_assistant_new_token_help()}>
				<input
					id="api-token-name"
					type="text"
					class="input"
					bind:value={name}
					placeholder={m.settings_assistant_name_placeholder()}
					autocomplete="off"
					maxlength="80"
					disabled={creating}
					aria-invalid={nameError ? 'true' : undefined}
					oninput={() => (nameError = null)}
				/>
			</Field>
			<fieldset class="grid min-w-0 gap-1.5" disabled={creating}>
				<legend class="mb-1.5 block text-sm font-semibold text-ink">{m.settings_assistant_scope()}</legend>
				<div class="flex w-fit max-w-full flex-wrap gap-1 rounded-lg border border-line bg-canvas-deep p-1" role="radiogroup" aria-label={m.settings_assistant_scope_aria()}>
					{#each [{ value: 'read', label: m.settings_assistant_scope_read() }, { value: 'write', label: m.settings_assistant_scope_write() }] as option (option.value)}
						<label
							class={`cursor-pointer rounded-md px-3 py-1.5 text-sm transition-colors has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-signal ${scope === option.value ? 'bg-surface font-semibold text-ink shadow-lift' : 'text-ink-2 hover:text-ink'}`}
						>
							<input type="radio" class="sr-only" name="api-token-scope" value={option.value} bind:group={scope} />
							{option.label}
						</label>
					{/each}
				</div>
				<p class="text-[0.8125rem] text-ink-2">
					{scope === 'read' ? m.settings_assistant_scope_read_note() : m.settings_assistant_scope_write_note()}
				</p>
			</fieldset>
			<Field label={m.settings_assistant_expires()} for="api-token-expiry" help={m.settings_assistant_expires_help()}>
				<select id="api-token-expiry" class="input" bind:value={expiry} disabled={creating}>
					{#each TOKEN_EXPIRY_CHOICES as choice (choice.label)}
						<option value={choice.days === null ? 'never' : String(choice.days)}>{choice.label}</option>
					{/each}
				</select>
			</Field>
			<Field
				label={m.settings_assistant_networks()}
				for="api-token-networks"
				help={m.settings_assistant_networks_help()}
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
					{m.settings_assistant_create()}
				</Button>
			</div>
		</form>
		{#if createError}
			<ErrorNotice error={createError} title={m.settings_assistant_create_error()} class="mt-3" />
		{/if}

		<div aria-live="polite">
			{#if created}
				<div class="rise-in mt-4 rounded-[var(--radius-card)] border border-advisory/40 bg-surface p-4">
					<div class="flex flex-wrap items-center justify-between gap-2">
						<div class="flex min-w-0 flex-wrap items-center gap-2">
							<p class="font-semibold text-ink">{m.settings_assistant_created_title({ name: created.name })}</p>
							<Plate tone="advisory" label={m.settings_assistant_shown_once()} />
							<Plate tone={created.scope === 'write' ? 'info' : 'ghost'} label={created.scope === 'write' ? m.settings_assistant_scope_write() : m.settings_assistant_scope_read_only()} bare />
						</div>
						<Button variant="ghost" size="sm" onclick={() => (created = null)}>{m.settings_assistant_copied()}</Button>
					</div>
					<div class="mt-3">
						<CopyBlock value={created.secret} label={m.settings_assistant_copy_token()} secret />
					</div>
					<p class="mt-2 text-sm text-ink-2">
						{created.expires_at ? m.settings_assistant_created_expires({ date: formatDateTime(created.expires_at) }) : m.settings_assistant_created_never()}
						{created.allowed_networks.length > 0 ? m.settings_assistant_created_networks({ networks: created.allowed_networks.join(', ') }) : m.settings_assistant_created_any()}
						{m.settings_assistant_created_note()}
					</p>
				</div>
			{/if}
		</div>

		<!-- Connection snippets -->
		<div class="mt-6">
			<div class="flex flex-wrap items-center justify-between gap-2">
				<p class="text-sm font-semibold text-ink">{m.settings_assistant_connect_title()}</p>
				<div class="flex max-w-full flex-wrap gap-1 rounded-lg border border-line bg-canvas-deep p-1" role="tablist" aria-label={m.settings_assistant_assistant_aria()}>
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
						<p class="mb-1.5 text-sm text-ink-2">{m.settings_assistant_one_command()}</p>
						<CopyBlock value={claudeCommand} label={m.settings_assistant_copy_command()} secret={live} />
					</div>
				{:else if client === 'claude-desktop'}
					<div>
						<p class="mb-1.5 text-sm text-ink-2">
							{m.settings_assistant_desktop_text()}
						</p>
						<CopyBlock value={claudeDesktop} label={m.settings_assistant_copy_json()} secret={live} />
					</div>
					{#if !isHttps}
						<p class="text-sm text-ink-2">{m.settings_assistant_http_warning()}</p>
					{/if}
				{:else if client === 'vscode'}
					<div>
						<p class="mb-1.5 text-sm text-ink-2">{m.settings_assistant_vscode_text()}</p>
						<CopyBlock value={vscodeJson} label={m.settings_assistant_copy_json()} secret={live} />
					</div>
				{:else if client === 'cursor'}
					<div>
						<p class="mb-1.5 text-sm text-ink-2">{m.settings_assistant_cursor_text()}</p>
						<CopyBlock value={cursorJson} label={m.settings_assistant_copy_json()} secret={live} />
					</div>
				{:else}
					<div class="grid min-w-0 gap-3">
						<p class="text-sm text-ink-2">
							{m.settings_assistant_chatgpt_text()}
						</p>
						{#if !isHttps}
							<Plate tone="advisory" label={m.settings_assistant_chatgpt_https_warning()} />
						{/if}
						<div>
							<p class="mb-1.5 text-sm text-ink-2">{m.settings_assistant_mcp_url()}</p>
							<CopyBlock value={url} label={m.settings_assistant_copy_url()} />
						</div>
						<div>
							<p class="mb-1.5 text-sm text-ink-2">{m.settings_assistant_auth_header()}</p>
							<CopyBlock value={bearer} label={m.settings_assistant_copy_header()} secret={live} />
						</div>
					</div>
				{/if}
				<p class="text-sm text-ink-2">
					{m.settings_assistant_mcp_note()} <a class={linkClass} href={ASSISTANT_DOCS_URL} target="_blank" rel="noreferrer">{m.settings_assistant_mcp_link()}</a>
				</p>
			</div>
		</div>

		<!-- REST API access -->
		<div class="mt-6">
			<p class="text-sm font-semibold text-ink">{m.settings_assistant_rest_title()}</p>
			<p class="mt-1 text-sm text-ink-2">
				{m.settings_assistant_rest_text()}
			</p>
			<div class="mt-3 grid min-w-0 gap-3">
				<div>
					<p class="mb-1.5 text-sm text-ink-2">{m.settings_assistant_rest_read()}</p>
					<CopyBlock value={curlRead} label={m.settings_assistant_copy_command()} secret={live} />
				</div>
				<div>
					<p class="mb-1.5 text-sm text-ink-2">{m.settings_assistant_rest_write()}</p>
					<CopyBlock value={curlWrite} label={m.settings_assistant_copy_command()} secret={live} />
				</div>
			</div>
			<div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2 text-sm text-ink-2">
				<a class={`inline-flex items-center gap-1.5 ${linkClass}`} href={OPENAPI_PATH} target="_blank" rel="noreferrer">
					<FileJson class="size-4 shrink-0" aria-hidden="true" />
					{m.settings_assistant_openapi()}
				</a>
				<a class={linkClass} href={API_DOCS_URL} target="_blank" rel="noreferrer">{m.settings_assistant_api_reference()}</a>
			</div>
			<p class="mt-2 text-sm text-ink-2">
				{m.settings_assistant_openapi_note()}
			</p>
			<p class="mt-2 text-sm text-ink-2">
				{m.settings_assistant_prometheus_text()} <a class={linkClass} href="https://dumbmonit.readthedocs.io/en/latest/reference/metrics/#scraping-dumbmonit" target="_blank" rel="noreferrer">{m.settings_assistant_prometheus_link()}</a>
			</p>
		</div>

		<!-- Token list -->
		<div class="mt-6">
			<p class="mb-2 text-sm font-semibold text-ink">{m.settings_assistant_tokens_title()}</p>
			{#if error}
				<ErrorNotice {error} title={m.settings_assistant_load_error()} onretry={() => void load()} />
			{:else if loading}
				<Skeleton class="h-14 w-full" />
			{:else if tokens.length === 0}
				<EmptyState icon={Bot} title={m.settings_assistant_empty_title()} description={m.settings_assistant_empty_description()} />
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
										<Plate tone={item.scope === 'write' ? 'info' : 'ghost'} label={item.scope === 'write' ? m.settings_assistant_scope_write() : m.settings_assistant_scope_read_only()} bare />
										{#if revoked}
											<Plate tone="ghost" label={m.settings_assistant_revoked()} />
										{:else if item.expired}
											<Plate tone="ghost" label={m.settings_assistant_expired()} />
										{:else if expiresSoon(item)}
											<Plate tone="advisory" label={m.settings_assistant_expires_in_plate({ when: remaining(item.expires_at ?? '') })} />
										{/if}
									</div>
									<p class="mt-1 text-sm text-ink-2">
										<time class="tnum" title={formatDateTime(item.created_at)}>{item.created_by ? m.settings_assistant_created_by({ when: formatRelative(item.created_at), user: item.created_by }) : m.settings_assistant_created_at({ when: formatRelative(item.created_at) })}</time>
										·
										{#if item.expires_at === null}
											{m.settings_assistant_never_expires()}
										{:else if item.expired}
											<time class="tnum" title={formatDateTime(item.expires_at)}>{m.settings_assistant_expired_at({ when: formatRelative(item.expires_at) })}</time>
										{:else}
											<time class="tnum" title={formatDateTime(item.expires_at)}>{m.settings_assistant_expires_at({ when: remaining(item.expires_at) })}</time>
										{/if}
										{#if revoked}
											· <time class="tnum" title={formatDateTime(item.revoked_at)}>{m.settings_assistant_revoked_at({ when: formatRelative(item.revoked_at) })}</time>
										{/if}
									</p>
									<p class="mt-0.5 text-sm text-ink-2">
										<time class="tnum" title={formatDateTime(item.last_used_at)}>{item.last_used_ip ? m.settings_assistant_last_used_from({ when: formatRelative(item.last_used_at), ip: item.last_used_ip }) : m.settings_assistant_last_used({ when: formatRelative(item.last_used_at) })}</time>
										·
										{#if item.allowed_networks.length === 0}
											{m.settings_assistant_any_network()}
										{:else}
											<span class="break-all">{m.settings_assistant_only_from({ networks: item.allowed_networks.join(', ') })}</span>
										{/if}
									</p>
								</div>
								{#if !revoked}
									<Confirm confirmLabel={m.settings_assistant_revoke_confirm()} loading={revoking === item.id} onconfirm={() => revoke(item)}>{m.settings_assistant_revoke()}</Confirm>
								{/if}
							</div>
							{#if revokeError?.id === item.id}
								<ErrorNotice error={revokeError.cause} title={m.settings_assistant_revoke_error()} class="mt-3" />
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>
</Panel>
