<script lang="ts">
	/**
	 * A device that cannot be reached, as a tile of the "Needs you" grid.
	 *
	 * No rule has to fire for this: a device that stopped reporting is a
	 * warning in itself. There is no alert to silence, so the only action is to
	 * open the device.
	 */
	import type { SkyRow } from '$lib/components/overview/sky';
	import { Button, Plate } from '$lib/ui';
	import { formatRelative, formatDateTime } from '$lib/format';
	import { TONE_BAR } from './helpers';

	interface Props {
		row: Extract<SkyRow, { kind: 'device' }>;
	}

	let { row }: Props = $props();
	const target = $derived(row.target);
</script>

<article
	class="relative flex h-full min-w-0 flex-col overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface py-3 pr-4 pl-5 shadow-lift transition"
	aria-label={`${row.plate}: ${target.name}`}
>
	<span class={`absolute inset-y-0 left-0 w-1 ${TONE_BAR[row.tone]}`} aria-hidden="true"></span>

	<div class="flex items-start justify-between gap-2">
		<Plate tone={row.tone} label={row.plate} pulse />
		<span class="tnum shrink-0 pt-0.5 text-[0.75rem] whitespace-nowrap text-ink-2" title={row.since ? formatDateTime(row.since) : undefined}>
			{#if row.since}
				last report {formatRelative(row.since)}
			{:else}
				never reported
			{/if}
		</span>
	</div>

	<p class="mt-2 truncate font-semibold text-ink" title={target.name}>{target.name}</p>
	<p class="mt-0.5 truncate text-[0.8125rem] text-ink-2" title={target.address}>{target.address}</p>
	{#if row.detail}
		<p class={`mt-1 line-clamp-3 text-[0.8125rem] break-words ${row.state === 'misconfigured' ? 'text-advisory-ink' : 'text-warning-ink'}`} title={row.detail}>{row.detail}</p>
	{/if}

	<div class="mt-auto pt-3">
		<div class="flex flex-wrap items-center gap-2 border-t border-line pt-2.5">
			<Button size="sm" variant="ghost" href={`/targets/${target.id}`}>Open device</Button>
		</div>
	</div>
</article>
