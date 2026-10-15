<script lang="ts">
	/**
	 * The wall's problems, read from across the room: what is wrong comes
	 * first, in big tiles — the severity plate (icon + word), the device's name
	 * in display type, what happened and since when. A warning tile breathes a
	 * slow ring (opacity only, five seconds a breath): it draws the eye without
	 * ever blinking. Past three tiles, and for what is only building up,
	 * suppressed or acknowledged, the rows shrink to one line each.
	 *
	 * Each tile wears the product icon of its device (the same marks as the
	 * device lists). The block never outgrows the room the wall gives it: when
	 * the tiles do not fit (720p, many problems) they shrink to one-line rows,
	 * then fold into "and N more", instead of overlapping the devices below.
	 *
	 * Same rows as the Overview's "Needs you" (`readSky`): the two can never
	 * disagree. Nothing to act on here — a wall is read, not operated — but
	 * each tile opens its device for whoever walks up with a mouse.
	 */
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import type { TargetId } from '#lib/api/index.js';
	import type { Sky, SkyRow } from '#lib/components/overview/sky.js';
	import { formatRelative } from '#lib/format.js';
	import { Plate } from '#lib/ui/index.js';
	import KindIcon from '#lib/components/KindIcon.svelte';

	interface Props {
		sky: Sky;
		/** Re-read every second by the wall, so "since" stays current. */
		now: Date;
		/** Height in px the problems may take; `Infinity` when the page simply scrolls. */
		room?: number;
		/** Devices written out above, so the device tiles below do not list them again. */
		listed?: Set<TargetId>;
	}

	let { sky, now, room = Infinity, listed = $bindable(new Set()) }: Props = $props();

	const BIG = 3;
	const SMALL = 5;

	const serious = $derived(sky.needsYou.filter((row) => row.rank <= 2));
	const quieter = $derived(sky.needsYou.filter((row) => row.rank > 2));
	/** 0: big tiles as designed; 1: one line per problem; 2: and fewer of them (`cut`). */
	let level = $state(0);
	let cut = $state(Infinity);
	const big = $derived(level === 0 ? serious.slice(0, BIG) : []);
	const small = $derived(level === 0 ? [...serious.slice(BIG), ...quieter] : sky.needsYou);
	const smallCap = $derived(level === 0 ? SMALL : Math.min(cut, level === 1 ? 12 : Infinity));
	const shownSmall = $derived(small.slice(0, smallCap));
	const hiddenCount = $derived(small.length - shownSmall.length);

	$effect(() => {
		const ids = new Set<TargetId>();
		for (const row of [...big, ...shownSmall]) {
			const id = row.kind === 'device' ? row.target.id : row.target?.id;
			if (id !== undefined) ids.add(id);
		}
		listed = ids;
	});

	let box = $state<HTMLElement | null>(null);
	const signature = $derived(sky.needsYou.map((row) => row.key).join('|') + `@${Math.round(room)}`);
	let lastSignature = '';

	// Fit to the room: measured after each render; each pass takes one step down.
	$effect(() => {
		const sig = signature;
		const wanted = room;
		void level;
		void cut;
		untrack(() => {
			if (sig !== lastSignature) {
				lastSignature = sig;
				level = 0;
				cut = Infinity;
			}
		});
		if (wanted === Infinity) return;
		const frame = requestAnimationFrame(() => {
			const el = box;
			if (!el || el.scrollHeight <= wanted + 1) return;
			if (level === 0) {
				level = 1;
				return;
			}
			const rows = Array.from(el.querySelectorAll<HTMLElement>('[data-row]'));
			const firstOut = rows.findIndex((row) => row.offsetTop + row.offsetHeight > wanted - 36);
			cut = Math.max(1, firstOut === -1 ? rows.length - 1 : firstOut);
			level = 2;
		});
		return () => cancelAnimationFrame(frame);
	});

	function name(row: SkyRow): string {
		if (row.kind === 'device') return row.target.name;
		return row.target?.name ?? row.alert.labels.host ?? row.alert.rule_name;
	}

	function what(row: SkyRow): string {
		if (row.kind === 'device') return row.detail || row.plate;
		if (row.parent) return m.wall_problems_parent_down({ rule: row.alert.rule_name, parent: row.parent.name });
		return row.alert.rule_name;
	}

	function since(row: SkyRow): string {
		void now;
		return row.since ? formatRelative(row.since) : '';
	}

	/** The device's kind, for its product icon; an alert with no device has none. */
	function kindOf(row: SkyRow): string | null {
		return row.target?.kind ?? null;
	}

	function href(row: SkyRow): string | null {
		const id = row.kind === 'device' ? row.target.id : row.target?.id;
		return id !== undefined ? `/targets/${id}` : null;
	}
</script>

{#if big.length > 0 || shownSmall.length > 0}
	<section
		class="relative -m-2 flex min-h-0 flex-col gap-3 overflow-hidden p-2"
		style:max-height={room === Infinity ? undefined : `${room}px`}
		aria-label={m.wall_problems_aria()}
		bind:this={box}
	>
		{#each big as row (row.key)}
			{@const link = href(row)}
			{@const kind = kindOf(row)}
			<svelte:element
				this={link ? 'a' : 'div'}
				href={link}
				class="tile tile--{row.tone} relative block shrink-0 rounded-[var(--radius-card)] border-2 bg-surface px-6 py-5 shadow-float"
			>
				<span class="ring" aria-hidden="true"></span>
				<span class="flex items-center justify-between gap-4">
					<Plate tone={row.tone} label={row.plate} size="md" class="plate-xl" />
					{#if row.since}
						<span class="tnum shrink-0 text-[1.0625rem] text-ink-2">{since(row)}</span>
					{/if}
				</span>
				<span class="mt-2.5 flex min-w-0 items-center gap-3">
					{#if kind}<KindIcon {kind} class="tile-icon text-ink-2" />{/if}
					<span class="tile-name display min-w-0 flex-1 text-ink" title={name(row)}>{name(row)}</span>
				</span>
				<span class="tile-what mt-1 line-clamp-2 text-ink-2" title={what(row)}>{what(row)}</span>
			</svelte:element>
		{/each}

		{#if shownSmall.length > 0}
			<ul class="flex flex-col gap-1.5">
				{#each shownSmall as row (row.key)}
					{@const kind = kindOf(row)}
					<li class="flex min-w-0 shrink-0 items-start gap-3 rounded-[10px] bg-surface/90 px-4 py-2.5 shadow-lift" data-row>
						<Plate tone={row.tone} label={row.plate} size="md" class="shrink-0" />
						{#if kind}<KindIcon {kind} class="size-5 text-ink-2" />{/if}
						<span class="row-name min-w-0 text-[1.1875rem] font-semibold [overflow-wrap:anywhere] text-ink" title={name(row)}>{name(row)}</span>
						<span class="line-clamp-2 min-w-0 flex-1 text-[1.0625rem] [overflow-wrap:anywhere] text-ink-2" title={what(row)}>{what(row)}</span>
					</li>
				{/each}
				{#if hiddenCount > 0}
					<li class="px-4 text-[1.0625rem] font-semibold text-ink-2">
						<a href="/alerts" class="underline decoration-line-strong underline-offset-4">{m.wall_problems_more({ count: hiddenCount })}</a>
					</li>
				{/if}
			</ul>
		{/if}
	</section>
{/if}

<style>
	/* A long name takes a second line rather than lose its end. */
	.tile-name {
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		overflow: hidden;
		overflow-wrap: anywhere;
		font-size: 2.375rem;
		line-height: 1.1;
	}
	@media (max-width: 1023px) {
		.tile-name {
			font-size: 1.75rem;
		}
	}
	.tile :global(.tile-icon) {
		width: 2rem;
		height: 2rem;
	}
	/* A long name never squeezes the cause out of a one-line row: it gives way past 55%. */
	.row-name {
		max-width: 45%;
	}
	.tile-what {
		font-size: 1.1875rem;
	}
	/*
	 * A 720p screen: the tiles keep to a narrow column, so the monument in the
	 * middle of the scene stays in view instead of hiding behind them.
	 */
	@media (min-width: 1024px) and (max-width: 1599px) {
		.tile {
			padding: 0.625rem 1rem;
		}
		.tile-name {
			margin-top: 0.25rem;
			font-size: 1.625rem;
		}
		.tile-what {
			margin-top: 0;
			font-size: 1rem;
		}
		.tile :global(.tile-icon) {
			width: 1.5rem;
			height: 1.5rem;
		}
	}
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
