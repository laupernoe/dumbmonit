<script lang="ts">
	/**
	 * Guests of a Proxmox VE device: one row per VM or container with what an
	 * administrator looks for first — is it running, is it busy, is its disk
	 * full, when was it last backed up. Grouped by node (each group folds),
	 * sortable by any column, filtered by node, pool and status. A row unfolds
	 * into its CPU and memory over the page's time range, its operating system
	 * and its address.
	 *
	 * The rows come from `GET /api/targets/{id}/proxmox/guests`, assembled
	 * server-side from the last probe; the sparklines are two range queries
	 * made only once a row is open. Because that list is built from the
	 * cluster-wide inventory, the guests of a node that stopped answering are
	 * still here — as "unknown", which is exactly what they are.
	 */
	import { page } from '$app/state';
	import { queryRange, type MetricSeries, type ProxmoxGuest, type Target } from '#lib/api/index.js';
	import { listProxmoxGuests } from '#lib/api/proxmox.js';
	import { formatDuration } from '#lib/format.js';
	import { RANGES, type RangeId } from '#lib/metrics.js';
	import { EmptyState, ErrorNotice, Led, Plate, Skeleton, type Tone } from '#lib/ui/index.js';
	import Chart, { type Serie } from '#lib/components/Chart.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import FoldSection from '../FoldSection.svelte';
	import Segmented from '../Segmented.svelte';
	import { formatRate } from '../metrics';
	import { FILL, fillTone, formatBytes, formatPercent } from './format';
	import { ChevronRight, ArrowUp, ArrowDown } from 'lucide-svelte';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	// --- Loading ------------------------------------------------------------------

	let guests = $state<ProxmoxGuest[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		try {
			guests = await listProxmoxGuests(target.id, signal);
			error = null;
		} catch (cause) {
			if (signal?.aborted) return;
			error = cause;
		} finally {
			if (!signal?.aborted) loading = false;
		}
	}

	// First load, then a refresh at the cadence the probes write at.
	$effect(() => {
		const controller = new AbortController();
		loading = true;
		void load(controller.signal);
		const timer = setInterval(() => void load(controller.signal), 30_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	// --- Filters and sort ---------------------------------------------------------

	type StatusFilter = 'all' | 'running' | 'stopped' | 'other';
	let nodeFilter = $state<string>('all');
	let statusFilter = $state<StatusFilter>('all');
	let poolFilter = $state<string>('all');

	type SortKey = 'vmid' | 'name' | 'status' | 'cpu' | 'memory' | 'disk' | 'net' | 'uptime' | 'backup';
	let sortKey = $state<SortKey>('vmid');
	let sortAsc = $state(true);

	function setSort(key: SortKey) {
		if (sortKey === key) sortAsc = !sortAsc;
		else {
			sortKey = key;
			// Numbers read best largest first; names and ids ascending.
			sortAsc = key === 'vmid' || key === 'name' || key === 'status';
		}
	}

	const nodes = $derived([...new Set(guests.map((g) => g.node))].sort((a, b) => a.localeCompare(b, 'en')));

	const nodeOptions = $derived([
		{ id: 'all', label: m.devicesb_proxmox_guests_all_nodes(), count: guests.length },
		...nodes.map((node) => ({ id: node, label: node, count: guests.filter((g) => g.node === node).length }))
	]);

	// Pools are how most clusters are actually carved up (per customer, per
	// service); the filter only appears once there is more than one.
	const pools = $derived([...new Set(guests.map((g) => g.pool).filter((p): p is string => !!p))].sort((a, b) => a.localeCompare(b, 'en')));
	const poolOptions = $derived([
		{ id: 'all', label: m.devicesb_proxmox_guests_all_pools(), count: guests.length },
		...pools.map((pool) => ({ id: pool, label: pool, count: guests.filter((g) => g.pool === pool).length }))
	]);

	/**
	 * The host-side memory of a guest, when it says something the guest's own
	 * figure does not. A QEMU process occupies more than the memory it hands to
	 * the guest — emulation, device models, page tables — and that surplus is
	 * what the hypervisor actually pays for. Below 64 MiB or 5 % it is noise,
	 * and the column stays as it was.
	 */
	function hostOverhead(g: ProxmoxGuest): number | null {
		const host = g.memory_host_bytes;
		const used = g.memory_used_bytes;
		if (host === null || used === null) return null;
		const extra = host - used;
		if (extra < 64 * 1024 * 1024 || extra < used * 0.05) return null;
		return extra;
	}

	function statusBucket(g: ProxmoxGuest): StatusFilter {
		if (g.status === 'running') return 'running';
		if (g.status === 'stopped') return 'stopped';
		return 'other';
	}
	const statusOptions = $derived.by(() => {
		const count = (bucket: StatusFilter) => guests.filter((g) => statusBucket(g) === bucket).length;
		const options: { id: StatusFilter; label: string; count?: number }[] = [
			{ id: 'all', label: m.devicesb_proxmox_guests_filter_all() },
			{ id: 'running', label: m.devicesb_proxmox_guests_running(), count: count('running') },
			{ id: 'stopped', label: m.devicesb_proxmox_guests_stopped(), count: count('stopped') }
		];
		if (count('other') > 0) options.push({ id: 'other', label: m.devicesb_proxmox_guests_other(), count: count('other') });
		return options;
	});

	/** Sort value of a row for a column; `null` sorts last whichever the direction. */
	function sortValue(g: ProxmoxGuest, key: SortKey): number | string | null {
		switch (key) {
			case 'vmid':
				return g.vmid;
			case 'name':
				return g.name.toLowerCase();
			case 'status':
				return g.status;
			case 'cpu':
				return g.cpu_percent;
			case 'memory':
				return g.memory_percent;
			case 'disk':
				return g.disk_percent;
			case 'net':
				return g.network_in_bps === null && g.network_out_bps === null
					? null
					: (g.network_in_bps ?? 0) + (g.network_out_bps ?? 0);
			case 'uptime':
				return g.uptime_seconds;
			case 'backup':
				return g.last_backup_age_seconds;
		}
	}

	function compare(a: ProxmoxGuest, b: ProxmoxGuest): number {
		const va = sortValue(a, sortKey);
		const vb = sortValue(b, sortKey);
		if (va === null && vb === null) return a.vmid - b.vmid;
		if (va === null) return 1;
		if (vb === null) return -1;
		const order = typeof va === 'string' && typeof vb === 'string' ? va.localeCompare(vb, 'en') : Number(va) - Number(vb);
		return (sortAsc ? order : -order) || a.vmid - b.vmid;
	}

	const shown = $derived(
		guests
			.filter((g) => nodeFilter === 'all' || g.node === nodeFilter)
			.filter((g) => poolFilter === 'all' || g.pool === poolFilter)
			.filter((g) => statusFilter === 'all' || statusBucket(g) === statusFilter)
			.sort(compare)
	);

	/** Rows per node, nodes in alphabetical order, rows in the chosen sort. */
	const groups = $derived.by(() => {
		const byNode = new Map<string, ProxmoxGuest[]>();
		for (const g of shown) {
			const list = byNode.get(g.node) ?? [];
			list.push(g);
			byNode.set(g.node, list);
		}
		return [...byNode.entries()].sort(([a], [b]) => a.localeCompare(b, 'en'));
	});

	/** Node groups the user folded; everything starts open. */
	let closedNodes = $state<Record<string, boolean>>({});
	/** The row whose detail is open, by vmid. */
	let openRow = $state<number | null>(null);

	// --- Presentation -------------------------------------------------------------

	const summary = $derived.by(() => {
		if (loading || error || guests.length === 0) return undefined;
		const running = guests.filter((g) => g.status === 'running').length;
		const stopped = guests.filter((g) => g.status === 'stopped').length;
		const parts = [`${guests.length}`, m.devicesb_proxmox_guests_summary_running({ count: running })];
		if (stopped > 0) parts.push(m.devicesb_proxmox_guests_summary_stopped({ count: stopped }));
		const other = guests.length - running - stopped;
		if (other > 0) parts.push(m.devicesb_proxmox_guests_summary_other({ count: other }));
		return parts.join(' · ');
	});

	interface StatusLook {
		tone: 'signal' | 'advisory' | 'warning' | 'ghost' | 'info';
		word: string;
		blink: boolean;
	}
	function statusLook(g: ProxmoxGuest): StatusLook {
		switch (g.status) {
			case 'running':
				return { tone: 'signal', word: m.devicesb_proxmox_guests_running(), blink: false };
			case 'stopped':
				return { tone: 'warning', word: m.devicesb_proxmox_guests_stopped(), blink: false };
			case 'paused':
				return { tone: 'advisory', word: m.devicesb_proxmox_guests_paused(), blink: false };
			case 'suspended':
				return { tone: 'advisory', word: m.devicesb_proxmox_guests_suspended(), blink: false };
			case 'template':
				return { tone: 'ghost', word: m.devicesb_proxmox_guests_template(), blink: false };
			default:
				return { tone: 'ghost', word: m.devicesb_proxmox_guests_unknown(), blink: false };
		}
	}

	function haTone(state: string): Tone {
		if (state === 'started') return 'signal';
		if (state === 'error' || state === 'fence' || state === 'recovery') return 'warning';
		if (state === 'stopped' || state === 'disabled' || state === 'ignored') return 'muted';
		return 'info';
	}

	/** "2.1 d" for a backup age; "never" when nothing is known. */
	function backupWord(g: ProxmoxGuest): { text: string; tone: 'ink' | 'advisory' | 'warning' } {
		if (g.status === 'template') return { text: '—', tone: 'ink' };
		if (g.last_backup_age_seconds === null) return { text: m.devicesb_proxmox_guests_backup_none(), tone: 'warning' };
		const age = g.last_backup_age_seconds;
		return {
			text: m.devicesb_proxmox_guests_backup_ago({ age: formatDuration(age) }),
			tone: age > 7 * 86400 ? 'warning' : age > 2 * 86400 ? 'advisory' : 'ink'
		};
	}

	const BACKUP_TONE = { ink: 'text-ink', advisory: 'text-advisory-ink', warning: 'text-warning-ink' };

	/** What the disk cell says when usage is unknown. */
	function diskNote(g: ProxmoxGuest): string {
		if (g.kind !== 'qemu' || g.status !== 'running') return m.devicesb_proxmox_guests_size_only();
		if (g.agent === false) return m.devicesb_proxmox_guests_size_only_agent_silent();
		return m.devicesb_proxmox_guests_size_only_needs_agent();
	}

	const COLUMNS: { key: SortKey; label: () => string; align: 'left' | 'right'; class?: string }[] = [
		{ key: 'status', label: () => m.devicesb_proxmox_guests_col_status(), align: 'left' },
		{ key: 'name', label: () => m.devicesb_proxmox_guests_col_guest(), align: 'left' },
		{ key: 'cpu', label: () => m.devicesb_proxmox_guests_col_cpu(), align: 'left', class: 'w-28' },
		{ key: 'memory', label: () => m.devicesb_proxmox_guests_col_memory(), align: 'left', class: 'w-40' },
		{ key: 'disk', label: () => m.devicesb_proxmox_guests_col_disk(), align: 'left', class: 'w-44' },
		{ key: 'net', label: () => m.devicesb_proxmox_guests_col_network(), align: 'right' },
		{ key: 'uptime', label: () => m.devicesb_proxmox_guests_col_uptime(), align: 'right' },
		{ key: 'backup', label: () => m.devicesb_proxmox_guests_col_backup(), align: 'right' }
	];

	// --- Row detail: sparklines over the page's range ---------------------------

	const range = $derived.by((): RangeId => {
		const raw = page.url.searchParams.get('range');
		return RANGES.some((r) => r.id === raw) ? (raw as RangeId) : '1h';
	});
	const rangeSeconds = $derived(RANGES.find((r) => r.id === range)?.seconds ?? 3600);
	const rangeLabel = $derived(RANGES.find((r) => r.id === range)?.label ?? '1 hour');

	interface Detail {
		cpu: Serie[];
		memory: Serie[];
		network: Serie[];
	}
	let detail = $state<Detail | null>(null);
	let detailLoading = $state(false);
	let detailError = $state<unknown>(null);

	function toSerie(label: string, series: MetricSeries[]): Serie[] {
		return series.map((s) => ({
			label,
			points: (s.values ?? [])
				.map(([ts, raw]): [number, number] => [ts, Number(raw)])
				.filter(([, v]) => Number.isFinite(v))
				.sort((a, b) => a[0] - b[0])
		}));
	}

	async function loadDetail(vmid: number, signal: AbortSignal) {
		detailLoading = true;
		detailError = null;
		const end = Date.now();
		const start = end - rangeSeconds * 1000;
		const step = Math.max(10, Math.round(rangeSeconds / 300));
		const selector = `target="${target.id}", vmid="${vmid}"`;
		const window = Math.max(120, step * 2);
		try {
			const [cpu, memory, netIn, netOut] = await Promise.all([
				queryRange({ query: `dumbmonit_proxmox_guest_cpu_percent{${selector}}`, start, end, step }, signal),
				queryRange({ query: `dumbmonit_proxmox_guest_memory_percent{${selector}}`, start, end, step }, signal),
				queryRange({ query: `rate(dumbmonit_proxmox_guest_network_in_bytes{${selector}}[${window}s])`, start, end, step }, signal),
				queryRange({ query: `rate(dumbmonit_proxmox_guest_network_out_bytes{${selector}}[${window}s])`, start, end, step }, signal)
			]);
			if (signal.aborted) return;
			detail = {
				cpu: toSerie(m.devicesb_proxmox_guests_col_cpu(), cpu),
				memory: toSerie(m.devicesb_proxmox_guests_col_memory(), memory),
				network: [...toSerie(m.devicesb_proxmox_guests_net_in(), netIn), ...toSerie(m.devicesb_proxmox_guests_net_out(), netOut)]
			};
		} catch (cause) {
			if (!signal.aborted) detailError = cause;
		} finally {
			if (!signal.aborted) detailLoading = false;
		}
	}

	$effect(() => {
		const vmid = openRow;
		if (vmid === null) {
			detail = null;
			return;
		}
		const controller = new AbortController();
		void loadDetail(vmid, controller.signal);
		return () => controller.abort();
	});

	function toggleRow(vmid: number) {
		openRow = openRow === vmid ? null : vmid;
	}
</script>

<FoldSection kind="proxmox-guests" title={m.devicesb_proxmox_guests_title()} {summary} defaultOpen={true} class="rise-in">
	{#snippet aside()}
		{#if !loading && !error && guests.length > 0}
			<div class="flex flex-wrap items-center gap-2">
				{#if nodes.length > 1}
					<Segmented options={nodeOptions} value={nodeFilter} onchange={(v) => (nodeFilter = v)} label={m.devicesb_proxmox_guests_filter_node()} size="sm" />
				{/if}
				{#if pools.length > 1}
					<Segmented options={poolOptions} value={poolFilter} onchange={(v) => (poolFilter = v)} label={m.devicesb_proxmox_guests_filter_pool()} size="sm" />
				{/if}
				<Segmented options={statusOptions} value={statusFilter} onchange={(v) => (statusFilter = v)} label={m.devicesb_proxmox_guests_col_status()} size="sm" />
			</div>
		{/if}
	{/snippet}

	{#if error}
		<div class="px-5 py-4">
			<ErrorNotice {error} title={m.devicesb_proxmox_guests_load_error()} onretry={() => void load()} />
		</div>
	{:else if loading}
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label={m.devicesb_proxmox_guests_loading()}>
			<Skeleton class="h-10 w-full" rows={4} />
		</div>
	{:else if guests.length === 0}
		<div class="px-5 py-4">
			<EmptyState title={m.devicesb_proxmox_guests_empty_title()} description={m.devicesb_proxmox_guests_empty_desc()} />
		</div>
	{:else if shown.length === 0}
		<p class="px-5 py-4 text-sm text-ink-2">{m.devicesb_proxmox_guests_no_match()}</p>
	{:else}
		<div class="overflow-x-auto">
			<table class="w-full min-w-[56rem] border-collapse text-sm">
				<thead>
					<tr class="border-b border-line text-[0.75rem] uppercase tracking-wide text-ink-3">
						{#each COLUMNS as column (column.key)}
							{@const active = sortKey === column.key}
							<th scope="col" class={`px-3 py-2 font-semibold first:pl-5 last:pr-5 ${column.align === 'right' ? 'text-right' : 'text-left'} ${column.class ?? ''}`} aria-sort={active ? (sortAsc ? 'ascending' : 'descending') : 'none'}>
								<button type="button" class={`inline-flex min-h-10 items-center gap-1 rounded hover:text-ink ${active ? 'text-ink' : ''}`} onclick={() => setSort(column.key)}>
									{column.label()}
									{#if active}
										{#if sortAsc}<ArrowUp class="size-3" aria-hidden="true" />{:else}<ArrowDown class="size-3" aria-hidden="true" />{/if}
									{/if}
								</button>
							</th>
						{/each}
						<th scope="col" class="px-3 py-2 pr-5 text-right font-semibold">{m.devicesb_proxmox_guests_col_ha()}</th>
					</tr>
				</thead>
				{#each groups as [node, rows] (node)}
					{@const closed = closedNodes[node] ?? false}
					{@const running = rows.filter((g) => g.status === 'running').length}
					<tbody class="border-b border-line last:border-b-0">
						<tr class="bg-surface-2/60">
							<th scope="rowgroup" colspan={COLUMNS.length + 1} class="px-5 py-1.5 text-left">
								<button type="button" class="inline-flex min-h-10 items-center gap-2 text-[0.8125rem] font-semibold text-ink hover:text-signal-ink" aria-expanded={!closed} onclick={() => (closedNodes = { ...closedNodes, [node]: !closed })}>
									<ChevronRight class={`size-3.5 shrink-0 text-ink-3 transition-transform duration-200 ease-out-expo ${closed ? '' : 'rotate-90'}`} aria-hidden="true" />
									<span>{node}</span>
									<span class="tnum font-normal text-ink-2">{rows.length === 1 ? m.devicesb_proxmox_guests_node_summary_one({ count: rows.length, running }) : m.devicesb_proxmox_guests_node_summary_other({ count: rows.length, running })}</span>
								</button>
							</th>
						</tr>
						{#if !closed}
							{#each rows as g, i (g.vmid)}
								{@const look = statusLook(g)}
								{@const open = openRow === g.vmid}
								{@const backup = backupWord(g)}
								<tr class={`rise-in border-t border-line/60 ${open ? 'bg-surface-2/40' : 'hover:bg-surface-2/40'}`} style="--rise-delay: {Math.min(i, 8) * 30}ms">
									<td class="px-3 py-2 pl-5 align-middle">
										<button type="button" class="inline-flex min-h-10 items-center gap-2 whitespace-nowrap text-left" aria-expanded={open} aria-controls={`guest-${target.id}-${g.vmid}`} onclick={() => toggleRow(g.vmid)}>
											<ChevronRight class={`size-3.5 shrink-0 text-ink-3 transition-transform duration-200 ease-out-expo ${open ? 'rotate-90' : ''}`} aria-hidden="true" />
											<Led tone={look.tone} blink={look.blink} label={look.word} size="sm" />
											<Plate tone={look.tone} label={look.word} bare size="sm" />
										</button>
									</td>
									<td class="px-3 py-2 align-middle">
										<button type="button" class="flex min-h-10 min-w-0 flex-col justify-center text-left" onclick={() => toggleRow(g.vmid)}>
											<span class="flex min-w-0 items-center gap-1.5">
												<span class="truncate font-semibold text-ink">{g.name}</span>
												{#if g.lock}<Plate tone="advisory" label={m.devicesb_proxmox_guests_locked({ lock: g.lock })} bare size="sm" />{/if}
											</span>
											<span class="tnum text-[0.75rem] text-ink-2">
												{g.kind === 'lxc' ? m.devicesb_proxmox_guests_kind_ct({ vmid: g.vmid }) : m.devicesb_proxmox_guests_kind_vm({ vmid: g.vmid })}{#if g.pool}{` · ${g.pool}`}{/if}
											</span>
										</button>
									</td>
									<td class="px-3 py-2 align-middle">
										{#if g.cpu_percent !== null}
											{@const tone = fillTone(g.cpu_percent)}
											<div class="flex flex-col gap-1">
												<span class="tnum text-ink">{formatPercent(g.cpu_percent)}<span class="text-ink-3">{' '}{g.cpu_count === 1 ? m.devicesb_proxmox_guests_cpu_of_one({ count: g.cpu_count }) : m.devicesb_proxmox_guests_cpu_of_other({ count: g.cpu_count ?? '?' })}</span></span>
												<div class="h-1.5 w-full overflow-hidden rounded-full bg-surface-2" role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(g.cpu_percent)} aria-label={m.devicesb_proxmox_guests_col_cpu()}>
													<div class={`h-full rounded-full ${FILL[tone]}`} style="width: {Math.min(100, g.cpu_percent)}%"></div>
												</div>
											</div>
										{:else}
											<span class="text-ink-3">—{#if g.cpu_count !== null}<span class="text-[0.75rem]">{' '}· {g.cpu_count === 1 ? m.devicesb_proxmox_guests_cores_one({ count: g.cpu_count }) : m.devicesb_proxmox_guests_cores_other({ count: g.cpu_count })}</span>{/if}</span>
										{/if}
									</td>
									<td class="px-3 py-2 align-middle">
										{#if g.memory_used_bytes !== null && g.memory_total_bytes !== null}
											{@const tone = fillTone(g.memory_percent)}
											{@const overhead = hostOverhead(g)}
											<div class="flex flex-col gap-1">
												<span class="tnum whitespace-nowrap text-ink">{formatBytes(g.memory_used_bytes)}<span class="text-ink-3">{' '}/ {formatBytes(g.memory_total_bytes)}</span></span>
												<div class="h-1.5 w-full overflow-hidden rounded-full bg-surface-2" role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(g.memory_percent ?? 0)} aria-label={m.devicesb_proxmox_guests_col_memory()}>
													<div class={`h-full rounded-full ${FILL[tone]}`} style="width: {Math.min(100, g.memory_percent ?? 0)}%"></div>
												</div>
												{#if overhead !== null}
													<span
														class="tnum whitespace-nowrap text-[0.75rem] text-ink-3"
														title={m.devicesb_proxmox_guests_overhead_title({ host: formatBytes(g.memory_host_bytes), extra: formatBytes(overhead) })}
													>
														{m.devicesb_proxmox_guests_overhead({ extra: formatBytes(overhead) })}
													</span>
												{/if}
											</div>
										{:else}
											<span class="tnum whitespace-nowrap text-ink-3">{g.memory_total_bytes !== null ? `— / ${formatBytes(g.memory_total_bytes)}` : '—'}</span>
										{/if}
									</td>
									<td class="px-3 py-2 align-middle">
										{#if g.disk_used_bytes !== null && g.disk_total_bytes !== null}
											{@const tone = fillTone(g.disk_percent)}
											<div class="flex flex-col gap-1">
												<span class="tnum whitespace-nowrap text-ink">{formatBytes(g.disk_used_bytes)}<span class="text-ink-3">{' '}/ {formatBytes(g.disk_total_bytes)}</span> <span class="text-ink-2">{formatPercent(g.disk_percent)}</span></span>
												<div class="h-1.5 w-full overflow-hidden rounded-full bg-surface-2" role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(g.disk_percent ?? 0)} aria-label={m.devicesb_proxmox_guests_col_disk()}>
													<div class={`h-full rounded-full ${FILL[tone]}`} style="width: {Math.min(100, g.disk_percent ?? 0)}%"></div>
												</div>
											</div>
										{:else}
											<div class="flex flex-col">
												<span class="tnum whitespace-nowrap text-ink">{formatBytes(g.disk_total_bytes)}</span>
												<span class="text-[0.75rem] text-ink-3">{diskNote(g)}</span>
											</div>
										{/if}
									</td>
									<td class="tnum whitespace-nowrap px-3 py-2 text-right align-middle text-ink-2">
										{#if g.network_in_bps !== null || g.network_out_bps !== null}
											↓ {formatRate(g.network_in_bps, 'B/s')}<br />↑ {formatRate(g.network_out_bps, 'B/s')}
										{:else}
											—
										{/if}
									</td>
									<td class="tnum whitespace-nowrap px-3 py-2 text-right align-middle text-ink-2">
										{g.uptime_seconds !== null ? formatDuration(g.uptime_seconds) : '—'}
									</td>
									<td class={`tnum whitespace-nowrap px-3 py-2 text-right align-middle ${BACKUP_TONE[backup.tone]}`}>{backup.text}</td>
									<td class="whitespace-nowrap px-3 py-2 pr-5 text-right align-middle">
										{#if g.ha_state}
											<Plate tone={haTone(g.ha_state)} label={g.ha_state} bare size="sm" />
										{:else}
											<span class="text-ink-3">—</span>
										{/if}
									</td>
								</tr>
								{#if open}
									<tr id={`guest-${target.id}-${g.vmid}`} class="bg-surface-2/40">
										<td colspan={COLUMNS.length + 1} class="px-5 pt-1 pb-4">
											<div class="flex flex-wrap items-baseline justify-between gap-2">
												<p class="text-sm text-ink-2">
													<span class="font-semibold text-ink">{g.name}</span> · {g.kind === 'lxc'
														? m.devicesb_proxmox_guests_detail_ct({ vmid: g.vmid, node: g.node })
														: m.devicesb_proxmox_guests_detail_vm({ vmid: g.vmid, node: g.node })}
													{#if g.pool}{` · ${m.devicesb_proxmox_guests_detail_pool({ pool: g.pool })}`}{/if}
													{#if g.os}{' '}· {g.os}{/if}
													{#if g.ip}{' '}· {g.ip}{/if}
													{#if g.lock}{' '}· {m.devicesb_proxmox_guests_detail_locked({ lock: g.lock })}{/if}
													{#if g.balloon_bytes !== null}{' '}· {m.devicesb_proxmox_guests_detail_balloon({ size: formatBytes(g.balloon_bytes) })}{/if}
													{#if g.memory_host_bytes !== null}
														{' '}· {hostOverhead(g) !== null
															? m.devicesb_proxmox_guests_detail_host_extra({ host: formatBytes(g.memory_host_bytes), extra: formatBytes(hostOverhead(g)) })
															: m.devicesb_proxmox_guests_detail_host({ host: formatBytes(g.memory_host_bytes) })}
													{/if}
													{#if g.kind === 'qemu' && g.status === 'running'}
														{' '}· {g.agent === true
															? m.devicesb_proxmox_guests_detail_agent_ok()
															: g.agent === false
																? m.devicesb_proxmox_guests_detail_agent_silent()
																: m.devicesb_proxmox_guests_detail_agent_off()}
													{/if}
													{#if g.disk_read_bps !== null || g.disk_write_bps !== null}
														{' '}· {m.devicesb_proxmox_guests_detail_disk_io({ read: formatRate(g.disk_read_bps, 'B/s'), write: formatRate(g.disk_write_bps, 'B/s') })}
													{/if}
												</p>
												<span class="text-[0.75rem] text-ink-3">{m.devicesb_proxmox_guests_detail_range({ range: rangeLabel })}</span>
											</div>
											{#if detailError}
												<div class="mt-2"><ErrorNotice error={detailError} title={m.devicesb_proxmox_guests_charts_error()} /></div>
											{:else if detailLoading && !detail}
												<Skeleton class="mt-2 h-28 w-full rounded-[var(--radius-card)]" />
											{:else if detail}
												<div class="mt-2 grid gap-3 md:grid-cols-3">
													{#each [[m.devicesb_proxmox_guests_col_cpu(), detail.cpu, '%'], [m.devicesb_proxmox_guests_col_memory(), detail.memory, '%'], [m.devicesb_proxmox_guests_col_network(), detail.network, 'B/s']] as [title, series, unit] (title)}
														<div class="rounded-lg border border-line bg-surface px-3 pt-2 pb-1">
															<div class="flex items-center justify-between gap-2">
																<span class="text-sm font-semibold text-ink">{title}</span>
																<span class="label-tape">{unit}</span>
															</div>
															{#if (series as Serie[]).some((s) => s.points.length > 0)}
																<Chart series={series as Serie[]} unit={unit as string} height={120} />
															{:else}
																<p class="py-6 text-center text-sm text-ink-3">{m.devicesb_proxmox_guests_no_points()}</p>
															{/if}
														</div>
													{/each}
												</div>
											{/if}
										</td>
									</tr>
								{/if}
							{/each}
						{/if}
					</tbody>
				{/each}
			</table>
		</div>
	{/if}
</FoldSection>
