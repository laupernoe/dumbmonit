<script lang="ts">
	/**
	 * IPsec (strongSwan) tunnels seen by the agent (`swanctl --list-conns` /
	 * `--list-sas`): one line per configured CHILD_SA, up or down.
	 *
	 * Nothing is shown on a machine without a configured connection: the agent
	 * emits `dumbmonit_agent_ipsec_tunnel_up` only for tunnels `swanctl` knows
	 * about, whether or not they are currently installed — a tunnel that never
	 * came up is "down", not invisible.
	 */
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate } from '#lib/ui/index.js';
	import { formatAge, formatBytes } from '../docker/api';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	interface TunnelStat {
		key: string;
		connection: string;
		name: string;
		up: boolean;
		bytesIn: number | null;
		bytesOut: number | null;
		rekeySeconds: number | null;
	}

	let tunnels = $state<TunnelStat[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	function fold(series: { metric: Record<string, string>; values: [number, string][] }[]): TunnelStat[] {
		const byKey = new Map<string, TunnelStat>();
		for (const serie of series) {
			const metric = serie.metric.__name__ ?? '';
			const value = Number(serie.values.at(-1)?.[1]);
			const connection = serie.metric.connection ?? '';
			const name = serie.metric.tunnel ?? '';
			if (!Number.isFinite(value) || !connection || !name) continue;
			const key = `${connection}:${name}`;
			let t = byKey.get(key);
			if (!t) {
				t = { key, connection, name, up: false, bytesIn: null, bytesOut: null, rekeySeconds: null };
				byKey.set(key, t);
			}
			if (metric.endsWith('_tunnel_up')) t.up = value >= 1;
			else if (metric.endsWith('_tunnel_bytes_in')) t.bytesIn = value;
			else if (metric.endsWith('_tunnel_bytes_out')) t.bytesOut = value;
			else if (metric.endsWith('_tunnel_rekey_seconds')) t.rekeySeconds = value;
		}
		return [...byKey.values()].sort(
			(a, b) => a.connection.localeCompare(b.connection, 'en') || a.name.localeCompare(b.name, 'en')
		);
	}

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(`{__name__=~"dumbmonit_agent_ipsec_tunnel_.+", target="${target.id}"}`, signal);
			tunnels = fold(series);
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
		tunnels = [];
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const down = $derived(tunnels.filter((t) => !t.up).length);
</script>

{#if !loading && (error || tunnels.length > 0)}
	<Panel title={m.devices_ipsec_title()} description={m.devices_ipsec_description()} padded={false} class="rise-in">
		{#snippet aside()}
			{#if down > 0}
				<Plate tone="warning" label={down === 1 ? m.devices_ipsec_down_one() : m.devices_ipsec_down_other({ count: down })} />
			{:else if tunnels.length > 0}
				<Plate tone="signal" label={m.devices_ipsec_all_up()} />
			{/if}
		{/snippet}
		{#if error}
			<div class="px-5 py-4">
				<ErrorNotice {error} title={m.devices_ipsec_err_title()} onretry={() => void load()} />
			</div>
		{:else}
			<ul class="divide-y divide-line">
				{#each tunnels as t (t.key)}
					<li class="flex flex-col gap-1 px-5 py-3 sm:flex-row sm:flex-wrap sm:items-center sm:gap-x-3">
						<Plate tone={t.up ? 'signal' : 'warning'} label={t.up ? m.devices_ipsec_up() : m.devices_ipsec_down()} />
						<span class="min-w-0 text-sm font-semibold text-ink break-all">{t.name}</span>
						<span class="text-sm text-ink-2">{t.connection}</span>
						{#if t.bytesIn !== null && t.bytesOut !== null}
							<span class="tnum text-sm text-ink-2">{`↓ ${formatBytes(t.bytesIn)} · ↑ ${formatBytes(t.bytesOut)}`}</span>
						{/if}
						{#if t.rekeySeconds !== null}
							<span class="text-sm text-ink-2">{m.devices_ipsec_rekey({ age: formatAge(t.rekeySeconds) })}</span>
						{/if}
					</li>
				{/each}
			</ul>
			<p class="border-t border-line px-5 py-3 text-sm text-ink-2">
				{m.devices_ipsec_note()}
			</p>
		{/if}
	</Panel>
{/if}
