<script lang="ts">
	/**
	 * Add a device — the one way to add anything.
	 *
	 * Step 1: choose what to watch — two front doors (install the agent, scan
	 * the network), then every kind, searchable. Step 2: the form that type asks
	 * for, with the setup notice beside it; the notice only appears once there is
	 * something to prepare. Every type, notice and option comes from
	 * `GET /api/collectors`. `?kind=` makes a choice linkable
	 * ("/targets/new?kind=snmp"), `?via=docker` names what the agent was chosen for.
	 */
	import { tick } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { ArrowRight, Cpu, Globe, Plug, Radar, RefreshCw } from 'lucide-svelte';
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
	/** `?scan=1` opens the network scan straight away: the first-run guide links to it. */
	let scanning = $state(page.url.searchParams.get('scan') === '1');
	// The command palette can ask for the scan while this page is already open.
	$effect(() => {
		if (page.url.searchParams.get('scan') === '1') scanning = true;
	});

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

	/** The chosen kind lives in the URL so the page is linkable and survives reloads. */
	const requestedKind = $derived(page.url.searchParams.get('kind'));
	const selected = $derived(collectors.find((c) => c.kind === requestedKind) ?? null);
	const snmp = $derived(collectors.find((c) => c.kind === SNMP_KIND) ?? null);
	const hasAgent = $derived(collectors.some((c) => c.kind === AGENT_KIND));
	/** What the agent was picked for (Docker, Plakar…), when it was picked that way. */
	const feature = $derived(selected?.kind === AGENT_KIND ? agentFeature(page.url.searchParams.get('via')) : null);
	const selectedId = $derived(selected ? (feature ? `${AGENT_KIND}:${feature.id}` : selected.kind) : null);
	const has = (kind: string) => collectors.some((c) => c.kind === kind);
	/** What the right column explains: the scan is an SNMP matter. */
	const noticeFor = $derived(selected ?? (scanning ? snmp : null));

	/**
	 * Once a kind is chosen the picker folds into one row and the form takes
	 * its place, so step 2 is never a screen away. Landing with `?kind=` starts
	 * folded; "Change type" unfolds the grid again.
	 */
	let expanded = $state(false);
	const folded = $derived(selected !== null && !expanded);
	let formSection = $state<HTMLElement | null>(null);

	async function select(kind: string, via: string | null = null) {
		scanning = false;
		expanded = false;
		const url = new URL(page.url);
		url.searchParams.delete('scan');
		url.searchParams.set('kind', kind);
		if (via) url.searchParams.set('via', via);
		else url.searchParams.delete('via');
		await goto(`${url.pathname}${url.search}`, { replaceState: true, keepFocus: true, noScroll: true });
		await tick();
		const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		formSection?.scrollIntoView({ block: 'start', behavior: reduced ? 'auto' : 'smooth' });
	}

	function changeType() {
		expanded = true;
		// Bring the current choice into view so the user sees where they are in the grid.
		void tick().then(() =>
			document.querySelector<HTMLButtonElement>(`[data-choice="${selectedId}"]`)?.focus()
		);
	}

	function openScan() {
		scanning = true;
		const url = new URL(page.url);
		url.searchParams.delete('kind');
		url.searchParams.delete('via');
		url.searchParams.set('scan', '1');
		void goto(`${url.pathname}${url.search}`, { replaceState: true, keepFocus: true, noScroll: true });
	}

	/** Leaving the scan drops the flag, so a reload does not reopen it. */
	function closeScan() {
		scanning = false;
		const url = new URL(page.url);
		url.searchParams.delete('scan');
		void goto(`${url.pathname}${url.search}`, { replaceState: true, keepFocus: true, noScroll: true });
	}

	const SelectedIcon = $derived(feature ? feature.icon : selected ? kindIcon(selected.kind) : null);
	const selectedLabel = $derived(feature ? feature.label : (selected?.label ?? ''));
	const selectedSummary = $derived(feature ? 'Comes through the agent: install it on the machine.' : (selected?.summary ?? ''));
	/** The setup notice belongs to step 2: until then the picker has the whole width. */
	const withNotice = $derived(noticeFor !== null);
</script>

<svelte:head><title>Add a device · DumbMonit</title></svelte:head>

{#snippet stamp()}
	{#if selected && SelectedIcon}
		<Plate tone="ghost" bare>
			<SelectedIcon class="size-3.5" aria-hidden="true" />
			{selected.kind}
		</Plate>
	{/if}
{/snippet}

<PageHeader
	title="Add a device"
	description="Pick what to watch. The next step says what to prepare and asks for the few fields it needs."
	back={{ href: '/targets', label: 'Devices' }}
/>

<div class="grid items-start gap-6 lg:grid-cols-12">
	<div class={`grid min-w-0 ${withNotice ? 'lg:col-span-7' : 'lg:col-span-12'} ${folded ? 'gap-5' : 'gap-8'}`}>
		<!-- Step 1 ------------------------------------------------------------ -->
		<section aria-labelledby="step-type" class="min-w-0 scroll-mt-20">
			{#if folded && selected && SelectedIcon}
				<!-- The choice, folded into one row: the grid is one click away. -->
				<div class="flex items-center gap-3 rounded-[var(--radius-card)] border border-signal/40 bg-signal-soft px-4 py-3">
					<span class="tnum hidden size-7 shrink-0 items-center justify-center rounded-full bg-surface text-sm text-signal-ink sm:flex" aria-hidden="true">1</span>
					<span class="flex size-9 shrink-0 items-center justify-center rounded-lg border border-signal/30 bg-surface text-signal-ink">
						<SelectedIcon class="size-[1.125rem]" aria-hidden="true" />
					</span>
					<div class="min-w-0 flex-1">
						<h2 id="step-type" class="leading-tight font-semibold text-ink sm:truncate">{selectedLabel}</h2>
						<!-- The summary is a desktop luxury: on a phone the label and the button are what matter. -->
						{#if selectedSummary}
							<p class="hidden truncate text-sm text-ink-2 sm:block">{selectedSummary}</p>
						{/if}
					</div>
					<Button size="sm" variant="ghost" class="shrink-0" onclick={changeType}>
						<RefreshCw class="size-3.5" aria-hidden="true" />
						Change type
					</Button>
				</div>
			{:else}
<h2 id="step-type" class="mb-4 flex items-center gap-3 text-lg font-semibold tracking-tight text-ink">
					<span class="tnum flex size-7 items-center justify-center rounded-full bg-signal-soft text-sm text-signal-ink" aria-hidden="true">1</span>
					What do you want to watch?
				</h2>

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
				{:else if scanning}
					<Panel title="Scan my network" description="Finds SNMP devices on a network range and adds them in one go.">
						{#snippet aside()}
							<Button size="sm" variant="ghost" onclick={closeScan}>Choose a type instead</Button>
						{/snippet}
						<Discovery />
					</Panel>
				{:else}
					{#if hasAgent || snmp}
						<!-- The two front doors: most of a homelab comes in through one of them. -->
						<div class={`mb-6 grid gap-3 ${hasAgent && snmp ? 'md:grid-cols-2' : ''}`}>
							{#if hasAgent}
								<button
									type="button"
									onclick={() => void select(AGENT_KIND)}
									class="group flex items-start gap-3.5 rounded-[var(--radius-card)] border border-signal/40 bg-signal-soft px-4 py-3.5 text-left transition-[transform,box-shadow] duration-200 ease-out-expo hover:-translate-y-px hover:shadow-float focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal"
								>
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
								<button
									type="button"
									onclick={openScan}
									class="group flex items-start gap-3.5 rounded-[var(--radius-card)] border border-line-strong bg-surface px-4 py-3.5 text-left transition-[transform,box-shadow] duration-200 ease-out-expo hover:-translate-y-px hover:shadow-float focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal"
								>
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
					<CollectorPicker
						{collectors}
						selected={selectedId}
						wide={!withNotice}
						onselect={(kind, via) => void select(kind, via)}
					>
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
									<Button size="sm" variant="secondary" onclick={() => void select('tcp')}>
										<Plug class="size-4" aria-hidden="true" />
										Any network port
									</Button>
								{/if}
								{#if has('http')}
									<Button size="sm" variant="secondary" onclick={() => void select('http')}>
										<Globe class="size-4" aria-hidden="true" />
										A web page
									</Button>
								{/if}
							</div>
						{/snippet}
					</CollectorPicker>
				{/if}
			{/if}
		</section>

		<!-- Step 2 ------------------------------------------------------------ -->
		{#if selected}
			<section bind:this={formSection} aria-labelledby="step-form" class="rise-in min-w-0 scroll-mt-20" style="--rise-delay: 60ms">
				<h2 id="step-form" class="mb-4 flex items-center gap-3 text-lg font-semibold tracking-tight text-ink">
					<span class="tnum flex size-7 items-center justify-center rounded-full bg-signal-soft text-sm text-signal-ink" aria-hidden="true">2</span>
					{selected.kind === AGENT_KIND ? 'Install the agent' : 'Tell DumbMonit where it is'}
				</h2>
				<!-- Folded, the row above already names the kind: the panel header would repeat it. -->
				<Panel
					title={folded ? undefined : selectedLabel}
					description={folded ? undefined : selectedSummary || undefined}
					aside={folded ? undefined : stamp}
				>
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
			</section>
		{/if}
	</div>

	{#if withNotice}
		<aside class="min-w-0 lg:sticky lg:top-20 lg:col-span-5">
			<SetupNotice collector={noticeFor} />
		</aside>
	{/if}
</div>
