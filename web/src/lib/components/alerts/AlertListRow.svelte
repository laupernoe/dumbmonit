<script lang="ts">
	/**
	 * One line of the Alerts "Now" list: what is wrong, where, why, since when,
	 * and the two actions that matter (Ack, Open). Everything else — every
	 * affected container or disk, who acknowledged it, why it is suppressed,
	 * Snooze and "Never for this device" — waits behind a click on the row,
	 * which unfolds it in place.
	 *
	 * The status is read from the icon's shape and its hidden word, and from
	 * the group heading above (Warning, Advisory…); the row shows a plate as
	 * well whenever the heading does not already say it.
	 *
	 * In selection mode a click on the row picks it instead of unfolding it.
	 */
	import type { Silence } from '#lib/api/index.js';
	import type { Tone } from '#lib/ui/index.js';
	import { Button, Plate } from '#lib/ui/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { alertDetail, coveringSilence } from './helpers';
	import type { Folded } from './list';
	import { m } from '#lib/paraglide/messages.js';
	import {
		AlertTriangle,
		BellOff,
		CheckCheck,
		ChevronDown,
		CircleAlert,
		CircleDashed,
		Info,
		ArrowUpRight,
		Unplug
	} from 'lucide-svelte';
	import AckControl from './AckControl.svelte';
	import SnoozeControl from './SnoozeControl.svelte';
	import IgnoreControl from './IgnoreControl.svelte';

	interface Props {
		folded: Folded;
		/** Show the row's plate (the group heading does not already carry its word). */
		showPlate?: boolean;
		/** "Open" on the row itself (off when the group heading already links the monitor). */
		showOpen?: boolean;
		silences?: Silence[];
		expanded?: boolean;
		ontoggle?: () => void;
		selecting?: boolean;
		selected?: boolean;
		onselect?: (on: boolean) => void;
		onchanged?: () => void;
	}

	let {
		folded,
		showPlate = false,
		showOpen = true,
		silences = [],
		expanded = false,
		ontoggle,
		selecting = false,
		selected = false,
		onselect,
		onchanged
	}: Props = $props();

	const row = $derived(folded.row);
	const alert = $derived(row.kind === 'alert' ? row.alert : null);
	const target = $derived(row.target);
	const detailsId = $derived(`alert-details-${row.key.replace(/[^a-zA-Z0-9_-]/g, '_')}`);

	/** What is wrong: the rule's name, or the device's state word. */
	const title = $derived(alert ? alert.rule_name || alert.rule_uid : row.plate);
	/** Where: the device, or "No monitor" for an instance-wide alert. */
	const where = $derived(
		target?.name ??
			(alert && alert.target_id !== null
				? m.alerts_scope_device({ id: alert.target_id })
				: m.alerts_needs_no_device())
	);
	/** Every affected series, in full (the details list them all). */
	const details = $derived(
		row.kind === 'alert'
			? [row.alert, ...folded.extra].map((a) => alertDetail(a, row.rule)).filter(Boolean)
			: []
	);
	/** Why, in one line: the first detail ("and 7 more"), or the device's error. */
	const why = $derived.by(() => {
		if (row.kind === 'device') return row.detail;
		if (details.length === 0) return '';
		const more = folded.extra.length;
		return more > 0 ? m.alerts_list_and_more({ first: details[0], count: more }) : details[0];
	});

	const ICON: Record<Tone, typeof AlertTriangle> = {
		warning: AlertTriangle,
		advisory: CircleAlert,
		info: Info,
		ghost: CircleDashed,
		signal: Info,
		muted: Unplug
	};
	const Icon = $derived(
		alert?.acked ? CheckCheck : alert?.silenced ? BellOff : ICON[row.tone]
	);
	const ICON_INK: Record<Tone, string> = {
		warning: 'text-warning-ink',
		advisory: 'text-advisory-ink',
		info: 'text-info-ink',
		ghost: 'text-ink-3',
		signal: 'text-signal-ink',
		muted: 'text-ink-3'
	};
	const quiet = $derived(row.tone === 'muted' || row.tone === 'ghost');

	function timeOf(value: string | null | undefined): string {
		if (!value) return '';
		const at = new Date(value);
		if (Number.isNaN(at.getTime())) return '';
		return at.toDateString() === new Date().toDateString()
			? at.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
			: formatDateTime(at);
	}
	const snoozeUntil = $derived(alert ? (coveringSilence(alert, silences)?.active_until ?? null) : null);
	const ackedLine = $derived(
		alert?.acked
			? m.alerts_row_acked({
					by: alert.acked_by ? m.alerts_row_acked_by({ name: alert.acked_by }) : '',
					until: timeOf(alert.acked_until) ? m.alerts_row_until({ time: timeOf(alert.acked_until) }) : '',
					note: alert.ack_note ? m.alerts_row_acked_note({ note: alert.ack_note }) : ''
				})
			: ''
	);
	/** The message split around the parent's name, so the name stays a link. */
	const suppressedParts = $derived(m.alerts_row_suppressed_by({ parent: '\u0000' }).split('\u0000'));
	const suppressed = $derived(alert?.effective_phase === 'suppressed');

	function onmain() {
		if (selecting) {
			if (alert) onselect?.(!selected);
		} else {
			ontoggle?.();
		}
	}
</script>

<li class={`group/row relative ${selected ? 'bg-signal-soft/60' : ''}`}>
	<div class="flex min-h-14 items-center gap-1 pr-1.5 pl-3 sm:gap-2 sm:pr-2 sm:pl-4">
		{#if selecting}
			<label class="-ml-1 inline-flex size-10 shrink-0 cursor-pointer items-center justify-center">
				<input
					type="checkbox"
					class="size-4 accent-[var(--c-signal)] disabled:opacity-30"
					checked={selected}
					disabled={!alert}
					aria-label={m.alerts_bulk_select_row({ name: `${title} — ${where}` })}
					onchange={(e) => onselect?.((e.currentTarget as HTMLInputElement).checked)}
				/>
			</label>
		{/if}

		<button
			type="button"
			class="flex min-w-0 flex-1 items-center gap-3 self-stretch rounded-md py-2.5 text-left focus-visible:outline-offset-2"
			aria-expanded={selecting ? undefined : expanded}
			aria-controls={selecting ? undefined : detailsId}
			aria-pressed={selecting && alert ? selected : undefined}
			onclick={onmain}
		>
			<Icon class={`size-[1.125rem] shrink-0 ${ICON_INK[row.tone]}`} aria-hidden="true" />
			<span class="sr-only">{row.plate}:</span>
			<span class="min-w-0 flex-1">
				<span class="flex min-w-0 items-center gap-2">
					<span class={`min-w-0 font-semibold max-sm:line-clamp-2 sm:truncate ${quiet ? 'text-ink-2' : 'text-ink'}`}>{title}</span>
					{#if showPlate && row.kind === 'alert'}
						<Plate tone={row.tone} label={row.plate} bare class="shrink-0 max-sm:hidden" />
					{/if}
					{#if row.since}
						<time class="tnum ml-auto shrink-0 text-[0.75rem] text-ink-2 sm:hidden" datetime={row.since}>
							{formatRelative(row.since)}
						</time>
					{/if}
				</span>
				<span class="mt-0.5 flex min-w-0 items-baseline gap-1.5 text-[0.8125rem] text-ink-2">
					<span class={`min-w-0 truncate font-medium ${why ? 'max-w-[55%] shrink-0' : ''}`}>{where}</span>
					{#if why}
						<span class="text-ink-3" aria-hidden="true">·</span>
						<span class={`min-w-0 truncate ${row.kind === 'device' && row.state !== 'misconfigured' ? 'text-warning-ink' : ''}`}>{why}</span>
					{/if}
				</span>
			</span>
			{#if row.since}
				<time
					class="tnum shrink-0 text-[0.8125rem] text-ink-2 max-sm:hidden"
					datetime={row.since}
					title={formatDateTime(row.since)}
				>
					{formatRelative(row.since)}
				</time>
			{/if}
		</button>

		{#if !selecting}
			<div class="flex shrink-0 items-center gap-0.5">
				{#if alert}
					<AckControl {alert} {onchanged} floating />
				{/if}
				{#if target && showOpen}
					<Button size="sm" variant="ghost" href={`/targets/${target.id}`} class="max-sm:hidden">
						{m.alerts_list_open()}
					</Button>
				{/if}
				<button
					type="button"
					tabindex="-1"
					aria-hidden="true"
					class="inline-flex h-10 w-6 items-center justify-center rounded-lg text-ink-3 transition-colors hover:bg-surface-2 hover:text-ink sm:size-8"
					onclick={() => ontoggle?.()}
				>
					<ChevronDown class={`size-4 transition-transform duration-200 ${expanded ? 'rotate-180' : ''}`} />
				</button>
			</div>
		{/if}
	</div>

	{#if expanded && !selecting}
		<div id={detailsId} class="border-t border-line bg-canvas-deep/40 px-4 pt-3 pb-4 text-[0.8125rem] sm:pl-[3.25rem]">
			<dl class="grid gap-x-6 gap-y-1.5 sm:grid-cols-[auto_minmax(0,1fr)]">
				<dt class="text-ink-3">{m.alerts_list_detail_state()}</dt>
				<dd class="text-ink">{row.plate}</dd>
				{#if row.since}
					<dt class="text-ink-3">
						{row.kind === 'device' ? m.alerts_list_detail_last_report() : m.alerts_list_detail_since()}
					</dt>
					<dd class="tnum text-ink">{formatDateTime(row.since)} · {formatRelative(row.since)}</dd>
				{:else if row.kind === 'device'}
					<dt class="text-ink-3">{m.alerts_list_detail_last_report()}</dt>
					<dd class="text-ink">{m.alerts_device_never_reported()}</dd>
				{/if}
				{#if row.kind === 'device'}
					<dt class="text-ink-3">{m.alerts_list_detail_address()}</dt>
					<dd class="break-all text-ink">{row.target.address}</dd>
					{#if row.detail}
						<dt class="text-ink-3">{m.alerts_list_detail_cause()}</dt>
						<dd class="break-words text-ink">{row.detail}</dd>
					{/if}
				{/if}
				{#if details.length > 0}
					<dt class="text-ink-3">{m.alerts_list_detail_affected({ count: details.length })}</dt>
					<dd>
						<ul class="space-y-0.5">
							{#each details as line, i (i)}
								<li class="break-words text-ink">{line}</li>
							{/each}
						</ul>
					</dd>
				{/if}
			</dl>

			{#if alert?.learning}
				<p class="mt-2 text-ink-2">
					{suppressed ? m.alerts_row_learning_suppressed() : m.alerts_row_learning({ days: 14 })}
				</p>
			{/if}
			{#if suppressed && row.kind === 'alert' && row.parent && row.parent.id === alert?.target_id}
				<p class="mt-2 text-ink-2">{m.alerts_row_suppressed_self()}</p>
			{:else if suppressed && row.kind === 'alert' && row.parent}
				<p class="mt-2 text-ink-2">
					{suppressedParts[0]}<a href={`/targets/${row.parent.id}`} class="font-medium text-ink underline decoration-line-strong underline-offset-2 hover:decoration-ink">{row.parent.name}</a>{suppressedParts[1]}
				</p>
			{/if}
			{#if alert?.acked}
				<p class="mt-2 text-ink-2">{ackedLine}</p>
			{:else if alert?.silenced}
				<p class="mt-2 text-ink-2">
					{m.alerts_row_snoozed({
						until: timeOf(snoozeUntil) ? m.alerts_row_until({ time: timeOf(snoozeUntil) }) : ''
					})}
				</p>
			{/if}

			<div class="mt-3 flex flex-wrap items-start gap-2">
				{#if target}
					<Button size="sm" variant="secondary" href={`/targets/${target.id}`}>
						{m.alerts_row_open_device()}
						<ArrowUpRight class="size-3.5" aria-hidden="true" />
					</Button>
				{/if}
				{#if alert && !alert.acked}
					<SnoozeControl {alert} {target} {silences} {onchanged} />
					{#if row.kind === 'alert'}
						<IgnoreControl rule={row.rule} {target} {onchanged} />
					{/if}
				{/if}
			</div>
		</div>
	{/if}
</li>
