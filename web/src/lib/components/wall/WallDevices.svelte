<script lang="ts">
	/**
	 * Every device on the wall, as small tiles: name, an icon for its state,
	 * and the state's word whenever it is anything but reporting — problems
	 * first, then what is waiting, then the healthy rest, alphabetically.
	 * A device that reports but has an alert of its own wears that alert's
	 * tone. When there are more devices than the space can show, the tail
	 * (always healthy ones, given the order) folds into "and N more".
	 */
	import { AlertTriangle, CircleAlert, CircleCheck, CircleDashed, Info } from 'lucide-svelte';
	import type { Sky } from '#lib/components/overview/sky.js';
	import type { Target, TargetId } from '#lib/api/index.js';
	import { displayState, STATE_LABEL, type ProbeStatus, type TargetState } from '#lib/format.js';

	interface Props {
		targets: Target[];
		probes: Map<TargetId, ProbeStatus>;
		sky: Sky;
	}

	let { targets, probes, sky }: Props = $props();

	type ChipTone = 'warning' | 'advisory' | 'info' | 'ghost' | 'signal';
	interface Chip {
		id: TargetId;
		name: string;
		tone: ChipTone;
		word: string | null;
		order: number;
	}

	const ORDER: Record<ChipTone, number> = { warning: 0, advisory: 1, info: 2, ghost: 3, signal: 4 };

	const chips = $derived.by(() => {
		// The worst alert per device, from the same rows as the problems list.
		const alertTone = new Map<TargetId, { tone: ChipTone; word: string }>();
		for (const row of sky.needsYou) {
			if (row.kind !== 'alert' || row.rank > 2 || !row.target) continue;
			const tone = row.tone as ChipTone;
			const known = alertTone.get(row.target.id);
			if (!known || ORDER[tone] < ORDER[known.tone]) alertTone.set(row.target.id, { tone, word: row.plate });
		}
		const list: Chip[] = [];
		for (const target of targets) {
			const state: TargetState = displayState(target, probes.get(target.id));
			if (state === 'disabled') continue;
			let tone: ChipTone =
				state === 'online'
					? 'signal'
					: state === 'offline' || state === 'down'
						? 'warning'
						: state === 'misconfigured'
							? 'advisory'
							: 'ghost';
			let word: string | null = state === 'online' ? null : STATE_LABEL[state];
			const alert = alertTone.get(target.id);
			if (alert && ORDER[alert.tone] < ORDER[tone]) {
				tone = alert.tone;
				word = alert.word;
			}
			list.push({ id: target.id, name: target.name, tone, word, order: ORDER[tone] });
		}
		return list.sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
	});

	const dense = $derived(chips.length > 24);

	// How many fit: measured after each render, the overflow folds into a count.
	let box = $state<HTMLDivElement | null>(null);
	let fits = $state(Infinity);
	let boxH = $state(0);
	$effect(() => {
		void chips;
		void boxH;
		const el = box;
		if (!el) return;
		fits = Infinity;
		const frame = requestAnimationFrame(() => {
			const limit = el.clientHeight;
			const items = Array.from(el.querySelectorAll<HTMLElement>('[data-chip]'));
			const firstOut = items.findIndex((item) => item.offsetTop + item.offsetHeight > limit + 1);
			// Keep a slot free for the "and N more" chip.
			fits = firstOut === -1 ? Infinity : Math.max(0, firstOut - 1);
		});
		return () => cancelAnimationFrame(frame);
	});
	const shown = $derived(fits === Infinity ? chips : chips.slice(0, fits));
	const more = $derived(chips.length - shown.length);

	const ICON = {
		warning: AlertTriangle,
		advisory: CircleAlert,
		info: Info,
		ghost: CircleDashed,
		signal: CircleCheck
	};
</script>

<div class="relative min-h-0 flex-1 overflow-hidden" bind:this={box} bind:clientHeight={boxH}>
	<ul class="devices" class:devices--dense={dense} aria-label="Devices">
		{#each shown as chip (chip.id)}
			{@const Icon = ICON[chip.tone]}
			<li class="min-w-0" class:wide={chip.word !== null && chip.tone !== 'ghost'} data-chip>
				<a href="/targets/{chip.id}" class="chip chip--{chip.tone}">
					<Icon class="icon shrink-0" aria-hidden="true" />
					<span class="min-w-0 flex-1 truncate">{chip.name}</span>
					{#if chip.word}
						<span class="word shrink-0">{chip.word}</span>
					{:else}
						<span class="sr-only">Reporting</span>
					{/if}
				</a>
			</li>
		{/each}
		{#if more > 0}
			<li class="min-w-0"><span class="chip chip--more">And {more} more, all reporting</span></li>
		{/if}
	</ul>
</div>

<style>
	.devices {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(13rem, 1fr));
		grid-auto-rows: min-content;
		align-content: start;
		gap: 0.5rem;
	}
	/* A device in trouble takes two cells: its name and its state both fit. */
	@media (min-width: 640px) {
		.wide {
			grid-column: span 2;
		}
	}
	.devices--dense {
		grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
		gap: 0.375rem;
	}
	.chip {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		min-width: 0;
		height: 2.875rem;
		padding: 0 0.875rem;
		border-radius: 10px;
		border: 1px solid var(--c-line);
		background: color-mix(in srgb, var(--c-surface) 92%, transparent);
		box-shadow: var(--shadow-lift);
		color: var(--c-ink);
		font-size: 1.1875rem;
		font-weight: 600;
		line-height: 1;
	}
	.devices--dense .chip {
		height: 2.375rem;
		font-size: 1rem;
		padding: 0 0.625rem;
	}
	.chip :global(.icon) {
		width: 1.25rem;
		height: 1.25rem;
	}
	.word {
		font-size: 0.8125em;
		font-weight: 700;
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}
	.chip--signal :global(.icon) {
		color: var(--c-signal);
	}
	.chip--ghost {
		color: var(--c-ink-2);
	}
	.chip--warning {
		border: 2px solid color-mix(in srgb, var(--c-warning) 65%, transparent);
		background: linear-gradient(var(--c-warning-soft), var(--c-warning-soft)), var(--c-surface);
		color: var(--c-warning-ink);
	}
	.chip--advisory {
		border: 2px solid color-mix(in srgb, var(--c-advisory) 65%, transparent);
		background: linear-gradient(var(--c-advisory-soft), var(--c-advisory-soft)), var(--c-surface);
		color: var(--c-advisory-ink);
	}
	.chip--info {
		border-color: color-mix(in srgb, var(--c-info) 50%, transparent);
		color: var(--c-info-ink);
	}
	.chip--more {
		color: var(--c-ink-2);
		font-weight: 500;
		box-shadow: none;
	}
	a.chip:hover {
		border-color: var(--c-line-strong);
	}
</style>
