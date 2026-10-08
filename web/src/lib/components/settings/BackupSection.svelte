<script lang="ts">
	/**
	 * Settings → Backup: the encrypted configuration bundle, the restore form
	 * with its dry run, and the state of the scheduled local backups.
	 *
	 * The bundle holds every credential of the instance, so this section says so
	 * plainly before the download, and says just as plainly that the file that
	 * decrypts device credentials on this server is `/data/secret.key`.
	 */
	import { Archive, Download, HardDriveDownload, KeyRound, Upload } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import {
		exportBackup,
		getBackupStatus,
		restoreBackup,
		runLocalBackup,
		saveBundle
	} from '#lib/api/backup.js';
	import type { BackupEnvelope, BackupStatus, RestoreReport } from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import {
		Button,
		Confirm,
		EmptyState,
		ErrorNotice,
		Field,
		Panel,
		Plate,
		Skeleton,
		Toggle
	} from '#lib/ui/index.js';
	import PasswordInput from './PasswordInput.svelte';

	let status = $state<BackupStatus | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			status = await getBackupStatus(signal);
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

	const minLength = $derived(status?.min_passphrase_length ?? 16);

	function bytes(value: number): string {
		if (value <= 0) return '0 B';
		const units = ['B', 'kB', 'MB', 'GB'];
		let n = value;
		let unit = 0;
		while (n >= 1024 && unit < units.length - 1) {
			n /= 1024;
			unit += 1;
		}
		return `${n < 10 && unit > 0 ? n.toFixed(1) : Math.round(n)} ${units[unit]}`;
	}

	// --- Export ---------------------------------------------------------------

	let passphrase = $state('');
	let confirmation = $state('');
	let withAccounts = $state(false);
	let exporting = $state(false);
	let exportErrors = $state<{ passphrase?: string; confirmation?: string }>({});
	let exportError = $state<unknown>(null);
	let exported = $state(false);

	async function download(event: SubmitEvent) {
		event.preventDefault();
		exportError = null;
		exported = false;
		const found: typeof exportErrors = {};
		if (passphrase.length < minLength) {
			found.passphrase = m.settings_backup_passphrase_short({ min: minLength });
		}
		if (!found.passphrase && passphrase !== confirmation) {
			found.confirmation = m.settings_backup_passphrase_mismatch();
		}
		exportErrors = found;
		if (Object.keys(found).length > 0) return;

		exporting = true;
		try {
			saveBundle(await exportBackup(passphrase, withAccounts));
			exported = true;
			passphrase = '';
			confirmation = '';
		} catch (cause) {
			exportError = cause;
		} finally {
			exporting = false;
		}
	}

	// --- Restore --------------------------------------------------------------

	let bundle = $state<BackupEnvelope | null>(null);
	let bundleName = $state('');
	let bundleError = $state<string | null>(null);
	let restorePassphrase = $state('');
	let checking = $state(false);
	let applying = $state(false);
	let restoreError = $state<unknown>(null);
	let report = $state<RestoreReport | null>(null);

	async function pick(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		bundle = null;
		bundleName = '';
		bundleError = null;
		report = null;
		restoreError = null;
		if (!file) return;
		try {
			const parsed = JSON.parse(await file.text());
			if (!parsed || typeof parsed !== 'object' || parsed.format !== 'dumbmonit-backup') {
				bundleError = m.settings_backup_not_backup();
				return;
			}
			bundle = parsed as BackupEnvelope;
			bundleName = file.name;
		} catch {
			bundleError = m.settings_backup_not_json();
		}
	}

	async function check() {
		if (!bundle) return;
		checking = true;
		restoreError = null;
		report = null;
		try {
			report = await restoreBackup(bundle, restorePassphrase, false);
		} catch (cause) {
			restoreError = cause;
		} finally {
			checking = false;
		}
	}

	async function apply() {
		if (!bundle) return;
		applying = true;
		restoreError = null;
		try {
			report = await restoreBackup(bundle, restorePassphrase, true);
			await load();
		} catch (cause) {
			restoreError = cause;
		} finally {
			applying = false;
		}
	}

	const summaryEntries = $derived(Object.entries(bundle?.summary ?? {}).filter(([, n]) => n > 0));

	// --- Scheduled local backups ---------------------------------------------

	let running = $state(false);
	let runError = $state<unknown>(null);

	async function runNow() {
		running = true;
		runError = null;
		try {
			const schedule = await runLocalBackup();
			if (status) status = { ...status, schedule };
		} catch (cause) {
			runError = cause;
		} finally {
			running = false;
		}
	}
</script>

<Panel
	id="backup"
	title={m.settings_backup_title()}
 description={m.settings_backup_description()}
>
	{#snippet aside()}
		{#if !auth.isAdmin}
			<Plate tone="ghost" label={auth.readOnlyLabel} />
		{/if}
	{/snippet}

	{#if error}
		<ErrorNotice {error} title={m.settings_backup_read_error()} onretry={() => void load()} />
	{:else if loading}
		<Skeleton class="h-32 w-full" />
	{:else if !auth.isAdmin}
		<EmptyState
			icon={Archive}
			title={m.settings_backup_admin_only_title()}
			description={m.settings_backup_admin_only_description()}
		/>
	{:else if status}
		<div class="grid gap-8">
			<!-- The one thing nobody should learn the hard way. -->
			<div class="rounded-[var(--radius-card)] border border-warning/40 bg-surface-2 p-4">
				<div class="flex items-start gap-3">
					<KeyRound class="mt-0.5 size-4 shrink-0 text-warning" aria-hidden="true" />
					<div class="min-w-0">
						<p class="font-semibold text-ink">{m.settings_backup_secret_title()}</p>
						{#if status.secret_source === 'file'}
							<p class="mt-1 text-sm text-ink-2">
								{m.settings_backup_secret_file({ path: status.secret_path })}
							</p>
						{:else}
							<p class="mt-1 text-sm text-ink-2">
								{m.settings_backup_secret_env()}
							</p>
						{/if}
						<p class="mt-2 text-sm text-ink-2">
							{m.settings_backup_secret_bundle()}
						</p>
					</div>
				</div>
			</div>

			<!-- Export -->
			<section>
				<h3 class="font-semibold text-ink">{m.settings_backup_export_title()}</h3>
				<p class="mt-1 text-sm text-ink-2">
					{m.settings_backup_export_intro({ version: status.bundle_version })}
				</p>

				<ul class="mt-3 grid gap-1 text-sm text-ink-2 sm:grid-cols-2">
					{#each status.contents as section (section.section)}
						<li class="flex items-baseline gap-2">
							<span class="tnum w-8 shrink-0 text-right font-semibold text-ink">{section.count}</span>
							<span>{section.description}</span>
						</li>
					{/each}
				</ul>
				<p class="mt-2 text-sm text-ink-3">
					{m.settings_backup_export_excluded()}
				</p>

				<form class="mt-4 grid max-w-md gap-4" onsubmit={download} novalidate>
					<Field
						label={m.settings_backup_passphrase()}
 for="backup-passphrase"
						error={exportErrors.passphrase}
						help={m.settings_backup_passphrase_help({ min: minLength })}
					>
						<PasswordInput
							id="backup-passphrase"
							bind:value={passphrase}
							autocomplete="new-password"
							disabled={exporting}
							invalid={!!exportErrors.passphrase}
							oninput={() => (exportErrors = { ...exportErrors, passphrase: undefined })}
						/>
					</Field>
					<Field label={m.settings_backup_passphrase_confirm()} for="backup-passphrase-2" error={exportErrors.confirmation}>
						<PasswordInput
							id="backup-passphrase-2"
							bind:value={confirmation}
							autocomplete="new-password"
							disabled={exporting}
							invalid={!!exportErrors.confirmation}
							oninput={() => (exportErrors = { ...exportErrors, confirmation: undefined })}
						/>
					</Field>
					<Field
						label={m.settings_backup_accounts_label()}
						for="backup-accounts"
						inline
						help={m.settings_backup_accounts_help()}
					>
						<Toggle id="backup-accounts" bind:checked={withAccounts} disabled={exporting} />
					</Field>

					{#if exportError}
						<ErrorNotice error={exportError} title={m.settings_backup_export_error()} />
					{/if}

					<div class="flex flex-wrap items-center gap-3" aria-live="polite">
						<Button type="submit" variant="primary" loading={exporting}>
							<Download class="size-4" aria-hidden="true" />
							{m.settings_backup_download()}

						</Button>
						{#if exported}
							<Plate tone="signal" label={m.settings_backup_downloaded()} />
						{/if}
					</div>
				</form>
			</section>

			<!-- Restore -->
			<section class="border-t border-line pt-6">
				<h3 class="font-semibold text-ink">{m.settings_backup_restore_title()}</h3>
				<p class="mt-1 text-sm text-ink-2">
					{m.settings_backup_restore_intro()}
				</p>

				<div class="mt-4 grid max-w-md gap-4">
					<Field label={m.settings_backup_bundle_file()} for="backup-file" error={bundleError}>
						<input
							id="backup-file"
							type="file"
							accept="application/json,.json"
							class="input"
							onchange={pick}
							disabled={checking || applying}
						/>
					</Field>

					{#if bundle}
						<div class="rounded-[var(--radius-card)] border border-line bg-surface-2 p-3 text-sm">
							<p class="font-semibold text-ink">{bundleName}</p>
							<p class="mt-1 text-ink-2">
								<time class="tnum" title={formatDateTime(bundle.created_at)}>{m.settings_backup_bundle_meta({ when: formatRelative(bundle.created_at), source: bundle.source_version, version: bundle.version })}</time>
							</p>
							{#if summaryEntries.length > 0}
								<p class="mt-1 text-ink-3">
									{summaryEntries.map(([name, n]) => `${n} ${name.replace(/_/g, ' ')}`).join(' · ')}
								</p>
							{/if}
						</div>

						<Field label={m.settings_backup_restore_passphrase()} for="restore-passphrase">
							<PasswordInput
								id="restore-passphrase"
								bind:value={restorePassphrase}
								autocomplete="off"
								disabled={checking || applying}
							/>
						</Field>

						<div>
							<Button variant="secondary" loading={checking} onclick={check} disabled={applying}>
								<Upload class="size-4" aria-hidden="true" />
								{m.settings_backup_check()}

							</Button>
						</div>
					{/if}

					{#if restoreError}
						<ErrorNotice error={restoreError} title={m.settings_backup_restore_error()} />
					{/if}
				</div>

				{#if report}
					<div class="mt-4 rounded-[var(--radius-card)] border border-line bg-surface p-4" aria-live="polite">
						<div class="flex flex-wrap items-center gap-2">
							{#if report.applied}
								<Plate tone="signal" label={m.settings_backup_restored()} />
							{:else}
								<Plate tone="info" label={m.settings_backup_dry_run()} />
							{/if}
							<p class="text-sm text-ink-2">
								<span class="tnum">{m.settings_backup_to_create({ count: report.created })}</span> · <span class="tnum">{m.settings_backup_to_update({ count: report.updated })}</span> · <span class="tnum">{m.settings_backup_identical({ count: report.skipped })}</span>
							</p>
						</div>

						<div class="mt-3 overflow-x-auto">
							<table class="w-full min-w-[26rem] text-sm">
								<thead>
									<tr class="text-left text-ink-3">
										<th class="py-1 font-medium">{m.settings_backup_col_section()}</th>
										<th class="py-1 text-right font-medium">{m.settings_backup_col_created()}</th>
										<th class="py-1 text-right font-medium">{m.settings_backup_col_updated()}</th>
										<th class="py-1 text-right font-medium">{m.settings_backup_col_unchanged()}</th>
									</tr>
								</thead>
								<tbody>
									{#each report.sections.filter((s) => s.created + s.updated + s.skipped > 0) as section (section.section)}
										<tr class="border-t border-line">
											<td class="py-1 text-ink">{section.section.replace(/_/g, ' ')}</td>
											<td class="tnum py-1 text-right text-ink-2">{section.created}</td>
											<td class="tnum py-1 text-right text-ink-2">{section.updated}</td>
											<td class="tnum py-1 text-right text-ink-3">{section.skipped}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>

						{#each report.sections.filter((s) => s.notes.length > 0) as section (section.section)}
							<ul class="mt-3 grid gap-1 text-sm text-ink-2">
								{#each section.notes as note, index (index)}
									<li>· {note}</li>
								{/each}
							</ul>
						{/each}

						{#if report.warnings.length > 0}
							<ul class="mt-3 grid gap-1 text-sm text-warning">
								{#each report.warnings as warning, index (index)}
									<li>· {warning}</li>
								{/each}
							</ul>
						{/if}

						{#if !report.applied}
							<div class="mt-4">
								<Confirm
									variant="secondary"
									size="md"
									confirmLabel={m.settings_backup_apply_confirm()}
									loading={applying}
									onconfirm={apply}
								>
									{m.settings_backup_apply()}

								</Confirm>
							</div>
						{/if}
					</div>
				{/if}
			</section>

			<!-- Scheduled local backups -->
			<section class="border-t border-line pt-6">
				<div class="flex flex-wrap items-center justify-between gap-2">
					<h3 class="font-semibold text-ink">{m.settings_backup_scheduled_title()}</h3>
					{#if status.schedule.enabled}
						<Plate tone="signal" label={m.settings_backup_plate_every({ hours: status.schedule.interval_hours, keep: status.schedule.keep })} />
					{:else}
						<Plate tone="ghost" label={m.settings_backup_disabled()} />
					{/if}
				</div>
				<p class="mt-1 text-sm text-ink-2">
					{status.schedule.includes_secret_key ? m.settings_backup_schedule_intro_key({ directory: status.schedule.directory }) : m.settings_backup_schedule_intro_nokey({ directory: status.schedule.directory })}
						{m.settings_backup_schedule_same_volume()}
						{#if !status.schedule.includes_secret_key}
							{m.settings_backup_schedule_env_secret()}
						{/if}
				</p>

				{#if status.schedule.directory_error}
					<p class="mt-2 text-sm text-warning">{status.schedule.directory_error}</p>
				{/if}

				<div class="mt-3 text-sm" aria-live="polite">
					{#if status.schedule.last_run}
						{@const run = status.schedule.last_run}
						<div class="flex flex-wrap items-center gap-2">
							<Plate tone={run.ok ? 'signal' : 'warning'} label={run.ok ? m.settings_backup_last_ok() : m.settings_backup_last_fail()} />
							<span class="text-ink-2">
								<time class="tnum" title={formatDateTime(run.at)}>{formatRelative(run.at)}</time>
								{#if run.ok}· {bytes(run.bytes)}{/if}
							</span>
						</div>
						{#if run.error}
							<p class="mt-1 text-warning">{run.error}</p>
						{/if}
					{:else}
						<p class="text-ink-2">{m.settings_backup_none_yet()}</p>
					{/if}
				</div>

				{#if status.schedule.files.length > 0}
					<ul class="mt-3 divide-y divide-line rounded-[var(--radius-card)] border border-line" role="list">
						{#each status.schedule.files as file (file.name)}
							<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2 text-sm">
								<code class="font-mono text-[0.75rem] text-ink">{file.name}</code>
								<span class="tnum text-ink-2">{bytes(file.bytes)}</span>
								<time class="tnum text-ink-3" title={formatDateTime(file.at)}>{formatRelative(file.at)}</time>
								{#if !file.with_secret && status.schedule.includes_secret_key}
									<Plate tone="warning" label={m.settings_backup_without_key()} />
								{/if}
							</li>
						{/each}
					</ul>
					<p class="mt-2 text-sm text-ink-3">
						{m.settings_backup_kept({ count: status.schedule.files.length, size: bytes(status.schedule.total_bytes) })}
					</p>
				{/if}

				{#if runError}
					<ErrorNotice error={runError} title={m.settings_backup_write_error()} class="mt-3" />
				{/if}

				<div class="mt-3">
					<Button variant="secondary" loading={running} onclick={runNow}>
						<HardDriveDownload class="size-4" aria-hidden="true" />
						{m.settings_backup_back_up_now()}

					</Button>
				</div>
			</section>
		</div>
	{/if}
</Panel>
