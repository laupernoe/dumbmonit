<script lang="ts">
	/**
	 * Add a device — the one way to add anything.
	 *
	 * Three views, all in the URL so the browser's Back button walks them:
	 * the picker (no parameter: two front doors, then every kind, searchable),
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
	import { tick } from 'svelte';
	import { page } from '$app/state';
	import { afterNavigate, goto } from '$app/navigation';
	import { ArrowLeft, ArrowRight, BookOpen, ChevronRight, Cpu, Globe, Plug, Radar } from 'lucide-svelte';
	import { ApiError, listCollectors, listTargets, type CollectorInfo, type Target } from '$lib/api';
	import { Button, EmptyState, ErrorNotice, PageHeader, Panel, Plate, Skeleton } from '$lib/ui';
	import CollectorPicker from '$lib/components/device-form/CollectorPicker.svelte';
	import TargetForm from '$lib/components/device-form/TargetForm.svelte';
	import AgentEnroll from '$lib/components/device-form/AgentEnroll.svelte';
	import Discovery from '$lib/components/device-form/Discovery.svelte';
	import SetupNotice from '$lib/components/device-form/SetupNotice.svelte';
	import { AGENT_KIND, SNMP_KIND, agentFeature, kindIcon } from '$lib/components/device-form/kinds';

	let collectors = $state<CollectorInfo[]>([]);
	let targets = $state<Target[]>([]);
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
			.then((list) => (targets = list))
			.catch(() => {});
		return () => controller.abort();
	});

	// --- Where we are: read from the URL only --------------------------------
	const params = $derived(page.url.searchParams);
	const requestedKind = $derived(params.get('kind'));
	const selected = $derived(collectors.find((c) => c.kind === requestedKind) ?? null);
	/** `?scan=1` opens the network scan: the first-run guide and the command palette link to it. */
	const scanning = $derived(!requestedKind && params.get('scan') === '1');
	const snmp = $derived(collectors.find((c) => c.kind === SNMP_KIND) ?? null);
	const hasAgent = $derived(collectors.some((c) => c.kind === AGENT_KIND));
	/** What the agent was picked for (Docker, Plakar…), when it was picked that way. */
	const feature = $derived(selected?.kind === AGENT_KIND ? agentFeature(params.get('via')) : null);
	const selectedId = $derived(selected ? (feature ? `${AGENT_KIND}:${feature.id}` : selected.kind) : null);
	const has = (kind: string) => collectors.some((c) => c.kind === kind);
	/** What the setup guide explains: the scan is an SNMP matter. */
	const noticeFor = $derived(selected ?? (scanning && snmp ? snmp : null));
	const guideOpen = $derived(noticeFor !== null && params.get('guide') === '1');
	/** The view the URL asks for, known before the kinds are loaded: what focus follows. */
	const viewOf = (p: URLSearchParams) => (p.get('kind') ? 'kind' : p.get('scan') === '1' ? 'scan' : 'picker');
	const urlView = $derived(viewOf(params));
	/** What is shown: an unknown `?kind=` falls back to the picker once the kinds are in. */
	const view = $derived(loading ? urlView : selected ? 'kind' : scanning ? 'scan' : 'picker');

	type Steps = { depth?: number; guide?: boolean };
	const steps = $derived(page.state as Steps);
	const depth = $derived(steps.depth ?? 0);

	/** One step inside the page: a new history entry, or the current one rewritten. */
	function navigate(edit: (p: URLSearchParams) => void, opts: { push: boolean; guide?: boolean; scroll?: boolean }) {
		const url = new URL(page.url);
		edit(url.searchParams);
		const state: Steps = { depth: opts.push ? depth + 1 : depth, guide: opts.guide ?? false };
		return goto(`${url.pathname}${url.search}`, {
			replaceState: !opts.push,
			keepFocus: true,
			noScroll: !opts.scroll,
			state: state as App.PageState
		});
	}

	/** The last choice, so the picker puts the focus back on it. */
	let lastChoice = $state<string | null>(null);

	function select(kind: string, via: string | null = null) {
		void navigate(
			(p) => {
				p.delete('scan');
				p.delete('guide');
				p.set('kind', kind);
				if (via) p.set('via', via);
				else p.delete('via');
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
		else void navigate((p) => ['kind', 'via', 'scan', 'guide'].forEach((key) => p.delete(key)), { push: false });
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
						(lastChoice === 'scan'
							? document.querySelector<HTMLElement>('[data-door="scan"]')
							: document.querySelector<HTMLElement>(`[data-choice="${lastChoice}"]`)) ??
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
	const selectedSummary = $derived(feature ? 'Comes through the agent: install it on the machine.' : (selected?.summary ?? ''));
	const crumb = $derived(view === 'kind' ? selectedLabel : view === 'scan' ? 'Scan my network' : null);
	const description = $derived(
		view === 'picker'
			? 'Pick what to watch. The next step asks for the few fields it needs.'
			: view === 'scan'
				? 'Finds SNMP devices on a network range and adds them in one go.'
				: selected?.kind === AGENT_KIND
					? 'One command on the machine, and it reports on its own.'
					: 'Tell DumbMonit where it is and how to read it.'
	);

	const doorClass =
		'group flex items-start gap-3.5 rounded-[var(--radius-card)] px-4 py-3.5 text-left transition-[transform,box-shadow] duration-200 ease-out-expo hover:-translate-y-px hover:shadow-float focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal';
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

<svelte:head><title>{crumb ? `${crumb} · ` : ''}Add a device · DumbMonit</title></svelte:head>

{#snippet stamp()}
	{#if selected && SelectedIcon}
		<Plate tone="ghost" bare>
			<SelectedIcon class="size-3.5" aria-hidden="true" />
			{selected.kind}
		</Plate>
	{/if}
{/snippet}

<!-- Where am I: every level above the current one is a link. -->
<nav aria-label="Breadcrumb" class="mb-2">
	<ol class="flex flex-wrap items-center gap-1 text-sm text-ink-2">
		<li class="flex">
			<a href="/targets" class="inline-flex items-center gap-1 rounded hover:text-ink hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal">
				<ArrowLeft class="size-3.5" aria-hidden="true" />
				Devices
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
					Add a device
				</a>
			</li>
			<li aria-hidden="true"><ChevronRight class="size-3.5 text-ink-3" /></li>
			<li aria-current="page" class="max-w-[16rem] truncate font-semibold text-ink">{crumb}</li>
		{:else}
			<li aria-current="page" class="font-semibold text-ink">Add a device</li>
		{/if}
	</ol>
</nav>

<PageHeader title="Add a device" {description} />

{#if view === 'picker'}
	<!-- The picker ------------------------------------------------------------ -->
	<section aria-labelledby="step-type" class="min-w-0">
		<h2 id="step-type" tabindex="-1" class="mb-4 text-lg font-semibold tracking-tight text-ink outline-none">What do you want to watch?</h2>

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
				title="This server does not list its device types"
				description="It is older than this interface. Update the server to add devices from here."
			/>
		{:else if error}
			<ErrorNotice {error} title="Could not load the device types" onretry={() => void load()} />
		{:else if collectors.length === 0}
			<EmptyState
				title="No device type is enabled"
				description="No collector is active on this instance. Check its configuration, then reload this page."
			/>
		{:else}
			{#if requestedKind}
				<p class="mb-4 rounded-lg border border-advisory/35 bg-advisory-soft px-3.5 py-2.5 text-sm text-ink">
					This server has no device type called <code class="font-mono">{requestedKind}</code>. Pick one below.
				</p>
			{/if}
			{#if hasAgent || snmp}
				<!-- The two front doors: most of a homelab comes in through one of them. -->
				<div class={`mb-6 grid gap-3 ${hasAgent && snmp ? 'md:grid-cols-2' : ''}`}>
					{#if hasAgent}
						<button type="button" data-door="agent" onclick={() => select(AGENT_KIND)} class={`${doorClass} border border-signal/40 bg-signal-soft`}>
							<span class="flex size-10 shrink-0 items-center justify-center rounded-lg border border-signal/30 bg-surface text-signal-ink">
								<Cpu class="size-5" aria-hidden="true" />
							</span>
							<span class="min-w-0 flex-1">
								<span class="block font-semibold text-ink">Install the agent on a machine</span>
								<span class="mt-0.5 block text-sm leading-snug text-ink-2">
									One command on Linux, Windows, macOS or FreeBSD: system, disks, Docker, services and backups.
								</span>
							</span>
							<ArrowRight class="mt-1 size-4 shrink-0 text-signal-ink transition-transform group-hover:translate-x-0.5" aria-hidden="true" />
						</button>
					{/if}
					{#if snmp}
						<button type="button" data-door="scan" onclick={openScan} class={`${doorClass} border border-line-strong bg-surface`}>
							<span class="flex size-10 shrink-0 items-center justify-center rounded-lg border border-line bg-surface-2 text-ink-2">
								<Radar class="size-5" aria-hidden="true" />
							</span>
							<span class="min-w-0 flex-1">
								<span class="block font-semibold text-ink">Scan my network</span>
								<span class="mt-0.5 block text-sm leading-snug text-ink-2">
									Give a range like 192.168.1.0/24: every device answering SNMP is listed, ready to add.
								</span>
							</span>
							<ArrowRight class="mt-1 size-4 shrink-0 text-ink-3 transition-transform group-hover:translate-x-0.5" aria-hidden="true" />
						</button>
					{/if}
				</div>
			{/if}
			<CollectorPicker {collectors} selected={lastChoice === 'scan' ? null : lastChoice} wide onselect={(kind, via) => select(kind, via)}>
				{#snippet empty()}
					<p class="mt-1 text-sm text-ink-2">Most things still fit one of these:</p>
					<div class="mt-3 flex flex-wrap justify-center gap-2">
						{#if snmp}
							<Button size="sm" variant="secondary" onclick={openScan}>
								<Radar class="size-4" aria-hidden="true" />
								Scan my network
							</Button>
						{/if}
						{#if has('tcp')}
							<Button size="sm" variant="secondary" onclick={() => select('tcp')}>
								<Plug class="size-4" aria-hidden="true" />
								Any network port
							</Button>
						{/if}
						{#if has('http')}
							<Button size="sm" variant="secondary" onclick={() => select('http')}>
								<Globe class="size-4" aria-hidden="true" />
								A web page
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
			All device types
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
				{guideOpen ? 'Hide setup guide' : 'Setup guide'}
			</Button>
		{/if}
	</div>

	<div class="grid items-start gap-6 lg:grid-cols-12">
		<section aria-labelledby="view-title" class={`min-w-0 ${guideOpen ? 'lg:col-span-7' : 'lg:col-span-12'}`}>
			{#if view === 'scan'}
				<Panel>
					<h2 id="view-title" tabindex="-1" class="mb-4 flex items-center gap-2 text-base font-semibold tracking-tight text-ink outline-none">
						<Radar class="size-[1.125rem] text-ink-2" aria-hidden="true" />
						Scan my network
					</h2>
					<Discovery />
				</Panel>
			{:else if selected}
				<Panel>
					<div class="-mx-5 -mt-4 mb-4 flex items-start justify-between gap-4 border-b border-line px-5 py-4">
						<div class="flex min-w-0 items-start gap-3">
							{#if SelectedIcon}
								<span class="flex size-9 shrink-0 items-center justify-center rounded-lg border border-signal/30 bg-signal-soft text-signal-ink">
									<SelectedIcon class="size-[1.125rem]" aria-hidden="true" />
								</span>
							{/if}
							<div class="min-w-0">
								<h2 id="view-title" tabindex="-1" class="text-base font-semibold tracking-tight text-ink outline-none">{selectedLabel}</h2>
								{#if selectedSummary}<p class="mt-0.5 text-sm text-ink-2">{selectedSummary}</p>{/if}
							</div>
						</div>
						<div class="hidden shrink-0 sm:block">{@render stamp()}</div>
					</div>
					<!-- Re-mounted per kind: the form seeds itself once from its collector. -->
					{#key selected.kind}
						{#if selected.kind === AGENT_KIND}
							{#if feature}
								<p class="mb-5 rounded-lg border border-line bg-surface-2 px-3.5 py-2.5 text-sm leading-relaxed text-ink">
									{feature.next}
								</p>
							{/if}
							<AgentEnroll cancelHref="/targets" />
						{:else}
							<TargetForm
								collector={selected}
								{targets}
								cancelHref="/targets"
								onsaved={(saved) => goto(`/targets/${saved.id}`)}
							/>
						{/if}
					{/key}
				</Panel>
			{:else if loading}
				<Panel><Skeleton class="h-40 w-full" /></Panel>
			{:else if error}
				<ErrorNotice {error} title="Could not load the device types" onretry={() => void load()} />
			{/if}
		</section>

		{#if guideOpen}
			<!-- On a phone the guide comes first: the user just asked for it. -->
			<aside
				id="setup-guide"
				tabindex="-1"
				aria-label="Setup guide"
				class="order-first min-w-0 scroll-mt-20 outline-none lg:sticky lg:top-20 lg:order-none lg:col-span-5"
			>
				<SetupNotice collector={noticeFor} onclose={closeGuide} />
			</aside>
		{/if}
	</div>
{/if}
