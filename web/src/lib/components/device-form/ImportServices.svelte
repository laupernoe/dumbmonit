<script lang="ts">
	/**
	 * "Import from UptimeRobot / Gatus / Healthchecks.io": a modal dialog.
	 *
	 * Same flow as the Uptime Kuma import: give the source (an API key, a CSV
	 * export or a config file), preview what will be created and what is skipped
	 * (and why), tick the monitors, confirm, read the summary. The server fetches
	 * or parses, converts and validates; this component only shows its report.
	 * API keys travel once and are never stored. Admin only: the caller decides
	 * whether to render it.
	 */
	import { tick } from 'svelte';
	import { Upload, X } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { importFromService, SERVICE_IMPORT_MAX_BYTES, type ServiceImport } from '#lib/api/targetsImport.js';
	import { importableRows, selectionState, tickedRows, toggleRow } from './kuma-import.js';
	import type { KumaImportReport, TargetImportStatus } from '#lib/api/types.js';
	import { Button, ErrorNotice, Field, Plate } from '#lib/ui/index.js';
	import type { Tone } from '#lib/ui/index.js';

	export type Provider = 'uptimerobot' | 'gatus' | 'healthchecks';

	interface Props {
		provider: Provider;
		/** Called after monitors were created, so the page can reload its list. */
		ondone?: () => void;
	}
	let { provider, ondone }: Props = $props();

	let open = $state(false);
	let busy = $state(false);
	let error = $state<unknown>(null);
	let notice = $state('');
	let apiKey = $state('');
	let baseUrl = $state('');
	let text = $state('');
	/** What the preview was made from, kept so the confirmation sends the same request. */
	let source = $state<ServiceImport | null>(null);
	let selected = $state<Set<number>>(new Set());
	let preview = $state<KumaImportReport | null>(null);
	let result = $state<KumaImportReport | null>(null);
	let dialog = $state<HTMLElement | null>(null);
	let fileInput = $state<HTMLInputElement | null>(null);
	let returnFocus: HTMLElement | null = null;

	const report = $derived(result ?? preview);
	const TONE: Record<TargetImportStatus, Tone> = { ok: 'info', created: 'signal', skipped: 'advisory', error: 'warning' };

	const title = $derived(
		provider === 'uptimerobot' ? m.devices_svc_uptimerobot_title() : provider === 'gatus' ? m.devices_svc_gatus_title() : m.devices_svc_healthchecks_title()
	);
	const description = $derived(
		provider === 'uptimerobot'
			? m.devices_svc_uptimerobot_description()
			: provider === 'gatus'
				? m.devices_svc_gatus_description()
				: m.devices_svc_healthchecks_description()
	);

	function statusLabel(status: TargetImportStatus): string {
		switch (status) {
			case 'ok':
				return m.devices_import_status_ok();
			case 'created':
				return m.devices_import_status_created();
			case 'skipped':
				return m.devices_import_status_skipped();
			default:
				return m.devices_import_status_error();
		}
	}

	export function show() {
		returnFocus = (document.activeElement as HTMLElement | null) ?? null;
		source = null;
		selected = new Set();
		preview = null;
		result = null;
		error = null;
		notice = '';
		apiKey = '';
		baseUrl = '';
		text = '';
		open = true;
		void tick().then(() => dialog?.focus());
	}

	function close() {
		open = false;
		// Never keep a key around once the dialog is gone.
		apiKey = '';
		returnFocus?.focus();
	}

	/** The request the form currently describes, or null while it is incomplete. */
	function describe(): ServiceImport | null {
		if (provider === 'gatus') return text.trim() ? { provider, yaml: text } : null;
		if (provider === 'healthchecks') return apiKey.trim() ? { provider, apiKey: apiKey.trim(), baseUrl: baseUrl.trim() || undefined } : null;
		return apiKey.trim() ? { provider, apiKey: apiKey.trim() } : null;
	}

	async function run(dryRun: boolean, request: ServiceImport | null = source) {
		if (!request) return;
		busy = true;
		error = null;
		try {
			const rows = dryRun ? undefined : tickedRows(selected, preview?.rows ?? []);
			const answer = await importFromService(request, dryRun, rows);
			if (dryRun) {
				source = request;
				preview = answer;
				selected = new Set(importableRows(answer.rows));
			} else {
				result = answer;
				if (answer.created > 0) ondone?.();
			}
		} catch (cause) {
			error = cause;
		} finally {
			busy = false;
		}
	}

	/** A CSV (UptimeRobot) or a YAML file (Gatus): read as text, then previewed or pasted. */
	async function onfile(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		notice = '';
		if (file.size > SERVICE_IMPORT_MAX_BYTES) {
			notice = m.devices_svc_file_too_large();
			return;
		}
		const content = await file.text();
		if (provider === 'gatus') {
			text = content;
			return;
		}
		await run(true, { provider: 'uptimerobot', csv: content });
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			close();
		} else if (event.key === 'Tab' && dialog) {
			const items = [...dialog.querySelectorAll<HTMLElement>('a[href], button:not([disabled]), input:not([disabled]), textarea:not([disabled])')];
			if (items.length === 0) return;
			const first = items[0];
			const final = items[items.length - 1];
			const active = document.activeElement;
			if (event.shiftKey && (active === first || active === dialog)) {
				event.preventDefault();
				final.focus();
			} else if (!event.shiftKey && active === final) {
				event.preventDefault();
				first.focus();
			}
		}
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div class="fixed inset-0 z-[60] bg-black/50" aria-hidden="true" onclick={close}></div>
	<div
		bind:this={dialog}
		role="dialog"
		aria-modal="true"
		aria-labelledby="svc-import-title"
		tabindex="-1"
		{onkeydown}
		class="fixed inset-x-0 bottom-0 z-[62] max-h-[92dvh] overflow-y-auto rounded-t-[var(--radius-card)] border border-line bg-surface p-5 pb-[calc(1.25rem+var(--safe-bottom))] text-ink shadow-float outline-none sm:inset-x-auto sm:top-1/2 sm:bottom-auto sm:left-1/2 sm:w-[48rem] sm:max-w-[calc(100vw-2rem)] sm:-translate-x-1/2 sm:-translate-y-1/2 sm:rounded-[var(--radius-card)] sm:pb-5"
	>
		<div class="flex items-start justify-between gap-3">
			<div class="min-w-0">
				<h2 id="svc-import-title" class="text-lg font-semibold">{title}</h2>
				<p class="mt-1 text-sm text-ink-2">{description}</p>
			</div>
			<button
				type="button"
				class="-mt-1.5 -mr-2 inline-flex size-10 shrink-0 items-center justify-center rounded-md text-ink-3 hover:bg-surface-2 hover:text-ink"
				aria-label={m.devices_import_close()}
				onclick={close}
			>
				<X class="size-4" aria-hidden="true" />
			</button>
		</div>

		{#if error}
			<ErrorNotice {error} title={m.devices_import_error_title()} class="mt-4" />
		{/if}
		{#if notice}
			<p class="mt-4 text-sm text-warning" role="alert">{notice}</p>
		{/if}

		{#if !report}
			<form
				class="mt-4 grid gap-4"
				onsubmit={(event) => {
					event.preventDefault();
					void run(true, describe());
				}}
			>
				{#if provider === 'gatus'}
					<Field label={m.devices_svc_yaml_label()} for="svc-yaml" help={m.devices_svc_yaml_help()}>
						<textarea id="svc-yaml" class="input min-h-40 font-mono text-[0.8125rem]" spellcheck="false" bind:value={text} placeholder={'endpoints:\n  - name: Website\n    url: "https://example.org"'}></textarea>
					</Field>
					<input bind:this={fileInput} type="file" accept=".yml,.yaml,text/yaml,text/plain" class="sr-only" tabindex="-1" onchange={onfile} />
					<div class="flex flex-wrap items-center gap-2">
						<Button variant="secondary" onclick={() => fileInput?.click()}>
							<Upload class="size-4" aria-hidden="true" />
							{m.devices_svc_yaml_choose()}
						</Button>
					</div>
				{:else}
					<Field
						label={m.devices_svc_api_key()}
						for="svc-key"
						help={provider === 'uptimerobot' ? m.devices_svc_uptimerobot_key_help() : m.devices_svc_healthchecks_key_help()}
					>
						<input id="svc-key" class="input" type="password" autocomplete="off" spellcheck="false" bind:value={apiKey} />
					</Field>
					{#if provider === 'healthchecks'}
						<Field label={m.devices_svc_base_url()} for="svc-base" help={m.devices_svc_base_url_help()}>
							<input id="svc-base" class="input" type="url" placeholder="https://healthchecks.example.org" bind:value={baseUrl} />
						</Field>
					{/if}
					{#if provider === 'uptimerobot'}
						<input bind:this={fileInput} type="file" accept=".csv,text/csv" class="sr-only" tabindex="-1" onchange={onfile} />
						<p class="text-sm text-ink-2">{m.devices_svc_or_csv()}</p>
						<div class="flex flex-wrap items-center gap-2">
							<Button variant="secondary" disabled={busy} onclick={() => fileInput?.click()}>
								<Upload class="size-4" aria-hidden="true" />
								{m.devices_svc_csv_choose()}
							</Button>
						</div>
					{/if}
				{/if}
				<div class="flex flex-wrap items-center justify-between gap-2">
					<p class="text-sm text-ink-3">{m.devices_svc_never_imported()}</p>
					<Button type="submit" variant="primary" loading={busy} disabled={describe() === null}>{m.devices_import_preview()}</Button>
				</div>
			</form>
			{#if busy}<p class="mt-3 text-sm text-ink-2" aria-live="polite">{m.devices_svc_working()}</p>{/if}
		{:else}
			<p class="mt-4 text-sm font-semibold" aria-live="polite">
				{(result ? m.devices_import_summary_done : m.devices_import_summary_preview)({
					created: report.created,
					skipped: report.skipped,
					errors: report.errors
				})}
			</p>
			<p class="mt-1 text-sm text-ink-2">{m.devices_svc_never_imported()}</p>
			<div class="mt-3 max-h-[46dvh] overflow-auto rounded-lg border border-line">
				<table class="block w-full text-left text-[0.8125rem] sm:table sm:min-w-[40rem]">
					<thead class="bg-surface-2 text-ink-2 max-sm:sr-only sm:sticky sm:top-0">
						<tr>
							<th scope="col" class="w-10 px-3 py-2 font-medium max-sm:hidden">
								{#if !result}
									{@const mark = selectionState(selected, report.rows)}
									<input
										type="checkbox"
										class="size-4 accent-[var(--c-signal)]"
										aria-label={m.devices_kuma_select_all()}
										checked={mark === 'all'}
										indeterminate={mark === 'some'}
										disabled={busy || importableRows(report.rows).length === 0}
										onchange={() => (selected = mark === 'all' ? new Set() : new Set(importableRows(report.rows)))}
									/>
								{/if}
							</th>
							<th scope="col" class="px-3 py-2 font-medium">{m.devices_kuma_col_monitor()}</th>
							<th scope="col" class="px-3 py-2 font-medium">{m.devices_kuma_col_becomes()}</th>
							<th scope="col" class="px-3 py-2 font-medium">{m.devices_import_col_folder()}</th>
							<th scope="col" class="px-3 py-2 font-medium">{m.devices_import_col_status()}</th>
						</tr>
					</thead>
					<tbody class="max-sm:block">
						{#each report.rows as row (row.row)}
							<tr class="border-t border-line align-top max-sm:block max-sm:px-3 max-sm:py-2 max-sm:first:border-t-0">
								<td class="px-3 py-2 max-sm:block max-sm:px-0 max-sm:py-0.5">
									{#if !result && row.status === 'ok'}
										<label class="-m-2 inline-flex min-h-10 min-w-10 cursor-pointer items-center justify-center p-2">
											<input
												type="checkbox"
												class="size-4 accent-[var(--c-signal)]"
												aria-label={m.devices_kuma_select_row({ name: row.name })}
												checked={selected.has(row.row)}
												disabled={busy}
												onchange={() => (selected = toggleRow(selected, row.row))}
											/>
										</label>
									{/if}
								</td>
								<td class="px-3 py-2 sm:max-w-[14rem] max-sm:block max-sm:px-0 max-sm:py-0.5">
									<span class="block truncate font-medium" title={row.name}>{row.name}</span>
									<span class="block truncate text-ink-3" title={row.source_type}>{row.source_type}</span>
								</td>
								<td class="px-3 py-2 break-words sm:max-w-[16rem] max-sm:block max-sm:px-0 max-sm:py-0.5">
									{#if row.kind}
										<span class="font-medium">{row.kind}</span>
										<span class="block truncate text-ink-3" title={row.address}>{row.address}</span>
										<span class="block text-ink-3">
											{#if row.interval_secs}{m.devices_kuma_every({ seconds: row.interval_secs })}{/if}
											{#if row.tags.length}· {row.tags.join(', ')}{/if}
										</span>
									{:else}
										<span class="text-ink-3" aria-hidden="true">—</span>
									{/if}
								</td>
								<td class="px-3 py-2 break-words text-ink-2 max-sm:block max-sm:px-0 max-sm:py-0.5">{row.folder}</td>
								<td class="px-3 py-2 max-sm:block max-sm:px-0 max-sm:py-0.5">
									<Plate tone={TONE[row.status]} label={statusLabel(row.status)} />
									{#if row.message}<p class="mt-1 text-ink-2">{row.message}</p>{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			<div class="mt-4 flex flex-wrap items-center justify-end gap-2">
				{#if result}
					<Button variant="ghost" onclick={close}>{m.devices_import_close()}</Button>
					<Button variant="primary" href="/targets" onclick={close}>{m.devices_import_view_devices()}</Button>
				{:else}
					<Button variant="ghost" disabled={busy} onclick={() => ((preview = null), (source = null))}>
						{m.devices_svc_back()}
					</Button>
					<Button variant="primary" loading={busy} disabled={selected.size === 0} onclick={() => run(false)}>
						{m.devices_kuma_confirm({ count: selected.size })}
					</Button>
				{/if}
			</div>
		{/if}
	</div>
{/if}
