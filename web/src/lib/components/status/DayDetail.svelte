<script lang="ts">
	/**
	 * One day of a public status page, hour by hour: for each service a strip of
	 * 24 hours (UTC) toned by availability, the outages of the day with their
	 * start and end, and the announcements that touched it. Opened from a bar of
	 * the history; takes focus when it opens, Escape or the button closes it.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { X } from 'lucide-svelte';
	import type { PublicDayDetail } from '#lib/api/index.js';
	import { formatDateTime, formatPercent } from '#lib/format.js';
	import { Button, ErrorNotice, Skeleton } from '#lib/ui/index.js';
	import { dayTone } from './words';
	import { formatUtcDay } from './utc-day';
	import { INCIDENT_STATUS, KIND_LABEL } from './words';

	interface Props {
		date: string;
		detail: PublicDayDetail | null;
		loading: boolean;
		error: unknown;
		onclose: () => void;
	}

	let { date, detail, loading, error, onclose }: Props = $props();

	const FILL: Record<ReturnType<typeof dayTone>, string> = {
		signal: 'bg-signal',
		advisory: 'bg-advisory',
		warning: 'bg-warning',
		ghost: 'bg-line-strong'
	};

	let heading = $state<HTMLHeadingElement | null>(null);
	$effect(() => {
		heading?.focus({ preventScroll: false });
	});
</script>

<svelte:window onkeydown={(event) => event.key === 'Escape' && onclose()} />

<section class="mt-4 rounded-[var(--radius-card)] border border-line bg-surface p-4 shadow-lift sm:p-5" aria-labelledby="day-detail-title">
	<div class="flex items-start justify-between gap-3">
		<h3 id="day-detail-title" bind:this={heading} tabindex="-1" class="text-base font-semibold tracking-tight text-ink outline-none">
			{m.status_day_title({ date: formatUtcDay(date, getLocale()) })}
		</h3>
		<Button variant="ghost" size="sm" onclick={onclose}>
			<X class="size-4" aria-hidden="true" />
			{m.status_day_close()}
		</Button>
	</div>

	{#if loading}
		<p class="mt-3 text-sm text-ink-2" role="status">{m.status_day_loading()}</p>
		<Skeleton class="mt-2 h-16" />
	{:else if error}
		<ErrorNotice {error} title={m.status_day_error()} class="mt-3" />
	{:else if detail}
		<ul class="mt-3 grid gap-4" role="list">
			{#each detail.services as service (service.key)}
				<li>
					<p class="font-semibold text-ink">{service.label}</p>
					<ul class="mt-1.5 flex h-6 items-stretch gap-px" aria-label={m.status_day_hours_aria({ label: service.label })}>
						{#each service.hours as hour, index (index)}
							<li class="min-w-0 flex-1">
								<span
									class={`block h-full w-full rounded-[2px] ${FILL[dayTone(hour)]}`}
									title={`${String(index).padStart(2, '0')}:00 · ${formatPercent(hour)}`}
								></span>
								<span class="sr-only">{String(index).padStart(2, '0')}:00 {formatPercent(hour)}</span>
							</li>
						{/each}
					</ul>
					<div class="mt-1 flex justify-between text-[0.6875rem] text-ink-3 tnum" aria-hidden="true">
						<span>00</span><span>06</span><span>12</span><span>18</span><span>24</span>
					</div>
					{#if service.outages.length === 0}
						<p class="mt-1 text-[0.8125rem] text-ink-2">{m.status_day_clean()}</p>
					{:else}
						<ul class="mt-1 grid gap-0.5 text-[0.8125rem] text-ink-2">
							{#each service.outages as outage (outage.from)}
								<li class="tnum">{m.status_day_outage({ from: formatDateTime(outage.from), to: formatDateTime(outage.to), minutes: outage.minutes })}</li>
							{/each}
						</ul>
					{/if}
				</li>
			{/each}
		</ul>
		{#if detail.incidents.length > 0}
			<h4 class="mt-4 text-sm font-semibold text-ink">{m.status_day_incidents()}</h4>
			<ul class="mt-1 grid gap-1 text-[0.8125rem] text-ink-2">
				{#each detail.incidents as incident (incident.starts_at + incident.title)}
					<li>
						<span class="font-medium text-ink">{incident.title}</span>
						· {KIND_LABEL[incident.kind]} · {INCIDENT_STATUS[incident.status].label}
						· <span class="tnum">{formatDateTime(incident.starts_at)}{incident.ends_at ? ` → ${formatDateTime(incident.ends_at)}` : ''}</span>
					</li>
				{/each}
			</ul>
		{/if}
	{/if}
</section>
