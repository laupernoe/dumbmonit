<script lang="ts">
	/**
	 * The wall's living backdrop: the view from a Glacier Express panoramic
	 * carriage. Big curved windows on the Alps (peaks, a Landwasser-like
	 * viaduct with a red train, a lake, firs, telegraph poles) scroll by in
	 * parallax; on the table, a cup of tea, the DumbMonit pigeon, a card with
	 * the number of problems and a lamp whose shade follows the network.
	 * A drop-in for the Paris scene (same props).
	 *
	 * Drawn on a 1920 x 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The hour sets
	 * the sky, the sun or moon, stars and reading lights, inside the brightness
	 * band of the wall's theme (see `paris/daylight.ts`).
	 *
	 * State as weather: calm is clear, clouded is a veil of clouds, storm is
	 * rain on the glass and lightning. Pulses are gestures: `down`, the train
	 * brakes, the landscape stops and the tea sloshes; `up`, it pulls away;
	 * `resolved`, it leaves a tunnel into bright light. A new problem
	 * (`startle`) jolts the table once. Hidden tabs freeze it; reduced motion
	 * keeps a still picture.
	 */
	import { reducedMotion } from '#lib/ui/index.js';
	import SceneDefs from '#lib/components/status/scenes/SceneDefs.svelte';
	import {
		phaseAt,
		celestialAt,
		sceneStyle,
		type SceneMood,
		type SceneTheme
	} from '#lib/components/wall/paris/daylight.js';
	import TrainCabin from './TrainCabin.svelte';
	import { moodLevel, trainStyle, type Pulse } from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood: SceneMood | 'ok' | 'warn' | 'critical';
		/** Problems on the board: the number on the card. */
		problems: number;
		/** Bumped by the wall when a new problem appears: the table jolts once. */
		startle?: number;
		/** The tab is hidden: nothing moves. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		/** A gesture to replay whenever `at` changes. */
		pulse?: Pulse;
		class?: string;
	}

	let {
		theme,
		now,
		mood,
		problems,
		startle = 0,
		paused = false,
		focus = 0,
		pulse = null,
		class: className = ''
	}: Props = $props();

	const motion = !reducedMotion();
	const STAGE_W = 1920;
	const STAGE_H = 1080;
	const MOODS: SceneMood[] = ['calm', 'clouded', 'storm'];

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// Re-read once a minute: nothing in the sky moves faster than that.
	const minute = $derived(new Date(Math.floor(now.getTime() / 60_000) * 60_000));
	const phase = $derived(phaseAt(minute));
	const body = $derived(celestialAt(minute));
	const level = $derived(moodLevel(mood));
	const tone = $derived(level === 2 || problems > 2 ? '#e0533d' : level === 1 || problems > 0 ? '#e8a33a' : '#4fae7c');
	const style = $derived(`${sceneStyle(theme, phase, MOODS[level])};${trainStyle(theme, phase)};--scene-tone:${tone}`);
</script>

<div
	class="scene {className}"
	class:scene--paused={paused}
	class:scene--still={!motion}
	{style}
	bind:clientWidth={boxW}
	bind:clientHeight={boxH}
	aria-hidden="true"
>
	<SceneDefs />
	{#if scale > 0}
		<svg
			class="stage"
			width={STAGE_W}
			height={STAGE_H}
			viewBox="0 0 {STAGE_W} {STAGE_H}"
			style:transform="translate({offsetX}px, {offsetY}px) scale({scale})"
			fill="none"
			focusable="false"
		>
			<TrainCabin
				W={STAGE_W}
				H={STAGE_H}
				windows={3}
				{mood}
				{pulse}
				{startle}
				{paused}
				{problems}
				sun={body.body === 'sun' ? { x: body.x, y: body.y } : null}
				moon={body.body === 'moon' ? { x: body.x, y: body.y } : null}
				idp="trw"
			/>
		</svg>
	{/if}
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
		contain: strict;
		background: var(--tr-int);
	}
	.stage {
		position: absolute;
		left: 0;
		top: 0;
		transform-origin: 0 0;
	}
	.scene--paused :global(*),
	.scene--still :global(*) {
		animation-play-state: paused !important;
	}
</style>
