<script lang="ts">
	/**
	 * One active alert, read the weather-bulletin way, as a tile of the
	 * "Needs you" grid: plate and age on top, the rule and its device in the
	 * middle, the actions on a footer rule so tiles of a row line up.
	 *
	 * The plate word and tone come from the sky helper: a firing alert wears its
	 * severity, a pending one is "Building up", a suppressed one is dimmed and
	 * names the parent that masks it, a learning baseline rule stays quiet on
	 * purpose and says so. An acknowledged alert wears "Acked" with who and
	 * until when — still a problem, a known one. Every row can be acknowledged
	 * or silenced for an hour, and — on the Alerts page — opened on its device.
	 */
	import type { Alert, Target } from '$lib/api';
	import type { SkyRow } from '$lib/components/overview/sky';
	import { Button, Plate } from '$lib/ui';
	import { formatRelative, formatDateTime } from '$lib/format';
	import { alertDetail, severityTone, severityWord, TONE_BAR } from './helpers';
	import AckControl from './AckControl.svelte';

	interface Props {
		row: Extract<SkyRow, { kind: 'alert' }>;
		/** Sibling alerts folded into this row (same device, rule and phase). */
		extra?: Alert[];
		showOpen?: boolean;
		silencing?: boolean;
		onsilence: (alert: Alert, target: Target) => void;
		/** An acknowledgement was made or lifted: the page should refresh its alerts. */
		onackchange?: (alert: Alert) => void;
	}

	let {
		row,
		extra = [],
		showOpen = false,
		silencing = false,
		onsilence,
		onackchange
	}: Props = $props();

	const alert = $derived(row.alert);
	const target = $derived(row.target);
	const suppressed = $derived(alert.effective_phase === 'suppressed');
	const acked = $derived(alert.acked);
	const firing = $derived(alert.effective_phase === 'firing' && !alert.learning && !acked);
	const detail = $derived(alertDetail(alert, row.rule));
	/** "Acked by admin until 14:30", the time alone when it is today. */
	const ackedUntil = $derived.by(() => {
		if (!alert.acked_until) return '';
		const until = new Date(alert.acked_until);
		if (Number.isNaN(until.getTime())) return '';
		const sameDay = until.toDateString() === new Date().toDateString();
		return sameDay
			? until.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
			: formatDateTime(until);
	});
	/** Details of the folded siblings, capped so the row stays one glance. */
	const MAX_SHOWN = 3;
	const siblings = $derived(extra.slice(0, MAX_SHOWN).map((a) => alertDetail(a, row.rule)).filter(Boolean));
	const hidden = $derived(extra.length - siblings.length);
</script>

<article
	class={`relative flex h-full min-w-0 flex-col overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface py-3 pr-4 pl-5 shadow-lift transition ${suppressed || acked ? 'opacity-60' : ''}`}
	aria-label={`${row.plate}: ${alert.rule_name || alert.rule_uid}${target ? ` on ${target.name}` : ''}`}
>
	<span class={`absolute inset-y-0 left-0 w-1 ${TONE_BAR[row.tone]}`} aria-hidden="true"></span>

	<div class="flex items-start justify-between gap-2">
		<div class="flex min-w-0 flex-wrap items-center gap-1.5">
			<Plate tone={row.tone} label={row.plate} pulse={firing} />
			{#if acked && !alert.learning}
				<Plate tone={severityTone(alert.severity)} label={severityWord(alert.severity)} bare />
			{/if}
			{#if extra.length > 0}
				<span class="tnum rounded-[var(--radius-plate)] border border-line bg-surface-2 px-1.5 py-0.5 text-[0.6875rem] font-semibold text-ink-2"
					>×{extra.length + 1}</span
				>
			{/if}
		</div>
		{#if row.since}
			<span class="tnum shrink-0 pt-0.5 text-[0.75rem] whitespace-nowrap text-ink-2" title={formatDateTime(row.since)}>
				since {formatRelative(row.since)}
			</span>
		{/if}
	</div>

	<p class="mt-2 line-clamp-2 font-semibold break-words text-ink" title={alert.rule_name || alert.rule_uid}>
		{alert.rule_name || alert.rule_uid}
	</p>

	<div class="mt-0.5 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 text-[0.8125rem] text-ink-2">
		{#if target}
			<a href={`/targets/${target.id}`} class="min-w-0 max-w-full truncate font-medium text-ink-2 hover:text-ink hover:underline"
				>{target.name}</a
			>
		{:else if alert.target_id !== null}
			<span>Device {alert.target_id}</span>
		{/if}
		{#if detail}
			<span class="text-ink-3" aria-hidden="true">·</span>
			<span class="min-w-0 max-w-full truncate" title={detail}>{detail}</span>
		{/if}
	</div>
	{#if siblings.length > 0}
		<p class="mt-1 line-clamp-2 text-[0.8125rem] text-ink-2">
			Also {siblings.join(' · ')}{#if hidden > 0}{' '}· +{hidden} more{/if}
		</p>
	{/if}

	{#if suppressed && alert.learning}
		<p class="mt-1 text-[0.8125rem] text-ink-2">Silent while it learns the normal shape.</p>
	{:else if alert.learning}
		<p class="mt-1 text-[0.8125rem] text-ink-2">Silent for its first 14 days of baseline.</p>
	{/if}

	{#if suppressed && row.parent}
		<p class="mt-1 text-[0.8125rem] text-ink-2">
			Suppressed by
			<a href={`/targets/${row.parent.id}`} class="font-medium hover:text-ink hover:underline"
				>{row.parent.name}</a
			> — probably a consequence, not a separate problem.
		</p>
	{/if}

	{#if acked}
		<p class="mt-1 text-[0.8125rem] text-ink-2" title={formatDateTime(alert.acked_until)}>
			Acked{#if alert.acked_by}
				by <span class="font-medium">{alert.acked_by}</span>{/if}{#if ackedUntil}
				until <span class="tnum">{ackedUntil}</span>{/if}{#if alert.ack_note}
				— {alert.ack_note}{/if}. Reminders paused; you will still hear when it resolves.
		</p>
	{/if}

	<div class="mt-auto pt-3">
		<div class="flex flex-wrap items-center gap-2 border-t border-line pt-2.5">
			<AckControl {alert} onchanged={onackchange} />
			{#if target}
				<Button
					size="sm"
					variant="ghost"
					loading={silencing}
					onclick={() => onsilence(alert, target)}
				>
					Silence 1 h
				</Button>
				{#if showOpen}
					<Button size="sm" variant="secondary" href={`/targets/${target.id}`} class="ml-auto">Open device</Button>
				{/if}
			{/if}
		</div>
	</div>
</article>
