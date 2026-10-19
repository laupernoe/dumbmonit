<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Scan my network, multi-product. The server runs a scan as a background
	 * job; this screen starts it, polls it every 2 s (results appear as they
	 * are found), can cancel it, and keeps the latest jobs one click away.
	 * Found services with a matching collector open the add form pre-filled.
	 */
	import { onMount } from 'svelte';
	import { LayoutList, Network, Radar, Square } from 'lucide-svelte';
	import { ApiError, localSubnets, type LocalSubnet, type SubnetSource } from '#lib/api/index.js';
	import { cancelDiscoveryJob, getDiscoveryJob, listDiscoveryJobs, startDiscoveryJob } from '#lib/api/discovery.js';
	import { listRelays } from '#lib/api/relay.js';
	import type { DiscoveryDepth, DiscoveryJob, DiscoveryJobStatus, DiscoveryJobSummary, DiscoveryPhase, RelayAgent } from '#lib/api/types.js';
	import { formatRelative } from '#lib/format.js';
	import { Button, EmptyState, ErrorNotice, Field, Plate, Toggle, type Tone } from '#lib/ui/index.js';
	import { PHONE_QUERY } from '#lib/ui/index.js';
	import DiscoveryGraph from './DiscoveryGraph.svelte';
	import DiscoveryList from './DiscoveryList.svelte';
	import { filterHosts, percent, stats } from './discovery-graph';
	import { defaultSubnet, usableSubnets } from './test-connection';

	const POLL_MS = 2000;

	let cidr = $state('');
	let subnets = $state<LocalSubnet[]>([]);
	let prefilled = $state<LocalSubnet | null>(null);
	let relays = $state<RelayAgent[]>([]);
	let agentId = $state('');
	let community = $state('');
	let depth = $state<DiscoveryDepth>('standard');

	let job = $state<DiscoveryJob | null>(null);
	let history = $state<DiscoveryJobSummary[]>([]);
	let starting = $state(false);
	let cancelling = $state(false);
	let startError = $state<unknown>(null);
	let pollError = $state<unknown>(null);
	let unavailable = $state(false);

	let onlyRecognised = $state(false);
	let chosenView = $state<'graph' | 'list' | null>(null);
	let phone = $state(false);
	/** The graph needs room: lists by default under 640px, unless the user chose. */
	const view = $derived(chosenView ?? (phone ? 'list' : 'graph'));

	let pollTimer: ReturnType<typeof setTimeout> | undefined;
	let pollController: AbortController | null = null;
	let alive = true;

	onMount(() => {
		const query = window.matchMedia(PHONE_QUERY);
		phone = query.matches;
		const onChange = () => (phone = query.matches);
		query.addEventListener('change', onChange);
		const controller = new AbortController();
		localSubnets(controller.signal)
			.then((list) => {
				subnets = usableSubnets(list);
				const fill = defaultSubnet(cidr, subnets);
				if (fill) {
					cidr = fill;
					prefilled = subnets[0] ?? null;
				}
			})
			.catch(() => {});
		listRelays(controller.signal)
			.then((list) => (relays = list.filter((r) => r.relay)))
			.catch(() => {});
		void refreshHistory(controller.signal, true);
		return () => {
			alive = false;
			query.removeEventListener('change', onChange);
			controller.abort();
			stopPolling();
		};
	});

	async function refreshHistory(signal?: AbortSignal, resume = false) {
		try {
			history = await listDiscoveryJobs(signal);
			// Back on this screen (after adding a service, say): show the scan that
			// still runs, else the latest one, so the results are not lost.
			if (resume && !job) {
				const latest = history.find((j) => j.status === 'running') ?? history[0];
				if (latest) void follow(latest.id);
			}
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			if (cause instanceof ApiError && cause.missing) unavailable = true;
		}
	}

	function stopPolling() {
		clearTimeout(pollTimer);
		pollController?.abort();
		pollController = null;
	}

	/** Shows a job and, while it runs, refreshes it every 2 s. */
	async function follow(id: string) {
		stopPolling();
		const controller = new AbortController();
		pollController = controller;
		pollError = null;
		const tick = async () => {
			try {
				const next = await getDiscoveryJob(id, controller.signal);
				if (controller.signal.aborted || !alive) return;
				job = next;
				if (next.status === 'running') {
					pollTimer = setTimeout(tick, POLL_MS);
				} else {
					void refreshHistory();
				}
			} catch (cause) {
				if (controller.signal.aborted || (cause instanceof DOMException && cause.name === 'AbortError')) return;
				pollError = cause;
				// A hiccup must not lose a running scan: retry unless the job is gone.
				if (!(cause instanceof ApiError && cause.missing)) pollTimer = setTimeout(tick, POLL_MS * 2);
			}
		};
		await tick();
	}

	const cidrValid = $derived(/^\s*[0-9a-f:.]+\/\d{1,3}\s*$/i.test(cidr));
	const running = $derived(job?.status === 'running');

	async function start(event?: SubmitEvent) {
		event?.preventDefault();
		if (!cidrValid || starting || running) return;
		starting = true;
		startError = null;
		try {
			const { id } = await startDiscoveryJob({
				cidr: cidr.trim(),
				agent_id: agentId ? Number(agentId) : null,
				depth,
				...(community.trim() ? { snmp_community: community.trim() } : {})
			});
			job = null;
			await follow(id);
			void refreshHistory();
		} catch (cause) {
			if (cause instanceof ApiError && cause.missing) unavailable = true;
			else startError = cause;
		} finally {
			starting = false;
		}
	}

	async function cancel() {
		if (!job || cancelling) return;
		cancelling = true;
		try {
			await cancelDiscoveryJob(job.id);
			await follow(job.id);
		} catch (cause) {
			pollError = cause;
		} finally {
			cancelling = false;
		}
	}

	const sourceLabel = (source: SubnetSource) =>
		source === 'config'
			? m.deviceform_discovery_source_config()
			: source === 'devices'
				? m.deviceform_discovery_source_devices()
				: m.deviceform_discovery_source_server();
	const helpText = $derived.by(() => {
		if (prefilled && cidr.trim() === prefilled.cidr) {
			return prefilled.source === 'config'
				? m.deviceform_discovery_range_prefilled_config()
				: prefilled.source === 'devices'
					? m.deviceform_discovery_range_prefilled_devices()
					: m.deviceform_discovery_range_prefilled_server();
		}
		return subnets.length === 0 ? m.deviceform_discovery_range_hint() : m.deviceform_scan_range_help();
	});

	const STATUS: Record<DiscoveryJobStatus, { tone: Tone; label: () => string }> = {
		running: { tone: 'info', label: () => m.deviceform_scan_status_running() },
		done: { tone: 'signal', label: () => m.deviceform_scan_status_done() },
		failed: { tone: 'warning', label: () => m.deviceform_scan_status_failed() },
		cancelled: { tone: 'ghost', label: () => m.deviceform_scan_status_cancelled() }
	};
	const phaseLabel = (phase: DiscoveryPhase) =>
		phase === 'sweep'
			? m.deviceform_scan_phase_sweep()
			: phase === 'fingerprint'
				? m.deviceform_scan_phase_fingerprint()
				: m.deviceform_scan_phase_done();

	const shownHosts = $derived(job ? filterHosts(job.hosts, onlyRecognised) : []);
	const totals = $derived(stats(job?.hosts ?? []));
	const pct = $derived(job ? percent(job.progress.probed, job.progress.total) : 0);
</script>

<div class="grid gap-5">
	<form onsubmit={start} class="grid gap-4">
		<div class="grid gap-4 sm:grid-cols-2">
			<Field label={m.deviceform_discovery_range_label()} for="scan-cidr" help={helpText}>
				<input id="scan-cidr" class="input font-mono text-[0.8125rem]" type="text" inputmode="decimal" placeholder="192.168.1.0/24" autocomplete="off" bind:value={cidr} />
			</Field>
			<Field label={m.deviceform_scan_from_label()} for="scan-from" help={m.deviceform_scan_from_help()}>
				<select id="scan-from" class="input" bind:value={agentId}>
					<option value="">{m.deviceform_scan_from_server()}</option>
					{#each relays as relay (relay.id)}
						<option value={String(relay.id)}>{relay.site ? m.deviceform_form_relay_site({ name: relay.name, site: relay.site }) : relay.name}</option>
					{/each}
				</select>
			</Field>
		</div>

		{#if subnets.length > 1}
			<div class="flex flex-wrap items-center gap-2" role="group" aria-label={m.deviceform_discovery_suggestions_label()}>
				<span class="text-[0.8125rem] text-ink-2">{m.deviceform_discovery_suggestions_label()}</span>
				{#each subnets as subnet (subnet.cidr)}
					<Button size="sm" variant={cidr.trim() === subnet.cidr ? 'secondary' : 'ghost'} onclick={() => (cidr = subnet.cidr)}>
						<span class="font-mono">{subnet.cidr}</span>
						<span class="text-ink-2">{sourceLabel(subnet.source)}</span>
					</Button>
				{/each}
			</div>
		{/if}

		<div class="grid gap-4 sm:grid-cols-2 sm:items-start">
			<Field label={m.deviceform_discovery_community_label()} for="scan-community" help={m.deviceform_scan_community_help()}>
				<input id="scan-community" class="input" type="text" autocomplete="off" bind:value={community} placeholder="public" />
			</Field>
			<fieldset class="grid gap-1.5">
				<legend class="mb-1.5 text-sm font-semibold text-ink">{m.deviceform_scan_depth_label()}</legend>
				<div class="grid grid-cols-2 gap-2">
					{#each [['standard', m.deviceform_scan_depth_standard()], ['extended', m.deviceform_scan_depth_extended()]] as const as [value, label] (value)}
						<label class={`flex min-h-10 cursor-pointer items-center justify-center rounded-lg border px-3 text-sm font-semibold transition-colors has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-signal ${depth === value ? 'border-signal/50 bg-signal-soft text-signal-ink' : 'border-line bg-surface text-ink-2 hover:text-ink'}`}>
							<input type="radio" name="scan-depth" class="sr-only" {value} bind:group={depth} />
							{label}
						</label>
					{/each}
				</div>
				<p class="text-[0.8125rem] text-ink-2 max-sm:hidden">{depth === 'standard' ? m.deviceform_scan_depth_help_standard() : m.deviceform_scan_depth_help_extended()}</p>
			</fieldset>
		</div>

		<div class="flex flex-wrap items-center gap-2">
			<Button type="submit" variant="primary" loading={starting} disabled={!cidrValid || running} class="max-sm:w-full">
				<Radar class="size-4" aria-hidden="true" />
				{m.deviceform_scan_start()}
			</Button>
			{#if running}
				<Button variant="secondary" loading={cancelling} onclick={cancel} class="max-sm:w-full">
					<Square class="size-4" aria-hidden="true" />
					{m.deviceform_scan_cancel()}
				</Button>
			{/if}
		</div>
	</form>

	<div aria-live="polite" class="grid gap-4">
		{#if unavailable}
			<EmptyState title={m.deviceform_discovery_unavailable_title()} description={m.deviceform_discovery_unavailable_desc()} />
		{:else}
			{#if startError}
				<ErrorNotice error={startError} title={m.deviceform_discovery_failed_title()} />
			{/if}
			{#if pollError}
				<ErrorNotice error={pollError} title={m.deviceform_scan_poll_error()} />
			{/if}

			{#if job}
				{@const status = STATUS[job.status]}
				<div class="grid gap-3 rounded-[var(--radius-card)] border border-line bg-surface px-4 py-3">
					<div class="flex flex-wrap items-center justify-between gap-2">
						<p class="min-w-0 truncate text-sm text-ink">
							<span class="font-mono font-semibold">{job.cidr}</span>
							<span class="text-ink-2"> · {job.agent_name ?? m.deviceform_scan_from_server()}</span>
						</p>
						<Plate tone={status.tone} label={status.label()} pulse={running} />
					</div>
					{#if running || job.progress.total > 0}
						<div class="grid gap-1.5">
							<div class="h-2 overflow-hidden rounded-full bg-ghost" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct} aria-label={phaseLabel(job.progress.phase)}>
								<div class="h-full rounded-full bg-signal transition-[width] duration-500 ease-out" style={`width:${pct}%`}></div>
							</div>
							<p class="flex flex-wrap justify-between gap-x-3 text-[0.8125rem] text-ink-2">
								<span>{phaseLabel(job.progress.phase)}</span>
								<span class="tnum">{m.deviceform_scan_progress({ probed: job.progress.probed, total: job.progress.total })}</span>
							</p>
						</div>
					{/if}
					{#if job.error}
						<p class="rounded-lg border border-warning/35 bg-warning-soft px-3 py-2 text-sm text-warning-ink" role="alert">{job.error}</p>
					{/if}
				</div>

				{#if job.hosts.length > 0}
					<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
						<p class="tnum text-sm text-ink-2">
							{m.deviceform_scan_stat_hosts({ count: totals.hosts })} · {m.deviceform_scan_stat_services({ count: totals.services })} · {m.deviceform_scan_stat_recognised({ count: totals.recognised })}
						</p>
						<div class="flex flex-wrap items-center gap-3">
							<span class="flex min-h-10 items-center gap-2 text-sm text-ink">
								<Toggle id="scan-only-recognised" label={m.deviceform_scan_only_recognised()} bind:checked={onlyRecognised} />
								<label for="scan-only-recognised" class="cursor-pointer">{m.deviceform_scan_only_recognised()}</label>
							</span>
							<div class="flex gap-1" role="group" aria-label={m.deviceform_scan_view_label()}>
								<Button size="sm" variant={view === 'graph' ? 'secondary' : 'ghost'} aria-pressed={view === 'graph'} onclick={() => (chosenView = 'graph')}>
									<Network class="size-4" aria-hidden="true" />
									{m.deviceform_scan_view_graph()}
								</Button>
								<Button size="sm" variant={view === 'list' ? 'secondary' : 'ghost'} aria-pressed={view === 'list'} onclick={() => (chosenView = 'list')}>
									<LayoutList class="size-4" aria-hidden="true" />
									{m.deviceform_scan_view_list()}
								</Button>
							</div>
						</div>
					</div>

					{#if shownHosts.length === 0}
						<EmptyState title={m.deviceform_scan_filtered_title()} description={m.deviceform_scan_filtered_desc()} />
					{:else if view === 'graph'}
						<DiscoveryGraph hosts={shownHosts} cidr={job.cidr} />
					{:else}
						<DiscoveryList hosts={shownHosts} />
					{/if}
				{:else if !running && job.status !== 'failed'}
					<EmptyState title={m.deviceform_discovery_none_title({ cidr: job.cidr })} description={m.deviceform_scan_none_desc()} />
				{/if}
			{:else}
				<p class="text-sm leading-relaxed text-ink-2">{m.deviceform_scan_intro()}</p>
			{/if}
		{/if}
	</div>

	{#if history.length > 0}
		<section aria-labelledby="scan-history" class="grid gap-2">
			<h3 id="scan-history" class="text-sm font-semibold text-ink">{m.deviceform_scan_history()}</h3>
			<ul class="divide-y divide-line overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface">
				{#each history as past (past.id)}
					{@const status = STATUS[past.status]}
					<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2 max-sm:py-2.5">
						<div class="min-w-0 flex-1">
							<p class="truncate text-sm text-ink"><span class="font-mono font-semibold">{past.cidr}</span> <span class="text-ink-2">· {past.agent_name ?? m.deviceform_scan_from_server()}</span></p>
							<p class="text-[0.75rem] text-ink-2"><span class="tnum">{m.deviceform_scan_stat_hosts({ count: past.hosts_found })}</span> · {formatRelative(past.started_at)}</p>
						</div>
						<Plate tone={status.tone} label={status.label()} />
						<Button size="sm" variant="ghost" aria-current={job?.id === past.id} onclick={() => follow(past.id)}>{m.deviceform_scan_history_show()}</Button>
					</li>
				{/each}
			</ul>
		</section>
	{/if}
</div>
