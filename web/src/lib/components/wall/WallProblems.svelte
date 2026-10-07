<script lang="ts">
	/**
	 * The wall's problems, read from across the room: what is wrong comes
	 * first, in big tiles — the severity plate (icon + word), the device's name
	 * in display type, what happened and since when. A warning tile breathes a
	 * slow ring (opacity only, five seconds a breath): it draws the eye without
	 * ever blinking. Past three tiles, and for what is only building up,
	 * suppressed or acknowledged, the rows shrink to one line each.
	 *
	 * Same rows as the Overview's "Needs you" (`readSky`): the two can never
	 * disagree. Nothing to act on here — a wall is read, not operated — but
	 * each tile opens its device for whoever walks up with a mouse.
	 */
	import type { Sky, SkyRow } from '#lib/components/overview/sky.js';
	import { formatRelative } from '#lib/format.js';
	import { Plate } from '#lib/ui/index.js';

	interface Props {
		sky: Sky;
		/** Re-read every second by the wall, so "since" stays current. */
		now: Date;
	}

	let { sky, now }: Props = $props();

	const BIG = 3;
	const SMALL = 5;

	const serious = $derived(sky.needsYou.filter((row) => row.rank <= 2));
	const quieter = $derived(sky.needsYou.filter((row) => row.rank > 2));
	const big = $derived(serious.slice(0, BIG));
	const small = $derived([...serious.slice(BIG), ...quieter]);
	const shownSmall = $derived(small.slice(0, SMALL));
	const hiddenCount = $derived(small.length - shownSmall.length);

	function name(row: SkyRow): string {
		if (row.kind === 'device') return row.target.name;
		return row.target?.name ?? row.alert.labels.host ?? row.alert.rule_name;
	}

	function what(row: SkyRow): string {
		if (row.kind === 'device') return row.detail || row.plate;
		if (row.parent) return `${row.alert.rule_name} · ${row.parent.name} is down`;
		return row.alert.rule_name;
	}

	function since(row: SkyRow): string {
		void now;
		return row.since ? formatRelative(row.since) : '';
	}

	function href(row: SkyRow): string | null {
		const id = row.kind === 'device' ? row.target.id : row.target?.id;
		return id !== undefined ? `/targets/${id}` : null;
	}
</script>

{#if big.length > 0 || shownSmall.length > 0}
	<section class="flex min-h-0 flex-col gap-3" aria-label="Problems">
		{#each big as row (row.key)}
			{@const link = href(row)}
			<svelte:element
				this={link ? 'a' : 'div'}
				href={link}
				class="tile tile--{row.tone} relative block rounded-[var(--radius-card)] border-2 bg-surface px-6 py-5 shadow-float"
			>
				<span class="ring" aria-hidden="true"></span>
				<span class="flex items-center justify-between gap-4">
					<Plate tone={row.tone} label={row.plate} size="md" class="plate-xl" />
					{#if row.since}
						<span class="tnum text-[1.0625rem] text-ink-2">{since(row)}</span>
					{/if}
				</span>
				<span class="display mt-2.5 block truncate text-[2.375rem] text-ink" title={name(row)}>{name(row)}</span>
				<span class="mt-1 block truncate text-[1.1875rem] text-ink-2" title={what(row)}>{what(row)}</span>
			</svelte:element>
		{/each}

		{#if shownSmall.length > 0}
			<ul class="flex flex-col gap-1.5">
				{#each shownSmall as row (row.key)}
					<li class="flex min-w-0 items-center gap-3 rounded-[10px] bg-surface/90 px-4 py-2.5 shadow-lift">
						<Plate tone={row.tone} label={row.plate} size="md" class="shrink-0" />
						<span class="min-w-0 truncate text-[1.1875rem] font-semibold text-ink">{name(row)}</span>
						<span class="min-w-0 flex-1 truncate text-[1.0625rem] text-ink-2">{what(row)}</span>
					</li>
				{/each}
				{#if hiddenCount > 0}
					<li class="px-4 text-[1.0625rem] text-ink-2">And {hiddenCount} more on the Alerts page.</li>
				{/if}
			</ul>
		{/if}
	</section>
{/if}

<style>
	.tile {
		transition: transform 300ms var(--ease-out-expo);
	}
	a.tile:hover {
		transform: translateY(-1px);
	}
	.tile--warning {
		border-color: color-mix(in srgb, var(--c-warning) 70%, transparent);
	}
	.tile--advisory {
		border-color: color-mix(in srgb, var(--c-advisory) 70%, transparent);
	}
	.tile--info {
		border-color: color-mix(in srgb, var(--c-info) 60%, transparent);
	}
	.tile--ghost,
	.tile--muted,
	.tile--signal {
		border-color: var(--c-line);
	}
	.tile :global(.plate-xl) {
		height: 2.125rem;
		padding-inline: 0.75rem;
		font-size: 1.0625rem;
		gap: 0.5rem;
	}
	.tile :global(.plate-xl svg) {
		width: 1.125rem;
		height: 1.125rem;
	}
	/* The slow ring around a warning: its own layer, opacity only. */
	.ring {
		position: absolute;
		inset: -8px;
		border-radius: calc(var(--radius-card) + 6px);
		border: 3px solid var(--c-warning);
		opacity: 0;
		pointer-events: none;
	}
	.tile--warning .ring {
		animation: ring 5s ease-in-out infinite;
	}
	@keyframes ring {
		50% {
			opacity: 0.5;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.tile--warning .ring {
			animation: none;
			opacity: 0.35;
		}
	}
</style>
