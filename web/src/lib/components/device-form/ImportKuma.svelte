<script lang="ts">
	/**
	 * "Import from Uptime Kuma": a trigger button and its modal dialog.
	 *
	 * Choose the `kuma.db` database of Uptime Kuma 1.x or 2.x (or the JSON backup
	 * of 1.x), preview what will be created and what is skipped (and why), tick
	 * the monitors to import, confirm, read the summary. The server reads the
	 * file, converts and validates; this component only shows its report. Admin
	 * only: the caller decides whether to render it.
	 */
	import { tick } from 'svelte';
	import { Upload, X } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { importKuma, importKumaDb, KUMA_DB_MAX_BYTES, KUMA_MAX_BYTES } from '#lib/api/targetsImport.js';
	import { importableRows, isSqliteHead, selectionState, tickedRows, toggleRow } from './kuma-import.js';
	import RelayPick from './RelayPick.svelte';
	import type { KumaImportReport, TargetImportStatus } from '#lib/api/types.js';
	import { Button, ErrorNotice, Plate } from '#lib/ui/index.js';
	import type { Tone } from '#lib/ui/index.js';

	interface Props {
		/** Called after devices were created, so the page can reload its list. */
		ondone?: () => void;
		variant?: 'primary' | 'secondary' | 'ghost';
		/** `false`: no button of its own; the caller opens the dialog with `show()` (a menu item). */
		trigger?: boolean;
		class?: string;
	}
	let { ondone, variant = 'secondary', trigger = true, class: className = '' }: Props = $props();

	let open = $state(false);
	/** Relay agent that probes the imported monitors; `null`: this server. */
	let viaAgent = $state<number | null>(null);
	let busy = $state(false);
	let error = $state<unknown>(null);
	let notice = $state('');
	/** The chosen file, kept between the preview and the confirmation (the server reads it twice). */
	let source = $state<{ db: File } | { json: unknown } | null>(null);
	/** Rows of the preview that are ticked. */
	let selected = $state<Set<number>>(new Set());
	let preview = $state<KumaImportReport | null>(null);
	let result = $state<KumaImportReport | null>(null);
	let dialog = $state<HTMLElement | null>(null);
	let fileInput = $state<HTMLInputElement | null>(null);
	let returnFocus: HTMLElement | null = null;

	const report = $derived(result ?? preview);
	const TONE: Record<TargetImportStatus, Tone> = {
		ok: 'info',
		created: 'signal',
		skipped: 'advisory',
		error: 'warning'
	};

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
		open = true;
		void tick().then(() => dialog?.focus());
	}

	function close() {
		open = false;
		returnFocus?.focus();
	}

	async function run(dryRun: boolean) {
		if (!source) return;
		busy = true;
		error = null;
		try {
			const rows = dryRun ? undefined : tickedRows(selected, preview?.rows ?? []);
			const answer = 'db' in source ? await importKumaDb(source.db, dryRun, rows, viaAgent) : await importKuma(source.json, dryRun, rows, viaAgent);
			if (dryRun) {
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

	async function onfile(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		notice = '';
		// SQLite starts with a fixed header; anything else must be the JSON backup.
		const head = new Uint8Array(await file.slice(0, 16).arrayBuffer());
		if (isSqliteHead(head)) {
			if (file.size > KUMA_DB_MAX_BYTES) {
				notice = m.devices_kuma_db_too_large();
				return;
			}
			source = { db: file };
		} else {
			if (file.size > KUMA_MAX_BYTES) {
				notice = m.devices_kuma_too_large();
				return;
			}
			try {
				source = { json: JSON.parse(await file.text()) };
			} catch {
				notice = m.devices_kuma_not_json();
				return;
			}
		}
		await run(true);
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			close();
		} else if (event.key === 'Tab' && dialog) {
			const items = [...dialog.querySelectorAll<HTMLElement>('a[href], button:not([disabled])')];
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

{#if trigger}
	<Button {variant} class={className} onclick={show} aria-haspopup="dialog">
		<Upload class="size-4" aria-hidden="true" />
		{m.devices_kuma_open()}
	</Button>
{/if}

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div class="fixed inset-0 z-[60] bg-black/50" aria-hidden="true" onclick={close}></div>
	<div
		bind:this={dialog}
		role="dialog"
		aria-modal="true"
		aria-labelledby="kuma-title"
		tabindex="-1"
		{onkeydown}
		class="fixed inset-x-0 bottom-0 z-[62] max-h-[92dvh] overflow-y-auto rounded-t-[var(--radius-card)] border border-line bg-surface p-5 pb-[calc(1.25rem+var(--safe-bottom))] text-ink shadow-float outline-none sm:inset-x-auto sm:top-1/2 sm:bottom-auto sm:left-1/2 sm:w-[48rem] sm:max-w-[calc(100vw-2rem)] sm:-translate-x-1/2 sm:-translate-y-1/2 sm:rounded-[var(--radius-card)] sm:pb-5"
	>
		<div class="flex items-start justify-between gap-3">
			<div class="min-w-0">
				<h2 id="kuma-title" class="text-lg font-semibold">{m.devices_kuma_title()}</h2>
				<p class="mt-1 text-sm text-ink-2">{m.devices_kuma_description()}</p>
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
			<div class="mt-4"><RelayPick id="kuma-via-agent" bind:value={viaAgent} disabled={busy} /></div>
			<input bind:this={fileInput} type="file" accept=".db,.sqlite,.sqlite3,.json,application/json,application/vnd.sqlite3,application/octet-stream" class="sr-only" tabindex="-1" onchange={onfile} />
			<div class="mt-4 flex flex-wrap items-center gap-2">
				<Button variant="primary" loading={busy} onclick={() => fileInput?.click()}>
					<Upload class="size-4" aria-hidden="true" />
					{m.devices_kuma_choose()}
				</Button>
			</div>
			{#if busy}<p class="mt-3 text-sm text-ink-2" aria-live="polite">{m.devices_kuma_working()}</p>{/if}
			<div class="mt-4 rounded-lg border border-line bg-surface-2 px-3.5 py-3 text-sm text-ink-2">
				<p>{m.devices_kuma_howto_intro()}</p>
				<p class="mt-1.5"><code class="rounded bg-surface px-1.5 py-0.5 font-mono text-[0.8125rem] break-all text-ink select-all">docker cp uptime-kuma:/app/data/kuma.db .</code></p>
				<p class="mt-1.5">{m.devices_kuma_howto_mariadb()}</p>
			</div>
		{:else}
			<p class="mt-4 text-sm font-semibold" aria-live="polite">
				{(result ? m.devices_import_summary_done : m.devices_import_summary_preview)({
					created: report.created,
					skipped: report.skipped,
					errors: report.errors
				})}
			</p>
			<p class="mt-1 text-sm text-ink-2">
				{m.devices_kuma_notifications({ count: report.notifications_ignored })}
				{m.devices_kuma_credentials()}
			</p>
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
						{m.devices_kuma_back()}
					</Button>
					<Button
						variant="primary"
						loading={busy}
						disabled={selected.size === 0}
						onclick={() => run(false)}
					>
						{m.devices_kuma_confirm({ count: selected.size })}
					</Button>
				{/if}
			</div>
		{/if}
	</div>
{/if}
