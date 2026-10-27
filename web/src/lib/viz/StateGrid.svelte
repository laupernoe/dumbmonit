<script lang="ts">
	/**
	 * Grid of states: disks of a bay, nodes of a cluster, ports of a switch
	 * faceplate (set `rows` for the two-row layout of real hardware). Each tile
	 * carries a glyph (✓ ! ✕ –), its name and, by default, the state in words,
	 * so nothing depends on colour; the degraded state is also hatched.
	 * Presentational: strings come in as props.
	 */
	import { TONE_COLOR, TONE_GLYPH, TONE_INK, TONE_SOFT } from './tone.js';
	import type { Tone } from './types.js';

	export interface StateCell {
		key: string;
		/** Short name: a port number, a disk slot, a node. */
		label: string;
		tone: Tone;
		/** The state in words ("Up", "Down", "Degraded"). */
		stateLabel: string;
		/** More, in the tooltip only ("1 Gb/s, 12 errors"). */
		detail?: string;
	}
	interface Props {
		cells: StateCell[];
		/** Accessible name of the list. */
		label: string;
		/** Fixed number of rows, filled column by column (switch faceplate). Auto grid when 0. */
		rows?: number;
		/** Write the state under the label. Off only when a legend names the glyphs. */
		showState?: boolean;
		onselect?: (key: string) => void;
		/** Key of the selected tile (pressed button), when `onselect` is set. */
		selected?: string | null;
		class?: string;
	}
	let { cells, label, rows = 0, showState = true, onselect, selected = null, class: cls = '' }: Props = $props();

	const faceplate = $derived(rows > 0);
</script>

<ul
	class={`m-0 grid list-none gap-1.5 p-0 ${cls}`}
	aria-label={label}
	style:grid-template-columns={faceplate ? undefined : 'repeat(auto-fill, minmax(4.5rem, 1fr))'}
	style:grid-template-rows={faceplate ? `repeat(${rows}, minmax(2.5rem, auto))` : undefined}
	style:grid-auto-flow={faceplate ? 'column' : undefined}
	style:grid-auto-columns={faceplate ? 'minmax(2.5rem, 1fr)' : undefined}
>
	{#each cells as cell (cell.key)}
		<li class="min-w-0">
			<svelte:element
				this={onselect ? 'button' : 'div'}
				type={onselect ? 'button' : undefined}
				class="tile"
				class:tile--hatched={cell.tone === 'advisory'}
				class:tile--selected={onselect !== undefined && selected === cell.key}
				aria-pressed={onselect ? selected === cell.key : undefined}
				title={cell.detail ? `${cell.label}: ${cell.stateLabel}. ${cell.detail}` : `${cell.label}: ${cell.stateLabel}`}
				style:border-color={TONE_COLOR[cell.tone]}
				style:background-color={TONE_SOFT[cell.tone]}
				onclick={onselect ? () => onselect(cell.key) : undefined}
			>
				<span class="flex items-center justify-center gap-1 text-xs font-medium text-ink">
					<span aria-hidden="true" class="glyph" style:color={TONE_INK[cell.tone]}>{TONE_GLYPH[cell.tone]}</span>
					<span class="truncate">{cell.label}</span>
				</span>
				{#if showState}
					<span class="truncate text-[0.6875rem]" style:color={TONE_INK[cell.tone]}>{cell.stateLabel}</span>
				{:else}
					<span class="sr-only">{cell.stateLabel}</span>
				{/if}
			</svelte:element>
		</li>
	{/each}
</ul>

<style>
	.tile {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		width: 100%;
		min-height: 2.5rem;
		padding: 0.25rem 0.375rem;
		border: 1px solid;
		border-radius: 0.375rem;
		text-align: center;
		font: inherit;
		color: inherit;
	}
	.tile--hatched {
		background-image: repeating-linear-gradient(135deg, transparent 0 4px, var(--c-advisory-soft) 4px 6px);
	}
	.tile--selected {
		box-shadow: 0 0 0 2px var(--c-ink);
		font-weight: 600;
	}
	.glyph {
		font-weight: 700;
		line-height: 1;
	}
	button.tile {
		cursor: pointer;
	}
	button.tile:focus-visible {
		outline: 2px solid var(--c-focus);
		outline-offset: 2px;
	}
</style>
