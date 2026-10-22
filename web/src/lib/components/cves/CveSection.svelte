<script lang="ts">
	/**
	 * CVE & vulnerabilities, the page body: off (explanation and the switch),
	 * or on (summary strip, filters, findings grouped by device, what is
	 * covered). Self-contained: own loads, own writes, own errors. Reads the
	 * `target` filter from the URL so the device page can link here. Writes
	 * (switch, key, refresh, ignore) are for administrators.
	 */
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ExternalLink, RefreshCw, ShieldAlert, Flame, Undo2, EyeOff } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import {
		getCveInventory,
		getCveSettings,
		getCveSummary,
		ignoreCve,
		listCves,
		refreshCves,
		restoreCve,
		setCveSettings
	} from '#lib/api/cves.js';
	import type { CveComponent, CveFinding, CveReason, CveSettings, CveSource, CveSummary } from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { formatRelative } from '#lib/format.js';
	import { Button, Disclosure, EmptyState, ErrorNotice, Plate, Skeleton } from '#lib/ui/index.js';
	import PasswordInput from '#lib/components/settings/PasswordInput.svelte';
	import {
		EMPTY_FILTERS,
		filtersActive,
		filtersToQuery,
		groupByDevice,
		parseTargetParam,
		severityTone,
		stateTone,
		type FilterState
	} from '#lib/cves.js';

	const POLL_MS = 5000;

	let settings = $state<CveSettings | null>(null);
	let summary = $state<CveSummary | null>(null);
	let findings = $state<CveFinding[]>([]);
	let inventory = $state<CveComponent[]>([]);
	let loaded = $state(false);
	let findingsLoaded = $state(false);
	let error = $state<unknown>(null);
	let actionError = $state<unknown>(null);
	let busy = $state(false);

	let filters = $state<FilterState>({ ...EMPTY_FILTERS, target: untrack(() => parseTargetParam(page.url.searchParams.get('target'))) });
	let nvdKey = $state('');
	let settingsOpen = $state(false);
	let sourcesOpen = $state(false);
	let inventoryOpen = $state(false);

	/** The row whose inline "ignore" form is open, and its note. */
	let ignoring = $state<string | null>(null);
	let note = $state('');

	const admin = $derived(auth.isAdmin);
	const enabled = $derived(settings?.enabled ?? false);
	const rowKey = (f: CveFinding) => `${f.target_id}:${f.cve_id}:${f.component}`;
	const groups = $derived(groupByDevice(findings));
	const devices = $derived.by(() => {
		const seen = new Map<number, string>();
		for (const c of inventory) seen.set(c.target_id, c.target_name);
		if (filters.target !== null && !seen.has(filters.target)) seen.set(filters.target, `#${filters.target}`);
		return [...seen].map(([id, name]) => ({ id, name })).sort((a, b) => a.name.localeCompare(b.name));
	});

	function aborted(cause: unknown): boolean {
		return cause instanceof DOMException && cause.name === 'AbortError';
	}

	async function loadBase(signal?: AbortSignal) {
		try {
			const [s, sum] = await Promise.all([getCveSettings(signal), getCveSummary(undefined, signal)]);
			settings = s;
			summary = sum;
			error = null;
		} catch (cause) {
			if (!aborted(cause)) error = cause;
		} finally {
			loaded = true;
		}
	}

	async function loadFindings(signal?: AbortSignal) {
		try {
			const list = await listCves(filtersToQuery(filters), signal);
			findings = list.findings;
			error = null;
		} catch (cause) {
			if (!aborted(cause)) error = cause;
		} finally {
			findingsLoaded = true;
		}
	}

	async function loadInventory(signal?: AbortSignal) {
		try {
			inventory = (await getCveInventory(undefined, signal)).components;
		} catch (cause) {
			if (!aborted(cause)) error = cause;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void loadBase(controller.signal);
		return () => controller.abort();
	});

	// Findings follow the filters (and the switch).
	$effect(() => {
		if (!enabled) return;
		const controller = new AbortController();
		void loadFindings(controller.signal);
		return () => controller.abort();
	});

	// Inventory feeds the device filter and the "what is covered" block.
	$effect(() => {
		if (!enabled) return;
		const controller = new AbortController();
		void loadInventory(controller.signal);
		return () => controller.abort();
	});

	// While a refresh runs, look at the summary every few seconds; when it ends, reload the list.
	const refreshing = $derived(summary?.refreshing ?? false);
	$effect(() => {
		if (!refreshing) return;
		const controller = new AbortController();
		const timer = setInterval(async () => {
			try {
				const next = await getCveSummary(undefined, controller.signal);
				const was = untrack(() => summary?.refreshing);
				summary = next;
				if (was && !next.refreshing) {
					void loadFindings(controller.signal);
					void loadInventory(controller.signal);
				}
			} catch {
				/* the next tick tries again */
			}
		}, POLL_MS);
		return () => {
			clearInterval(timer);
			controller.abort();
		};
	});

	async function save(input: { enabled: boolean; nvd_api_key?: string }) {
		actionError = null;
		busy = true;
		try {
			settings = await setCveSettings(input);
			nvdKey = '';
			summary = await getCveSummary();
		} catch (cause) {
			actionError = cause;
		} finally {
			busy = false;
		}
	}

	async function refreshNow() {
		actionError = null;
		busy = true;
		try {
			await refreshCves();
			summary = await getCveSummary();
		} catch (cause) {
			actionError = cause;
		} finally {
			busy = false;
		}
	}

	async function afterWrite() {
		summary = await getCveSummary();
		await loadFindings();
	}

	async function ignore(f: CveFinding) {
		actionError = null;
		busy = true;
		try {
			await ignoreCve(f.target_id, f.cve_id, note.trim());
			ignoring = null;
			note = '';
			await afterWrite();
		} catch (cause) {
			actionError = cause;
		} finally {
			busy = false;
		}
	}

	async function restore(f: CveFinding) {
		actionError = null;
		busy = true;
		try {
			await restoreCve(f.target_id, f.cve_id);
			await afterWrite();
		} catch (cause) {
			actionError = cause;
		} finally {
			busy = false;
		}
	}

	function setTarget(value: string) {
		filters.target = value ? Number(value) : null;
		const url = new URL(page.url);
		if (filters.target === null) url.searchParams.delete('target');
		else url.searchParams.set('target', String(filters.target));
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}

	function resetFilters() {
		filters = { ...EMPTY_FILTERS };
		setTarget('');
	}

	const REASON: Record<CveReason, () => string> = {
		below_fixed_version: () => m.security_cves_reason_below_fixed_version(),
		in_affected_range: () => m.security_cves_reason_in_affected_range(),
		no_fix_available: () => m.security_cves_reason_no_fix_available(),
		fixed_version_installed: () => m.security_cves_reason_fixed_version_installed(),
		vendor_build: () => m.security_cves_reason_vendor_build(),
		undetermined: () => m.security_cves_reason_undetermined(),
		version_unparsed: () => m.security_cves_reason_version_unparsed()
	};
	const STATE = {
		vulnerable: () => m.security_cves_state_vulnerable(),
		unknown: () => m.security_cves_state_unknown(),
		patched: () => m.security_cves_state_patched()
	};
	const SEVERITY = {
		critical: () => m.security_cves_severity_critical(),
		high: () => m.security_cves_severity_high(),
		medium: () => m.security_cves_severity_medium(),
		low: () => m.security_cves_severity_low(),
		none: () => m.security_cves_severity_none()
	};
	const SOURCE: Record<CveSource['name'], () => string> = {
		nvd: () => m.security_cves_source_nvd(),
		osv: () => m.security_cves_source_osv(),
		debian: () => m.security_cves_source_debian(),
		kev: () => m.security_cves_source_kev()
	};

	function versionText(f: CveFinding): string {
		if (f.fixed_version) return m.security_cves_version_fixed({ installed: f.installed_version, fixed: f.fixed_version });
		if (f.reason === 'no_fix_available') return m.security_cves_version_no_fix({ installed: f.installed_version });
		return m.security_cves_version_fix_unknown({ installed: f.installed_version });
	}

	const counts = $derived(summary?.counts);
</script>

{#snippet keyField()}
	<div class="grid gap-1.5">
		<label for="cve-nvd-key" class="text-sm font-semibold text-ink">{m.security_cves_nvd_key_label()}</label>
		<div class="flex flex-wrap items-center gap-2">
			<div class="min-w-48 flex-1">
				<PasswordInput id="cve-nvd-key" bind:value={nvdKey} autocomplete="off" disabled={busy} />
			</div>
			<Button size="md" disabled={busy || !nvdKey.trim()} onclick={() => save({ enabled, nvd_api_key: nvdKey.trim() })}>{m.security_cves_nvd_key_save()}</Button>
			{#if settings?.has_nvd_key}
				<Button size="md" variant="ghost" disabled={busy} onclick={() => save({ enabled, nvd_api_key: '' })}>{m.security_cves_nvd_key_clear()}</Button>
			{/if}
		</div>
		<p class="text-[0.8125rem] text-ink-2">
			<span class="font-semibold">{settings?.has_nvd_key ? m.security_cves_nvd_key_set() : m.security_cves_nvd_key_none()}.</span>
			{m.security_cves_nvd_key_help()}
		</p>
	</div>
{/snippet}

{#snippet countCell(label: string, value: number, tone: 'warning' | 'advisory' | 'signal' | 'muted', icon?: boolean)}
	<div class="min-w-0 rounded-[var(--radius-card)] border border-line bg-surface px-3 py-2.5">
		<p class="flex items-center gap-1 text-[0.8125rem] text-ink-2">
			{#if icon}<Flame class="size-3.5 text-warning-ink" aria-hidden="true" />{/if}
			<span class="truncate">{label}</span>
		</p>
		<p class={`tnum mt-0.5 text-2xl font-semibold ${value === 0 ? 'text-ink-3' : tone === 'warning' ? 'text-warning-ink' : tone === 'advisory' ? 'text-advisory-ink' : tone === 'signal' ? 'text-signal-ink' : 'text-ink'}`}>{value}</p>
	</div>
{/snippet}

{#if !loaded}
	<div class="grid gap-3" aria-busy="true" aria-label={m.security_cves_loading()}>
		<Skeleton class="h-20 w-full" />
		<Skeleton class="h-40 w-full" />
	</div>
{:else if error && !settings}
	<ErrorNotice {error} title={m.security_cves_load_error()} onretry={() => loadBase()} />
{:else if settings && !enabled}
	<section class="faceplate px-5 py-6 max-sm:px-4" aria-labelledby="cve-off-title">
		<div class="flex items-start gap-3">
			<ShieldAlert class="mt-0.5 size-5 shrink-0 text-ink-3" aria-hidden="true" />
			<div class="min-w-0">
				<h2 id="cve-off-title" class="text-base font-semibold text-ink">{m.security_cves_off_title()}</h2>
				<p class="mt-2 max-w-prose text-sm text-ink-2">{m.security_cves_off_body()}</p>
				<p class="mt-2 max-w-prose text-sm text-ink-2">{m.security_cves_off_sources()}</p>
			</div>
		</div>
		{#if admin}
			<div class="mt-5 grid max-w-xl gap-4">
				{@render keyField()}
				<div><Button variant="primary" loading={busy} onclick={() => save({ enabled: true, nvd_api_key: nvdKey.trim() || undefined })}>{m.security_cves_enable()}</Button></div>
			</div>
		{:else}
			<p class="mt-4 text-sm font-semibold text-ink">{m.security_cves_off_admin_only()}</p>
		{/if}
		{#if actionError}<ErrorNotice class="mt-4" error={actionError} local={false} />{/if}
	</section>
{:else if settings}
	<div class="grid gap-4">
		<!-- Summary strip -->
		{#if counts}
			<section aria-label={m.security_cves_title()} class="grid gap-3">
				<div class="grid grid-cols-2 gap-2 sm:grid-cols-5">
					{@render countCell(m.security_cves_count_vulnerable(), counts.vulnerable, 'warning')}
					{@render countCell(m.security_cves_count_exploited(), counts.exploited, 'warning', true)}
					{@render countCell(m.security_cves_count_unknown(), counts.unknown, 'advisory')}
					{@render countCell(m.security_cves_count_patched(), counts.patched, 'signal')}
					{@render countCell(m.security_cves_count_ignored(), counts.ignored, 'muted')}
				</div>
				<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
					<p class="text-sm text-ink-2" aria-live="polite">
						{#if refreshing}{m.security_cves_refreshing()}{:else if summary?.last_refresh}{m.security_cves_last_refresh({ when: formatRelative(summary.last_refresh) })}{:else}{m.security_cves_never_refreshed()}{/if}
					</p>
					{#if admin}
						<Button size="md" loading={refreshing || busy} disabled={refreshing} onclick={refreshNow}>
							<RefreshCw class="size-4" aria-hidden="true" />{m.security_cves_refresh()}
						</Button>
					{/if}
				</div>
			</section>
		{/if}

		{#if actionError}<ErrorNotice error={actionError} title={m.security_cves_action_error()} />{/if}
		{#if error}<ErrorNotice {error} title={m.security_cves_load_error()} onretry={() => loadFindings()} />{/if}

		<!-- Filters -->
		<fieldset class="m-0 grid grid-cols-2 gap-2 border-0 p-0 sm:grid-cols-[repeat(3,minmax(0,12rem))_auto_auto] sm:items-end">
			<legend class="sr-only">{m.security_cves_filters_label()}</legend>
			<label class="grid gap-1 text-[0.8125rem] font-semibold text-ink-2">
				{m.security_cves_filter_state()}
				<select class="input" bind:value={filters.state}>
					<option value="">{m.security_cves_filter_all_states()}</option>
					<option value="vulnerable">{m.security_cves_state_vulnerable()}</option>
					<option value="unknown">{m.security_cves_state_unknown()}</option>
					<option value="patched">{m.security_cves_state_patched()}</option>
				</select>
			</label>
			<label class="grid gap-1 text-[0.8125rem] font-semibold text-ink-2">
				{m.security_cves_filter_severity()}
				<select class="input" bind:value={filters.severity}>
					<option value="">{m.security_cves_filter_all_severities()}</option>
					<option value="critical">{m.security_cves_severity_critical()}</option>
					<option value="high">{m.security_cves_severity_high()}</option>
					<option value="medium">{m.security_cves_severity_medium()}</option>
					<option value="low">{m.security_cves_severity_low()}</option>
					<option value="none">{m.security_cves_severity_none()}</option>
				</select>
			</label>
			<label class="col-span-2 grid gap-1 text-[0.8125rem] font-semibold text-ink-2 sm:col-span-1">
				{m.security_cves_filter_device()}
				<select class="input" value={filters.target === null ? '' : String(filters.target)} onchange={(e) => setTarget(e.currentTarget.value)}>
					<option value="">{m.security_cves_filter_all_devices()}</option>
					{#each devices as d (d.id)}<option value={String(d.id)}>{d.name}</option>{/each}
				</select>
			</label>
			<label class="flex min-h-10 items-center gap-2 text-sm text-ink">
				<input type="checkbox" class="size-5 accent-[var(--c-signal)]" bind:checked={filters.exploited} />
				{m.security_cves_filter_exploited()}
			</label>
			<label class="flex min-h-10 items-center gap-2 text-sm text-ink">
				<input type="checkbox" class="size-5 accent-[var(--c-signal)]" bind:checked={filters.showIgnored} />
				{m.security_cves_filter_show_ignored()}
			</label>
		</fieldset>

		<!-- Findings, grouped by device -->
		{#if !findingsLoaded}
			<Skeleton class="h-40 w-full" />
		{:else if findings.length === 0}
			{#if filtersActive(filters)}
				<EmptyState title={m.security_cves_empty_filtered_title()} icon={ShieldAlert} secondary={{ label: m.security_cves_filter_reset(), onclick: resetFilters }} />
			{:else if (summary?.components ?? 0) === 0}
				<EmptyState title={m.security_cves_empty_pending_title()} description={m.security_cves_empty_pending_body()} icon={ShieldAlert} />
			{:else}
				<EmptyState tone="signal" title={m.security_cves_empty_clean_title()} description={m.security_cves_empty_clean_body()} icon={ShieldAlert} />
			{/if}
		{:else}
			<div class="grid gap-4">
				{#each groups as group (group.target_id)}
					<section class="overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface" aria-labelledby={`cve-dev-${group.target_id}`}>
						<header class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1 border-b border-line bg-surface-2 px-4 py-2.5">
							<h2 id={`cve-dev-${group.target_id}`} class="min-w-0 text-sm font-semibold text-ink">
								<a href={`/targets/${group.target_id}`} class="inline-flex min-h-10 items-center hover:underline sm:min-h-0" aria-label={m.security_cves_open_device({ name: group.target_name })}>{group.target_name}</a>
							</h2>
							<p class="tnum text-[0.8125rem] text-ink-2">{m.security_cves_group_summary({ vulnerable: group.vulnerable, unknown: group.unknown, patched: group.patched })}</p>
						</header>
						<ul class="m-0 list-none divide-y divide-line p-0">
							{#each group.findings as f (rowKey(f))}
								{@const key = rowKey(f)}
								<li class={`grid gap-2 px-4 py-3 ${f.ignored ? 'opacity-60' : ''}`}>
									<div class="flex flex-wrap items-center gap-x-2 gap-y-1.5">
										<a href={f.link} target="_blank" rel="noopener" class="inline-flex min-h-10 items-center gap-1 font-mono text-sm font-semibold text-ink hover:underline sm:min-h-0">
											{f.cve_id}<ExternalLink class="size-3.5" aria-hidden="true" />
										</a>
										<Plate tone={stateTone(f.state, false)}>{STATE[f.state]()}</Plate>
										{#if f.exploited}
											<Plate tone="warning" title={m.security_cves_exploited_hint()}>{m.security_cves_exploited()}</Plate>
										{/if}
										{#if f.ransomware}<Plate tone="warning" bare>{m.security_cves_ransomware()}</Plate>{/if}
										{#if f.ignored}<Plate tone="muted">{m.security_cves_ignored()}</Plate>{/if}
										<span class="tnum ml-auto flex items-center gap-1.5 text-[0.8125rem] text-ink-2">
											{#if f.cvss !== null}{m.security_cves_cvss({ score: f.cvss.toFixed(1) })}{/if}
											<Plate tone={severityTone(f.severity)} size="sm">{SEVERITY[f.severity]()}</Plate>
										</span>
									</div>
									{#if f.summary}<p class="m-0 line-clamp-2 max-w-prose text-sm text-ink">{f.summary}</p>{/if}
									<p class="m-0 text-[0.8125rem] text-ink-2 [overflow-wrap:anywhere]">
										<span class="font-semibold text-ink">{f.component}</span>, {versionText(f)}
									</p>
									<p class="m-0 text-[0.8125rem] text-ink-2">{REASON[f.reason]()}</p>
									{#if f.ignored && (f.ignored_by || f.ignore_note)}
										<p class="m-0 text-[0.8125rem] text-ink-2">
											{#if f.ignored_by}{m.security_cves_ignored_by({ user: f.ignored_by, when: formatRelative(f.ignored_at) })}{/if}
											{#if f.ignore_note} {m.security_cves_ignore_note({ note: f.ignore_note })}{/if}
										</p>
									{/if}
									{#if admin}
										{#if f.ignored}
											<div><Button size="sm" variant="ghost" disabled={busy} onclick={() => restore(f)}><Undo2 class="size-4" aria-hidden="true" />{m.security_cves_restore()}</Button></div>
										{:else if ignoring === key}
											<form class="grid gap-2 rounded-lg border border-line bg-surface-2 p-3" onsubmit={(e) => { e.preventDefault(); void ignore(f); }}>
												<p class="m-0 text-sm font-semibold text-ink">{m.security_cves_ignore_title({ cve: f.cve_id, device: f.target_name })}</p>
												<label class="grid gap-1 text-[0.8125rem] font-semibold text-ink-2">
													{m.security_cves_ignore_note_label()}
													<input class="input" type="text" maxlength="500" bind:value={note} />
												</label>
												<div class="flex flex-wrap gap-2">
													<Button type="submit" size="md" variant="primary" loading={busy}>{m.security_cves_ignore_confirm()}</Button>
													<Button size="md" variant="ghost" onclick={() => { ignoring = null; note = ''; }}>{m.security_cves_cancel()}</Button>
												</div>
											</form>
										{:else}
											<div><Button size="sm" variant="ghost" disabled={busy} onclick={() => { ignoring = key; note = ''; }}><EyeOff class="size-4" aria-hidden="true" />{m.security_cves_ignore()}</Button></div>
										{/if}
									{/if}
								</li>
							{/each}
						</ul>
					</section>
				{/each}
			</div>
		{/if}

		<!-- Sources -->
		{#if summary}
			<Disclosure title={m.security_cves_sources_title()} summary={m.security_cves_sources_summary({ count: summary.sources.length })} bind:open={sourcesOpen}>
				<ul class="m-0 grid list-none gap-3 p-0">
					{#each summary.sources as s (s.name)}
						<li class="grid gap-0.5 text-sm">
							<span class="font-semibold text-ink">{SOURCE[s.name]()}</span>
							<span class="text-[0.8125rem] text-ink-2">
								{#if s.last_success}{m.security_cves_source_ok({ when: formatRelative(s.last_success) })}{:else}{m.security_cves_source_never()}{/if}
								, {m.security_cves_source_items({ count: s.items })}
							</span>
							{#if s.last_error}<span class="text-[0.8125rem] text-warning-ink [overflow-wrap:anywhere]">{m.security_cves_source_error({ error: s.last_error })}</span>{/if}
						</li>
					{/each}
				</ul>
			</Disclosure>
		{/if}

		<!-- What is covered -->
		<Disclosure title={m.security_cves_inventory_title()} summary={m.security_cves_inventory_summary({ count: inventory.length })} bind:open={inventoryOpen}>
			{#if inventory.length === 0}
				<p class="m-0 text-sm text-ink-2">{m.security_cves_inventory_empty()}</p>
			{:else}
				<div class="grid gap-4">
					{#each [...new Map(inventory.map((c) => [c.target_id, c.target_name]))] as [tid, tname] (tid)}
						<div>
							<h4 class="m-0 text-sm font-semibold text-ink">{tname}</h4>
							<ul class="m-0 mt-1 list-none divide-y divide-line p-0">
								{#each inventory.filter((c) => c.target_id === tid) as c (c.component + c.version + c.origin)}
									<li class="flex flex-wrap items-baseline gap-x-3 gap-y-0.5 py-1.5 text-[0.8125rem]">
										<span class="min-w-0 font-semibold text-ink [overflow-wrap:anywhere]">{c.component}</span>
										<span class="font-mono text-ink-2" title={m.security_cves_inventory_version()}>{c.version}</span>
										<span class="text-ink-3" title={m.security_cves_inventory_origin()}>{c.origin}</span>
										<span class="tnum ml-auto text-ink-2">{m.security_cves_inventory_findings({ count: c.findings })}</span>
									</li>
								{/each}
							</ul>
						</div>
					{/each}
				</div>
			{/if}
		</Disclosure>

		<!-- Settings (administrators) -->
		{#if admin}
			<Disclosure title={m.security_cves_settings_title()} bind:open={settingsOpen}>
				<div class="grid max-w-xl gap-4">
					{@render keyField()}
					<div><Button variant="danger" loading={busy} onclick={() => save({ enabled: false })}>{m.security_cves_disable()}</Button></div>
				</div>
			</Disclosure>
		{/if}
	</div>
{/if}
