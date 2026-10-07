<script lang="ts">
	/**
	 * A Kubernetes cluster as its last probe left it: one sentence saying what
	 * is wrong, the counts, then only what needs attention — nodes not ready or
	 * under pressure, pods crash looping, pending, blocked or not ready,
	 * workloads missing replicas, claims that never got bound, and the warning
	 * events of the last hour by reason. A healthy cluster shows its counts and
	 * its nodes, nothing else.
	 *
	 * Everything is read from the stored `dumbmonit_k8s_*` series: opening the
	 * page never calls the API server.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, type Tone } from '#lib/ui/index.js';
	import Figure from '../Figure.svelte';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	interface NodeStat {
		name: string;
		ready: boolean | null;
		pressures: string[];
		cordoned: boolean;
		kubelet: string | null;
	}

	interface PodStat {
		namespace: string;
		pod: string;
		workload: string;
		ready: boolean | null;
		crashlooping: boolean;
		pending: boolean;
		blocked: string | null;
		restarts: number | null;
	}

	interface WorkloadStat {
		kind: string;
		namespace: string;
		name: string;
		desired: number;
		ready: number;
		unavailable: number;
	}

	interface Cluster {
		version: string | null;
		totals: Record<string, number>;
		nodes: NodeStat[];
		pods: PodStat[];
		workloads: WorkloadStat[];
		stuckClaims: { namespace: string; pvc: string }[];
		reasons: { reason: string; count: number }[];
	}

	let cluster = $state<Cluster | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	function fold(series: { metric: Record<string, string>; values: [number, string][] }[]): Cluster {
		const out: Cluster = { version: null, totals: {}, nodes: [], pods: [], workloads: [], stuckClaims: [], reasons: [] };
		const nodes = new Map<string, NodeStat>();
		const pods = new Map<string, PodStat>();
		const workloads = new Map<string, WorkloadStat>();
		const node = (name: string) => {
			let n = nodes.get(name);
			if (!n) {
				n = { name, ready: null, pressures: [], cordoned: false, kubelet: null };
				nodes.set(name, n);
			}
			return n;
		};
		const pod = (m: Record<string, string>) => {
			const key = `${m.namespace}/${m.pod}`;
			let p = pods.get(key);
			if (!p) {
				p = { namespace: m.namespace ?? '', pod: m.pod ?? '', workload: m.workload ?? '', ready: null, crashlooping: false, pending: false, blocked: null, restarts: null };
				pods.set(key, p);
			}
			return p;
		};
		const workload = (m: Record<string, string>) => {
			const key = `${m.kind}/${m.namespace}/${m.workload}`;
			let w = workloads.get(key);
			if (!w) {
				w = { kind: m.kind ?? '', namespace: m.namespace ?? '', name: m.workload ?? '', desired: 0, ready: 0, unavailable: 0 };
				workloads.set(key, w);
			}
			return w;
		};
		for (const serie of series) {
			const m = serie.metric;
			const name = (m.__name__ ?? '').replace(/^dumbmonit_k8s_/, '');
			const value = Number(serie.values.at(-1)?.[1]);
			if (!Number.isFinite(value)) continue;
			switch (name) {
				case 'version_info':
					out.version = m.version ?? null;
					break;
				case 'node_ready':
					node(m.node).ready = value >= 1;
					break;
				case 'node_pressure':
					if (value >= 1) node(m.node).pressures.push(m.condition);
					break;
				case 'node_unschedulable':
					node(m.node).cordoned = value >= 1;
					break;
				case 'node_info':
					node(m.node).kubelet = m.kubelet_version ?? null;
					break;
				case 'pod_ready':
					pod(m).ready = value >= 1;
					break;
				case 'pod_crashlooping':
					pod(m).crashlooping = value >= 1;
					break;
				case 'pod_pending':
					pod(m).pending = value >= 1;
					break;
				case 'pod_blocked':
					if (value >= 1) pod(m).blocked = m.reason ?? 'blocked';
					break;
				case 'pod_restarts':
					pod(m).restarts = value;
					break;
				case 'workload_desired':
					workload(m).desired = value;
					break;
				case 'workload_ready':
					workload(m).ready = value;
					break;
				case 'workload_unavailable':
					workload(m).unavailable = value;
					break;
				case 'pvc_pending':
					if (value >= 1) out.stuckClaims.push({ namespace: m.namespace, pvc: m.pvc });
					break;
				case 'warning_events_by_reason':
					out.reasons.push({ reason: m.reason, count: value });
					break;
				default:
					if (!m.node && !m.pod && !m.workload && !m.pvc && !m.reason) out.totals[name] = value;
			}
		}
		out.nodes = [...nodes.values()].sort((a, b) => a.name.localeCompare(b.name, 'en'));
		out.pods = [...pods.values()]
			.filter((p) => p.crashlooping || p.pending || p.blocked !== null || p.ready === false)
			.sort((a, b) => Number(b.crashlooping) - Number(a.crashlooping) || a.pod.localeCompare(b.pod, 'en'));
		out.workloads = [...workloads.values()].filter((w) => w.unavailable > 0).sort((a, b) => a.name.localeCompare(b.name, 'en'));
		out.stuckClaims.sort((a, b) => a.pvc.localeCompare(b.pvc, 'en'));
		out.reasons.sort((a, b) => b.count - a.count || a.reason.localeCompare(b.reason, 'en'));
		return out;
	}

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(`{__name__=~"dumbmonit_k8s_.+", target="${target.id}"}`, signal);
			cluster = series.length > 0 ? fold(series) : null;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void target.id;
		loading = true;
		cluster = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	function plural(n: number, one: string, many: string): string {
		return `${n} ${n === 1 ? one : many}`;
	}

	/** The one sentence at the top: what is wrong, worst first, or that nothing is. */
	const verdict = $derived.by((): { tone: Tone; text: string } | null => {
		if (!cluster) return null;
		const notReady = cluster.nodes.filter((n) => n.ready === false).length;
		const crash = cluster.pods.filter((p) => p.crashlooping).length;
		const pending = cluster.pods.filter((p) => p.pending).length;
		const problems: string[] = [];
		if (notReady > 0) problems.push(`${plural(notReady, 'node', 'nodes')} not ready`);
		if (crash > 0) problems.push(`${plural(crash, 'pod', 'pods')} crash looping`);
		if (pending > 0) problems.push(`${plural(pending, 'pod', 'pods')} pending`);
		if (cluster.workloads.length > 0) problems.push(`${plural(cluster.workloads.length, 'workload', 'workloads')} missing replicas`);
		if (cluster.stuckClaims.length > 0) problems.push(`${plural(cluster.stuckClaims.length, 'volume claim', 'volume claims')} not bound`);
		const pressured = cluster.nodes.filter((n) => n.pressures.length > 0).length;
		if (pressured > 0) problems.push(`${plural(pressured, 'node', 'nodes')} under pressure`);
		if (problems.length === 0) return { tone: 'signal', text: 'Every node is ready and every workload has its replicas.' };
		const text = problems.join(', ');
		return { tone: notReady > 0 || crash > 0 ? 'warning' : 'advisory', text: `${text.charAt(0).toUpperCase()}${text.slice(1)}.` };
	});

	function podPlate(p: PodStat): { tone: Tone; label: string } {
		if (p.crashlooping) return { tone: 'warning', label: 'Crash looping' };
		if (p.blocked) return { tone: 'warning', label: p.blocked };
		if (p.pending) return { tone: 'advisory', label: 'Pending' };
		return { tone: 'advisory', label: 'Not ready' };
	}

	const PRESSURE_WORD: Record<string, string> = { memory: 'Memory pressure', disk: 'Disk pressure', pid: 'PID pressure', network: 'Network unavailable' };
</script>

{#if !loading && (error || cluster)}
	<Panel title="Cluster" description={cluster?.version ? `Kubernetes ${cluster.version}, read from its API server.` : 'Read from its API server.'} padded={false} class="rise-in">
		{#snippet aside()}
			{#if verdict}<Plate tone={verdict.tone} label={verdict.tone === 'signal' ? 'Healthy' : 'Needs attention'} />{/if}
		{/snippet}
		{#if error}
			<div class="px-5 py-4">
				<ErrorNotice {error} title="Could not load the cluster" onretry={() => void load()} />
			</div>
		{:else if cluster}
			{#if verdict}
				<p class={`px-5 pt-4 text-sm ${verdict.tone === 'signal' ? 'text-ink-2' : verdict.tone === 'warning' ? 'text-warning-ink' : 'text-advisory-ink'}`}>{verdict.text}</p>
			{/if}
			<div class="grid grid-cols-2 gap-x-6 gap-y-2 px-5 pt-4 sm:grid-cols-4">
				<Figure label="Nodes ready" value={cluster.totals.nodes !== undefined ? `${cluster.totals.nodes_ready ?? 0}/${cluster.totals.nodes}` : null} tone={(cluster.totals.nodes_ready ?? 0) < (cluster.totals.nodes ?? 0) ? 'warning' : 'ink'} />
				<Figure label="Pods running" value={cluster.totals.pods !== undefined ? `${cluster.totals.pods_running ?? 0}/${cluster.totals.pods}` : null} />
				<Figure label="Workloads short" value={String(cluster.workloads.length)} tone={cluster.workloads.length > 0 ? 'advisory' : 'ink'} />
				<Figure label="Warnings, last hour" value={cluster.totals.warning_events !== undefined ? String(cluster.totals.warning_events) : null} />
			</div>

			<section class="border-t border-line px-5 py-4">
				<h3 class="label-tape">Nodes</h3>
				<ul class="mt-2 flex flex-col gap-2">
					{#each cluster.nodes as n (n.name)}
						<li class="flex flex-col gap-1 sm:flex-row sm:flex-wrap sm:items-center sm:gap-x-3">
							<Plate tone={n.ready === false ? 'warning' : n.ready ? 'signal' : 'ghost'} label={n.ready === false ? 'Not ready' : n.ready ? 'Ready' : 'Unknown'} />
							<span class="min-w-0 text-sm font-semibold text-ink break-all">{n.name}</span>
							{#each n.pressures as p (p)}<Plate tone="advisory" label={PRESSURE_WORD[p] ?? p} bare />{/each}
							{#if n.cordoned}<Plate tone="muted" label="Cordoned" bare />{/if}
							{#if n.kubelet}<span class="text-sm text-ink-2">{`kubelet ${n.kubelet}`}</span>{/if}
						</li>
					{/each}
				</ul>
			</section>

			{#if cluster.pods.length > 0}
				<section class="border-t border-line px-5 py-4">
					<h3 class="label-tape">Pods needing attention</h3>
					<ul class="mt-2 flex flex-col gap-2">
						{#each cluster.pods as p (`${p.namespace}/${p.pod}`)}
							{@const plate = podPlate(p)}
							<li class="flex flex-col gap-1 sm:flex-row sm:flex-wrap sm:items-center sm:gap-x-3">
								<Plate tone={plate.tone} label={plate.label} />
								<span class="min-w-0 text-sm text-ink break-all">{`${p.namespace}/${p.pod}`}</span>
								{#if p.restarts !== null && p.restarts > 0}
									<span class="tnum text-sm text-ink-2">{plural(p.restarts, 'restart', 'restarts')}</span>
								{/if}
							</li>
						{/each}
					</ul>
				</section>
			{/if}

			{#if cluster.workloads.length > 0}
				<section class="border-t border-line px-5 py-4">
					<h3 class="label-tape">Workloads missing replicas</h3>
					<ul class="mt-2 flex flex-col gap-2">
						{#each cluster.workloads as w (`${w.kind}/${w.namespace}/${w.name}`)}
							<li class="flex flex-col gap-1 sm:flex-row sm:flex-wrap sm:items-center sm:gap-x-3">
								<Plate tone="advisory" label={`${w.ready}/${w.desired} ready`} />
								<span class="min-w-0 text-sm text-ink break-all">{`${w.namespace}/${w.name}`}</span>
								<span class="text-sm text-ink-2">{w.kind}</span>
							</li>
						{/each}
					</ul>
				</section>
			{/if}

			{#if cluster.stuckClaims.length > 0}
				<section class="border-t border-line px-5 py-4">
					<h3 class="label-tape">Volume claims not bound</h3>
					<ul class="mt-2 flex flex-col gap-2">
						{#each cluster.stuckClaims as c (`${c.namespace}/${c.pvc}`)}
							<li class="flex flex-col gap-1 sm:flex-row sm:flex-wrap sm:items-center sm:gap-x-3">
								<Plate tone="advisory" label="Pending" />
								<span class="min-w-0 text-sm text-ink break-all">{`${c.namespace}/${c.pvc}`}</span>
							</li>
						{/each}
					</ul>
				</section>
			{/if}

			{#if cluster.reasons.length > 0}
				<section class="border-t border-line px-5 py-4">
					<h3 class="label-tape">Warning events, last hour</h3>
					<p class="tnum mt-2 flex flex-wrap gap-x-4 gap-y-1 text-sm text-ink-2">
						{#each cluster.reasons as r (r.reason)}
							<span><span class="text-ink">{r.reason}</span>{` ×${r.count}`}</span>
						{/each}
					</p>
				</section>
			{/if}
		{/if}
	</Panel>
{/if}
