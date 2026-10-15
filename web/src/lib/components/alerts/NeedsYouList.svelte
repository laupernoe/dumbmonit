<script lang="ts">
	/**
	 * The "Needs you" list, shared by the Overview and the Alerts "Now" tab.
	 *
	 * Rows come pre-ordered from the sky helper: unreachable devices and
	 * warnings first, then advisories, building up, and the suppressed last.
	 * They are laid out as tiles in a responsive grid (one column on phones,
	 * two, then three on wide screens) that fills left to right, so the order
	 * still reads top-left first.
	 * When `grouped` is set, rows are gathered under their device (host
	 * grouping), which is how the Alerts page reads them; the Overview leaves
	 * them as one flat stream. Acknowledged and snoozed alerts each leave the
	 * stream for their own quieter group at the bottom, collapsed by default
	 * ("Snoozed · N", "Dismissed · N") so they stop crowding the list rather than sitting
	 * there faded. The empty state only appears when the sky says so — a
	 * device that has stopped reporting is never "nothing".
	 *
	 * Every tile ends the same way: Ack (with its durations, "until resolved"
	 * included) and Open device; Snooze and "Never for this device" sit in the
	 * tile's ⋮ menu.
	 *
	 * With `selectable` (the Alerts page), operators get a checkbox on each
	 * alert tile and a bulk bar above the list: acknowledge, unacknowledge or
	 * ignore the selected alerts for their device in one go.
	 */
	import type { Alert, Silence, Target } from '#lib/api/index.js';
	import { ackAlert, putRuleOverride, unackAlert } from '#lib/api/index.js';
	import { isAckedRow, isSnoozedRow, type Sky, type SkyRow } from '#lib/components/overview/sky.js';
	import { Button, Confirm, EmptyState } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { UNTIL_RESOLVED_SECS } from './helpers';
	import {
		afterRun,
		eligibleItems,
		pruneSelection,
		runBulk,
		selectionState,
		setMany,
		toggleKey,
		type BulkAction,
		type SelectableItem
	} from './selection';
	import { CloudSun, ChevronRight } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import AlertRow from './AlertRow.svelte';
	import DeviceRow from './DeviceRow.svelte';

	interface Props {
		sky: Sky;
		/** Relative time of the last check, for the empty state's second line. */
		checkedLabel?: string;
		grouped?: boolean;
		/** Maintenance windows, passed through to `SnoozeControl` for "time left". */
		silences?: Silence[];
		/** The pigeon instead of the icon in the quiet state (the Overview smiles). */
		mascot?: 'watch' | 'dizzy' | 'happy';
		/** A checkbox per alert tile and a bulk bar (operators only). */
		selectable?: boolean;
		/** An acknowledgement, a snooze or an ignore was made or lifted: refresh the alerts. */
		onchanged?: () => void;
	}

	let {
		sky,
		checkedLabel,
		grouped = false,
		silences = [],
		mascot,
		selectable = false,
		onchanged
	}: Props = $props();

	/**
	 * One rule firing on several series of the same device (four filesystems
	 * nearly full) is one problem, not four: consecutive alert rows sharing the
	 * device, rule and phase fold into the first, which lists the others.
	 */
	interface Folded {
		row: SkyRow;
		extra: Alert[];
	}
	function fold(input: SkyRow[]): Folded[] {
		const out: Folded[] = [];
		for (const row of input) {
			const last = out[out.length - 1];
			if (
				row.kind === 'alert' &&
				last &&
				last.row.kind === 'alert' &&
				last.row.alert.target_id === row.alert.target_id &&
				last.row.alert.rule_uid === row.alert.rule_uid &&
				last.row.alert.effective_phase === row.alert.effective_phase
			) {
				last.extra.push(row.alert);
				continue;
			}
			out.push({ row, extra: [] });
		}
		return out;
	}

	let dismissedOpen = $state(false);
	let snoozedOpen = $state(false);

	const rows = $derived(
		fold(
			sky.needsYou.filter((row) => !isAckedRow(row) && !isSnoozedRow(row))
		)
	);
	const ackedRows = $derived(fold(sky.needsYou.filter(isAckedRow)));
	const snoozedRows = $derived(
		fold(sky.needsYou.filter((row) => isSnoozedRow(row) && !isAckedRow(row)))
	);

	/** What "quiet" means right now: everything reporting, or still waiting. */
	const quietDescription = $derived.by(() => {
		const { waiting, devices } = sky.counts;
		const base =
			devices === 0
				? m.alerts_needs_quiet_no_devices()
				: waiting > 0
					? waiting === 1
						? m.alerts_needs_quiet_waiting_one()
						: m.alerts_needs_quiet_waiting_many({ count: waiting })
					: m.alerts_needs_quiet_all_reporting();
		return checkedLabel ? m.alerts_needs_quiet_checked({ text: base, when: checkedLabel }) : base;
	});

	// --- Selection and bulk actions -------------------------------------------

	const canSelect = $derived(selectable && auth.canOperate);
	let selected = $state<Set<string>>(new Set());
	let bulkBusy = $state<BulkAction | null>(null);
	let bulkMessage = $state<{ text: string; error: boolean } | null>(null);
	let selectAllBox = $state<HTMLInputElement | null>(null);

	function itemOf(folded: Folded): SelectableItem | null {
		const row = folded.row;
		if (row.kind !== 'alert') return null;
		return {
			key: row.key,
			fingerprints: [row.alert.fingerprint, ...folded.extra.map((a) => a.fingerprint)],
			acked: row.alert.acked,
			ruleId: row.rule?.id,
			targetId: row.alert.target_id
		};
	}
	/** Every alert tile on screen; the collapsed groups are left out. */
	const items = $derived(
		[...rows, ...(snoozedOpen ? snoozedRows : []), ...(dismissedOpen ? ackedRows : [])]
			.map(itemOf)
			.filter((item): item is SelectableItem => item !== null)
	);
	const itemKeys = $derived(items.map((item) => item.key));
	const allState = $derived(selectionState(selected, itemKeys));
	const selectedCount = $derived(itemKeys.filter((key) => selected.has(key)).length);
	const canAck = $derived(eligibleItems('ack', items, selected).length);
	const canUnack = $derived(eligibleItems('unack', items, selected).length);
	const canIgnore = $derived(eligibleItems('ignore', items, selected).length);

	$effect(() => {
		// Tiles that left the screen (resolved, refreshed, collapsed) leave the selection too.
		const kept = pruneSelection(selected, itemKeys);
		if (kept.size !== selected.size) selected = kept;
	});
	$effect(() => {
		if (selectAllBox) selectAllBox.indeterminate = allState === 'some';
	});

	async function bulk(action: BulkAction) {
		const todo = eligibleItems(action, items, selected);
		if (todo.length === 0) return;
		bulkBusy = action;
		bulkMessage = null;
		const result = await runBulk(todo, async (item) => {
			if (action === 'ignore') {
				await putRuleOverride(item.ruleId!, item.targetId!, { enabled: false });
			} else {
				for (const fingerprint of item.fingerprints) {
					if (action === 'ack') await ackAlert(fingerprint, { duration_secs: UNTIL_RESOLVED_SECS });
					else await unackAlert(fingerprint);
				}
			}
		});
		bulkBusy = null;
		selected = afterRun(result);
		bulkMessage =
			result.failed.length === 0
				? { text: m.alerts_bulk_done({ count: result.done }), error: false }
				: {
						text: m.alerts_bulk_partial({
							done: result.done,
							failed: result.failed.length,
							error: result.failed[0].message
						}),
						error: true
					};
		onchanged?.();
	}

	/** Rows gathered per device, keeping the ordered stream within each group. */
	interface Group {
		key: string;
		name: string;
		target?: Target;
		items: Folded[];
	}
	const groups = $derived.by<Group[]>(() => {
		const map = new Map<string, Group>();
		for (const folded of rows) {
			const row = folded.row;
			const target = row.target;
			const targetId = row.kind === 'device' ? row.target.id : row.alert.target_id;
			const key = targetId === null ? 'none' : String(targetId);
			let group = map.get(key);
			if (!group) {
				group = {
					key,
					name:
						target?.name ??
						(targetId === null
							? m.alerts_needs_no_device()
							: m.alerts_scope_device({ id: targetId })),
					target,
					items: []
				};
				map.set(key, group);
			}
			group.items.push(folded);
		}
		return [...map.values()];
	});
</script>

{#snippet rowView(folded: Folded)}
	{#if folded.row.kind === 'device'}
		<DeviceRow row={folded.row} />
	{:else}
		{@const key = folded.row.key}
		<AlertRow
			row={folded.row}
			extra={folded.extra}
			{silences}
			{onchanged}
			selectable={canSelect}
			selected={selected.has(key)}
			onselect={(on) => (selected = toggleKey(selected, key, on))}
		/>
	{/if}
{/snippet}

{#snippet bulkBar()}
	{#if canSelect && items.length > 0}
		<div
			class="sticky top-0 z-10 mb-3 flex flex-wrap items-center gap-x-4 gap-y-2 rounded-[var(--radius-card)] border border-line bg-surface px-4 py-2 shadow-lift"
			role="toolbar"
			aria-label={m.alerts_bulk_label()}
		>
			<label class="inline-flex min-h-10 cursor-pointer items-center gap-2 text-[0.8125rem] font-medium text-ink sm:min-h-8">
				<input
					type="checkbox"
					class="size-4 accent-[var(--c-signal)]"
					bind:this={selectAllBox}
					checked={allState === 'all'}
					onchange={(e) =>
						(selected = setMany(selected, itemKeys, (e.currentTarget as HTMLInputElement).checked))}
				/>
				{m.alerts_bulk_select_all({ count: items.length })}
			</label>
			<span class="tnum text-[0.8125rem] text-ink-2" aria-live="polite">
				{m.alerts_bulk_selected({ count: selectedCount })}
			</span>
			{#if selectedCount > 0}
				<div class="flex flex-wrap items-center gap-2 sm:ml-auto">
					<Button
						size="sm"
						variant="secondary"
						disabled={canAck === 0 || bulkBusy !== null}
						loading={bulkBusy === 'ack'}
						onclick={() => void bulk('ack')}
					>
						{m.alerts_bulk_ack()}
					</Button>
					<Button
						size="sm"
						variant="secondary"
						disabled={canUnack === 0 || bulkBusy !== null}
						loading={bulkBusy === 'unack'}
						onclick={() => void bulk('unack')}
					>
						{m.alerts_bulk_unack()}
					</Button>
					<Confirm
						variant="secondary"
						size="sm"
						confirmLabel={m.alerts_bulk_ignore_confirm({ count: canIgnore })}
						disabled={canIgnore === 0 || bulkBusy !== null}
						loading={bulkBusy === 'ignore'}
						onconfirm={() => bulk('ignore')}
					>
						{m.alerts_bulk_ignore()}
					</Confirm>
					<Button size="sm" variant="ghost" onclick={() => (selected = new Set())}>
						{m.alerts_bulk_clear()}
					</Button>
				</div>
			{/if}
		</div>
	{/if}
	{#if bulkMessage}
		<p
			class={`mb-3 text-[0.8125rem] font-medium ${bulkMessage.error ? 'text-warning-ink' : 'text-ink-2'}`}
			role={bulkMessage.error ? 'alert' : 'status'}
		>
			{bulkMessage.text}
		</p>
	{/if}
{/snippet}

{#snippet snoozedSection()}
	{#if snoozedRows.length > 0}
		<div class={rows.length > 0 ? 'mt-6' : ''}>
			<button
				type="button"
				class="label-tape mb-2 flex min-h-10 w-full items-center gap-2 text-left"
				aria-expanded={snoozedOpen}
				onclick={() => (snoozedOpen = !snoozedOpen)}
			>
				<ChevronRight
					class={`size-3.5 shrink-0 transition-transform ${snoozedOpen ? 'rotate-90' : ''}`}
					aria-hidden="true"
				/>
				{m.alerts_needs_snoozed()}
				<span class="tnum font-normal text-ink-3">· {snoozedRows.length}</span>
			</button>
			{#if snoozedOpen}
				<div class="grid grid-cols-1 items-stretch gap-2.5 sm:grid-cols-2 xl:grid-cols-3">
					{#each snoozedRows as folded, i (folded.row.key)}
						<div class="rise-in min-w-0" style={`--rise-delay: ${i * 30}ms`}>
							{@render rowView(folded)}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
{/snippet}

{#snippet dismissedSection()}
	{#if ackedRows.length > 0}
		<div class={rows.length > 0 || snoozedRows.length > 0 ? 'mt-6' : ''}>
			<button
				type="button"
				class="label-tape mb-2 flex min-h-10 w-full items-center gap-2 text-left"
				aria-expanded={dismissedOpen}
				onclick={() => (dismissedOpen = !dismissedOpen)}
			>
				<ChevronRight
					class={`size-3.5 shrink-0 transition-transform ${dismissedOpen ? 'rotate-90' : ''}`}
					aria-hidden="true"
				/>
				{m.alerts_needs_dismissed()}
				<span class="tnum font-normal text-ink-3">· {ackedRows.length}</span>
			</button>
			{#if dismissedOpen}
				<p class="mb-2 text-[0.8125rem] text-ink-2">
					{m.alerts_needs_dismissed_hint()}
				</p>
				<div class="grid grid-cols-1 items-stretch gap-2.5 sm:grid-cols-2 xl:grid-cols-3">
					{#each ackedRows as folded, i (folded.row.key)}
						<div class="rise-in min-w-0" style={`--rise-delay: ${i * 30}ms`}>
							{@render rowView(folded)}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
{/snippet}

{#if sky.quiet}
	<EmptyState
		tone="signal"
		icon={mascot ? undefined : CloudSun}
		{mascot}
		title={sky.counts.devices === 0
			? m.alerts_needs_empty_no_devices()
			: m.alerts_needs_empty_quiet()}
		description={quietDescription}
	/>
{:else if grouped}
	{@render bulkBar()}
	<div class="grid grid-cols-1 items-start gap-x-2.5 gap-y-6 md:grid-cols-2 xl:grid-cols-3">
		{#each groups as group (group.key)}
			<div class="min-w-0">
				<h3 class="label-tape mb-2">
					{#if group.target}
						<a href={`/targets/${group.target.id}`} class="hover:text-ink hover:underline"
							>{group.name}</a
						>
					{:else}
						{group.name}
					{/if}
				</h3>
				<div class="grid grid-cols-1 items-stretch gap-2.5">
					{#each group.items as folded, i (folded.row.key)}
						<div class="rise-in min-w-0" style={`--rise-delay: ${i * 30}ms`}>
							{@render rowView(folded)}
						</div>
					{/each}
				</div>
			</div>
		{/each}
	</div>
	{@render snoozedSection()}
	{@render dismissedSection()}
{:else}
	{@render bulkBar()}
	<div class="grid grid-cols-1 items-stretch gap-2.5 sm:grid-cols-2 xl:grid-cols-3">
		{#each rows as folded, i (folded.row.key)}
			<div class="rise-in min-w-0" style={`--rise-delay: ${i * 30}ms`}>
				{@render rowView(folded)}
			</div>
		{/each}
	</div>
	{@render snoozedSection()}
	{@render dismissedSection()}
{/if}
