<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The "To review" list of the network discovery: devices the scans found
	 * that are neither monitored nor ignored. Rows have a checkbox, a "select
	 * all" and one action bar (add as monitors / ignore) act on the selection;
	 * the type the detection proposed is shown on each row and used on add.
	 * An ignore can be undone from the result line. State comes with a word
	 * ("New"), never with a colour alone.
	 */
	import { CircleCheck, EyeOff, Undo2 } from 'lucide-svelte';
	import { bulkNetscanDevices } from '#lib/api/discovery.js';
	import type { NetscanBulkReport, NetscanDevice } from '#lib/api/types.js';
	import { formatRelative } from '#lib/format.js';
	import { Button, EmptyState, ErrorNotice, Plate, SelectionBar } from '#lib/ui/index.js';

	interface Props {
		/** The whole inventory; the queue keeps the new, not ignored devices. */
		devices: NetscanDevice[];
		/** Reload the screen after an action: the server is the source of truth. */
		onchanged: () => Promise<void>;
	}

	let { devices, onchanged }: Props = $props();

	const queue = $derived(devices.filter((d) => !d.known && !d.ignored));
	let selected = $state<number[]>([]);
	let working = $state<'add' | 'ignore' | 'undo' | null>(null);
	let report = $state<NetscanBulkReport | null>(null);
	/** What the last action did, for the result line. */
	let last = $state<'add' | 'ignore' | 'restore' | null>(null);
	/** The devices the last ignore hid, so "Undo" can put them back. */
	let undoIds = $state<number[]>([]);
	let failure = $state<unknown>(null);

	// A device that left the queue (added elsewhere, scan reload) leaves the selection.
	const chosen = $derived(selected.filter((id) => queue.some((d) => d.id === id)));
	const selection = $derived<'none' | 'some' | 'all'>(
		chosen.length === 0 ? 'none' : chosen.length === queue.length ? 'all' : 'some'
	);

	const title = (d: NetscanDevice) => d.hostname ?? d.vendor ?? d.ip;
	const proposed = (d: NetscanDevice) => d.ports.find((p) => p.kind)?.kind ?? null;
	const nameOf = (id: number) => {
		const d = devices.find((x) => x.id === id);
		return d ? title(d) : String(id);
	};

	function toggle(id: number, on: boolean) {
		selected = on ? [...chosen.filter((x) => x !== id), id] : chosen.filter((x) => x !== id);
	}

	async function run(action: 'add' | 'ignore' | 'restore', ids: number[]) {
		if (working || ids.length === 0) return;
		working = action === 'restore' ? 'undo' : action;
		failure = null;
		try {
			report = await bulkNetscanDevices(action, ids);
			last = action;
			undoIds = action === 'ignore' ? report.results.filter((r) => r.outcome === 'ignored').map((r) => r.device_id) : [];
			selected = [];
			await onchanged();
		} catch (cause) {
			failure = cause;
		} finally {
			working = null;
		}
	}

	const failed = $derived(report?.results.filter((r) => r.outcome === 'error') ?? []);
	const ignoredCount = $derived(report?.results.filter((r) => r.outcome === 'ignored').length ?? 0);
	const restoredCount = $derived(report?.results.filter((r) => r.outcome === 'restored').length ?? 0);
</script>

<section aria-labelledby="inv-review" class="grid gap-3">
	<h3 id="inv-review" class="flex items-center gap-2 text-sm font-semibold text-ink">
		{m.discovery_review_heading({ count: queue.length })}
	</h3>

	{#if failure}
		<ErrorNotice error={failure} title={m.discovery_review_action_error()} />
	{/if}

	{#if report}
		<div class="grid gap-1.5 rounded-lg border border-line bg-surface-2 px-3.5 py-3 text-sm text-ink" role="status">
			<ul class="flex flex-wrap items-center gap-x-4 gap-y-1">
				{#if last === 'add'}
					<li class="flex items-center gap-1.5">
						<CircleCheck class="size-4 text-signal-ink" aria-hidden="true" />
						<span class="tnum">{m.discovery_review_result_added({ count: report.added })}</span>
					</li>
					{#if report.pending_credentials > 0}
						<li class="tnum">{m.discovery_review_result_pending({ count: report.pending_credentials })}</li>
					{/if}
					{#if report.already_present > 0}
						<li class="tnum">{m.discovery_review_result_present({ count: report.already_present })}</li>
					{/if}
					{#if report.errors > 0}
						<li class="tnum font-semibold text-warning-ink">{m.discovery_review_result_failed({ count: report.errors })}</li>
					{/if}
				{:else if last === 'ignore'}
					<li class="flex items-center gap-1.5">
						<EyeOff class="size-4 text-ink-2" aria-hidden="true" />
						<span class="tnum">{m.discovery_review_result_ignored({ count: ignoredCount })}</span>
					</li>
				{:else if last === 'restore'}
					<li class="tnum">{m.discovery_review_result_restored({ count: restoredCount })}</li>
				{/if}
				{#if last === 'ignore' && undoIds.length > 0}
					<li>
						<Button size="sm" variant="secondary" disabled={working !== null} loading={working === 'undo'} onclick={() => run('restore', undoIds)}>
							<Undo2 class="size-4" aria-hidden="true" />
							{m.discovery_review_undo()}
						</Button>
					</li>
				{/if}
			</ul>
			{#if last === 'add' && report.pending_credentials > 0}
				<p class="text-[0.8125rem] text-ink-2">{m.discovery_review_pending_hint()}</p>
			{/if}
			{#each failed as item (item.device_id)}
				<p class="break-words text-[0.8125rem] text-warning-ink">{m.discovery_review_item_error({ name: nameOf(item.device_id), message: item.message ?? '' })}</p>
			{/each}
		</div>
	{/if}

	{#if queue.length === 0}
		<EmptyState title={m.discovery_review_empty_title()} description={m.discovery_review_empty_desc()} />
	{:else}
		<label class="inline-flex min-h-10 w-fit cursor-pointer items-center gap-3 text-sm font-semibold text-ink">
			<input
				type="checkbox"
				class="size-5 accent-[var(--c-signal)]"
				checked={selection === 'all'}
				indeterminate={selection === 'some'}
				onchange={(event) => (selected = event.currentTarget.checked ? queue.map((d) => d.id) : [])}
			/>
			{m.discovery_review_bar_select_all({ count: queue.length })}
		</label>
		<ul class="divide-y divide-line overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface">
			{#each queue as device (device.id)}
				{@const kind = proposed(device)}
				<li>
					<label class="flex min-h-10 cursor-pointer items-start gap-3 border-l-4 border-l-advisory bg-advisory-soft px-4 py-3 has-[:focus-visible]:outline-2 has-[:focus-visible]:-outline-offset-2 has-[:focus-visible]:outline-signal">
						<input
							type="checkbox"
							class="mt-0.5 size-5 shrink-0 accent-[var(--c-signal)]"
							aria-label={m.discovery_review_select_aria({ name: title(device) })}
							checked={chosen.includes(device.id)}
							onchange={(event) => toggle(device.id, event.currentTarget.checked)}
						/>
						<span class="min-w-0 flex-1">
							<span class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-ink">
								<span class="font-semibold">{title(device)}</span>
								<Plate tone="advisory" label={m.discovery_review_new()} />
							</span>
							<span class="block break-all font-mono text-[0.75rem] text-ink-2">
								{device.ip}{device.mac ? ` · ${device.mac}` : ''}
							</span>
							<span class="block text-[0.8125rem] text-ink-2">
								{#if kind}
									{m.discovery_review_proposed({ kind })}
								{:else}
									{m.discovery_review_no_type()}
								{/if}
								· {m.discovery_inventory_first_seen({ when: formatRelative(device.first_seen) })}
							</span>
						</span>
					</label>
				</li>
			{/each}
		</ul>
	{/if}
</section>

{#if chosen.length > 0}
	<SelectionBar
		countLabel={m.discovery_review_bar_count({ count: chosen.length })}
		selectAllLabel={m.discovery_review_bar_select_all({ count: queue.length })}
		label={m.discovery_review_bar_label()}
		exitLabel={m.discovery_review_bar_done()}
		{selection}
		onselectall={(on) => (selected = on ? queue.map((d) => d.id) : [])}
		onexit={() => (selected = [])}
	>
		<Button size="sm" variant="primary" disabled={working !== null} loading={working === 'add'} onclick={() => run('add', chosen)}>
			{m.discovery_review_add_selected()}
		</Button>
		<Button size="sm" variant="secondary" disabled={working !== null} loading={working === 'ignore'} onclick={() => run('ignore', chosen)}>
			<EyeOff class="size-4" aria-hidden="true" />
			{m.discovery_review_ignore_selected()}
		</Button>
	</SelectionBar>
{/if}
