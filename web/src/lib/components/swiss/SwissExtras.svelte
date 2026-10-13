<script lang="ts">
	/**
	 * The Swiss scenes' extras in one SVG group: station clock, split-flap board
	 * and the calendar effects (snow, 1 August fireworks, Escalade cauldron).
	 * Each scene says where each piece goes, in its own coordinates; leave a
	 * piece out and it is not drawn. `?date=YYYY-MM-DD` forces the day.
	 */
	import { resolveSwissDay } from '#lib/scene/swiss.js';
	import SbbClock from './SbbClock.svelte';
	import SplitFlapBoard from './SplitFlapBoard.svelte';
	import SwissSeason from './SwissSeason.svelte';

	interface Props {
		mood?: 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
		problems?: number | null;
		clock?: { x: number; y: number; r?: number; ground?: number | null } | null;
		board?: { x: number; y: number; w?: number; ground?: number | null } | null;
		area: { x: number; y: number; w: number; h: number };
		marmite?: { x: number; y: number; s?: number } | null;
		unit?: number;
		paused?: boolean;
	}
	let {
		mood = 'ok',
		pulse = null,
		problems = null,
		clock = null,
		board = null,
		area,
		marmite = null,
		unit = 1,
		paused = false
	}: Props = $props();

	const day = resolveSwissDay(
		new Date(),
		typeof window === 'undefined' ? '' : window.location.search
	);
</script>

<g class="swiss-extras">
	<SwissSeason {day} {area} {marmite} {unit} {paused} />
	{#if clock}<SbbClock {...clock} {paused} />{/if}
	{#if board}<SplitFlapBoard {...board} {mood} {pulse} {problems} {paused} />{/if}
</g>
