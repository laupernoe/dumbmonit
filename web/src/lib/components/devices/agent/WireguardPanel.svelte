<script lang="ts">
	/**
	 * WireGuard tunnels seen by the agent (`wg show all dump`): one block per
	 * interface, one line per peer with the time since its last handshake.
	 *
	 * Nothing is shown on a machine without WireGuard: the agent emits
	 * `dumbmonit_agent_wireguard_*` only when `wg` runs and finds an interface.
	 * A peer that never completed a handshake still has an age — the time the
	 * agent has watched it in silence — so `has_handshake` tells the two apart.
	 * The "silent" wording matches the built-in rule: only peers with a
	 * persistent keepalive are expected to stay up; the others (a phone, a
	 * laptop) are idle, not broken.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '$lib/api';
	import { ErrorNotice, Panel, Plate, type Tone } from '$lib/ui';
	import { formatAge, formatBytes } from '../docker/api';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	/** Past this, a session key has expired: WireGuard rekeys every two minutes while traffic flows. */
	const CONNECTED_WITHIN = 180;
	/** The built-in rule's threshold. */
	const SILENT_AFTER = 15 * 60;

	interface PeerStat {
		key: string;
		name: string | null;
		allowedIps: string;
		age: number | null;
		handshake: boolean;
		keepalive: number;
		rx: number | null;
		tx: number | null;
	}

	interface InterfaceStat {
		name: string;
		peers: PeerStat[];
	}

	let interfaces = $state<InterfaceStat[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	function plateOf(p: PeerStat): { tone: Tone; label: string } {
		if (p.age === null) return { tone: 'ghost', label: 'No reading' };
		if (!p.handshake) {
			return p.keepalive > 0 && p.age > SILENT_AFTER
				? { tone: 'warning', label: `Never connected (${formatAge(p.age)} watched)` }
				: { tone: 'ghost', label: 'Never connected' };
		}
		if (p.age <= CONNECTED_WITHIN) return { tone: 'signal', label: `Connected · ${formatAge(p.age)} ago` };
		if (p.keepalive > 0 && p.age > SILENT_AFTER) return { tone: 'warning', label: `Silent for ${formatAge(p.age)}` };
		return { tone: 'muted', label: `Idle · last handshake ${formatAge(p.age)} ago` };
	}

	function shortKey(key: string): string {
		return key.length > 12 ? `${key.slice(0, 8)}…` : key;
	}

	function fold(series: { metric: Record<string, string>; values: [number, string][] }[]): InterfaceStat[] {
		const byName = new Map<string, InterfaceStat>();
		const iface = (name: string) => {
			let found = byName.get(name);
			if (!found) {
				found = { name, peers: [] };
				byName.set(name, found);
			}
			return found;
		};
		for (const serie of series) {
			const metric = serie.metric.__name__ ?? '';
			const value = Number(serie.values.at(-1)?.[1]);
			if (!Number.isFinite(value)) continue;
			const name = serie.metric.interface ?? '';
			if (!name) continue;
			const i = iface(name);
			if (metric === 'dumbmonit_agent_wireguard_interface_peers') continue;
			const key = serie.metric.peer ?? '';
			if (!key) continue;
			let p = i.peers.find((x) => x.key === key);
			if (!p) {
				p = {
					key,
					name: serie.metric.name || null,
					allowedIps: serie.metric.allowed_ips ?? '',
					age: null,
					handshake: true,
					keepalive: 0,
					rx: null,
					tx: null
				};
				i.peers.push(p);
			}
			if (metric.endsWith('_peer_handshake_age_seconds')) p.age = value;
			else if (metric.endsWith('_peer_has_handshake')) p.handshake = value >= 1;
			else if (metric.endsWith('_peer_keepalive_seconds')) p.keepalive = value;
			else if (metric.endsWith('_peer_rx_bytes')) p.rx = value;
			else if (metric.endsWith('_peer_tx_bytes')) p.tx = value;
		}
		const list = [...byName.values()].sort((a, b) => a.name.localeCompare(b.name, 'en'));
		for (const i of list) {
			i.peers.sort((a, b) => (a.name ?? a.allowedIps).localeCompare(b.name ?? b.allowedIps, 'en'));
		}
		return list;
	}

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(`{__name__=~"dumbmonit_agent_wireguard_.+", target="${target.id}"}`, signal);
			interfaces = fold(series);
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
		interfaces = [];
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const silent = $derived(
		interfaces.flatMap((i) => i.peers).filter((p) => plateOf(p).tone === 'warning').length
	);
</script>

{#if !loading && (error || interfaces.length > 0)}
	<Panel title="WireGuard" description="Tunnels on this machine, read with wg show." padded={false} class="rise-in">
		{#snippet aside()}
			{#if silent > 0}
				<Plate tone="warning" label={silent === 1 ? '1 tunnel silent' : `${silent} tunnels silent`} />
			{:else if interfaces.length > 0}
				<Plate tone="signal" label="Tunnels up" />
			{/if}
		{/snippet}
		{#if error}
			<div class="px-5 py-4">
				<ErrorNotice {error} title="Could not load the tunnels" onretry={() => void load()} />
			</div>
		{:else}
			<ul class="divide-y divide-line">
				{#each interfaces as i (i.name)}
					<li class="px-5 py-4">
						<p class="font-mono font-semibold text-ink">{i.name}</p>
						{#if i.peers.length === 0}
							<p class="mt-1 text-sm text-ink-2">No peer configured on this interface.</p>
						{:else}
							<ul class="mt-2 flex flex-col gap-2">
								{#each i.peers as p (p.key)}
									{@const plate = plateOf(p)}
									<li class="flex flex-col gap-1 sm:flex-row sm:flex-wrap sm:items-center sm:gap-x-3">
										<Plate tone={plate.tone} label={plate.label} />
										<span class="min-w-0 text-sm text-ink break-all">{p.name ?? (p.allowedIps || 'no allowed IPs')}</span>
										<span class="font-mono text-xs text-ink-3" title={p.key}>{shortKey(p.key)}</span>
										{#if p.rx !== null && p.tx !== null}
											<span class="tnum text-sm text-ink-2">{`↓ ${formatBytes(p.rx)} · ↑ ${formatBytes(p.tx)}`}</span>
										{/if}
										{#if p.keepalive > 0}
											<span class="text-sm text-ink-2">{`keepalive ${p.keepalive} s`}</span>
										{/if}
									</li>
								{/each}
							</ul>
						{/if}
					</li>
				{/each}
			</ul>
			<p class="border-t border-line px-5 py-3 text-sm text-ink-2">
				Only peers with a persistent keepalive raise an alert when they stay silent for 15 minutes; the others may idle for days.
			</p>
		{/if}
	</Panel>
{/if}
