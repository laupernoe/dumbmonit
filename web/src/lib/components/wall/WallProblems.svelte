<script lang="ts">
	/**
	 * The wall's problems, read from across the room, always whole: what is
	 * down first, then warnings, then what its parent masks, then what is
	 * snoozed or acknowledged. Identical lines fold into one ("lab-immich ·
	 * Client device stale ×5"), and the probe's long error is said short
	 * ("No data for 15 d"), on two lines at most.
	 *
	 * The block never outgrows the height the wall gives it (`room`), and
	 * never hides a line without saying so. Measured after each render, it
	 * steps down until it fits: big tiles for the first problems, then one
	 * row per problem, then the quiet ones summed up on one line ("2
	 * suppressed by parent · 5 snoozed"), then one line of cause per row,
	 * and only then "N more problems" for what is left.
	 *
	 * Same rows as the Overview's "Needs you" (`readSky`): the two can never
	 * disagree. Nothing to act on here — a wall is read, not operated — but
	 * each line opens its device for whoever walks up with a mouse.
	 */
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import type { TargetId } from '#lib/api/index.js';
	import type { Sky, SkyRow } from '#lib/components/overview/sky.js';
	import { formatDuration, formatRelative } from '#lib/format.js';
	import { Plate } from '#lib/ui/index.js';
	import KindIcon from '#lib/components/KindIcon.svelte';
	import { groupProblems, quietCounts, shortCause, QUIET_RANK, type ProblemLine } from '#lib/wall/problems.js';

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

	function name(row: SkyRow): string {
		if (row.kind === 'device') return row.target.name;
		return row.target?.name ?? row.alert.labels.host ?? row.alert.rule_name;
	}

	function what(row: SkyRow): string {
		if (row.kind === 'device') {
			const cause = shortCause(row.detail || '');
			if (cause.code === 'no_data') return m.wall_problems_no_data({ duration: formatDuration(cause.seconds) });
			if (cause.code === 'agent_silent') return m.wall_problems_agent_silent({ duration: formatDuration(cause.seconds) });
			return cause.text || row.plate;
		}
		if (row.parent) return m.wall_problems_parent_down({ rule: row.alert.rule_name, parent: row.parent.name });
		return row.alert.rule_name;
	}

	const lines = $derived(
		groupProblems(
			sky.needsYou.map((row) => ({
				key: row.key,
				rank: row.rank,
				tone: row.tone,
				plate: row.plate,
				name: name(row),
				what: what(row),
				since: row.since,
				kind: row.target?.kind ?? null,
				targetId: (row.kind === 'device' ? row.target.id : row.target?.id) ?? null
			}))
		)
	);
	const serious = $derived(lines.filter((line) => line.rank < QUIET_RANK));
	const quiet = $derived(lines.filter((line) => line.rank >= QUIET_RANK));

	/**
	 * 0: big tiles first; 1: one row per line; 2: the quiet lines summed up;
	 * 3: one line of cause per row; 4: and fewer rows (`cut`), the rest counted.
	 */
	let level = $state(0);
	let cut = $state(Infinity);
	const big = $derived(level === 0 ? serious.filter((line) => line.rank <= 2).slice(0, BIG) : []);
	const rows = $derived(level === 0 ? lines.filter((line) => !big.includes(line)) : level >= 2 ? serious : lines);
	const shownRows = $derived(rows.slice(0, cut));
	const hiddenCount = $derived(rows.slice(cut).reduce((sum, line) => sum + line.count, 0));
	const summary = $derived.by(() => {
		if (level < 2 || quiet.length === 0) return null;
		const counts = quietCounts(quiet);
		const parts: string[] = [];
		if (counts.suppressed > 0) parts.push(m.overview_sky_n_suppressed({ count: counts.suppressed }));
		if (counts.snoozed > 0) parts.push(m.overview_sky_n_snoozed({ count: counts.snoozed }));
		if (counts.acked > 0) parts.push(m.overview_sky_n_acked({ count: counts.acked }));
		return parts.join(' · ');
	});
	const compact = $derived(level >= 3);

	$effect(() => {
		const ids = new Set<TargetId>();
		for (const line of [...big, ...shownRows]) for (const id of line.targetIds) ids.add(id);
		listed = ids;
	});

	let box = $state<HTMLElement | null>(null);
	const signature = $derived(lines.map((line) => `${line.key}:${line.count}`).join('|') + `@${Math.round(room)}`);
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
			// The section's own padding (p-2) is room the margins (-m-2) give back.
			if (!el || el.scrollHeight <= wanted + 17) return;
			if (level < 4) {
				// A step that changes nothing (no tiles, no quiet lines) is skipped.
				let next = level + 1;
				if (next === 1 && big.length === 0) next = 2;
				if (next === 2 && quiet.length === 0) next = 3;
				level = next;
				return;
			}
			if (cut === Infinity) {
				// First guess: the rows that end above the room, less one for "N more".
				const items = Array.from(el.querySelectorAll<HTMLElement>('[data-row]'));
				const firstOut = items.findIndex((item) => item.offsetTop + item.offsetHeight > wanted);
				cut = Math.max(1, (firstOut === -1 ? items.length : firstOut) - 1);
			} else if (cut > 1) {
				cut -= 1;
			}
		});
		return () => cancelAnimationFrame(frame);
	});

	function since(line: ProblemLine): string {
		void now;
		return line.since ? formatRelative(line.since) : '';
	}

	function href(line: ProblemLine): string | null {
		return line.targetIds.length === 1 ? `/targets/${line.targetIds[0]}` : null;
	}

	function title(line: ProblemLine): string {
		return line.names.join(', ');
	}
</script>

{#if lines.length > 0}
	<section
		class="relative -m-2 flex min-h-0 shrink-0 flex-col gap-3 overflow-hidden p-2"
		style:max-height={room === Infinity ? undefined : `${room + 16}px`}
		aria-label={m.wall_problems_aria()}
		data-wall-info
		bind:this={box}
	>
		{#each big as line (line.key)}
			{@const link = href(line)}
			<svelte:element
				this={link ? 'a' : 'div'}
				href={link}
				class="tile tile--{line.tone} relative block shrink-0 rounded-[var(--radius-card)] border bg-surface px-5 py-3.5 shadow-lift"
			>
				<span class="flex items-center justify-between gap-4">
					<Plate tone={line.tone as SkyRow['tone']} label={line.plate} size="md" class="plate-xl" />
					{#if line.since}
						<span class="tnum shrink-0 text-[1.0625rem] text-ink-2">{since(line)}</span>
					{/if}
				</span>
				<span class="mt-1.5 flex min-w-0 items-center gap-2.5">
					{#if line.kind}<KindIcon kind={line.kind} class="tile-icon text-ink-2" />{/if}
					<span class="tile-name display min-w-0 flex-1 text-ink" title={title(line)}>{title(line)}</span>
					{#if line.count > 1 && line.names.length === 1}
						<span class="times tnum shrink-0">{m.wall_problems_times({ count: line.count })}</span>
					{/if}
				</span>
				<span class="tile-what mt-1 line-clamp-2 text-ink-2" title={line.what}>{line.what}</span>
			</svelte:element>
		{/each}

		{#if shownRows.length > 0 || summary || hiddenCount > 0}
			<ul class="flex flex-col gap-1.5">
				{#each shownRows as line (line.key)}
					{@const link = href(line)}
					<li class="shrink-0" data-row>
						<svelte:element
							this={link ? 'a' : 'div'}
							href={link}
							class="row row--{line.rank >= QUIET_RANK ? 'quiet' : 'serious'} flex min-w-0 items-start gap-3 rounded-[10px] bg-surface/90 px-4 shadow-lift"
							class:row--compact={compact}
						>
							<Plate tone={line.tone as SkyRow['tone']} label={line.plate} size="md" class="mt-0.5 shrink-0" />
							<span class="flex min-w-0 flex-1 flex-col">
								<span class="flex min-w-0 items-center gap-2">
									{#if line.kind}<KindIcon kind={line.kind} class="size-5 shrink-0 text-ink-2" />{/if}
									<span class="row-name min-w-0 truncate font-semibold text-ink" title={title(line)}>{title(line)}</span>
									{#if line.count > 1 && line.names.length === 1}
										<span class="times tnum shrink-0">{m.wall_problems_times({ count: line.count })}</span>
									{/if}
									{#if line.since && line.rank < QUIET_RANK}
										<span class="row-since tnum ml-auto shrink-0 pl-2 text-ink-3">{since(line)}</span>
									{/if}
								</span>
								<span class="row-what [overflow-wrap:anywhere] text-ink-2" class:line-clamp-1={compact} class:line-clamp-2={!compact} title={line.what}>{line.what}</span>
							</span>
						</svelte:element>
					</li>
				{/each}
				{#if hiddenCount > 0}
					<li class="shrink-0 px-4 text-[1.0625rem] font-semibold text-ink" data-row>
						<a href="/alerts" class="underline decoration-line-strong underline-offset-4">{m.wall_problems_more({ count: hiddenCount })}</a>
					</li>
				{/if}
				{#if summary}
					<li class="shrink-0" data-row>
						<a href="/alerts" class="summary flex min-w-0 items-center gap-3 rounded-[10px] bg-surface/80 px-4 py-2 text-ink-2 shadow-lift">
							<span class="line-clamp-2 min-w-0" title={summary}>{summary}</span>
						</a>
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
		font-size: 1.75rem;
		line-height: 1.15;
	}
	@media (max-width: 1023px) {
		.tile-name {
			font-size: 1.5rem;
		}
	}
	.tile :global(.tile-icon) {
		width: 1.5rem;
		height: 1.5rem;
	}
	.tile-what {
		font-size: 1.125rem;
	}
	/* "×5": the same problem on the same device, folded into one line. */
	.times {
		border-radius: 999px;
		background: var(--c-surface-2);
		padding: 0.0625rem 0.5rem;
		font-size: 0.9375rem;
		font-weight: 700;
		color: var(--c-ink);
	}
	.row {
		padding-block: 0.5rem;
	}
	.row-name {
		font-size: 1.1875rem;
		line-height: 1.3;
	}
	.row-since {
		font-size: 0.9375rem;
	}
	.row-what {
		font-size: 1.0625rem;
		line-height: 1.3;
	}
	.row--quiet .row-name {
		font-size: 1.0625rem;
		font-weight: 600;
		color: var(--c-ink-2);
	}
	.row--quiet .row-what {
		font-size: 0.9375rem;
	}
	.row--compact {
		padding-block: 0.3125rem;
	}
	.summary {
		font-size: 1rem;
		font-weight: 600;
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
			margin-top: 0.125rem;
			font-size: 1.375rem;
		}
		.tile-what {
			margin-top: 0;
			font-size: 1rem;
		}
		.tile :global(.tile-icon) {
			width: 1.25rem;
			height: 1.25rem;
		}
		.row {
			gap: 0.5rem;
			padding-inline: 0.75rem;
		}
		.row-name {
			font-size: 1.0625rem;
		}
		.row-what {
			font-size: 0.9375rem;
		}
	}
	.tile {
		transition: transform 300ms var(--ease-out-expo);
	}
	a.tile:hover {
		transform: translateY(-1px);
	}
	.tile--warning {
		border-color: color-mix(in srgb, var(--c-warning) 55%, transparent);
	}
	.tile--advisory {
		border-color: color-mix(in srgb, var(--c-advisory) 55%, transparent);
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
		height: 1.875rem;
		padding-inline: 0.625rem;
		font-size: 0.9375rem;
		gap: 0.375rem;
	}
	.tile :global(.plate-xl svg) {
		width: 1rem;
		height: 1rem;
	}
</style>
