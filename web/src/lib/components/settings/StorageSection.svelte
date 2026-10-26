<script lang="ts">
	/**
	 * Settings → System → Data & retention (administrators): what the disk
	 * holds and how fast it grows, the raw metrics retention, the long-term
	 * 5-minute history (second small VictoriaMetrics) and the optional log
	 * store (VictoriaLogs). A change restarts the embedded process concerned,
	 * so saving asks for a confirmation first.
	 */
	import { ExternalLink, TriangleAlert } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getStorage, updateStorage } from '#lib/api/storage.js';
	import type { SettingSource, StorageUsageKey, StorageView } from '#lib/api/types.js';
	import { formatDateTime } from '#lib/format.js';
	import { localizedDocsUrl } from '#lib/docs.js';
	import { Button, Disclosure, ErrorNotice, Field, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import RetentionInput from './RetentionInput.svelte';
	import { diffUpdate, diskShare, formatBytes, formFrom, lowersRawRetention, needsRestart, parseDays, type StorageForm } from './storage';

	const EXTERNAL_VM_DOCS = 'https://dumbmonit.readthedocs.io/en/latest/reference/external-victoriametrics/';

	let view = $state<StorageView | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let form = $state<StorageForm>({ metricsDays: 90, rollupEnabled: false, rollupDays: 730, logsEnabled: false, logsDays: 30 });
	let saving = $state(false);
	let saveError = $state<unknown>(null);
	let confirming = $state(false);
	let restarted = $state<string[] | null>(null);
	let invalid = $state<Record<string, boolean>>({});

	function fill(value: StorageView) {
		view = value;
		form = formFrom(value);
	}

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			fill(await getStorage(signal));
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

	function usageLabel(key: StorageUsageKey): string {
		switch (key) {
			case 'metrics':
				return m.storage_usage_metrics();
			case 'rollup':
				return m.storage_usage_rollup();
			case 'logs_store':
				return m.storage_usage_logs_store();
			case 'database':
				return m.storage_usage_database();
			case 'logs':
				return m.storage_usage_logs();
			case 'backups':
				return m.storage_usage_backups();
		}
	}

	function sourceLabel(source: SettingSource): string {
		switch (source) {
			case 'settings':
				return m.storage_source_settings();
			case 'environment':
				return m.storage_source_environment();
			case 'default':
				return m.storage_source_default();
		}
	}

	function processLabel(name: string): string {
		if (name === 'metrics') return m.storage_process_metrics();
		if (name === 'rollup') return m.storage_process_rollup();
		return m.storage_process_logs_store();
	}

	const update = $derived(view ? diffUpdate(view, form) : {});
	const dirty = $derived(needsRestart(update));
	const lowers = $derived(view ? lowersRawRetention(view, update) : false);
	const freePercent = $derived(
		view?.disk.free_bytes != null && view.disk.total_bytes ? Math.round((100 * view.disk.free_bytes) / view.disk.total_bytes) : null
	);

	function validate(): boolean {
		if (!view) return false;
		const l = view.limits;
		invalid = {
			metrics: view.metrics.embedded && parseDays(form.metricsDays, l.min_retention_days, l.max_retention_days) === null,
			rollup: parseDays(form.rollupDays, l.min_rollup_retention_days, l.max_rollup_retention_days) === null,
			logs: parseDays(form.logsDays, l.min_logs_retention_days, l.max_logs_retention_days) === null
		};
		return !Object.values(invalid).some(Boolean);
	}

	function ask(event: SubmitEvent) {
		event.preventDefault();
		saveError = null;
		restarted = null;
		if (!dirty || !validate()) return;
		confirming = true;
	}

	async function apply() {
		saving = true;
		saveError = null;
		try {
			const result = await updateStorage(update);
			fill(result);
			restarted = result.restarted;
			confirming = false;
		} catch (cause) {
			saveError = cause;
			confirming = false;
		} finally {
			saving = false;
		}
	}
</script>

<Panel id="storage" title={m.storage_title()} description={m.storage_description()}>
	{#if error}
		<ErrorNotice {error} title={m.storage_read_error()} onretry={() => void load()} />
	{:else if loading && !view}
		<Skeleton class="h-40 w-full" />
	{:else if view}
		<form class="grid gap-3" onsubmit={ask} novalidate>
			<!-- Disk -->
			<Disclosure title={m.storage_disk_title()} open id="storage-disk">
				<div class="mt-3 grid gap-4">
					<ul class="grid gap-3">
						{#each view.disk.usage as row (row.key)}
							<li>
								<div class="flex flex-wrap items-baseline justify-between gap-x-4">
									<span class="text-sm font-semibold text-ink">{usageLabel(row.key)}</span>
									<span class="tnum text-sm text-ink">{formatBytes(row.bytes)}</span>
								</div>
								<div class="mt-1 h-1.5 overflow-hidden rounded-full bg-line" aria-hidden="true">
									<div class="h-full rounded-full bg-ink-2" style:width={`${diskShare(row.bytes, view.disk.total_bytes)}%`}></div>
								</div>
								<p class="mt-0.5 break-all font-mono text-[0.75rem] text-ink-3">{row.path}</p>
							</li>
						{/each}
					</ul>
					<dl class="grid gap-x-6 gap-y-2 text-sm sm:grid-cols-2">
						<div>
							<dt class="text-ink-3">{m.storage_free()}</dt>
							<dd class="tnum text-ink">
								{#if view.disk.free_bytes !== null && view.disk.total_bytes !== null}
									{m.storage_free_of({ free: formatBytes(view.disk.free_bytes), total: formatBytes(view.disk.total_bytes), percent: freePercent ?? 0 })}
								{:else}
									{m.storage_unknown()}
								{/if}
							</dd>
						</div>
						<div>
							<dt class="text-ink-3">{m.storage_growth()}</dt>
							<dd class="tnum text-ink">
								{#if view.growth.bytes_per_day !== null}
									{m.storage_growth_per_day({ size: formatBytes(view.growth.bytes_per_day) })}
									{#if view.growth.days_until_full !== null}
										<span class="block text-ink-2">{m.storage_full_in({ count: Math.round(view.growth.days_until_full) })}</span>
									{/if}
								{:else}
									<span class="text-ink-2">{m.storage_not_enough_history()}</span>
								{/if}
							</dd>
						</div>
					</dl>
				</div>
			</Disclosure>

			<!-- Raw metrics -->
			<Disclosure title={m.storage_metrics_title()} summary={m.storage_days({ count: view.metrics.retention_days })} id="storage-metrics">
				<div class="mt-3 grid max-w-xl gap-3">
					{#if view.metrics.embedded}
						<p class="max-w-prose text-sm text-ink-2">{m.storage_metrics_intro()}</p>
						<Field label={m.storage_metrics_retention()} for="storage-metrics-days" help={m.storage_limits({ min: view.limits.min_retention_days, max: view.limits.max_retention_days })} error={invalid.metrics ? m.storage_days_invalid({ min: view.limits.min_retention_days, max: view.limits.max_retention_days }) : null}>
							<RetentionInput id="storage-metrics-days" bind:value={form.metricsDays} min={view.limits.min_retention_days} max={view.limits.max_retention_days} disabled={saving} invalid={invalid.metrics} />
						</Field>
						<p class="text-[0.8125rem] text-ink-3">{m.storage_source({ source: sourceLabel(view.metrics.retention_source) })}</p>
					{:else}
						<p class="max-w-prose text-sm text-ink-2">{m.storage_metrics_external({ days: view.metrics.retention_days })}</p>
						<a href={localizedDocsUrl(EXTERNAL_VM_DOCS)} target="_blank" rel="noopener" class="inline-flex min-h-10 items-center gap-1 text-[0.8125rem] font-semibold text-ink-2 hover:text-ink hover:underline">
							{m.storage_docs_external()}
							<ExternalLink class="size-3.5" aria-hidden="true" />
						</a>
					{/if}
				</div>
			</Disclosure>

			<!-- Long-term history -->
			<Disclosure title={m.storage_rollup_title()} summary={view.rollup.enabled ? m.storage_on() : m.storage_off()} id="storage-rollup">
				<div class="mt-3 grid max-w-xl gap-3">
					<p class="max-w-prose text-sm text-ink-2">{m.storage_rollup_intro()}</p>
					{#if !view.rollup.available}
						<p class="max-w-prose text-sm text-ink-2">{m.storage_rollup_unavailable()}</p>
					{:else}
						<Field label={m.storage_rollup_enabled()} for="storage-rollup-enabled" inline>
							<Toggle id="storage-rollup-enabled" bind:checked={form.rollupEnabled} disabled={saving} label={m.storage_rollup_enabled()} />
						</Field>
						<Field label={m.storage_rollup_retention()} for="storage-rollup-days" help={m.storage_limits({ min: view.limits.min_rollup_retention_days, max: view.limits.max_rollup_retention_days })} error={invalid.rollup ? m.storage_days_invalid({ min: view.limits.min_rollup_retention_days, max: view.limits.max_rollup_retention_days }) : null}>
							<RetentionInput id="storage-rollup-days" bind:value={form.rollupDays} min={view.limits.min_rollup_retention_days} max={view.limits.max_rollup_retention_days} disabled={saving} invalid={invalid.rollup} />
						</Field>
						{#if view.rollup.enabled}
							<dl class="grid gap-x-6 gap-y-2 text-sm sm:grid-cols-2" aria-live="polite">
								<div>
									<dt class="text-ink-3">{m.storage_status()}</dt>
									<dd><Plate tone={view.rollup.running ? 'signal' : 'warning'} label={view.rollup.running ? m.storage_running() : m.storage_stopped()} /></dd>
								</div>
								<div>
									<dt class="text-ink-3">{m.storage_rollup_last_window()}</dt>
									<dd class="tnum text-ink">{view.rollup.last_window ? formatDateTime(view.rollup.last_window) : m.storage_none_yet()}</dd>
								</div>
								<div>
									<dt class="text-ink-3">{m.storage_rollup_backfill()}</dt>
									<dd class="tnum text-ink">
										{#if view.rollup.backfill_done}
											{m.storage_rollup_backfill_done()}
										{:else if view.rollup.backfilled_until}
											{m.storage_rollup_backfill_until({ date: formatDateTime(view.rollup.backfilled_until) })}
										{:else}
											{m.storage_none_yet()}
										{/if}
									</dd>
								</div>
								<div>
									<dt class="text-ink-3">{m.storage_rollup_series()}</dt>
									<dd class="tnum text-ink">{view.rollup.series ?? m.storage_unknown()}</dd>
								</div>
							</dl>
							{#if view.rollup.last_error}
								<p class="break-words text-[0.8125rem] text-warning-ink">{m.storage_last_error({ error: view.rollup.last_error })}</p>
							{/if}
						{/if}
					{/if}
				</div>
			</Disclosure>

			<!-- Log store -->
			<Disclosure title={m.storage_logstore_title()} summary={view.logs_store.enabled ? m.storage_on() : m.storage_off()} id="storage-logstore">
				<div class="mt-3 grid max-w-xl gap-3">
					<p class="max-w-prose text-sm text-ink-2">{m.storage_logstore_intro()}</p>
					{#if !view.logs_store.available}
						<p class="max-w-prose text-sm text-ink-2">{m.storage_logstore_unavailable()}</p>
					{:else}
						<Field label={m.storage_logstore_enabled()} for="storage-logs-enabled" inline>
							<Toggle id="storage-logs-enabled" bind:checked={form.logsEnabled} disabled={saving} label={m.storage_logstore_enabled()} />
						</Field>
						<Field label={m.storage_logstore_retention()} for="storage-logs-days" help={m.storage_limits({ min: view.limits.min_logs_retention_days, max: view.limits.max_logs_retention_days })} error={invalid.logs ? m.storage_days_invalid({ min: view.limits.min_logs_retention_days, max: view.limits.max_logs_retention_days }) : null}>
							<RetentionInput id="storage-logs-days" bind:value={form.logsDays} min={view.limits.min_logs_retention_days} max={view.limits.max_logs_retention_days} disabled={saving} invalid={invalid.logs} />
						</Field>
						{#if view.logs_store.enabled}
							<div class="flex flex-wrap items-center gap-3" aria-live="polite">
								<Plate tone={view.logs_store.running ? 'signal' : 'warning'} label={view.logs_store.running ? m.storage_running() : m.storage_stopped()} />
								{#if view.logs_store.running}
									<a href="/logs" class="inline-flex min-h-10 items-center text-sm font-semibold text-signal-ink hover:underline">{m.storage_open_logs()}</a>
								{/if}
							</div>
							{#if view.logs_store.last_error}
								<p class="break-words text-[0.8125rem] text-warning-ink">{m.storage_last_error({ error: view.logs_store.last_error })}</p>
							{/if}
						{/if}
					{/if}
				</div>
			</Disclosure>

			<!-- Save -->
			{#if confirming}
				<div class="grid max-w-xl gap-3 rounded-[var(--radius-card)] border border-warning bg-surface p-4" role="alert">
					<p class="flex items-start gap-2 text-sm text-ink">
						<TriangleAlert class="mt-0.5 size-4 shrink-0 text-warning-ink" aria-hidden="true" />
						<span>{m.storage_confirm_restart()}</span>
					</p>
					{#if lowers}
						<p class="text-sm text-ink">{m.storage_confirm_lowering()}</p>
					{/if}
					<div class="flex flex-wrap gap-3">
						<Button variant="primary" loading={saving} onclick={() => void apply()}>{m.storage_confirm_apply()}</Button>
						<Button variant="ghost" disabled={saving} onclick={() => (confirming = false)}>{m.storage_confirm_cancel()}</Button>
					</div>
				</div>
			{:else}
				<div class="flex flex-wrap items-center gap-3" aria-live="polite">
					<Button type="submit" variant="primary" disabled={!dirty || saving}>{m.storage_save()}</Button>
					{#if restarted !== null}
						<Plate tone="signal" draw label={restarted.length ? m.storage_saved_restarted({ processes: restarted.map(processLabel).join(', ') }) : m.storage_saved()} />
					{/if}
				</div>
			{/if}
			{#if saveError}
				<ErrorNotice error={saveError} title={m.storage_save_error()} />
			{/if}
		</form>
	{/if}
</Panel>
