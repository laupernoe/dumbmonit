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
	 * ("Dismissed · N") so they stop crowding the list rather than sitting
	 * there faded. The empty state only appears when the sky says so — a
	 * device that has stopped reporting is never "nothing".
	 *
	 * Each `AlertRow`'s one-click × (dismiss) is handled here: `dismissAlert`
	 * acks until resolved and hides the row immediately (`dismissing`, before
	 * the server answers), `undoDismiss` lifts that ack, and a `Toast` offers
	 * the undo for a few seconds either way.
	 */
	import type { Alert, Silence, Target } from '#lib/api/index.js';
	import { ackAlert, unackAlert } from '#lib/api/index.js';
	import { isAckedRow, isSnoozedRow, type Sky, type SkyRow } from '#lib/components/overview/sky.js';
	import { EmptyState, Toast } from '#lib/ui/index.js';
	import { CloudSun, ChevronRight } from 'lucide-svelte';
	import { UNTIL_RESOLVED_SECS } from './helpers';
	import { m } from '#lib/paraglide/messages.js';
	import AlertRow from './AlertRow.svelte';
	import DeviceRow from './DeviceRow.svelte';

	interface Props {
		sky: Sky;
		/** Relative time of the last check, for the empty state's second line. */
		checkedLabel?: string;
		grouped?: boolean;
		showOpen?: boolean;
		/** Maintenance windows, passed through to `SnoozeControl` for "time left". */
		silences?: Silence[];
		/** The pigeon instead of the icon in the quiet state (the Overview smiles). */
		mascot?: 'watch' | 'dizzy' | 'happy';
		/** An acknowledgement, a snooze or an ignore was made or lifted: refresh the alerts. */
		onchanged?: () => void;
	}

	let {
		sky,
		checkedLabel,
		grouped = false,
		showOpen = false,
		silences = [],
		mascot,
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

	// --- Dismiss (one click, optimistic) ------------------------------------
	//
	// The × is an ack until resolved without the menu: the row leaves the main
	// list the instant it is clicked (before the server answers), a toast
	// offers "Undo" for a few seconds, and the alert only truly comes back to
	// "needs you" if it resolves and fires again. `dismissing` hides a row
	// locally while the request is in flight; once it resolves, `onchanged`
	// refreshes the shared store and the alert (now acked) settles into the
	// "Dismissed" section on its own — `dismissing` is cleared either way so
	// it never masks the real state for long.

	let dismissing = $state<Set<string>>(new Set());
	let toast = $state<{ fingerprint: string } | { error: string } | null>(null);

	function isDismissingRow(row: SkyRow): boolean {
		return row.kind === 'alert' && dismissing.has(row.alert.fingerprint);
	}

	async function dismissAlert(alert: Alert) {
		dismissing = new Set(dismissing).add(alert.fingerprint);
		toast = { fingerprint: alert.fingerprint };
		try {
			await ackAlert(alert.fingerprint, { duration_secs: UNTIL_RESOLVED_SECS });
			onchanged?.();
		} catch (cause) {
			dismissing = new Set(dismissing);
			dismissing.delete(alert.fingerprint);
			toast = { error: cause instanceof Error ? cause.message : m.alerts_needs_dismiss_error() };
		}
	}

	async function undoDismiss(fingerprint: string) {
		toast = null;
		dismissing = new Set(dismissing);
		dismissing.delete(fingerprint);
		try {
			await unackAlert(fingerprint);
		} catch {
			// Best effort: the next refresh shows the server's real state either way.
		}
		onchanged?.();
	}

	let dismissedOpen = $state(false);

	const rows = $derived(
		fold(
			sky.needsYou.filter((row) => !isAckedRow(row) && !isSnoozedRow(row) && !isDismissingRow(row))
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
		{@const alert = folded.row.alert}
		<AlertRow
			row={folded.row}
			extra={folded.extra}
			{showOpen}
			{silences}
			{onchanged}
			ondismiss={() => void dismissAlert(alert)}
		/>
	{/if}
{/snippet}

{#snippet snoozedSection()}
	{#if snoozedRows.length > 0}
		<div class={rows.length > 0 ? 'mt-6' : ''}>
			<h3 class="label-tape mb-2 flex items-center gap-2">
				{m.alerts_needs_snoozed()}
				<span class="tnum font-normal text-ink-3">· {snoozedRows.length}</span>
			</h3>
			<p class="mb-2 text-[0.8125rem] text-ink-2">
				{m.alerts_needs_snoozed_hint()}
			</p>
			<div class="grid grid-cols-1 items-stretch gap-2.5 sm:grid-cols-2 xl:grid-cols-3">
				{#each snoozedRows as folded, i (folded.row.key)}
					<div class="rise-in min-w-0" style={`--rise-delay: ${i * 30}ms`}>
						{@render rowView(folded)}
					</div>
				{/each}
			</div>
		</div>
	{/if}
{/snippet}

{#snippet dismissedSection()}
	{#if ackedRows.length > 0}
		<div class={rows.length > 0 || snoozedRows.length > 0 ? 'mt-6' : ''}>
			<button
				type="button"
				class="label-tape mb-2 flex w-full items-center gap-2 text-left"
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
	<div class="space-y-6">
		{#each groups as group (group.key)}
			<div>
				<h3 class="label-tape mb-2">
					{#if group.target}
						<a href={`/targets/${group.target.id}`} class="hover:text-ink hover:underline"
							>{group.name}</a
						>
					{:else}
						{group.name}
					{/if}
				</h3>
				<div class="grid grid-cols-1 items-stretch gap-2.5 sm:grid-cols-2 xl:grid-cols-3">
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

{#if toast}
	{#if 'fingerprint' in toast}
		{@const fingerprint = toast.fingerprint}
		<Toast
			message={m.alerts_needs_toast_dismissed()}
			actionLabel={m.alerts_ignore_undo()}
			onaction={() => void undoDismiss(fingerprint)}
			onclose={() => (toast = null)}
		/>
	{:else}
		<Toast message={toast.error} onclose={() => (toast = null)} />
	{/if}
{/if}
