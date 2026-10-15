<script lang="ts">
	/**
	 * "Needs you", the Overview's way: one compact line per problem, in the sky
	 * helper's order (unreachable devices and warnings first), cut to the
	 * number of whole lines that fit the room it is given. What does not fit is
	 * counted ("+3 more") and leads to /alerts, where every alert has its
	 * actions (ack, snooze, ignore); here a line opens its device.
	 *
	 * Acknowledged and snoozed alerts are already known: they leave the list
	 * for a single quiet count at the bottom. The plate carries the word, the
	 * bar only colours the margin, so nothing is read by colour alone.
	 *
	 * `rows` caps the visible lines; the Overview computes it from the height
	 * it has (desktop) or passes a fixed number (phones).
	 */
	import type { Alert } from '#lib/api/index.js';
	import { isAckedRow, isSnoozedRow, type Sky, type SkyRow } from './sky.js';
	import { EmptyState, Plate } from '#lib/ui/index.js';
	import { formatRelative, formatDateTime } from '#lib/format.js';
	import { alertDetail, TONE_BAR } from '#lib/components/alerts/helpers';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		sky: Sky;
		/** Relative time of the last check, for the quiet state's second line. */
		checkedLabel?: string;
		/** How many lines fit, the "+N more" line included when it shows. */
		rows: number;
		/** Fill the given height (desktop fit) instead of the natural one. */
		fill?: boolean;
	}
	let { sky, checkedLabel, rows, fill = false }: Props = $props();

	/** Several series of one rule on one device are one problem (see NeedsYouList). */
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
				last?.row.kind === 'alert' &&
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

	const active = $derived(fold(sky.needsYou.filter((row) => !isAckedRow(row) && !isSnoozedRow(row))));
	const snoozed = $derived(fold(sky.needsYou.filter((row) => isSnoozedRow(row) && !isAckedRow(row))).length);
	const dismissed = $derived(fold(sky.needsYou.filter(isAckedRow)).length);
	const known = $derived(snoozed > 0 || dismissed > 0);

	/** Lines left for problems once the footer line (count of the rest, known ones) takes its own. */
	const shown = $derived.by(() => {
		const room = Math.max(1, rows);
		if (active.length <= room && !known) return active.length;
		return Math.min(active.length, Math.max(1, room - 1));
	});
	const hidden = $derived(active.length - shown);

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

	function href(row: SkyRow): string {
		const id = row.kind === 'device' ? row.target.id : row.alert.target_id;
		return id === null ? '/alerts' : `/targets/${id}`;
	}
	function title(row: SkyRow): string {
		return row.kind === 'device' ? row.target.name : row.alert.rule_name || row.alert.rule_uid;
	}
	function subtitle(folded: Folded): string {
		const row = folded.row;
		if (row.kind === 'device') return row.detail || row.target.address;
		const device =
			row.target?.name ??
			(row.alert.target_id === null ? m.alerts_needs_no_device() : m.alerts_scope_device({ id: row.alert.target_id }));
		const detail = alertDetail(row.alert, row.rule);
		return detail ? `${device} · ${detail}` : device;
	}
	function dimmed(row: SkyRow): boolean {
		return row.kind === 'alert' && row.alert.effective_phase === 'suppressed';
	}
</script>

{#if sky.quiet}
	<EmptyState
		tone="signal"
		mascot="happy"
		title={sky.counts.devices === 0 ? m.alerts_needs_empty_no_devices() : m.alerts_needs_empty_quiet()}
		description={quietDescription}
		class={fill ? 'h-full !py-4' : ''}
	/>
{:else}
	<ul class="flex flex-col gap-1.5">
		{#each active.slice(0, shown) as folded, i (folded.row.key)}
			{@const row = folded.row}
			<li class="rise-in min-w-0" style={`--rise-delay: ${i * 30}ms`}>
				<a
					href={href(row)}
					class={`group relative flex h-14 min-w-0 flex-col justify-center gap-0.5 overflow-hidden rounded-lg border border-line bg-surface py-1.5 pr-3 pl-4 shadow-lift transition hover:-translate-y-px hover:border-line-strong motion-reduce:transition-none motion-reduce:hover:translate-y-0 ${dimmed(row) ? 'opacity-60' : ''}`}
				>
					<span class={`absolute inset-y-0 left-0 w-1 ${TONE_BAR[row.tone]}`} aria-hidden="true"></span>
					<span class="flex min-w-0 items-center gap-2">
						<Plate tone={row.tone} label={row.plate} size="sm" class="shrink-0" />
						{#if folded.extra.length > 0}
							<span class="tnum shrink-0 text-[0.75rem] font-semibold text-ink-2">×{folded.extra.length + 1}</span>
						{/if}
						<span class="min-w-0 flex-1 truncate font-semibold text-ink group-hover:underline" title={title(row)}>{title(row)}</span>
						{#if row.since}
							<span class="tnum shrink-0 text-[0.75rem] whitespace-nowrap text-ink-2" title={formatDateTime(row.since)}>
								{formatRelative(row.since)}
							</span>
						{/if}
					</span>
					<span class="truncate text-[0.8125rem] text-ink-2" title={subtitle(folded)}>{subtitle(folded)}</span>
				</a>
			</li>
		{/each}
		{#if hidden > 0 || known}
			<li class="flex min-h-10 flex-wrap items-center gap-x-4 gap-y-1 text-[0.8125rem]">
				{#if hidden > 0}
					<a href="/alerts" class="inline-flex min-h-10 items-center font-semibold text-ink underline decoration-line-strong underline-offset-2 hover:decoration-ink">
						{m.overview_needs_more({ count: hidden })}
					</a>
				{/if}
				{#if known}
					<a href="/alerts" class="tnum inline-flex min-h-10 items-center text-ink-2 hover:text-ink hover:underline">
						{#if snoozed > 0}{m.alerts_needs_snoozed()} · {snoozed}{/if}{#if snoozed > 0 && dismissed > 0}&ensp;{/if}{#if dismissed > 0}{m.alerts_needs_dismissed()} · {dismissed}{/if}
					</a>
				{/if}
			</li>
		{/if}
	</ul>
{/if}
