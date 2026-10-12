<script lang="ts">
	/**
	 * One monitor on a single dense line, for the compact view of /targets:
	 * LED, product icon, name, kind, address, last seen and the state plate
	 * (icon + word, so the state never rests on the LED's colour alone).
	 * Columns drop out as the line narrows; the name and the state stay.
	 */
	import type { Target } from '#lib/api/index.js';
	import type { TargetState } from '#lib/format.js';
	import { STATE_LABEL, STATE_TONE, formatRelative } from '#lib/format.js';
	import { Led, Plate } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import KindIcon from '#lib/components/KindIcon.svelte';

	interface Props {
		target: Target;
		state: TargetState;
		kindLabel?: string;
		depth?: number;
		shadowed?: boolean;
	}

	let { target, state, kindLabel, depth = 0, shadowed = false }: Props = $props();

	const tone = $derived(STATE_TONE[state]);
	const blink = $derived(state === 'offline' || state === 'down');
</script>

<a
	href={`/targets/${target.id}`}
	class={`@container flex min-h-11 items-center gap-2.5 px-3 py-1.5 transition-colors hover:bg-surface-2 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-signal ${shadowed ? 'opacity-60' : ''}`}
	style={depth ? `padding-left: ${0.75 + Math.min(depth, 3) * 1.25}rem` : undefined}
	aria-label={`${target.name}, ${STATE_LABEL[state]}`}
>
	<Led {tone} {blink} size="sm" />
	<span class="flex size-6 shrink-0 items-center justify-center text-ink-2" title={kindLabel ?? target.kind}>
		<KindIcon kind={target.kind} />
	</span>
	<span class="grid min-w-0 flex-1 items-center gap-x-4 @lg:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_minmax(0,0.8fr)]">
		<span class="flex min-w-0 items-baseline gap-2">
			<span class="truncate text-[0.875rem] font-semibold text-ink">{target.name}</span>
			<span class="label-tape hidden shrink-0 !text-[0.625rem] text-ink-3 @md:inline">{kindLabel ?? target.kind}</span>
		</span>
		<span class="hidden min-w-0 truncate text-[0.8125rem] text-ink-2 @lg:block" title={target.address}>{target.address}</span>
		<span class="tnum hidden min-w-0 truncate text-[0.8125rem] text-ink-2 @lg:block" title={target.last_error ?? undefined}>
			{#if target.last_error}
				<span class={target.error_kind === 'config' ? 'text-advisory-ink' : 'text-warning-ink'}>{target.last_error}</span>
			{:else}
				{m.devices_target_last_seen({ when: formatRelative(target.last_probe_at) })}
			{/if}
		</span>
	</span>
	<Plate {tone} label={STATE_LABEL[state]} pulse={blink} class="shrink-0" />
</a>
