<script lang="ts">
	/**
	 * Settings → System → Logs (administrators): the server's own logs on disk
	 * (retention, size cap), an export of the last days, and destinations that
	 * forward the logs to a log server (Graylog, Loki, Elasticsearch, Syslog…).
	 */
	import { Download, Pencil, Plus, Send } from 'lucide-svelte';
	import {
		deleteLogDestination,
		exportUrl,
		getExportEstimate,
		getLogs,
		listLogDestinations,
		listLogKinds,
		setLogsSettings,
		testLogDestination
	} from '#lib/api/logs.js';
	import type { LogDestination, LogExportEstimate, LogsInfo, LogTestResult } from '#lib/api/types.js';
	import { formatRelative } from '#lib/format.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Field, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { normalizeKind, type KindInfo } from '#lib/components/notifications/kinds';
	import { m } from '#lib/paraglide/messages.js';
	import LogDestinationForm from './LogDestinationForm.svelte';
	import { levelLabel } from './logLabels';
	import { MAX_MB_MAX, MAX_MB_MIN, RETENTION_MAX, RETENTION_MIN, clampDays, exportChoices, formatSize, inRange } from './logs';

	let info = $state<LogsInfo | null>(null);
	let kinds = $state<KindInfo[]>([]);
	let kindsError = $state<unknown>(null);
	let destinations = $state<LogDestination[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const [logs, dests] = await Promise.all([getLogs(signal), listLogDestinations(signal)]);
			fill(logs);
			destinations = dests;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
		// The catalogue only matters for the form: a failure must not hide the rest.
		try {
			kinds = (await listLogKinds(signal)).map(normalizeKind);
			kindsError = null;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			kindsError = cause;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	// --- Local storage --------------------------------------------------------
	let retention = $state('14');
	let maxMb = $state('1024');
	let retentionError = $state<string | null>(null);
	let maxMbError = $state<string | null>(null);
	let saving = $state(false);
	let saved = $state(false);
	let saveError = $state<unknown>(null);

	function fill(value: LogsInfo) {
		info = value;
		retention = String(value.retention_days);
		maxMb = String(value.max_total_mb);
	}

	async function saveSettings(event: SubmitEvent) {
		event.preventDefault();
		saved = false;
		saveError = null;
		retentionError = inRange(retention, RETENTION_MIN, RETENTION_MAX)
			? null
			: m.settings_logs_range_error({ min: RETENTION_MIN, max: RETENTION_MAX });
		maxMbError = inRange(maxMb, MAX_MB_MIN, MAX_MB_MAX)
			? null
			: m.settings_logs_range_error({ min: MAX_MB_MIN, max: MAX_MB_MAX });
		if (retentionError || maxMbError) return;
		saving = true;
		try {
			fill(await setLogsSettings({ retention_days: Number(retention), max_total_mb: Number(maxMb) }));
			saved = true;
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = false;
		}
	}

	// --- Export ---------------------------------------------------------------
	const retentionDays = $derived(info?.retention_days ?? 14);
	const choices = $derived(exportChoices(retentionDays));
	/** `'custom'` or one of the quick choices, as a string. */
	let period = $state('7');
	let customDays = $state('');
	const days = $derived(period === 'custom' ? clampDays(customDays, retentionDays) : clampDays(period, retentionDays));

	// A choice that stopped fitting after the retention shrank falls back.
	$effect(() => {
		if (period !== 'custom' && Number(period) > retentionDays) period = String(Math.min(7, retentionDays));
	});

	let estimate = $state<LogExportEstimate | null>(null);
	let estimating = $state(false);
	let estimateError = $state(false);

	// The size is asked a moment after the choice settles, never on each keystroke.
	$effect(() => {
		const wanted = days;
		estimate = null;
		estimateError = false;
		if (wanted === null || !info) {
			estimating = false;
			return;
		}
		estimating = true;
		const controller = new AbortController();
		const timer = setTimeout(async () => {
			try {
				estimate = await getExportEstimate(wanted, controller.signal);
			} catch (cause) {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				estimateError = true;
			} finally {
				if (!controller.signal.aborted) estimating = false;
			}
		}, 300);
		return () => {
			clearTimeout(timer);
			controller.abort();
		};
	});

	const canDownload = $derived(days !== null && estimate !== null && estimate.days === days);

	// --- Destinations ---------------------------------------------------------
	let formOpen = $state(false);
	let editing = $state<LogDestination | undefined>(undefined);
	let busy = $state<{ id: number; action: 'delete' | 'test' } | null>(null);
	let actionError = $state<unknown>(null);
	let testResult = $state<({ id: number } & LogTestResult) | null>(null);

	function openAdd() {
		editing = undefined;
		formOpen = true;
	}
	function openEdit(destination: LogDestination) {
		editing = destination;
		formOpen = true;
	}
	function closeForm() {
		formOpen = false;
		editing = undefined;
	}
	function onsaved(saved: LogDestination) {
		const i = destinations.findIndex((d) => d.id === saved.id);
		destinations = i >= 0 ? destinations.map((d) => (d.id === saved.id ? saved : d)) : [...destinations, saved];
		closeForm();
	}

	async function remove(destination: LogDestination) {
		busy = { id: destination.id, action: 'delete' };
		actionError = null;
		try {
			await deleteLogDestination(destination.id);
			destinations = destinations.filter((d) => d.id !== destination.id);
		} catch (cause) {
			actionError = cause;
		} finally {
			busy = null;
		}
	}

	async function test(destination: LogDestination) {
		busy = { id: destination.id, action: 'test' };
		actionError = null;
		testResult = null;
		try {
			testResult = { id: destination.id, ...(await testLogDestination(destination.id)) };
		} catch (cause) {
			actionError = cause;
		} finally {
			busy = null;
		}
	}

	function kindLabel(kind: string): string {
		return kinds.find((k) => k.kind === kind)?.label ?? kind;
	}

	function state(destination: LogDestination): { tone: 'signal' | 'warning' | 'ghost'; label: string } {
		if (!destination.enabled) return { tone: 'ghost', label: m.settings_logs_dest_state_off() };
		const s = destination.stats;
		if (s?.last_error_at && (!s.last_success_at || s.last_error_at > s.last_success_at)) {
			return { tone: 'warning', label: m.settings_logs_dest_state_failing() };
		}
		if (s && s.sent > 0) return { tone: 'signal', label: m.settings_logs_dest_state_ok() };
		return { tone: 'ghost', label: m.settings_logs_dest_state_idle() };
	}
</script>

<Panel id="logs" title={m.settings_logs_title()} description={m.settings_logs_description()}>
	{#if error}
		<ErrorNotice {error} title={m.settings_logs_read_error()} onretry={() => void load()} />
	{:else if loading && !info}
		<Skeleton class="h-40 w-full" />
	{:else if info}
		<div class="grid gap-8">
			<!-- Local storage -->
			<section aria-labelledby="logs-storage-title">
				<h3 id="logs-storage-title" class="font-semibold text-ink">{m.settings_logs_storage_title()}</h3>
				<dl class="mt-3 grid gap-x-6 gap-y-2 text-sm sm:grid-cols-3">
					<div class="min-w-0">
						<dt class="text-ink-3">{m.settings_logs_directory()}</dt>
						<dd class="break-all font-mono text-[0.8125rem] text-ink">{info.directory}</dd>
					</div>
					<div>
						<dt class="text-ink-3">{m.settings_logs_total_size()}</dt>
						<dd class="tnum text-ink">{formatSize(info.total_bytes)}</dd>
					</div>
					<div>
						<dt class="text-ink-3">{m.settings_logs_days_available()}</dt>
						<dd class="tnum text-ink">{info.days_available}</dd>
					</div>
				</dl>

				{#if info.writer.dropped > 0}
					<p class="mt-3 flex flex-wrap items-center gap-2 text-sm text-ink-2">
						<Plate tone="warning" label={m.settings_logs_dropped_label()} />
						<span>{m.settings_logs_dropped({ count: info.writer.dropped })}</span>
					</p>
				{/if}

				<form class="mt-4 grid max-w-xl gap-4 sm:grid-cols-2" onsubmit={saveSettings} novalidate>
					<Field
						label={m.settings_logs_retention()}
						for="logs-retention"
						error={retentionError}
						help={m.settings_logs_retention_help({ min: RETENTION_MIN, max: RETENTION_MAX })}
					>
						<input
							id="logs-retention"
							type="number"
							inputmode="numeric"
							min={RETENTION_MIN}
							max={RETENTION_MAX}
							step="1"
							class="input"
							bind:value={retention}
							disabled={saving}
							aria-invalid={retentionError ? 'true' : undefined}
							oninput={() => ((retentionError = null), (saved = false))}
						/>
					</Field>
					<Field
						label={m.settings_logs_max_mb()}
						for="logs-max-mb"
						error={maxMbError}
						help={m.settings_logs_max_mb_help({ min: MAX_MB_MIN, max: MAX_MB_MAX })}
					>
						<input
							id="logs-max-mb"
							type="number"
							inputmode="numeric"
							min={MAX_MB_MIN}
							max={MAX_MB_MAX}
							step="1"
							class="input"
							bind:value={maxMb}
							disabled={saving}
							aria-invalid={maxMbError ? 'true' : undefined}
							oninput={() => ((maxMbError = null), (saved = false))}
						/>
					</Field>
					{#if saveError}
						<div class="sm:col-span-2"><ErrorNotice error={saveError} title={m.settings_logs_save_error()} /></div>
					{/if}
					<div class="flex flex-wrap items-center gap-3 sm:col-span-2" aria-live="polite">
						<Button type="submit" variant="primary" loading={saving}>{m.settings_logs_save()}</Button>
						{#if saved}<Plate tone="signal" draw label={m.settings_logs_saved()} />{/if}
					</div>
				</form>
			</section>

			<!-- Export -->
			<section class="border-t border-line pt-6" aria-labelledby="logs-export-title">
				<h3 id="logs-export-title" class="font-semibold text-ink">{m.settings_logs_export_title()}</h3>
				<p class="mt-1 max-w-prose text-sm text-ink-2">{m.settings_logs_export_hint()}</p>

				<fieldset class="mt-3">
					<legend class="text-sm font-semibold text-ink">{m.settings_logs_export_period()}</legend>
					<div class="mt-2 flex flex-wrap gap-2">
						{#each choices as choice (choice.days)}
							<label
								class={`inline-flex min-h-10 cursor-pointer items-center gap-2 rounded-lg border px-3 text-sm ${period === String(choice.days) ? 'border-signal bg-signal-soft text-ink' : 'border-line-strong bg-surface text-ink'} ${choice.disabled ? 'cursor-not-allowed opacity-50' : ''}`}
							>
								<input type="radio" name="logs-period" class="sr-only" value={String(choice.days)} bind:group={period} disabled={choice.disabled} />
								{choice.days === 1 ? m.settings_logs_export_last_day() : m.settings_logs_export_last_days({ days: choice.days })}
							</label>
						{/each}
						<label
							class={`inline-flex min-h-10 cursor-pointer items-center gap-2 rounded-lg border px-3 text-sm ${period === 'custom' ? 'border-signal bg-signal-soft text-ink' : 'border-line-strong bg-surface text-ink'}`}
						>
							<input type="radio" name="logs-period" class="sr-only" value="custom" bind:group={period} />
							{m.settings_logs_export_custom()}
						</label>
					</div>
					{#if period === 'custom'}
						<div class="mt-3 max-w-40">
							<Field label={m.settings_logs_export_custom_days()} for="logs-custom-days" help={m.settings_logs_export_custom_help({ max: retentionDays })}>
								<input
									id="logs-custom-days"
									type="number"
									inputmode="numeric"
									min="1"
									max={retentionDays}
									step="1"
									class="input"
									bind:value={customDays}
								/>
							</Field>
						</div>
					{/if}
				</fieldset>

				<p class="mt-3 min-h-6 text-sm text-ink-2" aria-live="polite">
					{#if estimateError}
						{m.settings_logs_export_estimate_error()}
					{:else if estimate && estimate.days === days}
						{m.settings_logs_export_estimate({ size: formatSize(estimate.bytes) })}
					{:else if estimating}
						{m.settings_logs_export_estimating()}
					{/if}
				</p>

				<div class="mt-2">
					{#if canDownload && days !== null}
						<Button variant="secondary" href={exportUrl(days)} download>
							<Download class="size-4" aria-hidden="true" />
							{m.settings_logs_export_download()}
						</Button>
					{:else}
						<Button variant="secondary" disabled>
							<Download class="size-4" aria-hidden="true" />
							{m.settings_logs_export_download()}
						</Button>
					{/if}
				</div>
			</section>

			<!-- Destinations -->
			<section class="border-t border-line pt-6" aria-labelledby="logs-dest-title">
				<div class="flex flex-wrap items-start justify-between gap-3">
					<div class="min-w-0">
						<h3 id="logs-dest-title" class="font-semibold text-ink">{m.settings_logs_dest_title()}</h3>
						<p class="mt-1 max-w-prose text-sm text-ink-2">{m.settings_logs_dest_hint()}</p>
					</div>
					{#if !formOpen}
						<Button variant="secondary" onclick={openAdd} disabled={kinds.length === 0}>
							<Plus class="size-4" aria-hidden="true" />
							{m.settings_logs_dest_add()}
						</Button>
					{/if}
				</div>

				{#if kindsError}
					<div class="mt-3"><ErrorNotice error={kindsError} title={m.settings_logs_kinds_error()} /></div>
				{/if}
				{#if actionError}
					<div class="mt-3"><ErrorNotice error={actionError} title={m.settings_logs_action_error()} /></div>
				{/if}

				{#if formOpen}
					<div class="mt-4">
						{#key editing?.id ?? 'new'}
							<LogDestinationForm {kinds} destination={editing} {onsaved} oncancel={closeForm} />
						{/key}
					</div>
				{/if}

				{#if destinations.length === 0 && !formOpen}
					<div class="mt-4"><EmptyState title={m.settings_logs_dest_empty()} /></div>
				{:else if destinations.length > 0}
					<ul class="mt-4 grid gap-3" role="list">
						{#each destinations as destination (destination.id)}
							{@const st = state(destination)}
							{@const stats = destination.stats}
							<li class="rounded-lg border border-line bg-surface-2/40 px-3 py-3 sm:px-4">
								<div class="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
									<div class="min-w-0">
										<p class="break-words text-sm font-semibold text-ink">{destination.name}</p>
										<p class="text-[0.8125rem] text-ink-2">
											{kindLabel(destination.kind)} · {m.settings_logs_dest_min_level({ level: levelLabel(destination.min_level) })}
										</p>
									</div>
									<Plate tone={st.tone} label={st.label} />
								</div>

								<dl class="tnum mt-2 flex flex-wrap gap-x-5 gap-y-1 text-[0.8125rem] text-ink-2">
									<div class="flex gap-1"><dt class="sr-only">{m.settings_logs_dest_sent_label()}</dt><dd>{m.settings_logs_dest_sent({ count: stats?.sent ?? 0 })}</dd></div>
									<div class="flex gap-1"><dt class="sr-only">{m.settings_logs_dest_lost_label()}</dt><dd>{m.settings_logs_dest_lost({ count: stats?.dropped ?? 0 })}</dd></div>
									{#if stats?.last_success_at}
										<div class="flex gap-1"><dt class="sr-only">{m.settings_logs_dest_last_success_label()}</dt><dd>{m.settings_logs_dest_last_success({ when: formatRelative(stats.last_success_at) })}</dd></div>
									{/if}
								</dl>
								{#if stats?.last_error}
									<p class="mt-1 break-words text-[0.8125rem] text-ink-2">
										{m.settings_logs_dest_last_error({ error: stats.last_error, when: formatRelative(stats.last_error_at) })}
									</p>
								{/if}

								<div class="mt-3 flex flex-wrap items-center gap-2">
									<Button size="sm" variant="secondary" loading={busy?.id === destination.id && busy.action === 'test'} disabled={busy !== null} onclick={() => void test(destination)}>
										<Send class="size-4" aria-hidden="true" />
										{m.settings_logs_dest_test()}
									</Button>
									<Button size="sm" variant="ghost" disabled={busy !== null} onclick={() => openEdit(destination)}>
										<Pencil class="size-4" aria-hidden="true" />
										{m.settings_logs_dest_edit()}
									</Button>
									<Confirm
										confirmLabel={m.settings_logs_dest_remove_confirm()}
										loading={busy?.id === destination.id && busy.action === 'delete'}
										disabled={busy !== null}
										onconfirm={() => remove(destination)}
									>
										{m.settings_logs_dest_remove()}
									</Confirm>
								</div>
								<div aria-live="polite">
									{#if testResult?.id === destination.id}
										<p class="mt-3 flex flex-wrap items-center gap-2 text-sm">
											<Plate tone={testResult.ok ? 'signal' : 'warning'} label={testResult.ok ? m.settings_logs_dest_test_ok() : m.settings_logs_dest_test_failed()} />
											<span class="min-w-0 break-words text-ink-2">{testResult.message}</span>
										</p>
									{/if}
								</div>
							</li>
						{/each}
					</ul>
				{/if}
			</section>
		</div>
	{/if}
</Panel>
