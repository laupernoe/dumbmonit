<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Devices (phones, tablets, desktops) signed in to an account, with each
	 * one's last connection and last backup — generic across every collector
	 * that tracks this (Immich today, more products later): the component
	 * only knows the metric family `dumbmonit_client_device_*`, never a
	 * product name. A panel embeds it with nothing beyond the target.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { EmptyState, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { buildClientDevices, type ClientDevice } from './clientDevices';
	import { formatAgo, formatUnix } from './pbs/format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let devices = $state<ClientDevice[] | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(`{__name__=~"dumbmonit_client_device_.+", target="${target.id}"}`, signal);
			devices = buildClientDevices(series);
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
		devices = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});
</script>

{#if error}
	<Panel title={m.devices_clients_title()} class="rise-in">
		<ErrorNotice {error} title={m.devices_clients_error()} onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title={m.devices_clients_title()} padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label={m.devices_clients_loading()}>
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else if devices && devices.length > 0}
	<Panel title={m.devices_clients_title()} description={m.devices_clients_description()} padded={false} class="rise-in">
		<ul class="divide-y divide-line sm:hidden">
			{#each devices as device (device.device)}
				<li class={`flex flex-col gap-1 px-4 py-3 text-sm ${device.stale ? 'bg-warning-soft/40' : ''}`}>
					<div class="flex items-center justify-between gap-3">
						<span class="min-w-0 truncate font-semibold text-ink">{device.device}</span>
						<Plate tone={device.stale ? 'warning' : 'signal'} label={device.stale ? m.devices_clients_stale() : m.devices_clients_ok()} />
					</div>
					<span class="text-ink-2">{[device.type, device.os, device.user].filter(Boolean).join(' · ') || '—'}</span>
					<span class="tnum text-ink-2">{m.devices_clients_col_connection()} {device.lastSeen === null ? '—' : formatAgo(device.lastSeen)}</span>
					<span class="tnum text-ink-2">{m.devices_clients_col_backup()} {device.lastBackup === null ? '—' : formatAgo(device.lastBackup)}</span>
				</li>
			{/each}
		</ul>
		<div class="hidden overflow-x-auto sm:block">
			<table class="w-full min-w-[40rem] text-sm">
				<thead>
					<tr class="border-b border-line text-left text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase">
						<th class="px-5 py-2 font-semibold">{m.devices_clients_col_device()}</th>
						<th class="px-3 py-2 font-semibold">{m.devices_clients_col_type()}</th>
						<th class="px-3 py-2 font-semibold">{m.devices_clients_col_user()}</th>
						<th class="px-3 py-2 font-semibold">{m.devices_clients_col_connection()}</th>
						<th class="px-3 py-2 font-semibold">{m.devices_clients_col_backup()}</th>
						<th class="px-5 py-2 font-semibold">{m.devices_clients_col_status()}</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-line">
					{#each devices as device (device.device)}
						<tr class={device.stale ? 'bg-warning-soft/40' : ''}>
							<td class="px-5 py-2.5 align-top font-semibold text-ink">{device.device}</td>
							<td class="px-3 py-2.5 align-top text-ink-2">{[device.type, device.os].filter(Boolean).join(' · ') || '—'}</td>
							<td class="px-3 py-2.5 align-top text-ink-2">{device.user || '—'}</td>
							<td class="tnum px-3 py-2.5 align-top text-ink-2" title={device.lastSeen === null ? '' : formatUnix(device.lastSeen)}>
								{device.lastSeen === null ? '—' : formatAgo(device.lastSeen)}
							</td>
							<td class="tnum px-3 py-2.5 align-top text-ink-2" title={device.lastBackup === null ? '' : formatUnix(device.lastBackup)}>
								{device.lastBackup === null ? '—' : formatAgo(device.lastBackup)}
							</td>
							<td class="px-5 py-2.5 align-top">
								<Plate tone={device.stale ? 'warning' : 'signal'} label={device.stale ? m.devices_clients_stale() : m.devices_clients_ok()} />
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</Panel>
{:else}
	<Panel title={m.devices_clients_title()} class="rise-in">
		<EmptyState title={m.devices_clients_empty_title()} description={m.devices_clients_empty_desc()} />
	</Panel>
{/if}
