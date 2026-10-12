<script lang="ts">
	/**
	 * One monitor as a tile, for the tiles view of /targets: the same label
	 * grid as a faceplate (product icon and kind, name, address, state plate,
	 * last seen) folded into a card, the 24 h sparkline along the bottom.
	 * A child names the unit it sits behind instead of being indented.
	 */
	import type { Target } from '#lib/api/index.js';
	import type { TargetState } from '#lib/format.js';
	import { STATE_LABEL, STATE_TONE, formatRelative } from '#lib/format.js';
	import { Led, Plate, Spotlight } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import Chart, { type Serie } from '#lib/components/Chart.svelte';
	import KindIcon from '#lib/components/KindIcon.svelte';
	import { CornerDownRight } from 'lucide-svelte';

	interface Props {
		target: Target;
		state: TargetState;
		kindLabel?: string;
		sparkline?: Serie[] | null;
		/** Name of the unit this one sits behind, when it is a child. */
		parentName?: string | null;
		shadowed?: boolean;
	}

	let { target, state, kindLabel, sparkline = null, parentName = null, shadowed = false }: Props = $props();

	const tone = $derived(STATE_TONE[state]);
	const blink = $derived(state === 'offline' || state === 'down');
</script>

<Spotlight
	tag="a"
	href={`/targets/${target.id}`}
	class={`faceplate flex h-full flex-col gap-3 p-4 ${shadowed ? 'opacity-60' : ''}`}
	data-interactive
	aria-label={`${target.name}, ${STATE_LABEL[state]}`}
>
	<div class="flex items-start gap-3">
		<span class="flex size-9 shrink-0 items-center justify-center rounded-lg border border-line bg-surface-2 text-ink-2">
			<KindIcon kind={target.kind} class="size-[1.125rem]" />
		</span>
		<div class="min-w-0 flex-1">
			<span class="block truncate font-semibold text-ink">{target.name}</span>
			<span class="label-tape mt-0.5 block truncate !text-[0.625rem]">{kindLabel ?? target.kind}</span>
		</div>
		<Led {tone} {blink} size="sm" class="mt-1.5" />
	</div>

	<div class="min-w-0 text-[0.8125rem] text-ink-2">
		<p class="truncate" title={target.address}>{target.address}</p>
		{#if parentName}
			<p class="mt-0.5 flex min-w-0 items-center gap-1 text-ink-3">
				<CornerDownRight class="size-3.5 shrink-0" aria-hidden="true" />
				<span class="truncate">{m.devices_target_behind({ name: parentName })}</span>
			</p>
		{/if}
	</div>

	<div class="mt-auto flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
		<Plate {tone} label={STATE_LABEL[state]} pulse={blink} />
		<p class="tnum min-w-0 flex-1 truncate text-[0.8125rem] text-ink-2" title={target.last_error ?? undefined}>
			{#if target.last_error}
				<span class={target.error_kind === 'config' ? 'text-advisory-ink' : 'text-warning-ink'}>{target.last_error}</span>
			{:else}
				{m.devices_target_last_seen({ when: formatRelative(target.last_probe_at) })}
			{/if}
		</p>
	</div>

	{#if sparkline && sparkline.length > 0}
		<div class="rounded-md border border-line bg-canvas-deep/60 px-1 pt-1">
			<Chart series={sparkline} compact height={30} tone={tone === 'warning' ? 'warning' : tone === 'advisory' ? 'advisory' : tone === 'ghost' ? 'ghost' : 'signal'} />
		</div>
	{:else}
		<div class="ghost-cell h-[38px] rounded-md border border-dashed border-line" aria-hidden="true"></div>
	{/if}
</Spotlight>
