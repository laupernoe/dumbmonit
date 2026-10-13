<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop: a server room run by cats (racks, cables, a
	 * cat asleep on a warm rack, one in a cardboard box, one batting a cable),
	 * with a window that follows the real hour. A drop-in for the Paris scene
	 * (same props, plus `pulse`).
	 *
	 * Drawn on a 1920 x 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The art sits
	 * on the right so the headline on the left reads on a plain wall.
	 *
	 * The network's state is woven in: advisories bring clouds and a cat on the
	 * keyboard, a storm brings rain, puffed tails and a chewed cable with
	 * sparks, problems hang a paper sign from the cable tray, a new problem
	 * startles the cats once. `pulse` plays a gesture: down = a cat pushes a
	 * server off the shelf, up = it puts it back proudly, resolved = everybody
	 * naps. Hidden tabs pause everything; reduced motion freezes the scene.
	 */
	import { untrack } from 'svelte';
	import CatsRoom from './CatsRoom.svelte';
	import { phaseAt } from '#lib/components/wall/paris/daylight.js';
	import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood?: 'calm' | 'clouded' | 'storm' | 'ok' | 'warn' | 'critical';
		problems?: number;
		/** Bumped by the wall when a new problem appears: everyone startles once. */
		startle?: number;
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage. */
		focus?: number;
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
		class?: string;
	}

	let {
		theme,
		now,
		mood = 'calm',
		problems = 0,
		startle = 0,
		paused = false,
		focus = 0,
		pulse = null,
		class: className = ''
	}: Props = $props();

	const STAGE_W = 1920;
	const STAGE_H = 1080;
	// The 1100 x 540 sheet sits bottom-right of the stage, scaled by 1.36.
	const VIEW = '-311 -254 1411 794';

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// The window changes once a minute at most.
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const phase = $derived(phaseAt(new Date(minuteKey * 60_000)));
	const sky = $derived(
		phase === 'dawn' || phase === 'golden' ? 'golden' : phase === 'dusk' ? 'dusk' : phase === 'night' ? 'night' : 'day'
	);

	const sign = $derived(
		problems > 0 ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null
	);

	// A new problem startles everyone for a moment.
	let startled = $state(false);
	let lastStartle = untrack(() => startle);
	$effect(() => {
		if (startle === lastStartle) return;
		lastStartle = startle;
		startled = true;
		const timer = setTimeout(() => (startled = false), 2200);
		return () => clearTimeout(timer);
	});
</script>

<div
	class="scene {className}"
	bind:clientWidth={boxW}
	bind:clientHeight={boxH}
	aria-hidden="true"
>
	{#if scale > 0}
		<div class="stage" style:transform="translate({offsetX}px, {offsetY}px) scale({scale})">
			<CatsRoom {mood} {pulse} {sky} {sign} {startled} {paused} tone={theme} view={VIEW} />
		</div>
	{/if}
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
	}
	.stage {
		position: absolute;
		left: 0;
		top: 0;
		width: 1920px;
		height: 1080px;
		transform-origin: 0 0;
	}
</style>
