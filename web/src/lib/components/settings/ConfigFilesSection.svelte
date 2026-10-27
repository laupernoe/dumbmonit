<script lang="ts">
	/**
	 * Settings → Configuration files: declare monitors, rules, channels,
	 * maintenance windows and status pages in YAML (GitOps).
	 *
	 * Three gestures: export the instance as a file (secrets become references,
	 * never values), paste a file to see its plan, apply it. A file applied
	 * here must hold no unresolved `${...}` reference: the terminal tool
	 * (`dumbmonit-cli`) and the watched folder resolve those where the secret
	 * lives. Objects a file declares are managed by it and refuse manual edits.
	 */
	import { FileCode, Download, ListChecks, Play, Unlink } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import {
		applyConfig,
		exportConfig,
		getConfigStatus,
		planConfig,
		releaseConfig,
		saveYaml
	} from '#lib/api/configFiles.js';
	import type { ConfigChange, ConfigExport, ConfigPlan, ConfigStatus } from '#lib/api/index.js';
	import { formatDateTime } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { configManaged } from '#lib/stores/configManaged.svelte.js';
	import {
		Button,
		Confirm,
		Disclosure,
		EmptyState,
		ErrorNotice,
		Field,
		Panel,
		Plate,
		Skeleton,
		Toggle
	} from '#lib/ui/index.js';

	let status = $state<ConfigStatus | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			status = await getConfigStatus(signal);
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

	const managedTotal = $derived(Object.values(status?.managed ?? {}).reduce((a, b) => a + b, 0));

	const SECTION_TEXT: Record<ConfigChange['section'], () => string> = {
		targets: m.settings_config_section_targets,
		channels: m.settings_config_section_channels,
		rules: m.settings_config_section_rules,
		rule_overrides: m.settings_config_section_rule_overrides,
		silences: m.settings_config_section_silences,
		status_pages: m.settings_config_section_status_pages
	};

	const ACTION_TEXT: Record<ConfigChange['action'], () => string> = {
		create: m.settings_config_action_create,
		update: m.settings_config_action_update,
		delete: m.settings_config_action_delete
	};
	const ACTION_MARK: Record<ConfigChange['action'], string> = { create: '+', update: '~', delete: '−' };

	// --- Export ---------------------------------------------------------------

	let exporting = $state(false);
	let exportError = $state<unknown>(null);
	let exported = $state<ConfigExport | null>(null);

	async function runExport() {
		exporting = true;
		exportError = null;
		try {
			exported = await exportConfig();
			saveYaml(exported.yaml);
		} catch (cause) {
			exportError = cause;
		} finally {
			exporting = false;
		}
	}

	// --- Plan and apply ---------------------------------------------------------

	let yaml = $state('');
	let prune = $state(false);
	let planning = $state(false);
	let applying = $state(false);
	let planError = $state<unknown>(null);
	let applyError = $state<unknown>(null);
	let result = $state<ConfigPlan | null>(null);
	/** The text the shown plan was computed for: applying a different text needs a new plan. */
	let planned = $state<{ yaml: string; prune: boolean } | null>(null);

	const fresh = $derived(!!planned && planned.yaml === yaml && planned.prune === prune);
	const hasChanges = $derived((result?.changes.length ?? 0) > 0);

	async function pick(event: Event) {
		const file = (event.currentTarget as HTMLInputElement).files?.[0];
		if (file) yaml = await file.text();
	}

	async function plan() {
		planning = true;
		planError = null;
		applyError = null;
		result = null;
		planned = null;
		try {
			result = await planConfig(yaml, prune);
			planned = { yaml, prune };
		} catch (cause) {
			planError = cause;
		} finally {
			planning = false;
		}
	}

	async function apply() {
		applying = true;
		applyError = null;
		try {
			result = await applyConfig(yaml, prune);
			planned = null;
			await load();
			await configManaged.reload();
		} catch (cause) {
			applyError = cause;
		} finally {
			applying = false;
		}
	}

	// --- Release ----------------------------------------------------------------

	let releasing = $state(false);
	let releaseError = $state<unknown>(null);
	let released = $state<number | null>(null);

	async function release() {
		releasing = true;
		releaseError = null;
		try {
			released = (await releaseConfig()).released;
			await load();
			await configManaged.reload();
		} catch (cause) {
			releaseError = cause;
		} finally {
			releasing = false;
		}
	}
</script>

<Panel id="config-files" title={m.settings_config_title()} description={m.settings_config_description()}>
	{#snippet aside()}
		{#if !auth.isAdmin}
			<Plate tone="ghost" label={auth.readOnlyLabel} />
		{/if}
	{/snippet}

	{#if error}
		<ErrorNotice {error} title={m.settings_config_read_error()} onretry={() => void load()} />
	{:else if loading}
		<Skeleton class="h-32 w-full" />
	{:else if !auth.isAdmin}
		<EmptyState
			icon={FileCode}
			title={m.settings_config_admin_only_title()}
			description={m.settings_config_admin_only_description()}
		/>
	{:else if status}
		<div class="grid gap-8">
			<!-- What is managed, and the watched folder. -->
			<div class="rounded-[var(--radius-card)] border border-line bg-surface-2 p-4 text-sm">
				<div class="flex flex-wrap items-center gap-2">
					<Plate tone="info">
						<FileCode class="size-3.5" aria-hidden="true" />
						{m.settings_config_managed_count({ count: managedTotal })}
					</Plate>
					<span class="text-ink-3">{m.settings_config_schema({ version: status.schema_version })}</span>
				</div>
				{#if status.watch.directory}
					<p class="mt-3 text-ink-2">
						{m.settings_config_watch_dir({ path: status.watch.directory })}
						{#if status.watch.files.length > 0}
							<span class="text-ink-3">· {status.watch.files.join(', ')}</span>
						{/if}
					</p>
					{#if status.watch.last_applied_at}
						<p class="mt-1 text-ink-2">
							{m.settings_config_watch_applied({ date: formatDateTime(status.watch.last_applied_at) })}
							{#if status.watch.last_changes}
								<span class="tnum text-ink-3">
									· {m.settings_config_result_summary({
										create: status.watch.last_changes[0],
										update: status.watch.last_changes[1],
										delete: status.watch.last_changes[2],
										unchanged: 0
									})}
								</span>
							{/if}
						</p>
					{/if}
					{#if status.watch.last_error}
						<p class="mt-2 whitespace-pre-line text-warning" role="alert">
							{m.settings_config_watch_error({ error: status.watch.last_error })}
						</p>
					{/if}
				{:else}
					<p class="mt-3 text-ink-2">{m.settings_config_watch_none()}</p>
				{/if}
				<p class="mt-3 text-ink-3">{m.settings_config_cli_hint()}</p>
			</div>

			<!-- Export -->
			<Disclosure title={m.settings_config_export_title()}>
				<p class="mt-1 text-sm text-ink-2">{m.settings_config_export_intro()}</p>
				<div class="mt-4 flex flex-wrap items-center gap-3" aria-live="polite">
					<Button variant="secondary" loading={exporting} onclick={runExport}>
						<Download class="size-4" aria-hidden="true" />
						{m.settings_config_export_button()}
					</Button>
				</div>
				{#if exportError}
					<ErrorNotice error={exportError} title={m.settings_config_export_error()} class="mt-3" />
				{/if}
				{#if exported && exported.variables.length > 0}
					<div class="mt-4 text-sm text-ink-2">
						<p>{m.settings_config_export_variables({ count: exported.variables.length })}</p>
						<ul class="mt-2 grid gap-1 font-mono text-[0.8125rem] text-ink">
							{#each exported.variables as name (name)}
								<li class="break-all">{name}</li>
							{/each}
						</ul>
					</div>
				{/if}
			</Disclosure>

			<!-- Plan and apply -->
			<Disclosure title={m.settings_config_plan_title()} open>
				<p class="mt-1 text-sm text-ink-2">{m.settings_config_plan_intro()}</p>

				<div class="mt-4 grid max-w-3xl gap-4">
					<Field label={m.settings_config_file_label()} for="config-file-input">
						<input
							id="config-file-input"
							type="file"
							accept=".yaml,.yml,text/yaml,application/yaml"
							class="input"
							onchange={pick}
							disabled={planning || applying}
						/>
					</Field>
					<Field label={m.settings_config_yaml_label()} for="config-yaml">
						<textarea
							id="config-yaml"
							class="input min-h-48 font-mono text-[0.8125rem] leading-relaxed"
							rows="12"
							spellcheck="false"
							autocomplete="off"
							placeholder={'version: 1\nchannels:\n  - name: Phone\n    kind: ntfy\n    enabled: true'}
							bind:value={yaml}
							disabled={planning || applying}
						></textarea>
					</Field>
					<Field label={m.settings_config_prune_label()} for="config-prune" inline help={m.settings_config_prune_help()}>
						<Toggle id="config-prune" bind:checked={prune} disabled={planning || applying} />
					</Field>

					{#if planError}
						<ErrorNotice error={planError} title={m.settings_config_plan_error()} />
					{/if}

					<div class="flex flex-wrap items-center gap-3">
						<Button variant="secondary" loading={planning} disabled={!yaml.trim() || applying} onclick={plan}>
							<ListChecks class="size-4" aria-hidden="true" />
							{m.settings_config_plan_button()}
						</Button>
						{#if fresh && hasChanges}
							<Confirm
								variant="secondary"
								size="md"
								loading={applying}
								confirmLabel={m.settings_config_apply_confirm({
									create: result?.create ?? 0,
									update: result?.update ?? 0,
									delete: result?.delete ?? 0
								})}
								onconfirm={apply}
							>
								<Play class="size-4" aria-hidden="true" />
								{m.settings_config_apply_button()}
							</Confirm>
						{/if}
					</div>

					{#if applyError}
						<ErrorNotice error={applyError} title={m.settings_config_apply_error()} />
					{/if}
				</div>

				{#if result}
					<div class="mt-5 rounded-[var(--radius-card)] border border-line bg-surface p-4" aria-live="polite">
						<div class="flex flex-wrap items-center gap-2">
							{#if result.applied}
								<Plate tone="signal" label={m.settings_config_applied()} />
							{:else}
								<Plate tone="info" label={m.settings_config_dry_run()} />
							{/if}
							<p class="tnum text-sm text-ink-2">
								{m.settings_config_result_summary({
									create: result.create,
									update: result.update,
									delete: result.delete,
									unchanged: result.unchanged
								})}
							</p>
						</div>

						{#if result.changes.length === 0}
							<p class="mt-3 text-sm text-ink-2">{m.settings_config_nothing()}</p>
						{:else}
							<ul class="mt-3 grid gap-2 text-sm" role="list">
								{#each result.changes as change (change.section + change.key + change.action)}
									<li class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
										<Plate
											tone={change.action === 'create' ? 'signal' : change.action === 'delete' ? 'warning' : 'info'}
											label={`${ACTION_MARK[change.action]} ${ACTION_TEXT[change.action]()}`}
										/>
										<span class="text-ink-3">{SECTION_TEXT[change.section]()}</span>
										<span class="break-all font-semibold text-ink">{change.key}</span>
										{#if change.fields.length > 0}
											<span class="break-all font-mono text-[0.75rem] text-ink-3">({change.fields.join(', ')})</span>
										{/if}
									</li>
								{/each}
							</ul>
						{/if}

						{#each result.warnings as warning, index (index)}
							<p class="mt-3 text-sm text-ink-2">· {warning}</p>
						{/each}
						{#if result.unresolved.length > 0}
							<p class="mt-3 text-sm text-ink-2">
								{m.settings_config_unresolved({ names: result.unresolved.join(', ') })}
							</p>
						{/if}
					</div>
				{/if}
			</Disclosure>

			<!-- Release -->
			{#if managedTotal > 0}
				<Disclosure title={m.settings_config_release_title()}>
					<p class="mt-1 text-sm text-ink-2">{m.settings_config_release_help()}</p>
					<div class="mt-4 flex flex-wrap items-center gap-3" aria-live="polite">
						<Confirm
							variant="secondary"
							size="md"
							loading={releasing}
							confirmLabel={m.settings_config_release_confirm()}
							onconfirm={release}
						>
							<Unlink class="size-4" aria-hidden="true" />
							{m.settings_config_release_button()}
						</Confirm>
						{#if released !== null}
							<Plate tone="signal" label={m.settings_config_release_done({ count: released })} />
						{/if}
					</div>
					{#if releaseError}
						<ErrorNotice error={releaseError} title={m.settings_config_release_error()} class="mt-3" />
					{/if}
				</Disclosure>
			{/if}
		</div>
	{/if}
</Panel>
