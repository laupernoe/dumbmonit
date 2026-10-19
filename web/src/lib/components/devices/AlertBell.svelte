<script lang="ts">
	/**
	 * A monitor's active alerts as a small bell with a count, linking to the
	 * alerts section of its page. Renders nothing at 0. The count is text and
	 * the icon changes (bell / bell crossed out) so the state never rests on
	 * the tone alone: firing alerts take the warning tone, acknowledged ones
	 * a quiet hairline, and alerts masked by a down parent stay neutral.
	 * The hit area is at least 40 px even though the plate is smaller.
	 */
	import { Bell, BellOff } from 'lucide-svelte';
	import type { TargetAlertCounts } from '#lib/alerts-attention.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		targetId: number;
		counts: TargetAlertCounts | undefined;
		class?: string;
	}

	let { targetId, counts, class: className = '' }: Props = $props();

	const label = $derived(counts ? m.devices_alertbell_aria({ count: counts.total }) : '');
	const title = $derived(
		counts && counts.firing !== counts.total
			? `${label}: ${m.devices_alertbell_detail({ firing: counts.firing, acked: counts.acked, suppressed: counts.suppressed })}`
			: label
	);
	const plateClass = $derived(
		counts && counts.firing > 0
			? 'border-warning/35 bg-warning-soft text-warning-ink'
			: counts && counts.acked > 0
				? 'border-line bg-transparent text-ink-2'
				: 'border-line bg-transparent text-ink-3'
	);
</script>

{#if counts && counts.total > 0}
	<a
		href={`/targets/${targetId}#device-alerts`}
		class={`group relative z-[1] inline-flex min-h-10 min-w-10 shrink-0 items-center justify-center rounded-[var(--radius-plate)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal ${className}`}
		aria-label={label}
		{title}
	>
		<span
			class={`tnum inline-flex h-6 items-center gap-1 rounded-[var(--radius-plate)] border px-1.5 text-[0.75rem] font-semibold leading-none transition-colors group-hover:border-line-strong ${plateClass}`}
		>
			{#if counts.firing > 0 || counts.acked > 0}
				<Bell class="size-3.5 shrink-0" aria-hidden="true" />
			{:else}
				<BellOff class="size-3.5 shrink-0" aria-hidden="true" />
			{/if}
			<span aria-hidden="true">{counts.total}</span>
		</span>
	</a>
{/if}
