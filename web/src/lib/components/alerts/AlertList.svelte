<script lang="ts">
	/**
	 * Alerts → Now: everything that needs you, as a calm list.
	 *
	 * Rows come pre-ordered from the sky helper and are folded (one rule on
	 * eight containers is one row). They are gathered by severity — Warning,
	 * Advisory, Info, Building up — or, at the reader's choice (remembered on
	 * this browser), by monitor. Each group folds away; the ones that need
	 * nothing from you (suppressed by a parent, snoozed, acknowledged) start
	 * folded. One line per problem; a click unfolds its details.
	 *
	 * Selection is a mode, off by default: "Select" shows a checkbox per row
	 * and a floating action bar (acknowledge, unacknowledge, ignore for the
	 * device), so nothing on the page moves when it turns on. Escape leaves it.
	 */
	import { ackAlert, putRuleOverride, unackAlert, type Silence } from '#lib/api/index.js';
	import type { Sky } from '#lib/components/overview/sky.js';
	import { Button, Confirm, EmptyState, Plate, SelectionBar } from '#lib/ui/index.js';
	import type { Tone } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import {
		AlertTriangle,
		ArrowUpRight,
		BellOff,
		CheckCheck,
		ChevronRight,
		CircleAlert,
		CircleDashed,
		CloudSun,
		Info,
		ListChecks,
		Unplug
	} from 'lucide-svelte';
	import { UNTIL_RESOLVED_SECS, severityWord } from './helpers';
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
	import {
		BUCKETS,
		BUCKET_TONE,
		QUIET_BUCKETS,
		bucketOf,
		deviceKey,
		fingerprintsOf,
		fold,
		type Bucket,
		type Folded
	} from './list';
	import AlertListRow from './AlertListRow.svelte';

	interface Props {
		sky: Sky;
		silences?: Silence[];
		/** An acknowledgement, a snooze or an ignore was made or lifted: refresh the alerts. */
		onchanged?: () => void;
	}

	let { sky, silences = [], onchanged }: Props = $props();

	type GroupBy = 'severity' | 'device';
	const GROUP_KEY = 'dumbmonit-alerts-group-by';
	function readGroupBy(): GroupBy {
		try {
			return localStorage.getItem(GROUP_KEY) === 'device' ? 'device' : 'severity';
		} catch {
			return 'severity';
		}
	}
	let groupBy = $state<GroupBy>(readGroupBy());
	function setGroupBy(next: GroupBy) {
		groupBy = next;
		try {
			localStorage.setItem(GROUP_KEY, next);
		} catch {
			// Private mode: the choice lasts for this visit only.
		}
	}

	const folded = $derived(fold(sky.needsYou));

	interface Group {
		key: string;
		label: string;
		tone: Tone;
		bucket?: Bucket;
		/** A plate next to the label (the device's worst state, in "by monitor"). */
		plate?: { tone: Tone; label: string };
		href?: string;
		items: Folded[];
		quiet: boolean;
	}

	const BUCKET_LABEL = $derived<Record<Bucket, string>>({
		warning: severityWord('critical'),
		advisory: severityWord('warning'),
		info: severityWord('info'),
		building: m.overview_sky_plate_building(),
		suppressed: m.overview_sky_plate_suppressed(),
		snoozed: m.alerts_needs_snoozed(),
		acked: m.alerts_needs_dismissed()
	});

	/** In "by monitor", a firing row's severity reads from its icon; only the odd states get a word. */
	const SEVERITY_WORDS = $derived(new Set([severityWord('critical'), severityWord('warning'), severityWord('info')]));

	const groups = $derived.by<Group[]>(() => {
		const byBucket = new Map<Bucket, Folded[]>();
		for (const item of folded) {
			const bucket = bucketOf(item.row);
			byBucket.set(bucket, [...(byBucket.get(bucket) ?? []), item]);
		}
		const bucketGroup = (bucket: Bucket): Group => ({
			key: `bucket:${bucket}`,
			label: BUCKET_LABEL[bucket],
			tone: BUCKET_TONE[bucket],
			bucket,
			items: byBucket.get(bucket) ?? [],
			quiet: QUIET_BUCKETS.has(bucket)
		});
		if (groupBy === 'severity') {
			return BUCKETS.filter((b) => byBucket.has(b)).map(bucketGroup);
		}
		// By monitor: the live problems per device (suppressed ones included,
		// dimmed), then the known ones in their own folded groups as above.
		const devices = new Map<string, Group>();
		for (const item of folded) {
			const bucket = bucketOf(item.row);
			if (bucket === 'snoozed' || bucket === 'acked') continue;
			const key = deviceKey(item.row);
			let group = devices.get(key);
			if (!group) {
				const target = item.row.target;
				group = {
					key: `device:${key}`,
					label:
						target?.name ??
						(key === 'none' ? m.alerts_needs_no_device() : m.alerts_scope_device({ id: Number(key) })),
					tone: item.row.tone,
					plate: { tone: item.row.tone, label: item.row.plate },
					href: target ? `/targets/${target.id}` : undefined,
					items: [],
					quiet: false
				};
				devices.set(key, group);
			}
			group.items.push(item);
		}
		const known = (['snoozed', 'acked'] as const).filter((b) => byBucket.has(b)).map(bucketGroup);
		return [...devices.values(), ...known];
	});

	/** Groups the reader opened or closed; the rest follow their default. */
	let toggled = $state<Record<string, boolean>>({});
	const isOpen = (group: Group) => toggled[group.key] ?? !group.quiet;

	let expandedKey = $state<string | null>(null);

	const activeCount = $derived(groups.filter((g) => !g.quiet).reduce((n, g) => n + g.items.length, 0));

	const GROUP_ICON: Record<string, typeof AlertTriangle> = {
		warning: AlertTriangle,
		advisory: CircleAlert,
		info: Info,
		building: CircleDashed,
		suppressed: Unplug,
		snoozed: BellOff,
		acked: CheckCheck
	};
	const TONE_INK: Record<Tone, string> = {
		warning: 'text-warning-ink',
		advisory: 'text-advisory-ink',
		info: 'text-info-ink',
		ghost: 'text-ink-3',
		signal: 'text-signal-ink',
		muted: 'text-ink-3'
	};

	// --- Selection mode ---------------------------------------------------------

	const canSelect = $derived(auth.canOperate);
	let selecting = $state(false);
	let selected = $state<Set<string>>(new Set());
	let bulkBusy = $state<BulkAction | null>(null);
	let bulkMessage = $state<{ text: string; error: boolean } | null>(null);

	function itemOf(item: Folded): SelectableItem | null {
		const row = item.row;
		if (row.kind !== 'alert') return null;
		return {
			key: row.key,
			fingerprints: fingerprintsOf(item),
			acked: row.alert.acked,
			ruleId: row.rule?.id,
			targetId: row.alert.target_id
		};
	}
	/** Every alert row on screen; folded groups are left out. */
	const items = $derived(
		groups
			.filter(isOpen)
			.flatMap((g) => g.items)
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
		// Rows that left the screen (resolved, refreshed, folded) leave the selection too.
		const kept = pruneSelection(selected, itemKeys);
		if (kept.size !== selected.size) selected = kept;
	});

	function startSelecting() {
		selecting = true;
		expandedKey = null;
		bulkMessage = null;
	}
	function stopSelecting() {
		selecting = false;
		selected = new Set();
		bulkMessage = null;
	}

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
	/** A new choice replaces the last result in the bar. */
	function choose(next: Set<string>) {
		selected = next;
		bulkMessage = null;
	}
</script>

{#if sky.quiet}
	<EmptyState
		tone="signal"
		mascot="happy"
		title={sky.counts.devices === 0 ? m.alerts_needs_empty_no_devices() : m.alerts_needs_empty_quiet()}
		description={sky.counts.devices === 0
			? m.alerts_needs_quiet_no_devices()
			: sky.counts.waiting > 0
				? sky.counts.waiting === 1
					? m.alerts_needs_quiet_waiting_one()
					: m.alerts_needs_quiet_waiting_many({ count: sky.counts.waiting })
				: m.alerts_needs_quiet_all_reporting()}
		primary={sky.counts.devices === 0 ? { label: m.alerts_list_add_monitor(), href: '/targets/new' } : undefined}
		secondary={sky.counts.devices === 0 ? undefined : { label: m.alerts_list_see_history(), href: '#history' }}
	/>
{:else}
	<div class="mb-4 flex flex-wrap items-center justify-between gap-x-4 gap-y-3">
		{#if activeCount === 0}
			<p class="flex items-center gap-2 font-semibold text-ink">
				<CloudSun class="size-[1.125rem] text-signal-ink" aria-hidden="true" />
				{m.alerts_list_known_only()}
			</p>
		{:else}
			<p class="display text-[1.25rem] leading-tight text-ink">{sky.sentence}</p>
		{/if}
		<div class="flex items-center gap-2">
			<div class="inline-flex rounded-lg border border-line bg-surface-2 p-0.5" role="group" aria-label={m.alerts_list_group_by()}>
				{#each [['severity', m.alerts_list_group_severity()], ['device', m.alerts_list_group_device()]] as [id, label] (id)}
					<button
						type="button"
						aria-pressed={groupBy === id}
						class={`inline-flex h-9 items-center rounded-md px-3 text-[0.8125rem] font-semibold transition sm:h-8 ${groupBy === id ? 'bg-surface text-ink shadow-lift' : 'text-ink-2 hover:text-ink'}`}
						onclick={() => setGroupBy(id as GroupBy)}
					>
						{label}
					</button>
				{/each}
			</div>
			{#if canSelect}
				<Button
					size="sm"
					variant="ghost"
					aria-pressed={selecting}
					class={selecting ? 'bg-surface-2 text-ink' : ''}
					onclick={() => (selecting ? stopSelecting() : startSelecting())}
				>
					<ListChecks class="size-4" aria-hidden="true" />
					{m.alerts_list_select()}
				</Button>
			{/if}
		</div>
	</div>

	<div class="grid grid-cols-1 gap-5">
		{#each groups as group (group.key)}
			{@const open = isOpen(group)}
			{@const GroupIcon = group.bucket ? GROUP_ICON[group.bucket] : null}
			<section aria-labelledby={`${group.key}-label`}>
				<div class="flex items-center gap-2">
					<button
						type="button"
						class="-ml-2 flex min-h-10 min-w-0 flex-1 items-center gap-2 rounded-md px-2 text-left hover:bg-surface-2/60"
						aria-expanded={open}
						onclick={() => (toggled = { ...toggled, [group.key]: !open })}
					>
						<ChevronRight
							class={`size-4 shrink-0 text-ink-3 transition-transform duration-200 ${open ? 'rotate-90' : ''}`}
							aria-hidden="true"
						/>
						{#if GroupIcon}
							<GroupIcon class={`size-4 shrink-0 ${TONE_INK[group.tone]}`} aria-hidden="true" />
						{/if}
						<h3 id={`${group.key}-label`} class={`truncate text-[0.9375rem] font-semibold ${group.quiet ? 'text-ink-2' : 'text-ink'}`}>
							{group.label}
						</h3>
						<span class="tnum shrink-0 text-[0.8125rem] text-ink-3">{group.items.reduce((n, item) => n + 1 + item.extra.length, 0)}</span>
						{#if group.plate && !group.bucket}
							<Plate tone={group.plate.tone} label={group.plate.label} class="ml-1 shrink-0 max-sm:hidden" />
						{/if}
					</button>
					{#if group.href}
						<Button size="sm" variant="ghost" href={group.href} aria-label={m.alerts_list_open_named({ name: group.label })}>
							<span class="max-sm:hidden">{m.alerts_list_open()}</span>
							<ArrowUpRight class="size-3.5" aria-hidden="true" />
						</Button>
					{/if}
				</div>
				{#if open}
					{#if group.bucket === 'acked'}
						<p class="mt-1 mb-2 pl-8 text-[0.8125rem] text-ink-2">{m.alerts_needs_dismissed_hint()}</p>
					{/if}
					<ul class="mt-1.5 divide-y divide-line rounded-[var(--radius-card)] border border-line bg-surface shadow-lift [&>li:first-child]:rounded-t-[var(--radius-card)] [&>li:last-child]:rounded-b-[var(--radius-card)]">
						{#each group.items as item (item.row.key)}
							<AlertListRow
								folded={item}
								showPlate={item.row.plate !== group.label && !(groupBy === 'device' && SEVERITY_WORDS.has(item.row.plate))}
								showOpen={groupBy === 'severity'}
								{silences}
								{onchanged}
								{selecting}
								selected={selected.has(item.row.key)}
								onselect={(on) => choose(toggleKey(selected, item.row.key, on))}
								expanded={expandedKey === item.row.key}
								ontoggle={() => (expandedKey = expandedKey === item.row.key ? null : item.row.key)}
							/>
						{/each}
					</ul>
				{/if}
			</section>
		{/each}
	</div>

	{#if selecting}
		<!-- Room under the last row, so the floating bar never covers it. -->
		<div class="h-24" aria-hidden="true"></div>
		<SelectionBar
			label={m.alerts_bulk_label()}
			countLabel={m.alerts_bulk_selected({ count: selectedCount })}
			selectAllLabel={m.alerts_bulk_select_all({ count: items.length })}
			exitLabel={m.alerts_list_select_done()}
			selection={allState}
			empty={items.length === 0}
			message={bulkMessage}
			onselectall={(on) => choose(setMany(selected, itemKeys, on))}
			onexit={stopSelecting}
		>
			<Button
				size="sm"
				variant="secondary"
				disabled={canAck === 0 || bulkBusy !== null}
				loading={bulkBusy === 'ack'}
				onclick={() => void bulk('ack')}
			>
				{m.alerts_bulk_ack()}
			</Button>
			{#if canUnack > 0}
				<Button
					size="sm"
					variant="ghost"
					disabled={bulkBusy !== null}
					loading={bulkBusy === 'unack'}
					onclick={() => void bulk('unack')}
				>
					{m.alerts_bulk_unack()}
				</Button>
			{/if}
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
		</SelectionBar>
	{/if}
{/if}
