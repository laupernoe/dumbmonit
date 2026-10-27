<script lang="ts">
	/**
	 * Log search (administrators): LogsQL over the embedded log store. The
	 * server's own logs plus what agents, the syslog server and the SNMP trap
	 * receiver collect; filter by device, source and level, and turn a search
	 * into an alert. Newest first, one expandable row per entry.
	 */
	import { onMount } from 'svelte';
	import { ChevronRight, Search } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getLogInputs, searchLogs } from '#lib/api/storage.js';
	import { createAlertRule, listTargets } from '#lib/api/index.js';
	import { ApiError } from '#lib/api/client.js';
	import type { LogInputs, LogSearchResult, LogSource, Target } from '#lib/api/types.js';
	import { formatDateTime, formatDuration, formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Disclosure, EmptyState, ErrorNotice, Field, PageHeader, Plate, Skeleton } from '#lib/ui/index.js';
	import { fromSeverityWord, severityOptions, type SeverityWord } from '#lib/components/alerts/rules/options.js';
	import type { Tone } from '#lib/ui/index.js';
	import { levelLabel } from '#lib/components/settings/logLabels.js';
	import { periodRange, type SearchPeriod } from '#lib/components/settings/storage.js';

	type Level = 'error' | 'warn' | 'info' | 'debug';

	let query = $state('*');
	let period = $state<SearchPeriod>('1h');
	let customStart = $state('');
	let customEnd = $state('');
	let level = $state<Level>('info');
	let source = $state<LogSource>('server');
	/** Device id as a string; empty: every device. */
	let device = $state('');
	let targets = $state<Target[]>([]);
	let loading = $state(false);
	let result = $state<LogSearchResult | null>(null);
	let error = $state<unknown>(null);
	let rangeError = $state(false);
	let open = $state<Record<number, boolean>>({});
	let controller: AbortController | null = null;

	const PERIODS: { id: SearchPeriod; label: () => string }[] = [
		{ id: '15m', label: () => m.logsearch_period_15m() },
		{ id: '1h', label: () => m.logsearch_period_1h() },
		{ id: '6h', label: () => m.logsearch_period_6h() },
		{ id: '24h', label: () => m.logsearch_period_24h() },
		{ id: '7d', label: () => m.logsearch_period_7d() },
		{ id: 'custom', label: () => m.logsearch_period_custom() }
	];

	const SOURCES: { id: LogSource; label: () => string }[] = [
		{ id: 'server', label: () => m.logsearch_source_server() },
		{ id: 'agent', label: () => m.logsearch_source_agent() },
		{ id: 'syslog', label: () => m.logsearch_source_syslog() },
		{ id: 'trap', label: () => m.logsearch_source_trap() },
		{ id: 'all', label: () => m.logsearch_source_all() }
	];
	const SERVICE: Record<Exclude<LogSource, 'all'>, string> = { server: 'dumbmonit', agent: 'agent', syslog: 'syslog', trap: 'trap' };
	const LEVELS_FROM: Record<Level, string[]> = {
		error: ['error'],
		warn: ['error', 'warn'],
		info: ['error', 'warn', 'info'],
		debug: ['error', 'warn', 'info', 'debug']
	};

	const status = $derived(error instanceof ApiError ? error.status : 0);

	onMount(() => {
		// `/logs?target=12&source=syslog`: the device page links here.
		const params = new URLSearchParams(window.location.search);
		const wantedSource = params.get('source');
		if (SOURCES.some((entry) => entry.id === wantedSource)) source = wantedSource as LogSource;
		const wantedTarget = params.get('target');
		if (wantedTarget && /^\d+$/.test(wantedTarget)) {
			device = wantedTarget;
			if (!wantedSource) source = 'all';
		}
		if (!auth.isAdmin) return;
		void listTargets()
			.then((list) => (targets = [...list].sort((a, b) => a.name.localeCompare(b.name))))
			.catch(() => {});
		if (device) void run();
	});

	function tone(level: string): Tone {
		return level === 'error' ? 'warning' : level === 'warn' ? 'advisory' : level === 'info' ? 'info' : 'ghost';
	}

	async function run(event?: SubmitEvent) {
		event?.preventDefault();
		rangeError = false;
		const range = periodRange(period, Date.now(), { start: customStart, end: customEnd });
		if (!range) {
			rangeError = true;
			return;
		}
		controller?.abort();
		controller = new AbortController();
		loading = true;
		error = null;
		open = {};
		try {
			result = await searchLogs(
				{ query: query.trim() || '*', start: range.start, end: range.end, level, source, target: device ? Number(device) : undefined, limit: 200 },
				controller.signal
			);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			result = null;
			error = cause;
		} finally {
			loading = false;
		}
	}

	// ---- Receivers (syslog, SNMP traps) -------------------------------------
	let inputsOpen = $state(false);
	let inputs = $state<LogInputs | null>(null);
	let inputsError = $state<unknown>(null);

	$effect(() => {
		if (!inputsOpen || inputs || !auth.isAdmin) return;
		getLogInputs()
			.then((value) => (inputs = value))
			.catch((cause) => (inputsError = cause));
	});

	// ---- Alert on this search -----------------------------------------------
	const WINDOWS = [60, 300, 900, 3600, 21600, 86400];
	let alertOpen = $state(false);
	let alertName = $state('');
	let alertCount = $state('1');
	let alertWindow = $state(300);
	let alertPerTarget = $state(true);
	let alertSeverity = $state<SeverityWord>('advisory');
	let alertSaving = $state(false);
	let alertError = $state<string | null>(null);
	let alertDone = $state(false);

	/** The rule's filter: the stream, the levels and the query, as this page applies them. */
	function ruleFilter(): string {
		const user = query.trim() || '*';
		const stream = source === 'all' ? '' : `service:=${SERVICE[source]} `;
		const levels = LEVELS_FROM[level].map((name) => `"${name}"`).join(', ');
		return `${stream}level:in(${levels}) (${user})`;
	}

	async function createAlert(event: SubmitEvent) {
		event.preventDefault();
		alertError = null;
		alertDone = false;
		const name = alertName.trim();
		if (!name) {
			alertError = m.logalert_name_required();
			return;
		}
		alertSaving = true;
		try {
			await createAlertRule({
				name,
				kind: 'log_pattern',
				query: ruleFilter(),
				operator: '>=',
				threshold: Math.max(1, Number(alertCount) || 1),
				for_secs: 0,
				severity: fromSeverityWord(alertSeverity),
				selector: device ? { kind: 'ids', ids: [Number(device)] } : { kind: 'all' },
				channels: [],
				params: { log_window_secs: alertWindow, log_per_target: alertPerTarget },
				unit: m.logalert_unit()
			});
			alertDone = true;
			alertName = '';
		} catch (cause) {
			alertError = cause instanceof Error ? cause.message : m.logalert_error();
		} finally {
			alertSaving = false;
		}
	}
</script>

<svelte:head><title>{m.logsearch_title()} · DumbMonit</title></svelte:head>

<PageHeader title={m.logsearch_title()} description={m.logsearch_description()}>
	{#snippet actions()}
		<a href="/settings/system#storage" class="inline-flex min-h-10 items-center text-sm text-ink-2 hover:text-ink hover:underline">{m.logsearch_settings_link()}</a>
	{/snippet}
</PageHeader>

{#if !auth.isAdmin}
	<Plate tone="ghost" label={auth.readOnlyLabel} size="md" />
	<p class="mt-3 text-sm text-ink-2">{m.logsearch_admin_only()}</p>
{:else}
	<form class="grid gap-4 rounded-[var(--radius-card)] border border-line bg-surface p-4 shadow-lift" onsubmit={run}>
		<Field label={m.logsearch_query()} for="logs-query" help={m.logsearch_query_help()}>
			<input id="logs-query" class="input font-mono" bind:value={query} autocomplete="off" spellcheck="false" placeholder="*" />
		</Field>
		<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
			<Field label={m.logsearch_period()} for="logs-period">
				<select id="logs-period" class="input" bind:value={period}>
					{#each PERIODS as p (p.id)}<option value={p.id}>{p.label()}</option>{/each}
				</select>
			</Field>
			<Field label={m.logsearch_level()} for="logs-level">
				<select id="logs-level" class="input" bind:value={level}>
					<option value="error">{m.logsearch_level_error()}</option>
					<option value="warn">{m.logsearch_level_warn()}</option>
					<option value="info">{m.logsearch_level_info()}</option>
					<option value="debug">{m.logsearch_level_debug()}</option>
				</select>
			</Field>
			<Field label={m.logsearch_source()} for="logs-source">
				<select id="logs-source" class="input" bind:value={source}>
					{#each SOURCES as entry (entry.id)}<option value={entry.id}>{entry.label()}</option>{/each}
				</select>
			</Field>
			<Field label={m.logsearch_target()} for="logs-device">
				<select id="logs-device" class="input" bind:value={device}>
					<option value="">{m.logsearch_target_all()}</option>
					{#each targets as entry (entry.id)}<option value={String(entry.id)}>{entry.name}</option>{/each}
				</select>
			</Field>
		</div>
		{#if period === 'custom'}
			<div class="grid gap-4 sm:grid-cols-2">
				<Field label={m.logsearch_from()} for="logs-from" error={rangeError ? m.logsearch_range_invalid() : null}>
					<input id="logs-from" type="datetime-local" class="input" bind:value={customStart} />
				</Field>
				<Field label={m.logsearch_to()} for="logs-to" help={m.logsearch_to_help()}>
					<input id="logs-to" type="datetime-local" class="input" bind:value={customEnd} />
				</Field>
			</div>
		{/if}
		<div>
			<Button type="submit" variant="primary" {loading}>
				<Search class="size-4" aria-hidden="true" />
				{m.logsearch_search()}
			</Button>
		</div>
	</form>

	<div class="mt-4 grid gap-3">
		<Disclosure title={m.logsearch_inputs_title()} bind:open={inputsOpen}>
			{#if inputsError}
				<ErrorNotice error={inputsError} title={m.logsearch_error()} />
			{:else if !inputs}
				<Skeleton class="h-16 w-full" />
			{:else}
				<div class="grid gap-4 sm:grid-cols-2">
					{#each [{ name: m.logsearch_inputs_syslog(), input: inputs.syslog, env: 'DUMBMONIT_SYSLOG_ENABLED=true' }, { name: m.logsearch_source_trap(), input: inputs.traps, env: 'DUMBMONIT_TRAPS_ENABLED=true' }] as card (card.env)}
						<div class="grid content-start gap-1.5 text-sm">
							<p class="flex flex-wrap items-center gap-2 font-semibold text-ink">
								{card.name}
								<Plate tone={card.input.listening.length > 0 ? 'signal' : 'ghost'} label={card.input.listening.length > 0 ? card.input.listening.join(', ') : m.logsearch_inputs_off()} />
							</p>
							{#if !card.input.enabled}
								<p class="text-ink-2">{m.logsearch_inputs_off_hint({ env: card.env })}</p>
							{:else}
								{#if card.input.listening.length > 0}
									<p class="text-ink-2">{m.logsearch_inputs_listening({ ports: card.input.listening.join(', ') })}</p>
								{/if}
								{#if card.input.error}<p class="text-warning-ink">{m.logsearch_inputs_error({ error: card.input.error })}</p>{/if}
								<p class="tnum text-ink-2">
									{m.logsearch_inputs_stats({ received: card.input.received, unmatched: card.input.unmatched, rejected: card.input.rejected })}
								</p>
								{#if card.input.last_at}
									<p class="text-ink-2">{m.logsearch_inputs_last({ when: formatRelative(card.input.last_at) })}</p>
								{/if}
							{/if}
						</div>
					{/each}
				</div>
				{#if !inputs.store_running}
					<p class="mt-3 text-sm text-warning-ink">{m.logsearch_inputs_store_off()}</p>
				{/if}
			{/if}
		</Disclosure>

		<Disclosure title={m.logalert_title()} bind:open={alertOpen}>
			<form class="grid gap-4" onsubmit={createAlert}>
				<p class="text-sm text-ink-2">{m.logalert_hint()}</p>
				<p class="break-words rounded-lg bg-canvas-deep px-2.5 py-1.5 font-mono text-[0.75rem] text-ink-2">{ruleFilter()}</p>
				<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
					<Field label={m.logalert_name()} for="logalert-name" required>
						<input id="logalert-name" class="input" bind:value={alertName} autocomplete="off" />
					</Field>
					<Field label={m.logalert_threshold()} for="logalert-count">
						<input id="logalert-count" class="input tnum" type="number" min="1" step="1" bind:value={alertCount} />
					</Field>
					<Field label={m.logalert_window()} for="logalert-window">
						<select id="logalert-window" class="input" bind:value={alertWindow}>
							{#each WINDOWS as seconds (seconds)}<option value={seconds}>{formatDuration(seconds)}</option>{/each}
						</select>
					</Field>
					<Field label={m.alerts_history_severity()} for="logalert-severity">
						<select id="logalert-severity" class="input" bind:value={alertSeverity}>
							{#each severityOptions() as option (option.id)}<option value={option.id}>{option.label}</option>{/each}
						</select>
					</Field>
				</div>
				<div>
					<label class="inline-flex min-h-10 items-center gap-2 text-sm text-ink">
						<input type="checkbox" class="size-4 accent-[var(--c-signal)]" bind:checked={alertPerTarget} />
						{m.logalert_per_target()}
					</label>
					<p class="text-[0.8125rem] text-ink-2">{m.logalert_per_target_help()}</p>
				</div>
				{#if alertError}<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{alertError}</p>{/if}
				{#if alertDone}
					<p class="text-sm text-ink" role="status">
						{m.logalert_created()}
						<a href="/alerts" class="font-semibold text-signal-ink hover:underline">{m.logalert_created_link()}</a>
					</p>
				{/if}
				<div><Button type="submit" variant="secondary" loading={alertSaving}>{m.logalert_create()}</Button></div>
			</form>
		</Disclosure>
	</div>

	<div class="mt-4" aria-live="polite">
		{#if loading && !result}
			<Skeleton class="h-40 w-full" />
		{:else if error}
			{#if status === 409}
				<div class="rounded-[var(--radius-card)] border border-line bg-surface p-4">
					<p class="text-sm text-ink">{m.logsearch_store_off()}</p>
					<a href="/settings/system#storage" class="mt-2 inline-flex min-h-10 items-center text-sm font-semibold text-signal-ink hover:underline">{m.logsearch_store_off_link()}</a>
				</div>
			{:else if status === 400}
				<div class="rounded-[var(--radius-card)] border border-warning bg-surface p-4" role="alert">
					<p class="text-sm font-semibold text-ink">{m.logsearch_query_refused()}</p>
					<p class="mt-1 break-words font-mono text-[0.8125rem] text-ink-2">{(error as Error).message}</p>
				</div>
			{:else}
				<ErrorNotice {error} title={m.logsearch_error()} onretry={() => void run()} />
			{/if}
		{:else if result}
			{#if result.entries.length === 0}
				<EmptyState title={m.logsearch_empty_title()} description={m.logsearch_empty_hint()} />
			{:else}
				<p class="mb-2 text-[0.8125rem] text-ink-3">
					{m.logsearch_count({ count: result.entries.length })}
					{#if result.truncated}· {m.logsearch_truncated()}{/if}
				</p>
				<ul class="divide-y divide-line rounded-[var(--radius-card)] border border-line bg-surface">
					{#each result.entries as entry, i (i)}
						{@const fields = Object.entries(entry.fields)}
						<li>
							<button
								type="button"
								class="flex min-h-10 w-full items-start gap-2 px-3 py-2 text-left hover:bg-line/40"
								aria-expanded={open[i] ? 'true' : 'false'}
								onclick={() => (open[i] = !open[i])}
							>
								<ChevronRight class={`mt-1 size-4 shrink-0 text-ink-3 transition-transform ${open[i] ? 'rotate-90' : ''}`} aria-hidden="true" />
								<span class="min-w-0 flex-1">
									<span class="flex flex-wrap items-center gap-x-3 gap-y-1">
										<time class="tnum text-[0.8125rem] text-ink-2">{formatDateTime(entry.time)}</time>
										<Plate tone={tone(entry.level)} label={levelLabel(entry.level)} />
										{#if entry.target_name}<span class="text-[0.8125rem] font-medium text-ink">{entry.target_name}</span>{/if}
										{#if entry.origin}<span class="break-all font-mono text-[0.75rem] text-ink-3">{entry.origin}</span>{/if}
									</span>
									<span class="mt-1 block break-words font-mono text-[0.8125rem] text-ink {open[i] ? '' : 'line-clamp-2'}">{entry.message}</span>
								</span>
							</button>
							{#if open[i]}
								<dl class="grid gap-x-4 gap-y-1 border-t border-line bg-canvas px-4 py-3 text-[0.8125rem] sm:grid-cols-[max-content_1fr]">
									<dt class="text-ink-3">{m.logsearch_field_source()}</dt>
									<dd class="break-all font-mono text-ink">{entry.source}</dd>
									{#if entry.origin}
										<dt class="text-ink-3">{m.logsearch_field_origin()}</dt>
										<dd class="break-all font-mono text-ink">{entry.origin}</dd>
									{/if}
									{#if entry.target_name}
										<dt class="text-ink-3">{m.logsearch_field_device()}</dt>
										<dd class="break-all text-ink">
											{#if entry.fields.target}<a href={`/targets/${entry.fields.target}`} class="underline decoration-line underline-offset-2 hover:text-ink">{entry.target_name}</a>{:else}{entry.target_name}{/if}
										</dd>
									{/if}
									{#if entry.host}
										<dt class="text-ink-3">{m.logsearch_field_host()}</dt>
										<dd class="break-all font-mono text-ink">{entry.host}</dd>
									{/if}
									{#each fields as [key, value] (key)}
										<dt class="break-all text-ink-3">{key}</dt>
										<dd class="break-all font-mono text-ink">{value}</dd>
									{/each}
								</dl>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		{/if}
	</div>
{/if}
