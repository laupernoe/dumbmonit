<script lang="ts">
	/**
	 * CVE & vulnerabilities, the page body: off (explanation and the switch),
	 * or on (counters, a pivot between CVEs and machines, quick filters, dense
	 * card galleries with foldable headers, what is covered). One read of every
	 * finding; pivots and filters run in the browser and live in the URL (the
	 * device page links here with `?target=`). Self-contained: own loads, own
	 * writes, own errors. Writes (switch, key, refresh, ignore) are for
	 * administrators.
	 */
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ChevronRight, ExternalLink, EyeOff, Flame, RefreshCw, Search, Server, ShieldAlert, Undo2, Wrench, X } from 'lucide-svelte';
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
	import { listClients } from '#lib/api/clients.js';
	import { listTargets } from '#lib/api/index.js';
	import type { CveComponent, CveFinding, CveReason, CveSeverity, CveSettings, CveSource, CveSummary } from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { mode } from '#lib/stores/mode.svelte.js';
	import { formatRelative } from '#lib/format.js';
	import { Button, Disclosure, EmptyState, ErrorNotice, Plate, Skeleton } from '#lib/ui/index.js';
	import PasswordInput from '#lib/components/settings/PasswordInput.svelte';
	import {
		EMPTY_VIEW,
		aggregateByCve,
		aggregateByMachine,
		filterFindings,
		parseViewState,
		severityRank,
		severityTone,
		stateTone,
		viewFiltersActive,
		viewStateToParams,
		type CveGroup,
		type MachineGroup,
		type MachineMeta,
		type ViewState
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

	/** Pivot, filters and focus: all of it lives in the URL (`?view=machine&sev=high&kev=1`). */
	let view = $state<ViewState>(untrack(() => parseViewState(page.url.searchParams)));
	let meta = $state<Map<number, MachineMeta>>(new Map());
	let clientNames = $state<Map<number, string>>(new Map());
	/** Group headers folded by the user, and cards whose findings are unfolded. */
	let folded = $state<Record<string, boolean>>({});
	let unfolded = $state<Record<string, boolean>>({});
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
	const visible = $derived(filterFindings(findings, view, meta));
	const cveGroups = $derived(view.pivot === 'cve' ? aggregateByCve(visible) : []);
	const machineGroups = $derived(view.pivot === 'machine' ? aggregateByMachine(visible, meta) : []);
	const totals = $derived({
		cves: new Set(visible.map((f) => f.cve_id)).size,
		machines: new Set(visible.map((f) => f.target_id)).size
	});
	const folders = $derived([...new Set([...meta.values()].map((x) => x.folder).filter((x) => x !== ''))].sort((a, b) => a.localeCompare(b)));
	const hasUnfoldered = $derived([...meta.values()].some((x) => x.folder === ''));
	const clientList = $derived(
		[...new Set([...meta.values()].map((x) => x.client).filter((x): x is number => x !== null))]
			.map((id) => ({ id, name: clientNames.get(id) ?? `#${id}` }))
			.sort((a, b) => a.name.localeCompare(b.name))
	);
	const dense = $derived(mode.enterprise);
	const filtersOn = $derived(viewFiltersActive(view));

	type Section<T> = { key: string; title: string; items: T[]; exploited: number };

	/** CVE pivot: one section per severity, worst first. */
	const cveSections = $derived.by<Section<CveGroup>[]>(() => {
		const by = new Map<CveSeverity, CveGroup[]>();
		for (const g of cveGroups) by.set(g.severity, [...(by.get(g.severity) ?? []), g]);
		return [...by]
			.sort((a, b) => severityRank(a[0]) - severityRank(b[0]))
			.map(([sev, items]) => ({ key: `sev:${sev}`, title: SEVERITY[sev](), items, exploited: items.filter((g) => g.exploited && g.open).length }));
	});

	/** Machine pivot: one section per client (MSP mode) or folder; a single unnamed one hides its header. */
	const machineSections = $derived.by<Section<MachineGroup>[]>(() => {
		const byClient = auth.mspOn;
		const by = new Map<string, { title: string; items: MachineGroup[] }>();
		for (const g of machineGroups) {
			const key = byClient ? `client:${g.client ?? 0}` : `folder:${g.folder}`;
			const title = byClient ? (g.client === null ? m.security_view_no_client() : (clientNames.get(g.client) ?? `#${g.client}`)) : g.folder || m.security_view_no_folder();
			const entry = by.get(key) ?? { title, items: [] };
			entry.items.push(g);
			by.set(key, entry);
		}
		return [...by].map(([key, v]) => ({ key, title: v.title, items: v.items, exploited: v.items.filter((g) => g.exploited > 0).length }));
	});
	const sections = $derived<Section<CveGroup | MachineGroup>[]>(view.pivot === 'cve' ? cveSections : machineSections);
	const headerless = $derived(view.pivot === 'machine' && machineSections.length === 1 && machineSections[0].key === 'folder:');

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
			const list = await listCves({}, signal);
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

	async function loadMeta(signal?: AbortSignal) {
		try {
			const targets = await listTargets(signal);
			meta = new Map(targets.map((t) => [t.id, { folder: t.group_name ?? '', client: t.client_id ?? null }]));
		} catch {
			/* without it only the folder and client filters are missing */
		}
		if (!auth.mspOn) return;
		try {
			clientNames = new Map((await listClients(signal)).clients.map((c) => [c.id, c.name]));
		} catch {
			/* a reader may not list clients: the filter shows their numbers */
		}
	}

	// Devices' folders and clients feed the matching filters.
	$effect(() => {
		if (!enabled) return;
		const controller = new AbortController();
		void loadMeta(controller.signal);
		return () => controller.abort();
	});

	// The URL follows the view (debounced for the search box); the page is the source of truth on first load only.
	$effect(() => {
		const next = viewStateToParams(view, untrack(() => page.url.searchParams)).toString();
		if (next === untrack(() => page.url.searchParams.toString())) return;
		const timer = setTimeout(() => {
			const url = new URL(untrack(() => page.url));
			url.search = next;
			void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
		}, 250);
		return () => clearTimeout(timer);
	});

	function resetFilters() {
		view = { ...EMPTY_VIEW, pivot: view.pivot };
	}

	/** A CVE clicked: the machines that have it. */
	function focusCve(id: string) {
		view = { ...view, pivot: 'machine', cve: id, machine: null };
	}

	/** A machine clicked: its CVEs. */
	function focusMachine(id: number) {
		view = { ...view, pivot: 'cve', machine: id, cve: '' };
	}

	function setPivot(pivot: 'cve' | 'machine') {
		view = { ...view, pivot };
	}

	function foldAll(fold: boolean) {
		folded = Object.fromEntries(sections.map((x) => [x.key, fold]));
	}

	const machineName = (id: number) => findings.find((f) => f.target_id === id)?.target_name ?? `#${id}`;

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
	<div class="flex min-w-0 items-baseline gap-1.5 rounded-[var(--radius-card)] border border-line bg-surface px-2.5 py-1.5">
		{#if icon}<Flame class="size-3.5 shrink-0 self-center text-warning-ink" aria-hidden="true" />{/if}
		<span class={`tnum text-base font-semibold ${value === 0 ? 'text-ink-3' : tone === 'warning' ? 'text-warning-ink' : tone === 'advisory' ? 'text-advisory-ink' : tone === 'signal' ? 'text-signal-ink' : 'text-ink'}`}>{value}</span>
		<span class="truncate text-[0.8125rem] text-ink-2">{label}</span>
	</div>
{/snippet}

{#snippet fixLine(available: boolean)}
	<span class={`inline-flex items-center gap-1 text-[0.75rem] font-semibold ${available ? 'text-signal-ink' : 'text-ink-3'}`}>
		<Wrench class="size-3.5 shrink-0" aria-hidden="true" />{available ? m.security_view_fix_yes() : m.security_view_fix_no()}
	</span>
{/snippet}

{#snippet selectFilter(label: string, value: string, onchange: (v: string) => void, options: { value: string; label: string }[])}
	<label class="grid min-w-0 gap-0.5 text-[0.75rem] font-semibold text-ink-2">
		{label}
		<select class="input !min-h-10 !py-1.5 text-sm font-normal" {value} onchange={(e) => onchange(e.currentTarget.value)}>
			{#each options as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
		</select>
	</label>
{/snippet}

{#snippet toggleChip(label: string, pressed: boolean, onclick: () => void, icon?: 'flame' | 'wrench')}
	<button
		type="button"
		aria-pressed={pressed}
		class={`inline-flex min-h-10 items-center gap-1.5 rounded-full border px-3 text-sm focus-visible:outline-2 focus-visible:outline-signal ${pressed ? 'border-ink bg-ink font-semibold text-surface' : 'border-line-strong bg-surface text-ink hover:bg-surface-2'}`}
		{onclick}
	>
		{#if icon === 'flame'}<Flame class="size-3.5" aria-hidden="true" />{:else if icon === 'wrench'}<Wrench class="size-3.5" aria-hidden="true" />{/if}
		{label}
	</button>
{/snippet}

{#snippet findingRow(f: CveFinding, who: 'machine' | 'cve')}
	{@const key = rowKey(f)}
	<li class={`grid gap-1.5 py-2 ${f.ignored ? 'opacity-70' : ''}`}>
		<div class="flex flex-wrap items-center gap-x-2 gap-y-1">
			{#if who === 'machine'}
				<a href={`/targets/${f.target_id}`} class="inline-flex min-h-10 items-center text-sm font-semibold text-ink hover:underline sm:min-h-0" aria-label={m.security_cves_open_device({ name: f.target_name })}>{f.target_name}</a>
			{:else}
				<a href={f.link} target="_blank" rel="noopener" class="inline-flex min-h-10 items-center gap-1 font-mono text-sm font-semibold text-ink hover:underline sm:min-h-0">{f.cve_id}<ExternalLink class="size-3.5" aria-hidden="true" /></a>
				<Plate tone={severityTone(f.severity)} size="sm">{SEVERITY[f.severity]()}</Plate>
				{#if f.exploited}<Plate tone="warning" title={m.security_cves_exploited_hint()}>{m.security_cves_exploited()}</Plate>{/if}
			{/if}
			<Plate tone={stateTone(f.state, false)}>{STATE[f.state]()}</Plate>
			{#if f.ignored}<Plate tone="muted">{m.security_cves_ignored()}</Plate>{/if}
		</div>
		<p class="m-0 text-[0.8125rem] text-ink-2 [overflow-wrap:anywhere]">
			<span class="font-semibold text-ink">{f.component}</span>, {versionText(f)}. {REASON[f.reason]()}
		</p>
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
{/snippet}

{#snippet detailsToggle(id: string)}
	<button
		type="button"
		class="inline-flex min-h-10 items-center gap-1 text-[0.8125rem] text-ink-2 hover:text-ink focus-visible:outline-2 focus-visible:outline-signal sm:min-h-8"
		aria-expanded={!!unfolded[id]}
		aria-controls={`cvecard-${id}`}
		onclick={() => (unfolded[id] = !unfolded[id])}
	>
		<ChevronRight class={`size-3.5 transition-transform duration-200 ease-out-expo ${unfolded[id] ? 'rotate-90' : ''}`} aria-hidden="true" />
		{unfolded[id] ? m.security_view_details_hide() : m.security_view_details_show()}
	</button>
{/snippet}

{#snippet cveCard(g: CveGroup)}
	{@const id = `c:${g.cve_id}`}
	<article class={`flex min-w-0 flex-col gap-1.5 rounded-[var(--radius-card)] border border-line bg-surface ${dense ? 'p-2.5' : 'p-3'} ${g.allIgnored ? 'opacity-70' : ''}`} aria-label={g.cve_id}>
		<div class="flex flex-wrap items-center gap-x-1.5 gap-y-1">
			<button
				type="button"
				class="inline-flex min-h-10 items-center rounded font-mono text-sm font-semibold text-ink underline-offset-2 hover:underline focus-visible:outline-2 focus-visible:outline-signal sm:min-h-8"
				title={m.security_view_card_machines({ cve: g.cve_id })}
				aria-label={m.security_view_card_machines({ cve: g.cve_id })}
				onclick={() => focusCve(g.cve_id)}
			>{g.cve_id}</button>
			<a href={g.link} target="_blank" rel="noopener" class="inline-flex size-8 items-center justify-center rounded text-ink-3 hover:text-ink" aria-label={m.security_view_external({ cve: g.cve_id })}><ExternalLink class="size-3.5" aria-hidden="true" /></a>
			<span class="tnum ml-auto flex items-center gap-1.5 text-[0.75rem] text-ink-2">
				{#if g.cvss !== null}{g.cvss.toFixed(1)}{/if}
				<Plate tone={severityTone(g.severity)} size="sm">{SEVERITY[g.severity]()}</Plate>
			</span>
		</div>
		<div class="flex flex-wrap items-center gap-x-2.5 gap-y-1">
			{#if g.exploited}<Plate tone="warning" title={m.security_cves_exploited_hint()}>{m.security_cves_exploited()}</Plate>{/if}
			{#if g.ransomware}<Plate tone="warning" bare>{m.security_cves_ransomware()}</Plate>{/if}
			{#if g.allIgnored}<Plate tone="muted">{m.security_cves_ignored()}</Plate>{:else if !g.open}<Plate tone="signal">{m.security_cves_state_patched()}</Plate>{:else}{@render fixLine(g.fixAvailable)}{/if}
		</div>
		{#if g.summary}<p class={`m-0 text-[0.8125rem] text-ink-2 ${dense ? 'line-clamp-1' : 'line-clamp-2'}`} title={g.summary}>{g.summary}</p>{/if}
		<ul class="m-0 flex list-none flex-wrap items-center gap-1 p-0" aria-label={m.security_view_machines_count({ count: g.machines.length })}>
			{#each g.machines.slice(0, 3) as mc (mc.id)}
				<li class="min-w-0 max-w-full">
					<button
						type="button"
						class="inline-flex min-h-10 max-w-full items-center gap-1 rounded-full border border-line bg-surface-2 px-2.5 text-[0.8125rem] text-ink hover:border-ink-3 focus-visible:outline-2 focus-visible:outline-signal sm:min-h-8"
						title={m.security_view_card_cves({ name: mc.name })}
						aria-label={m.security_view_card_cves({ name: mc.name })}
						onclick={() => focusMachine(mc.id)}
					><Server class="size-3 shrink-0 text-ink-3" aria-hidden="true" /><span class="truncate">{mc.name}</span></button>
				</li>
			{/each}
			{#if g.machines.length > 3}<li class="text-[0.8125rem] text-ink-2">{m.security_view_more({ count: g.machines.length - 3 })}</li>{/if}
		</ul>
		<div class="flex flex-wrap items-center justify-between gap-2">
			<span class="tnum text-[0.75rem] text-ink-3">{m.security_view_machines_count({ count: g.machines.length })}</span>
			{@render detailsToggle(id)}
		</div>
		<ul id={`cvecard-${id}`} hidden={!unfolded[id]} class="m-0 list-none divide-y divide-line border-t border-line p-0">
			{#if unfolded[id]}{#each g.findings as f (rowKey(f))}{@render findingRow(f, 'machine')}{/each}{/if}
		</ul>
	</article>
{/snippet}

{#snippet machineCard(g: MachineGroup)}
	{@const id = `m:${g.target_id}`}
	<article class={`flex min-w-0 flex-col gap-1.5 rounded-[var(--radius-card)] border border-line bg-surface ${dense ? 'p-2.5' : 'p-3'}`} aria-label={g.target_name}>
		<div class="flex flex-wrap items-center gap-x-1.5 gap-y-1">
			<button
				type="button"
				class="inline-flex min-h-10 min-w-0 max-w-full items-center gap-1.5 rounded text-sm font-semibold text-ink underline-offset-2 hover:underline focus-visible:outline-2 focus-visible:outline-signal sm:min-h-8"
				title={m.security_view_card_cves({ name: g.target_name })}
				aria-label={m.security_view_card_cves({ name: g.target_name })}
				onclick={() => focusMachine(g.target_id)}
			><Server class="size-3.5 shrink-0 text-ink-3" aria-hidden="true" /><span class="truncate">{g.target_name}</span></button>
			<a href={`/targets/${g.target_id}`} class="inline-flex size-8 items-center justify-center rounded text-ink-3 hover:text-ink" aria-label={m.security_cves_open_device({ name: g.target_name })}><ExternalLink class="size-3.5" aria-hidden="true" /></a>
			<span class="ml-auto"><Plate tone={severityTone(g.worst)} size="sm">{SEVERITY[g.worst]()}</Plate></span>
		</div>
		<div class="flex flex-wrap items-center gap-x-2.5 gap-y-1">
			{#if g.exploited > 0}<Plate tone="warning" title={m.security_cves_exploited_hint()}>{m.security_cves_exploited()}: {g.exploited}</Plate>{/if}
			{@render fixLine(g.fixable > 0)}
		</div>
		<p class="tnum m-0 text-[0.8125rem] text-ink-2">{m.security_cves_group_summary({ vulnerable: g.vulnerable, unknown: g.unknown, patched: g.patched })}</p>
		<div class="flex flex-wrap items-center justify-between gap-2">
			<span class="tnum text-[0.75rem] text-ink-3">{m.security_view_cves_count({ count: g.cves })}</span>
			{@render detailsToggle(id)}
		</div>
		<ul id={`cvecard-${id}`} hidden={!unfolded[id]} class="m-0 list-none divide-y divide-line border-t border-line p-0">
			{#if unfolded[id]}{#each g.findings as f (rowKey(f))}{@render findingRow(f, 'cve')}{/each}{/if}
		</ul>
	</article>
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
		<!-- Summary strip: one line of counters, refresh on the right -->
		{#if counts}
			<section aria-label={m.security_cves_title()} class="flex flex-wrap items-center gap-2">
				{@render countCell(m.security_cves_count_vulnerable(), counts.vulnerable, 'warning')}
				{@render countCell(m.security_cves_count_exploited(), counts.exploited, 'warning', true)}
				{@render countCell(m.security_cves_count_unknown(), counts.unknown, 'advisory')}
				{@render countCell(m.security_cves_count_patched(), counts.patched, 'signal')}
				{@render countCell(m.security_cves_count_ignored(), counts.ignored, 'muted')}
				<div class="ml-auto flex flex-wrap items-center gap-x-3 gap-y-1">
					<p class="m-0 text-[0.8125rem] text-ink-2" aria-live="polite">
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

		<!-- Pivot, search and quick filters -->
		<div class="grid gap-2.5" role="search">
			<div class="flex flex-wrap items-center gap-2">
				<div class="inline-flex rounded-lg border border-line-strong bg-surface p-0.5" role="group" aria-label={m.security_view_pivot_label()}>
					{#each [{ id: 'cve', label: m.security_view_pivot_cve() }, { id: 'machine', label: m.security_view_pivot_machine() }] as p (p.id)}
						<button
							type="button"
							aria-pressed={view.pivot === p.id}
							class={`min-h-10 rounded-md px-3.5 text-sm focus-visible:outline-2 focus-visible:outline-signal ${view.pivot === p.id ? 'bg-ink font-semibold text-surface' : 'text-ink hover:bg-surface-2'}`}
							onclick={() => setPivot(p.id as 'cve' | 'machine')}
						>{p.label}</button>
					{/each}
				</div>
				<div class="relative min-w-48 flex-1">
					<Search class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-ink-3" aria-hidden="true" />
					<input type="search" class="input !pl-9" bind:value={view.q} placeholder={m.security_view_search_placeholder()} aria-label={m.security_view_search_label()} autocomplete="off" />
				</div>
			</div>

			<div class="flex flex-wrap items-end gap-2">
				{@render selectFilter(m.security_cves_filter_severity(), view.severity, (v) => (view.severity = v as CveSeverity | ''), [
					{ value: '', label: m.security_cves_filter_all_severities() },
					{ value: 'critical', label: m.security_cves_severity_critical() },
					{ value: 'high', label: m.security_cves_severity_high() },
					{ value: 'medium', label: m.security_cves_severity_medium() },
					{ value: 'low', label: m.security_cves_severity_low() },
					{ value: 'none', label: m.security_cves_severity_none() }
				])}
				{@render selectFilter(m.security_cves_filter_state(), view.status, (v) => (view.status = v as ViewState['status']), [
					{ value: 'open', label: m.security_view_status_open() },
					{ value: 'all', label: m.security_cves_filter_all_states() },
					{ value: 'vulnerable', label: m.security_cves_state_vulnerable() },
					{ value: 'unknown', label: m.security_cves_state_unknown() },
					{ value: 'patched', label: m.security_cves_state_patched() }
				])}
				{@render selectFilter(m.security_view_filter_ignored(), view.ignored, (v) => (view.ignored = v as ViewState['ignored']), [
					{ value: 'all', label: m.security_cves_filter_show_ignored() },
					{ value: 'hide', label: m.security_view_ignored_hide() },
					{ value: 'only', label: m.security_view_ignored_only() }
				])}
				{#if folders.length > 0}
					{@render selectFilter(m.security_view_filter_folder(), view.folder ?? '__all__', (v) => (view.folder = v === '__all__' ? null : v), [
						{ value: '__all__', label: m.security_view_all_folders() },
						...(hasUnfoldered ? [{ value: '', label: m.security_view_no_folder() }] : []),
						...folders.map((f) => ({ value: f, label: f }))
					])}
				{/if}
				{#if auth.mspOn && clientList.length > 1}
					{@render selectFilter(m.security_view_filter_client(), view.client === null ? '' : String(view.client), (v) => (view.client = v ? Number(v) : null), [
						{ value: '', label: m.security_view_all_clients() },
						...clientList.map((c) => ({ value: String(c.id), label: c.name }))
					])}
				{/if}
				{@render toggleChip(m.security_cves_filter_exploited(), view.kev, () => (view.kev = !view.kev), 'flame')}
				{@render toggleChip(m.security_view_filter_fix(), view.fix, () => (view.fix = !view.fix), 'wrench')}
				{#if filtersOn}
					<Button size="md" variant="ghost" onclick={resetFilters}>{m.security_cves_filter_reset()}</Button>
				{/if}
			</div>

			{#if view.cve || view.machine !== null}
				<div class="flex flex-wrap items-center gap-2" aria-label={m.security_view_focus_label()}>
					{#if view.cve}
						<span class="inline-flex min-h-10 items-center gap-1 rounded-full border border-ink bg-surface-2 pr-1 pl-3 text-sm text-ink">
							<span class="font-mono font-semibold">{view.cve}</span>
							<button type="button" class="inline-flex size-8 items-center justify-center rounded-full hover:bg-surface focus-visible:outline-2 focus-visible:outline-signal" aria-label={m.security_view_focus_clear({ what: view.cve })} onclick={() => (view.cve = '')}><X class="size-4" aria-hidden="true" /></button>
						</span>
					{/if}
					{#if view.machine !== null}
						<span class="inline-flex min-h-10 items-center gap-1 rounded-full border border-ink bg-surface-2 pr-1 pl-3 text-sm text-ink">
							<Server class="size-3.5" aria-hidden="true" /><span class="font-semibold">{machineName(view.machine)}</span>
							<button type="button" class="inline-flex size-8 items-center justify-center rounded-full hover:bg-surface focus-visible:outline-2 focus-visible:outline-signal" aria-label={m.security_view_focus_clear({ what: machineName(view.machine) })} onclick={() => (view.machine = null)}><X class="size-4" aria-hidden="true" /></button>
						</span>
					{/if}
				</div>
			{/if}
		</div>

		<!-- Results -->
		{#if !findingsLoaded}
			<Skeleton class="h-40 w-full" />
		{:else if visible.length === 0}
			{#if filtersOn}
				<EmptyState title={m.security_cves_empty_filtered_title()} icon={ShieldAlert} secondary={{ label: m.security_cves_filter_reset(), onclick: resetFilters }} />
			{:else if (summary?.components ?? 0) === 0}
				<EmptyState title={m.security_cves_empty_pending_title()} description={m.security_cves_empty_pending_body()} icon={ShieldAlert} />
			{:else}
				<EmptyState tone="signal" title={m.security_cves_empty_clean_title()} description={m.security_cves_empty_clean_body()} icon={ShieldAlert} />
			{/if}
		{:else}
			<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-1">
				<p class="tnum m-0 text-[0.8125rem] text-ink-2" aria-live="polite">{m.security_view_result({ cves: totals.cves, machines: totals.machines })}</p>
				{#if !headerless && sections.length > 1}
					<div class="flex gap-1">
						<Button size="sm" variant="ghost" onclick={() => foldAll(true)}>{m.security_view_collapse_all()}</Button>
						<Button size="sm" variant="ghost" onclick={() => foldAll(false)}>{m.security_view_expand_all()}</Button>
					</div>
				{/if}
			</div>
			<div class="grid gap-3">
				{#each sections as section (section.key)}
					<section class="grid gap-2" aria-label={section.title}>
						{#if !headerless}
							<h2 class="m-0 text-sm font-semibold text-ink">
								<button
									type="button"
									class="flex min-h-10 w-full items-center gap-2 rounded-md border-b border-line px-1 text-left hover:bg-surface-2 focus-visible:outline-2 focus-visible:outline-signal"
									aria-expanded={!folded[section.key]}
									onclick={() => (folded[section.key] = !folded[section.key])}
								>
									<ChevronRight class={`size-4 shrink-0 text-ink-3 transition-transform duration-200 ease-out-expo ${folded[section.key] ? '' : 'rotate-90'}`} aria-hidden="true" />
									<span class="min-w-0 truncate">{section.title}</span>
									<span class="tnum rounded-full bg-surface-2 px-2 text-[0.8125rem] font-normal text-ink-2" title={m.security_view_group_total({ count: section.items.length })}>{section.items.length}</span>
									{#if section.exploited > 0}<Plate tone="warning" size="sm" title={m.security_cves_exploited_hint()}>{m.security_cves_exploited()}: {section.exploited}</Plate>{/if}
								</button>
							</h2>
						{/if}
						{#if !folded[section.key] || headerless}
							<div class={`grid items-start gap-2 ${dense ? 'grid-cols-[repeat(auto-fill,minmax(min(100%,15rem),1fr))]' : 'grid-cols-[repeat(auto-fill,minmax(min(100%,18rem),1fr))]'}`}>
								{#each section.items as item (view.pivot === 'cve' ? (item as CveGroup).cve_id : (item as MachineGroup).target_id)}
									{#if view.pivot === 'cve'}{@render cveCard(item as CveGroup)}{:else}{@render machineCard(item as MachineGroup)}{/if}
								{/each}
							</div>
						{/if}
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
