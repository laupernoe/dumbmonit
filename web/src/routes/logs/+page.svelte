<script lang="ts">
	/**
	 * Log search (administrators): LogsQL over the embedded log store. Server
	 * logs only for now; agents come later. Newest first, one expandable row
	 * per entry.
	 */
	import { ChevronRight, Search } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { searchLogs } from '#lib/api/storage.js';
	import { ApiError } from '#lib/api/client.js';
	import type { LogSearchResult } from '#lib/api/types.js';
	import { formatDateTime } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, EmptyState, ErrorNotice, Field, PageHeader, Plate, Skeleton } from '#lib/ui/index.js';
	import type { Tone } from '#lib/ui/index.js';
	import { levelLabel } from '#lib/components/settings/logLabels.js';
	import { periodRange, type SearchPeriod } from '#lib/components/settings/storage.js';

	type Level = 'error' | 'warn' | 'info' | 'debug';

	let query = $state('*');
	let period = $state<SearchPeriod>('1h');
	let customStart = $state('');
	let customEnd = $state('');
	let level = $state<Level>('info');
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

	const status = $derived(error instanceof ApiError ? error.status : 0);

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
			result = await searchLogs({ query: query.trim() || '*', start: range.start, end: range.end, level, source: 'server', limit: 200 }, controller.signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			result = null;
			error = cause;
		} finally {
			loading = false;
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
		<div class="grid gap-4 sm:grid-cols-3">
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
				<select id="logs-source" class="input" value="server">
					<option value="server">{m.logsearch_source_server()}</option>
					<option value="agents" disabled>{m.logsearch_source_agents()}</option>
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
									</span>
									<span class="mt-1 block break-words font-mono text-[0.8125rem] text-ink {open[i] ? '' : 'line-clamp-2'}">{entry.message}</span>
								</span>
							</button>
							{#if open[i]}
								<dl class="grid gap-x-4 gap-y-1 border-t border-line bg-canvas px-4 py-3 text-[0.8125rem] sm:grid-cols-[max-content_1fr]">
									<dt class="text-ink-3">{m.logsearch_field_source()}</dt>
									<dd class="break-all font-mono text-ink">{entry.source}</dd>
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
