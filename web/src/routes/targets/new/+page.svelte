<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Add a device — the one way to add anything.
	 *
	 * Three views, all in the URL so the browser's Back button walks them:
	 * the picker (no parameter: two front doors — the agent on a machine, a
	 * network scan — then every kind, searchable),
	 * a kind's form (`?kind=snmp`, `?kind=agent&via=docker`), the network scan
	 * (`?scan=1`). The setup guide (`&guide=1`) opens only on request, beside
	 * the form on a desktop, above it on a phone. Every type, notice and option
	 * comes from `GET /api/collectors`.
	 *
	 * Each step inside this page is its own history entry, counted in
	 * `page.state.depth`: "All device types" rewinds exactly that many, so Back
	 * from the picker still leaves for wherever the user came from. A page
	 * opened straight on `?kind=` (depth 0) replaces its entry instead.
	 */
	import { tick, untrack } from 'svelte';
	import { page } from '$app/state';
	import { afterNavigate, goto } from '$app/navigation';
	import { ArrowLeft, BookOpen, CalendarClock, ChevronRight, FileSpreadsheet, Globe, Plug, Radar, Upload } from 'lucide-svelte';
	import { ApiError, listCollectors, listTargets, type CollectorInfo, type Target } from '#lib/api/index.js';
	import { Button, EmptyState, ErrorNotice, PageHeader, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import ImportWebsites from '#lib/components/device-form/ImportWebsites.svelte';
	import ImportKuma from '#lib/components/device-form/ImportKuma.svelte';
	import ImportServices from '#lib/components/device-form/ImportServices.svelte';
	import { auth } from '#lib/stores/auth.svelte.js';
	import CollectorPicker from '#lib/components/device-form/CollectorPicker.svelte';
	import TargetForm from '#lib/components/device-form/TargetForm.svelte';
	import AgentEnroll from '#lib/components/device-form/AgentEnroll.svelte';
	import { isMailKind } from '#lib/components/devices/mdaemon/agent-link.js';
	import { readPrefill } from '#lib/components/device-form/discovery-prefill.js';
	import Discovery from '#lib/components/device-form/Discovery.svelte';
	import DiscoveryInventory from '#lib/components/device-form/DiscoveryInventory.svelte';
	import AddBySite from '#lib/components/device-form/AddBySite.svelte';
	import { NO_DEFAULTS, defaultsParams, type SiteDefaults } from '#lib/components/device-form/add-by-site.js';
	import SetupNotice from '#lib/components/device-form/SetupNotice.svelte';
	import KindIcon from '#lib/components/KindIcon.svelte';
	import { AGENT_KIND, SNMP_KIND, agentFeature, kindIcon, type ImportChoice } from '#lib/components/device-form/kinds.js';

	let kumaImport = $state<ImportKuma | null>(null);
	let websitesImport = $state<ImportWebsites | null>(null);
	let uptimerobotImport = $state<ImportServices | null>(null);
	let gatusImport = $state<ImportServices | null>(null);
	let healthchecksImport = $state<ImportServices | null>(null);
	/** Ways to bring monitors over, listed last in the picker. Administrators only. */
	const imports = $derived<ImportChoice[]>(
		auth.isAdmin
			? [
					{ id: 'kuma', label: m.devices_kuma_open(), summary: m.deviceform_picker_import_kuma_summary(), icon: Upload, onrun: () => kumaImport?.show() },
					{ id: 'uptimerobot', label: m.devices_svc_uptimerobot_open(), summary: m.deviceform_picker_import_uptimerobot_summary(), icon: Upload, onrun: () => uptimerobotImport?.show() },
					{ id: 'gatus', label: m.devices_svc_gatus_open(), summary: m.deviceform_picker_import_gatus_summary(), icon: Upload, onrun: () => gatusImport?.show() },
					{ id: 'healthchecks', label: m.devices_svc_healthchecks_open(), summary: m.deviceform_picker_import_healthchecks_summary(), icon: Upload, onrun: () => healthchecksImport?.show() },
					{ id: 'websites', label: m.devices_import_open(), summary: m.deviceform_picker_import_websites_summary(), icon: FileSpreadsheet, onrun: () => websitesImport?.show() }
				]
			: []
	);

	let collectors = $state<CollectorInfo[]>([]);
	let targets = $state<Target[]>([]);
	/** True once the device list came back: only then is "first device" known. */
	let targetsKnown = $state(false);
	let loading = $state(true);
	let error = $state<unknown>(null);
	/** True when the server predates `/api/collectors`. */
	let unavailable = $state(false);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		unavailable = false;
		try {
			collectors = await listCollectors(signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			if (cause instanceof ApiError && cause.missing) unavailable = true;
			else error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		// Only feeds the "Parent device" list: its failure must not block adding.
		void listTargets(controller.signal)
			.then((list) => {
				targets = list;
				targetsKnown = true;
			})
			.catch(() => {});
		return () => controller.abort();
	});

	// --- Where we are: read from the URL only --------------------------------
	const params = $derived(page.url.searchParams);
	const requestedKind = $derived(params.get('kind'));
	/** Values from the network scan (`?prefill=`), for the kind's form only. */
	const prefill = $derived(readPrefill(params));
	/** A text from the URL (`?folder=`), or empty. */
	const textParam = (key: string) => params.get(key)?.trim() ?? '';
	/** A positive id from the URL (`?agent=`, `?via_agent=`), or `null`. */
	function idParam(key: string): number | null {
		const value = Number(params.get(key));
		return Number.isInteger(value) && value > 0 ? value : null;
	}
	const selected = $derived(collectors.find((c) => c.kind === requestedKind) ?? null);
	/** `?scan=1` opens the network scan: the first-run guide and the command palette link to it. */
	const scanning = $derived(!requestedKind && params.get('scan') === '1');
	const snmp = $derived(collectors.find((c) => c.kind === SNMP_KIND) ?? null);
	/** What the agent was picked for (Docker, Plakar…), when it was picked that way. */
	const feature = $derived(selected?.kind === AGENT_KIND ? agentFeature(params.get('via')) : null);
	/** `?os=windows&then=mdaemon`: the agent install for a mail server, then its monitor. */
	const windowsSetup = $derived(params.get('os') === 'windows');
	const afterKind = $derived(isMailKind(params.get('then') ?? '') ? (params.get('then') ?? '') : '');
	const selectedId = $derived(selected ? (feature ? `${AGENT_KIND}:${feature.id}` : selected.kind) : null);
	const has = (kind: string) => collectors.some((c) => c.kind === kind);
	/** What the setup guide explains: the scan is an SNMP matter. */
	const noticeFor = $derived(selected ?? (scanning && snmp ? snmp : null));
	const guideOpen = $derived(noticeFor !== null && params.get('guide') === '1');
	/** The view the URL asks for, known before the kinds are loaded: what focus follows. */
	const viewOf = (p: Pick<URLSearchParams, 'get'>) => (p.get('kind') ? 'kind' : p.get('scan') === '1' ? 'scan' : 'picker');
	const urlView = $derived(viewOf(params));
	/** What is shown: an unknown `?kind=` falls back to the picker once the kinds are in. */
	const view = $derived(loading ? urlView : selected ? 'kind' : scanning ? 'scan' : 'picker');

	type Steps = { depth?: number; guide?: boolean };
	const steps = $derived(page.state as Steps);
	const depth = $derived(steps.depth ?? 0);

	/** One step inside the page: a new history entry, or the current one rewritten. */
	function navigate(edit: (p: URLSearchParams) => void, opts: { push: boolean; guide?: boolean; scroll?: boolean }) {
		const url = new URL(page.url.href);
		edit(url.searchParams);
		const state: Steps = { depth: opts.push ? depth + 1 : depth, guide: opts.guide ?? false };
		// Focus is managed below (afterNavigate); SvelteKit 3 merged `keepFocus`
		// and `noScroll` into `reset`, so a step that scrolls does it by hand.
		const done = goto(`${url.pathname}${url.search}`, {
			replace: !opts.push,
			reset: false,
			state: state as App.PageState
		});
		return opts.scroll ? done.then(() => window.scrollTo(0, 0)) : done;
	}

	/** The last choice, so the picker puts the focus back on it. */
	let lastChoice = $state<string | null>(null);

	/** The site chosen in the picker (`?site=`), and what it pre-fills. */
	let site = $state<string | null>(untrack(() => page.url.searchParams.get('site')));
	let siteDefaults = $state<SiteDefaults>(NO_DEFAULTS);
	/** Keeps the choice in the URL (rewritten, never a history step) so a reload or Back keeps it. */
	$effect(() => {
		const wanted = site;
		untrack(() => {
			if ((page.url.searchParams.get('site') ?? null) === wanted || view !== 'picker') return;
			void navigate((p) => (wanted ? p.set('site', wanted) : p.delete('site')), { push: false });
		});
	});

	function select(kind: string, via: string | null = null, defaults: SiteDefaults | null = null) {
		// A kind picked from the full list still gets the site's defaults.
		const withSite = defaults ?? (site === null ? null : siteDefaults);
		void navigate(
			(p) => {
				p.delete('scan');
				p.delete('guide');
				p.set('kind', kind);
				if (via) p.set('via', via);
				else p.delete('via');
				if (withSite) {
					for (const key of ['location', 'via_agent', 'folder']) p.delete(key);
					for (const [key, value] of Object.entries(defaultsParams(withSite))) p.set(key, value);
				}
			},
			{ push: true, scroll: true }
		);
	}

	function openScan() {
		void navigate(
			(p) => {
				p.delete('kind');
				p.delete('via');
				p.delete('guide');
				p.set('scan', '1');
			},
			{ push: true, scroll: true }
		);
	}

	/** Back to the picker: rewinds our own steps, or rewrites a page opened on a kind. */
	function toPicker() {
		lastChoice = selectedId ?? (scanning ? 'scan' : null);
		if (depth > 0) history.go(-depth);
		else void navigate((p) => ['kind', 'via', 'scan', 'guide', 'location', 'folder'].forEach((key) => p.delete(key)), { push: false });
	}

	function openGuide() {
		void navigate((p) => p.set('guide', '1'), { push: true, guide: true });
	}

	/** Closing the guide undoes its step, so Back never reopens it. */
	function closeGuide() {
		if (steps.guide) history.back();
		else void navigate((p) => p.delete('guide'), { push: false });
	}

	// --- Focus follows the view, for keyboard and screen reader users --------
	// Seeded from the landing URL: on first load the focus stays where the browser puts it.
	let previousView = viewOf(page.url.searchParams);
	let wasGuideOpen = page.url.searchParams.get('guide') === '1';
	let previousChoice: string | null = null;
	afterNavigate(() => {
		const from = previousView;
		const guideWas = wasGuideOpen;
		const choiceWas = previousChoice;
		previousView = urlView;
		wasGuideOpen = guideOpen;
		previousChoice = selectedId ?? (scanning ? 'scan' : null);
		// Browser Back skips toPicker(): remember the choice being left all the same.
		if (urlView === 'picker' && from !== 'picker') lastChoice = choiceWas ?? lastChoice;
		void tick().then(() => {
			let target: HTMLElement | null = null;
			if (urlView !== from) {
				if (urlView === 'picker') {
					target =
						document.querySelector<HTMLElement>(`[data-choice="${lastChoice}"]`) ??
						document.getElementById('step-type');
				} else {
					target = document.getElementById('view-title');
				}
			} else if (guideOpen && !guideWas) {
				target = document.getElementById('setup-guide');
				if (target && !window.matchMedia('(min-width: 64rem)').matches) {
					const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
					target.scrollIntoView({ block: 'start', behavior: reduced ? 'auto' : 'smooth' });
				}
			} else if (!guideOpen && guideWas) {
				target = document.getElementById('guide-toggle');
			}
			target?.focus({ preventScroll: urlView === 'picker' || (guideOpen && !guideWas) });
		});
	});

	const SelectedIcon = $derived(feature ? feature.icon : selected ? kindIcon(selected.kind) : null);
	const selectedLabel = $derived(feature ? feature.label : (selected?.label ?? ''));
	const relaying = $derived(feature?.id === 'relay');
	const selectedSummary = $derived(
		relaying
			? m.deviceform_new_summary_relay()
			: feature
				? m.deviceform_new_summary_agent()
				: (selected?.summary ?? '')
	);
	const crumb = $derived(view === 'kind' ? selectedLabel : view === 'scan' ? m.deviceform_new_scan() : null);
	const description = $derived(
		view === 'picker'
			? m.deviceform_new_desc_picker()
			: view === 'scan'
				? m.deviceform_new_desc_scan()
				: relaying
					? m.deviceform_new_desc_relay()
					: selected?.kind === AGENT_KIND
						? m.deviceform_new_desc_agent()
						: m.deviceform_new_desc_form()
	);

</script>

<!-- Esc inside the open guide puts it away. -->
<svelte:window
	onkeydown={(event) => {
		if (event.key !== 'Escape' || !guideOpen || event.defaultPrevented) return;
		if (event.target instanceof Node && document.getElementById('setup-guide')?.contains(event.target)) {
			event.preventDefault();
			closeGuide();
		}
	}}
/>

<svelte:head><title>{crumb ? m.deviceform_new_page_title_crumb({ crumb }) : m.deviceform_new_page_title()}</title></svelte:head>

{#snippet stamp()}
	{#if selected && SelectedIcon}
		<Plate tone="ghost" bare>
			{#if feature}<SelectedIcon class="size-3.5" aria-hidden="true" />{:else}<KindIcon kind={selected.kind} class="size-3.5" />{/if}
			{selected.kind}
		</Plate>
	{/if}
{/snippet}

<!-- Where am I: every level above the current one is a link. -->
<nav aria-label={m.deviceform_new_breadcrumb()} class="mb-2">
	<!-- Phones: only the way back; the page title says where this is. -->
	<ol class="flex flex-wrap items-center gap-1 text-sm text-ink-2 max-sm:[&>li:not(:first-child)]:hidden">
		<li class="flex">
			<a href="/targets" class="inline-flex items-center gap-1 rounded max-sm:min-h-10 hover:text-ink hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal">
				<ArrowLeft class="size-3.5" aria-hidden="true" />
				{m.deviceform_new_devices()}
			</a>
		</li>
		<li aria-hidden="true"><ChevronRight class="size-3.5 text-ink-3" /></li>
		{#if crumb}
			<li>
				<a
					href="/targets/new"
					onclick={(event) => {
						event.preventDefault();
						toPicker();
					}}
					class="rounded hover:text-ink hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal"
				>
					{m.deviceform_new_add()}
				</a>
			</li>
			<li aria-hidden="true"><ChevronRight class="size-3.5 text-ink-3" /></li>
			<li aria-current="page" class="max-w-[16rem] truncate font-semibold text-ink">{crumb}</li>
		{:else}
			<li aria-current="page" class="font-semibold text-ink">{m.deviceform_new_add()}</li>
		{/if}
	</ol>
</nav>

<PageHeader title={m.deviceform_new_add()} {description} />

<!-- Opened from the "Import" category of the picker. -->
{#if auth.isAdmin}
	<ImportKuma bind:this={kumaImport} trigger={false} />
	<ImportServices bind:this={uptimerobotImport} provider="uptimerobot" />
	<ImportServices bind:this={gatusImport} provider="gatus" />
	<ImportServices bind:this={healthchecksImport} provider="healthchecks" />
	<ImportWebsites bind:this={websitesImport} trigger={false} />
{/if}

{#if view === 'picker'}
	<!-- The picker ------------------------------------------------------------ -->
	<section aria-labelledby="step-type" class="min-w-0">
		<h2 id="step-type" tabindex="-1" class="mb-4 text-lg font-semibold tracking-tight text-ink outline-none max-sm:sr-only">{m.deviceform_new_step_title()}</h2>

		{#if loading}
			<div class="grid gap-2 sm:grid-cols-2">
				{#each { length: 6 } as _, i (i)}
					<div class="rounded-[var(--radius-card)] border border-line bg-surface px-3.5 py-3">
						<Skeleton class="h-4 w-1/2" />
						<Skeleton class="mt-2 h-3.5 w-full" />
						<Skeleton class="mt-1.5 h-3 w-2/3" />
					</div>
				{/each}
			</div>
		{:else if unavailable}
			<EmptyState
				title={m.deviceform_new_unavailable_title()}
				description={m.deviceform_new_unavailable_desc()}
			/>
		{:else if error}
			<ErrorNotice {error} title={m.deviceform_new_load_error()} onretry={() => void load()} />
		{:else if collectors.length === 0}
			<EmptyState
				title={m.deviceform_new_none_title()}
				description={m.deviceform_new_none_desc()}
			/>
		{:else}
			<AddBySite {collectors} bind:site bind:defaults={siteDefaults} onkind={(kind, d) => select(kind, null, d)} onadded={(t) => (targets = [...targets, t])} />
			{#if requestedKind}
				<p class="mb-4 rounded-lg border border-advisory/35 bg-advisory-soft px-3.5 py-2.5 text-sm text-ink">
					{m.deviceform_new_unknown_kind({ kind: requestedKind })}
				</p>
			{/if}
			<CollectorPicker {collectors} {imports} selected={lastChoice === 'scan' ? null : lastChoice} wide onscan={snmp ? openScan : undefined} onselect={(kind, via) => select(kind, via)}>
				{#snippet empty()}
					<p class="mt-1 text-sm text-ink-2">{m.deviceform_new_empty_lead()}</p>
					<div class="mt-3 flex flex-wrap justify-center gap-2">
						{#if snmp}
							<Button size="sm" variant="secondary" onclick={openScan}>
								<Radar class="size-4" aria-hidden="true" />
								{m.deviceform_new_scan()}
							</Button>
						{/if}
						{#if has('tcp')}
							<Button size="sm" variant="secondary" onclick={() => select('tcp')}>
								<Plug class="size-4" aria-hidden="true" />
								{m.deviceform_new_any_port()}
							</Button>
						{/if}
						{#if has('http')}
							<Button size="sm" variant="secondary" onclick={() => select('http')}>
								<Globe class="size-4" aria-hidden="true" />
								{m.deviceform_new_web_page()}
							</Button>
						{/if}
					</div>
				{/snippet}
			</CollectorPicker>
		{/if}
	</section>
{:else}
	<!-- A kind's form, or the scan ------------------------------------------ -->
	<div class="mb-4 flex flex-wrap items-center justify-between gap-2">
		<Button size="sm" variant="secondary" onclick={toPicker}>
			<ArrowLeft class="size-4" aria-hidden="true" />
			{m.deviceform_new_all_types()}
		</Button>
		{#if noticeFor}
			<Button
				id="guide-toggle"
				size="sm"
				variant={guideOpen ? 'secondary' : 'ghost'}
				aria-expanded={guideOpen}
				aria-controls={guideOpen ? 'setup-guide' : undefined}
				onclick={guideOpen ? closeGuide : openGuide}
			>
				<BookOpen class="size-4" aria-hidden="true" />
				{guideOpen ? m.deviceform_new_guide_hide() : m.deviceform_new_guide_show()}
			</Button>
		{/if}
	</div>

	<div class="grid items-start gap-6 lg:grid-cols-12">
		<section aria-labelledby="view-title" class={`min-w-0 ${guideOpen ? 'lg:col-span-7' : 'lg:col-span-12'}`}>
			{#if view === 'scan'}
				<div class="grid gap-6">
					<Panel>
						<h2 id="view-title" tabindex="-1" class="mb-4 flex items-center gap-2 text-base font-semibold tracking-tight text-ink outline-none">
							<Radar class="size-[1.125rem] text-ink-2" aria-hidden="true" />
							{m.deviceform_new_scan()}
						</h2>
						<Discovery agent={idParam('agent')} />
					</Panel>
					<Panel>
						<h2 class="mb-4 flex items-center gap-2 text-base font-semibold tracking-tight text-ink">
							<CalendarClock class="size-[1.125rem] text-ink-2" aria-hidden="true" />
							{m.discovery_inventory_title()}
						</h2>
						<DiscoveryInventory />
					</Panel>
				</div>
			{:else if selected}
				<Panel>
					<div class="-mx-5 -mt-4 mb-4 flex items-start justify-between gap-4 border-b border-line px-5 py-4 max-sm:-mx-4 max-sm:-mt-3 max-sm:mb-3 max-sm:px-4 max-sm:py-3">
						<div class="flex min-w-0 items-start gap-3">
							{#if SelectedIcon}
								<span class="flex size-9 shrink-0 items-center justify-center rounded-lg border border-signal/30 bg-signal-soft text-signal-ink">
									{#if feature || !selected}<SelectedIcon class="size-[1.125rem]" aria-hidden="true" />{:else}<KindIcon kind={selected.kind} class="size-[1.125rem]" />{/if}
								</span>
							{/if}
							<div class="min-w-0">
								<h2 id="view-title" tabindex="-1" class="text-base font-semibold tracking-tight text-ink outline-none">{selectedLabel}</h2>
								{#if selectedSummary}<p class="mt-0.5 text-sm text-ink-2 max-sm:hidden">{selectedSummary}</p>{/if}
							</div>
						</div>
						<div class="hidden shrink-0 sm:block">{@render stamp()}</div>
					</div>
					<!-- Re-mounted per kind: the form seeds itself once from its collector. -->
					{#key selected.kind}
						{#if selected.kind === AGENT_KIND}
							{#if feature && !relaying}
								<p class="mb-5 rounded-lg border border-line bg-surface-2 px-3.5 py-2.5 text-sm leading-relaxed text-ink">
									{feature.next}
								</p>
							{/if}
							{#key relaying}
								<AgentEnroll cancelHref="/targets" relay={relaying} windows={windowsSetup} {afterKind} />
							{/key}
						{:else}
							{#if prefill}
								<p class="mb-5 rounded-lg border border-signal/30 bg-signal-soft px-3.5 py-2.5 text-sm text-ink">
									{m.deviceform_new_prefilled()}
								</p>
							{/if}
							<TargetForm
								collector={selected}
								{prefill}
								viaAgentDefault={idParam('via_agent')}
								locationDefault={idParam('location')}
								folderDefault={textParam('folder')}
								{targets}
								cancelHref="/targets"
								onsaved={(saved) =>
									goto(`/targets/${saved.id}`, {
										// The very first device gets a small celebration on its page.
										state: { firstDevice: targetsKnown && targets.length === 0 } as App.PageState
									})}
							/>
						{/if}
					{/key}
				</Panel>
			{:else if loading}
				<Panel><Skeleton class="h-40 w-full" /></Panel>
			{:else if error}
				<ErrorNotice {error} title={m.deviceform_new_load_error()} onretry={() => void load()} />
			{/if}
		</section>

		{#if guideOpen}
			<!-- On a phone the guide comes first: the user just asked for it. -->
			<aside
				id="setup-guide"
				tabindex="-1"
				aria-label={m.deviceform_new_guide_label()}
				class="order-first min-w-0 scroll-mt-20 outline-none lg:sticky lg:top-20 lg:order-none lg:col-span-5"
			>
				<SetupNotice collector={noticeFor} onclose={closeGuide} />
			</aside>
		{/if}
	</div>
{/if}
