<script lang="ts">
	/**
	 * The federated instances, the ones the console cannot reach first. Each row
	 * says the product, the version, what it runs and — when it failed — the
	 * message the console itself received, which is the only thing that explains
	 * the outage.
	 */
	import type { PdmRemote } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { EmptyState, Plate } from '#lib/ui/index.js';
	import {
		formatAgo,
		formatBytes,
		formatCount,
		formatPercent,
		formatUnix,
		remoteKindLabel,
		remoteState,
		subscriptionState
	} from './format';

	interface Props {
		remotes: PdmRemote[];
	}

	let { remotes }: Props = $props();
</script>

{#if remotes.length === 0}
	<EmptyState
		title={m.devicesb_pdm_remotes_empty_title()}
		description={m.devicesb_pdm_remotes_empty_description()}
	/>
{:else}
	<ul class="divide-y divide-line">
		{#each remotes as remote, i (remote.id)}
			{@const state = remoteState(remote)}
			{@const subscription = subscriptionState(remote.subscription)}
			<li class="rise-in px-4 py-3 sm:px-5" style="--rise-delay: {Math.min(i, 8) * 40}ms">
				<div class="flex flex-col gap-1.5 sm:flex-row sm:flex-wrap sm:items-center sm:gap-x-3">
					<Plate tone={state.tone} label={state.word} />
					<span class="min-w-0 font-semibold break-all text-ink">{remote.id}</span>
					<span class="text-[0.8125rem] text-ink-2">{remoteKindLabel(remote.kind)}</span>
					{#if remote.version}
						<span class="tnum text-[0.8125rem] text-ink-2">
							{remote.version_behind
								? m.devicesb_pdm_remotes_version_behind({ version: remote.version })
								: m.devicesb_pdm_remotes_version({ version: remote.version })}
						</span>
					{/if}
					{#if subscription}
						<Plate tone={subscription.tone} label={subscription.word} />
					{/if}
					{#if remote.last_collection !== null}
						<span class="tnum text-[0.75rem] text-ink-3 sm:ml-auto" title={formatUnix(remote.last_collection)}>
							{m.devicesb_pdm_remotes_collected({ ago: formatAgo(remote.last_collection) })}
						</span>
					{/if}
				</div>

				{#if remote.error}
					<p class="mt-1 text-sm break-words text-warning-ink">{remote.error}</p>
				{/if}

				<dl class="tnum mt-1.5 flex flex-wrap gap-x-5 gap-y-1 text-[0.8125rem] text-ink-2">
					{#if remote.guests_running !== null}
						<div class="flex gap-1.5">
							<dt class="text-ink-3">{m.devicesb_pdm_remotes_guests()}</dt>
							<dd>{m.devicesb_pdm_remotes_guests_value({ running: formatCount(remote.guests_running), stopped: formatCount(remote.guests_stopped) })}</dd>
						</div>
					{/if}
					{#if remote.nodes_online !== null}
						<div class="flex gap-1.5">
							<dt class="text-ink-3">{m.devicesb_pdm_remotes_nodes()}</dt>
							<dd>
								{remote.nodes_offline
									? m.devicesb_pdm_remotes_nodes_value_offline({ online: formatCount(remote.nodes_online), offline: formatCount(remote.nodes_offline) })
									: m.devicesb_pdm_remotes_nodes_value({ online: formatCount(remote.nodes_online) })}
							</dd>
						</div>
					{/if}
					{#if remote.memory_used_percent !== null}
						<div class="flex gap-1.5">
							<dt class="text-ink-3">{m.devicesb_pdm_remotes_memory()}</dt>
							<dd>{m.devicesb_pdm_remotes_of_total({ percent: formatPercent(remote.memory_used_percent), total: formatBytes(remote.memory_total_bytes) })}</dd>
						</div>
					{/if}
					{#if remote.storage_used_percent !== null}
						<div class="flex gap-1.5">
							<dt class="text-ink-3">{m.devicesb_pdm_remotes_storage()}</dt>
							<dd>{m.devicesb_pdm_remotes_of_total({ percent: formatPercent(remote.storage_used_percent), total: formatBytes(remote.storage_total_bytes) })}</dd>
						</div>
					{/if}
					{#if remote.datastores}
						<div class="flex gap-1.5">
							<dt class="text-ink-3">{m.devicesb_pdm_remotes_datastores()}</dt>
							<dd>{formatCount(remote.datastores)}</dd>
						</div>
					{/if}
					{#if remote.updates_pending !== null}
						<div class="flex gap-1.5">
							<dt class="text-ink-3">{m.devicesb_pdm_remotes_updates()}</dt>
							<dd>{m.devicesb_pdm_remotes_updates_pending({ count: formatCount(remote.updates_pending) })}</dd>
						</div>
					{/if}
					{#if remote.nodes.length > 0}
						<div class="flex gap-1.5">
							<dt class="text-ink-3">{m.devicesb_pdm_remotes_address()}</dt>
							<dd class="break-all">{remote.nodes.join(', ')}</dd>
						</div>
					{/if}
				</dl>
			</li>
		{/each}
	</ul>
{/if}
