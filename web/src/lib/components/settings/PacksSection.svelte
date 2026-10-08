<script lang="ts">
	/**
	 * Settings → Integration packs: device types declared in YAML. Lists what
	 * is installed, installs a pasted or uploaded pack.yaml, turns packs on and
	 * off, and uninstalls the ones no device uses any more.
	 *
	 * A refused install answers 400 with every validation error on its own
	 * "- " line: they are shown as a list, not as one run-on sentence.
	 */
	import { ExternalLink, FileUp, Plus, Puzzle, RotateCw, Trash2 } from 'lucide-svelte';
	import { installPack, listPacks, PACKS_DOC_URL, setPackEnabled, uninstallPack } from '#lib/api/packs.js';
	import { toApiError, type PackInstallReport, type PackView } from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Field, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';

	let packs = $state<PackView[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			packs = await listPacks(signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if (!auth.isAdmin) {
			loading = false;
			return;
		}
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	// --- Enable, disable, uninstall -------------------------------------------

	let busy = $state<Record<string, 'toggle' | 'remove'>>({});
	let rowErrors = $state<Record<string, { error: unknown; hint?: string }>>({});

	function settle(id: string, failure?: { error: unknown; hint?: string }) {
		const { [id]: _, ...rest } = busy;
		busy = rest;
		const { [id]: __, ...others } = rowErrors;
		rowErrors = failure ? { ...others, [id]: failure } : others;
	}

	async function toggle(pack: PackView, enabled: boolean) {
		busy = { ...busy, [pack.id]: 'toggle' };
		packs = packs.map((p) => (p.id === pack.id ? { ...p, enabled } : p));
		try {
			const updated = await setPackEnabled(pack.id, enabled);
			packs = packs.map((p) => (p.id === pack.id ? updated : p));
			settle(pack.id);
		} catch (cause) {
			// The switch moved optimistically: put it back where the server left it.
			packs = packs.map((p) => (p.id === pack.id ? { ...p, enabled: !enabled } : p));
			settle(pack.id, {
				error: cause,
				hint:
					toApiError(cause).status === 409
						? m.settings_packs_hint_enable()
						: undefined
			});
		}
	}

	async function uninstall(pack: PackView) {
		busy = { ...busy, [pack.id]: 'remove' };
		try {
			await uninstallPack(pack.id);
			packs = packs.filter((p) => p.id !== pack.id);
			settle(pack.id);
		} catch (cause) {
			settle(pack.id, {
				error: cause,
				hint:
					toApiError(cause).status === 409
						? m.settings_packs_hint_uninstall()
						: undefined
			});
		}
	}

	// --- Install --------------------------------------------------------------

	let yaml = $state('');
	let fileName = $state('');
	let fileError = $state<string | null>(null);
	let installing = $state(false);
	let report = $state<PackInstallReport | null>(null);
	let installError = $state<unknown>(null);

	/** A 400 from the validator: its message is a headline, then "- " lines. */
	const validation = $derived.by(() => {
		if (!installError) return null;
		const api = toApiError(installError);
		if (api.status !== 400) return null;
		const lines = api.message.split('\n').map((line) => line.trim()).filter(Boolean);
		const items = lines.filter((line) => line.startsWith('- ')).map((line) => line.slice(2));
		const headline = lines.filter((line) => !line.startsWith('- ')).join(' ');
		return { headline: headline || m.settings_packs_val_headline(), items };
	});

	async function pick(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		fileError = null;
		if (!file) return;
		if (file.size > 1024 * 1024) {
			fileError = m.settings_packs_file_too_big();
			input.value = '';
			return;
		}
		try {
			yaml = await file.text();
			fileName = file.name;
			report = null;
			installError = null;
		} catch {
			fileError = m.settings_packs_file_unreadable();
		}
		input.value = '';
	}

	async function install(event: SubmitEvent) {
		event.preventDefault();
		if (!yaml.trim()) return;
		installing = true;
		installError = null;
		report = null;
		try {
			report = await installPack(yaml);
			yaml = '';
			fileName = '';
			const installed = report.pack;
			packs = packs.some((p) => p.id === installed.id)
				? packs.map((p) => (p.id === installed.id ? installed : p))
				: [...packs, installed].sort((a, b) => a.id.localeCompare(b.id));
		} catch (cause) {
			installError = cause;
		} finally {
			installing = false;
		}
	}

	const OUTCOME: Record<PackInstallReport['outcome'], { tone: 'signal' | 'info'; label: () => string }> = {
		created: { tone: 'signal', label: () => m.settings_packs_outcome_created() },
		updated: { tone: 'signal', label: () => m.settings_packs_outcome_updated() },
		unchanged: { tone: 'info', label: () => m.settings_packs_outcome_unchanged() }
	};
</script>

<Panel
	id="packs"
	title={m.settings_packs_title()}
	description={m.settings_packs_description()}
>
	{#snippet aside()}
		<div class="flex flex-wrap items-center gap-2">
			{#if !auth.isAdmin}<Plate tone="ghost" label={m.settings_packs_admins_only()} />{/if}
			<a
				href={PACKS_DOC_URL}
				target="_blank"
				rel="noreferrer"
				class="inline-flex items-center gap-1 text-sm font-semibold text-ink-2 hover:text-ink hover:underline"
			>
				{m.settings_packs_docs()}
				<ExternalLink class="size-3.5" aria-hidden="true" />
			</a>
		</div>
	{/snippet}

	{#if !auth.isAdmin}
		<EmptyState
			icon={Puzzle}
			title={m.settings_packs_admin_only_title()}
			description={m.settings_packs_admin_only_description()}
		/>
	{:else if error}
		<ErrorNotice {error} title={m.settings_packs_list_error()} onretry={() => void load()} />
	{:else if loading}
		<Skeleton class="h-24 w-full" />
	{:else}
		<div class="grid gap-8">
			<section aria-labelledby="packs-installed">
				<h3 id="packs-installed" class="sr-only">{m.settings_packs_installed_heading()}</h3>
				{#if packs.length === 0}
					<EmptyState
						icon={Puzzle}
						title={m.settings_packs_empty_title()}
						description={m.settings_packs_empty_description()}
					/>
				{:else}
					<ul class="grid gap-3">
						{#each packs as pack (pack.id)}
							{@const failure = rowErrors[pack.id]}
							<li class="rounded-[var(--radius-card)] border border-line bg-surface p-4">
								<div class="flex flex-wrap items-start gap-x-4 gap-y-3">
									<div class="min-w-0 flex-1">
										<div class="flex flex-wrap items-center gap-2">
											<p class="font-semibold text-ink">{pack.label}</p>
											{#if pack.error}
												<Plate tone="warning" label={m.settings_packs_invalid()} />
											{:else if pack.enabled}
												<Plate tone="signal" label={m.settings_packs_enabled()} />
											{:else}
												<Plate tone="ghost" label={m.settings_packs_disabled()} />
											{/if}
										</div>
										<p class="mt-0.5 font-mono text-[0.75rem] text-ink-3">
											{pack.id} · v{pack.version}{#if pack.kind} · {pack.kind}{/if}
										</p>
										{#if pack.summary}<p class="mt-1.5 text-sm text-ink-2">{pack.summary}</p>{/if}
										<p class="mt-1.5 text-[0.8125rem] text-ink-2">
											<span class="tnum font-semibold text-ink">{m.settings_packs_n_devices({ count: pack.targets })}</span>
											· {m.settings_packs_n_rules({ count: pack.rules.length })}
											· {m.settings_packs_n_metrics({ count: pack.metrics.length })}
											{#if pack.snmp_profiles.length > 0}· {m.settings_packs_n_snmp({ count: pack.snmp_profiles.length })}{/if}
											· <time title={formatDateTime(pack.installed_at)}>{m.settings_packs_installed({ when: formatRelative(pack.installed_at) })}</time>
										</p>
									</div>
									<div class="flex shrink-0 items-center gap-3">
										<Toggle
											checked={pack.enabled}
											label={pack.enabled ? m.settings_packs_disable_pack({ label: pack.label }) : m.settings_packs_enable_pack({ label: pack.label })}
											disabled={!!busy[pack.id]}
											onchange={(value) => void toggle(pack, value)}
										/>
										<Confirm
											confirmLabel={m.settings_packs_uninstall_confirm()}
											loading={busy[pack.id] === 'remove'}
											disabled={!!busy[pack.id]}
											onconfirm={() => uninstall(pack)}
										>
											<Trash2 class="size-4" aria-hidden="true" />
											{m.settings_packs_uninstall()}
										</Confirm>
									</div>
								</div>

								{#if pack.error}
									<p class="mt-3 rounded-md border border-warning/35 bg-warning-soft px-3 py-2 text-sm whitespace-pre-line text-ink">{pack.error}</p>
								{/if}
								{#if pack.warnings.length > 0}
									<ul class="mt-3 grid gap-1 text-[0.8125rem] text-ink-2">
										{#each pack.warnings as warning, index (index)}
											<li class="flex items-start gap-2">
												<Plate tone="advisory" label={m.settings_packs_note()} />
												<span class="min-w-0 font-mono break-words">{warning}</span>
											</li>
										{/each}
									</ul>
								{/if}
								{#if failure}
									<ErrorNotice
										class="mt-3"
										error={failure.error}
										title={m.settings_packs_change_error()}
										hint={failure.hint}
									/>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</section>

			<section class="border-t border-line pt-6" aria-labelledby="packs-install">
				<h3 id="packs-install" class="font-semibold text-ink">{m.settings_packs_install_heading()}</h3>
				<p class="mt-1 text-sm text-ink-2">
					{m.settings_packs_install_intro()}
				</p>

				<form class="mt-4 grid gap-4" onsubmit={install} novalidate>
					<Field label={m.settings_packs_upload_label()} for="pack-file" error={fileError}>
						<label
							class="inline-flex w-fit cursor-pointer items-center gap-2 rounded-lg border border-line bg-surface px-3 py-1.5 text-sm font-semibold text-ink transition-colors hover:border-line-strong has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-signal"
						>
							<FileUp class="size-4" aria-hidden="true" />
							{fileName ? m.settings_packs_loaded({ name: fileName }) : m.settings_packs_choose()}
							<input
								id="pack-file"
								type="file"
								accept=".yaml,.yml,application/yaml,application/x-yaml,text/yaml,text/plain"
								class="sr-only"
								onchange={pick}
								disabled={installing}
							/>
						</label>
					</Field>
					<Field label={m.settings_packs_yaml_label()} for="pack-yaml">
						<textarea
							id="pack-yaml"
							class="input min-h-48 font-mono text-[0.8125rem] leading-relaxed"
							rows="12"
							spellcheck="false"
							autocomplete="off"
							placeholder={'schema: 1\nid: shelly-plug\nversion: 1.0.0\nlabel: Shelly plug (Gen2+)\n…'}
							bind:value={yaml}
							oninput={() => (fileName = '')}
							disabled={installing}
						></textarea>
					</Field>

					{#if validation}
						<div role="alert" class="rounded-[var(--radius-card)] border border-warning/35 bg-warning-soft px-4 py-3">
							<p class="font-semibold text-warning-ink">{validation.headline}</p>
							{#if validation.items.length > 0}
								<ul class="mt-2 grid gap-1 text-sm text-ink">
									{#each validation.items as item, index (index)}
										<li class="flex gap-2">
											<span class="text-warning" aria-hidden="true">·</span>
											<span class="min-w-0 font-mono text-[0.8125rem] break-words">{item}</span>
										</li>
									{/each}
								</ul>
							{/if}
							<p class="mt-2 text-sm text-ink-2">
								{m.settings_packs_val_note()}
								<a href={PACKS_DOC_URL} target="_blank" rel="noreferrer" class="font-semibold text-ink underline underline-offset-2">{m.settings_packs_val_link()}</a>
								{m.settings_packs_val_link_tail()}
							</p>
						</div>
					{:else if installError}
						<ErrorNotice
							error={installError}
							title={m.settings_packs_install_error()}
							hint={toApiError(installError).status === 409
								? m.settings_packs_hint_conflict()
								: undefined}
						/>
					{/if}

					<div>
						<Button type="submit" variant="primary" loading={installing} disabled={!yaml.trim()}>
							<Plus class="size-4" aria-hidden="true" />
							{m.settings_packs_install_button()}
						</Button>
					</div>
				</form>

				{#if report}
					{@const outcome = OUTCOME[report.outcome]}
					<div class="mt-4 rounded-[var(--radius-card)] border border-line bg-surface p-4" aria-live="polite">
						<div class="flex flex-wrap items-center gap-2">
							<Plate tone={outcome.tone} label={outcome.label()} />
							<p class="text-sm text-ink-2">
								<span class="font-semibold text-ink">{report.pack.label}</span>
								<span class="font-mono text-[0.75rem]">v{report.pack.version}</span>
								· {m.settings_packs_rules_added({ count: report.rules_added })}
							</p>
						</div>

						{#if report.restart_required}
							<div class="mt-3 flex items-start gap-2 rounded-md border border-advisory/35 bg-advisory-soft px-3 py-2 text-sm text-ink">
								<RotateCw class="mt-0.5 size-4 shrink-0 text-advisory" aria-hidden="true" />
								<p>
									{m.settings_packs_restart()}
								</p>
							</div>
						{/if}

						{#if report.pack.warnings.length > 0}
							<p class="mt-3 text-sm font-semibold text-ink">{m.settings_packs_installed_notes({ count: report.pack.warnings.length })}</p>
							<ul class="mt-1 grid gap-1 text-[0.8125rem] text-ink-2">
								{#each report.pack.warnings as warning, index (index)}
									<li class="flex items-start gap-2">
										<Plate tone="advisory" label={m.settings_packs_note()} />
										<span class="min-w-0 font-mono break-words">{warning}</span>
									</li>
								{/each}
							</ul>
						{/if}

						{#if report.pack.kind && report.pack.enabled}
							<div class="mt-3">
								<Button size="sm" variant="secondary" href={`/targets/new?kind=${encodeURIComponent(report.pack.kind)}`}>
									<Plus class="size-4" aria-hidden="true" />
									{m.settings_packs_add_device({ label: report.pack.label })}
								</Button>
							</div>
						{/if}
					</div>
				{/if}
			</section>
		</div>
	{/if}
</Panel>
