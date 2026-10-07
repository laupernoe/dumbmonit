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

	function plural(n: number, one: string, many = `${one}s`): string {
		return `${n} ${n === 1 ? one : many}`;
	}

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
						? 'Install a corrected version of this pack, then enable it again.'
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
						? 'Disabling keeps those devices and their history; uninstalling needs them gone.'
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
		return { headline: headline || 'This pack cannot be installed.', items };
	});

	async function pick(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		fileError = null;
		if (!file) return;
		if (file.size > 1024 * 1024) {
			fileError = 'This file is larger than 1 MB: a pack.yaml is a few kilobytes.';
			input.value = '';
			return;
		}
		try {
			yaml = await file.text();
			fileName = file.name;
			report = null;
			installError = null;
		} catch {
			fileError = 'This file could not be read as text.';
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

	const OUTCOME: Record<PackInstallReport['outcome'], { tone: 'signal' | 'info'; label: string }> = {
		created: { tone: 'signal', label: 'Installed' },
		updated: { tone: 'signal', label: 'Updated' },
		unchanged: { tone: 'info', label: 'Already installed — nothing changed' }
	};
</script>

<Panel
	id="packs"
	title="Integration packs"
	description="Device types written in YAML: what to ask a device, which numbers to keep, when to alert. Installing one adds it to the devices you can watch."
>
	{#snippet aside()}
		<div class="flex flex-wrap items-center gap-2">
			{#if !auth.isAdmin}<Plate tone="ghost" label="Administrators only" />{/if}
			<a
				href={PACKS_DOC_URL}
				target="_blank"
				rel="noreferrer"
				class="inline-flex items-center gap-1 text-sm font-semibold text-ink-2 hover:text-ink hover:underline"
			>
				Pack documentation
				<ExternalLink class="size-3.5" aria-hidden="true" />
			</a>
		</div>
	{/snippet}

	{#if !auth.isAdmin}
		<EmptyState
			icon={Puzzle}
			title="Only an administrator can manage integration packs."
			description="A pack decides what this server asks your devices."
		/>
	{:else if error}
		<ErrorNotice {error} title="Could not list the integration packs" onretry={() => void load()} />
	{:else if loading}
		<Skeleton class="h-24 w-full" />
	{:else}
		<div class="grid gap-8">
			<section aria-labelledby="packs-installed">
				<h3 id="packs-installed" class="sr-only">Installed packs</h3>
				{#if packs.length === 0}
					<EmptyState
						icon={Puzzle}
						title="No pack installed yet."
						description="Paste a pack.yaml below, or upload one. Packs you install appear under Community packs when you add a device."
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
												<Plate tone="warning" label="Invalid — inactive" />
											{:else if pack.enabled}
												<Plate tone="signal" label="Enabled" />
											{:else}
												<Plate tone="ghost" label="Disabled" />
											{/if}
										</div>
										<p class="mt-0.5 font-mono text-[0.75rem] text-ink-3">
											{pack.id} · v{pack.version}{#if pack.kind} · {pack.kind}{/if}
										</p>
										{#if pack.summary}<p class="mt-1.5 text-sm text-ink-2">{pack.summary}</p>{/if}
										<p class="mt-1.5 text-[0.8125rem] text-ink-2">
											<span class="tnum font-semibold text-ink">{plural(pack.targets, 'device')}</span>
											· {plural(pack.rules.length, 'alert rule')}
											· {plural(pack.metrics.length, 'metric')}
											{#if pack.snmp_profiles.length > 0}· {plural(pack.snmp_profiles.length, 'SNMP profile')}{/if}
											· installed <time title={formatDateTime(pack.installed_at)}>{formatRelative(pack.installed_at)}</time>
										</p>
									</div>
									<div class="flex shrink-0 items-center gap-3">
										<Toggle
											checked={pack.enabled}
											label={pack.enabled ? `Disable ${pack.label}` : `Enable ${pack.label}`}
											disabled={!!busy[pack.id]}
											onchange={(value) => void toggle(pack, value)}
										/>
										<Confirm
											confirmLabel="Uninstall for good?"
											loading={busy[pack.id] === 'remove'}
											disabled={!!busy[pack.id]}
											onconfirm={() => uninstall(pack)}
										>
											<Trash2 class="size-4" aria-hidden="true" />
											Uninstall
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
												<Plate tone="advisory" label="Note" />
												<span class="min-w-0 font-mono break-words">{warning}</span>
											</li>
										{/each}
									</ul>
								{/if}
								{#if failure}
									<ErrorNotice
										class="mt-3"
										error={failure.error}
										title="Could not change this pack"
										hint={failure.hint}
									/>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</section>

			<section class="border-t border-line pt-6" aria-labelledby="packs-install">
				<h3 id="packs-install" class="font-semibold text-ink">Install a pack</h3>
				<p class="mt-1 text-sm text-ink-2">
					Paste its pack.yaml or upload the file. The pack is checked before anything is saved;
					installing a newer version of an installed pack updates it and keeps it on or off. Its
					requests only ever go to the devices that use it.
				</p>

				<form class="mt-4 grid gap-4" onsubmit={install} novalidate>
					<Field label="Upload a file" for="pack-file" error={fileError}>
						<label
							class="inline-flex w-fit cursor-pointer items-center gap-2 rounded-lg border border-line bg-surface px-3 py-1.5 text-sm font-semibold text-ink transition-colors hover:border-line-strong has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-signal"
						>
							<FileUp class="size-4" aria-hidden="true" />
							{fileName ? `Loaded ${fileName}` : 'Choose pack.yaml'}
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
					<Field label="Pack YAML" for="pack-yaml">
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
								Nothing was saved. Fix these points in the YAML, then install again — the
								<a href={PACKS_DOC_URL} target="_blank" rel="noreferrer" class="font-semibold text-ink underline underline-offset-2">pack format</a>
								explains each field.
							</p>
						</div>
					{:else if installError}
						<ErrorNotice
							error={installError}
							title="Could not install this pack"
							hint={toApiError(installError).status === 409
								? 'Two device types cannot share a name: change the pack’s id.'
								: undefined}
						/>
					{/if}

					<div>
						<Button type="submit" variant="primary" loading={installing} disabled={!yaml.trim()}>
							<Plus class="size-4" aria-hidden="true" />
							Install the pack
						</Button>
					</div>
				</form>

				{#if report}
					{@const outcome = OUTCOME[report.outcome]}
					<div class="mt-4 rounded-[var(--radius-card)] border border-line bg-surface p-4" aria-live="polite">
						<div class="flex flex-wrap items-center gap-2">
							<Plate tone={outcome.tone} label={outcome.label} />
							<p class="text-sm text-ink-2">
								<span class="font-semibold text-ink">{report.pack.label}</span>
								<span class="font-mono text-[0.75rem]">v{report.pack.version}</span>
								· {plural(report.rules_added, 'alert rule')} added
							</p>
						</div>

						{#if report.restart_required}
							<div class="mt-3 flex items-start gap-2 rounded-md border border-advisory/35 bg-advisory-soft px-3 py-2 text-sm text-ink">
								<RotateCw class="mt-0.5 size-4 shrink-0 text-advisory" aria-hidden="true" />
								<p>
									<strong>Restart DumbMonit</strong> to use its SNMP profiles: they are read when the
									server starts. Everything else in the pack works already.
								</p>
							</div>
						{/if}

						{#if report.pack.warnings.length > 0}
							<p class="mt-3 text-sm font-semibold text-ink">Installed, with {plural(report.pack.warnings.length, 'note')} from the checker</p>
							<ul class="mt-1 grid gap-1 text-[0.8125rem] text-ink-2">
								{#each report.pack.warnings as warning, index (index)}
									<li class="flex items-start gap-2">
										<Plate tone="advisory" label="Note" />
										<span class="min-w-0 font-mono break-words">{warning}</span>
									</li>
								{/each}
							</ul>
						{/if}

						{#if report.pack.kind && report.pack.enabled}
							<div class="mt-3">
								<Button size="sm" variant="secondary" href={`/targets/new?kind=${encodeURIComponent(report.pack.kind)}`}>
									<Plus class="size-4" aria-hidden="true" />
									Add a {report.pack.label}
								</Button>
							</div>
						{/if}
					</div>
				{/if}
			</section>
		</div>
	{/if}
</Panel>
