<script lang="ts">
	/**
	 * "Needs attention": only what is broken now (see `attention.ts`), worst
	 * first, each row with its quick action — acknowledge or snooze an alert,
	 * silence a monitor for an hour — then the week ahead in one line at the
	 * foot (what expires in the next seven days). With nothing broken, the
	 * pigeon sleeps and the card says so. Phones keep the first five rows.
	 */
	import { ArrowRight } from 'lucide-svelte';
	import { createSilence, listAlertRules, toApiError, type AlertRule, type Deadline, type Target } from '#lib/api/index.js';
	import { listDeadlines } from '#lib/api/deadlines.js';
	import AlertActions from '#lib/components/alerts/AlertActions.svelte';
	import { quickSilencePayload, TONE_BAR } from '#lib/components/alerts/helpers.js';
	import { kindName, whenLabel } from '#lib/components/deadlines/labels.js';
	import Mascot from '#lib/components/Mascot.svelte';
	import { formatRelative } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Plate } from '#lib/ui/index.js';
	import { readSky, type SkyRow } from './sky.js';
	import { attentionRows, weekAhead } from './attention.js';

	/** Rows shown on a larger screen; phones stop at five (CSS). */
	const LIMIT = 8;
	const PHONE_LIMIT = 5;
	const WEEK_SHOWN = 3;

	let rules = $state<AlertRule[]>([]);
	$effect(() => {
		const controller = new AbortController();
		listAlertRules(controller.signal)
			.then((list) => (rules = list))
			.catch(() => {});
		return () => controller.abort();
	});

	let deadlines = $state<Deadline[]>([]);
	$effect(() => {
		const controller = new AbortController();
		listDeadlines({ maxDays: 7, signal: controller.signal })
			.then((list) => (deadlines = list))
			.catch(() => {});
		return () => controller.abort();
	});

	const sky = $derived(readSky({ targets: alertsStore.targets, probes: alertsStore.probes, alerts: alertsStore.alerts, rules }));
	const rows = $derived(attentionRows(sky.needsYou));
	const shown = $derived(rows.slice(0, LIMIT));
	const week = $derived(weekAhead(deadlines));
	const monitors = $derived(alertsStore.targets.length);

	function title(row: SkyRow): string {
		if (row.kind === 'device') return row.target.name;
		return row.target?.name ?? row.alert.rule_name;
	}
	function detail(row: SkyRow): string {
		if (row.kind === 'device') return row.detail;
		return row.target ? row.alert.rule_name : (row.rule?.description ?? '');
	}
	function href(row: SkyRow): string {
		if (row.kind === 'device') return `/targets/${row.target.id}`;
		return row.target ? `/targets/${row.target.id}` : '/alerts';
	}

	// --- Silence a monitor for an hour (a device row has no alert to snooze) --
	let busy = $state<number | null>(null);
	let silenced = $state<number[]>([]);
	let failure = $state<{ id: number; message: string } | null>(null);

	async function silence(target: Target) {
		if (busy !== null) return;
		busy = target.id;
		failure = null;
		try {
			await createSilence(quickSilencePayload(target));
			silenced = [...silenced, target.id];
			void alertsStore.refresh();
		} catch (cause) {
			failure = { id: target.id, message: toApiError(cause).message };
		} finally {
			busy = null;
		}
	}
</script>

<section class="flex min-w-0 flex-col overflow-hidden rounded-xl border border-line bg-surface" aria-labelledby="needs-title">
	<div class="flex flex-wrap items-baseline gap-x-2.5 gap-y-1 px-4 pt-3.5 pb-3">
		<h2 id="needs-title" class="m-0 text-base font-semibold text-ink">{m.console_panel_attention()}</h2>
		{#if rows.length > 0}
			<span class="font-mono text-xs text-ink-2 tabular-nums">{rows.length}</span>
		{/if}
		<a href="/targets" class="ml-auto inline-flex min-h-10 items-center gap-1 text-sm font-medium text-ink-2 hover:text-ink max-sm:-my-2">
			{m.overview_needs_see_all({ count: monitors })}<ArrowRight class="size-3.5" aria-hidden="true" />
		</a>
	</div>

	{#if rows.length === 0}
		<div class="state-in flex flex-1 flex-col items-center justify-center gap-1.5 border-t border-line px-6 py-9 text-center">
			<Mascot mood="sleep" blink={false} label={m.shell_pigeon_sleep()} class="mb-1 size-14" />
			<p class="m-0 text-base font-semibold text-ink">{m.overview_all_good_title()}</p>
			<p class="m-0 max-w-sm text-sm text-ink-2">{m.overview_all_good_text()}</p>
		</div>
	{:else}
		<ul class="m-0 list-none p-0">
			{#each shown as { row, held }, i (row.key)}
				<li class={`flex items-center gap-3 border-t border-line px-4 py-2.5 ${i >= PHONE_LIMIT ? 'max-sm:hidden' : ''}`}>
					<span aria-hidden="true" class={`h-9 w-[3px] shrink-0 self-center rounded-sm ${TONE_BAR[row.tone]}`}></span>
					<div class="min-w-0 flex-1 md:grid md:grid-cols-[minmax(0,8.5rem)_minmax(0,13rem)_minmax(0,1fr)] md:items-center md:gap-3">
						<div class="flex min-w-0 items-center gap-2">
							<Plate tone={row.tone} size="sm" class="max-w-full min-w-0 truncate">{row.plate}</Plate>
							{#if row.since}
								<span class="ml-auto shrink-0 font-mono text-xs text-ink-3 tabular-nums md:hidden">{formatRelative(row.since)}</span>
							{/if}
						</div>
						<a href={href(row)} class="mt-1 flex min-w-0 items-baseline gap-1.5 text-sm font-semibold text-ink hover:underline md:mt-0">
							<span class="truncate">{title(row)}</span>
							{#if held > 0}
								<span class="shrink-0 text-xs font-normal text-ink-2">{m.overview_needs_held({ count: held })}</span>
							{/if}
						</a>
						{#if detail(row)}
							<span class="block truncate text-sm text-ink-2 max-md:mt-0.5" title={detail(row)}>{detail(row)}</span>
						{/if}
						{#if failure && row.kind === 'device' && failure.id === row.target.id}
							<p class="m-0 mt-1 text-xs text-warning-ink md:col-span-3" role="alert">{failure.message}</p>
						{/if}
					</div>
					{#if row.since}
						<span class="w-16 shrink-0 whitespace-nowrap text-right font-mono text-xs text-ink-3 tabular-nums max-md:hidden">{formatRelative(row.since)}</span>
					{/if}
					<div class="flex shrink-0 items-center">
						{#if row.kind === 'alert'}
							<AlertActions alert={row.alert} target={row.target} onchanged={() => void alertsStore.refresh()} />
						{:else if auth.canOperate}
							{#if silenced.includes(row.target.id)}
								<Plate tone="muted" size="sm" draw>{m.console_panel_silenced()}</Plate>
							{:else}
								<Button size="sm" variant="secondary" loading={busy === row.target.id} disabled={busy !== null} onclick={() => void silence(row.target)}>
									{m.console_panel_silence()}
								</Button>
							{/if}
						{/if}
					</div>
				</li>
			{/each}
		</ul>
		{#if rows.length > PHONE_LIMIT}
			<a href="/alerts" class={`flex min-h-10 items-center gap-1 border-t border-line px-4 text-sm font-medium text-ink-2 hover:text-ink ${rows.length > LIMIT ? '' : 'sm:hidden'}`}>
				{m.overview_needs_more({ count: rows.length })}<ArrowRight class="size-3.5" aria-hidden="true" />
			</a>
		{/if}
	{/if}

	<div class="mt-auto flex flex-wrap items-center gap-x-3 border-t border-line px-4 py-0.5 text-sm text-ink-2">
		<span class="flex min-h-10 items-center font-medium text-ink">{m.overview_week_title()}</span>
		{#if week.length === 0}
			<span class="flex min-h-10 items-center">{m.overview_week_none()}</span>
		{:else}
			{#each week.slice(0, WEEK_SHOWN) as item (`${item.target}:${item.kind}:${item.what}`)}
				<a href={`/targets/${item.target}`} class="inline-flex min-h-10 min-w-0 items-center gap-1.5 hover:text-ink">
					<span>{kindName(item.kind)}</span>
					<span class="max-w-[12rem] truncate font-medium text-ink">{item.what || item.target_name}</span>
					<span class={item.days_left < 3 ? 'text-warning-ink' : ''}>{whenLabel(item.days_left)}</span>
				</a>
			{/each}
			{#if week.length > WEEK_SHOWN}
				<a href="/status?tab=reports" class="inline-flex min-h-10 items-center font-medium hover:text-ink">{m.overview_week_more({ count: week.length - WEEK_SHOWN })}</a>
			{/if}
		{/if}
	</div>
</section>
