<script lang="ts">
	/**
	 * The wall's living backdrop: a misty autumn forest, a stag grazing on the
	 * bank of a stream, toadstools, drifting leaves. A drop-in for the Paris
	 * scene (same props, plus `pulse`).
	 *
	 * Drawn on a 1920 x 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky
	 * follows the real hour inside the brightness band of the wall's theme;
	 * after dark the toadstools glow and fireflies wake.
	 *
	 * The network's state is the weather and the stag's mood: `mood`
	 * (`calm|clouded|storm`, or `ok|warn|critical`) brings clouds, then rain
	 * and lightning; while problems are open the stag keeps its head up, and a
	 * new problem makes it snap its head up and freeze. `pulse` plays one
	 * gesture per change of `at`: `down` the stag lifts its head and freezes,
	 * `up` it goes back to grazing, `resolved` a gust swirls the leaves.
	 * Hidden tabs pause everything; reduced motion freezes the scene.
	 */
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import { phaseAt, isLit, celestialAt, type SceneTheme } from '#lib/components/wall/paris/daylight.js';
	import ForestDeer from './ForestDeer.svelte';
	import ForestTrees from './ForestTrees.svelte';
	import ForestFx from './ForestFx.svelte';
	import Mushroom from './Mushroom.svelte';
	import { forestStyle } from './palette.js';
	import {
		STAGE_W,
		STAGE_H,
		makeTrees,
		makeLeaves,
		makeGust,
		makeStars,
		makeFireflies,
		normalizeMood,
		type ForestMood,
		type ForestPulse
	} from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood: ForestMood;
		/** Problems on the board: while any are open the stag keeps its head up. */
		problems: number;
		/** Bumped by the wall when a new problem appears: the stag snaps its head up once. */
		startle?: number;
		/** The tab is hidden: nothing moves. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		pulse?: ForestPulse;
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

	// --- Stage fit ---------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light and weather -------------------------------------------------------

	// Re-read once a minute: nothing in the sky moves faster than that.
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const weather = $derived(normalizeMood(mood));
	const style = $derived(forestStyle(theme, phase, weather));
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));
	const shafts = $derived(weather === 'ok' && !lit);

	// --- The stag ----------------------------------------------------------------

	const trouble = $derived(problems > 0);

	// One gesture at a time: the latest of the `pulse` prop and a startle.
	let seq = 0;
	let gesture = $state<ForestPulse>(null);
	let lastAt = untrack(() => pulse?.at);
	let lastStartle = untrack(() => startle);
	$effect(() => {
		const next = pulse;
		if (!next || next.at === lastAt) return;
		lastAt = next.at;
		untrack(() => {
			if (motion) gesture = { kind: next.kind, at: ++seq };
		});
	});
	$effect(() => {
		const value = startle;
		if (value === lastStartle) return;
		lastStartle = value;
		untrack(() => {
			if (motion) gesture = { kind: 'down', at: ++seq };
		});
	});
	const gust = $derived(gesture?.kind === 'resolved' ? gesture : null);

	// --- Shapes (built once) -----------------------------------------------------

	const FAR = makeTrees(11, 44, -40, 1960, 690, 230, 340);
	const MID = makeTrees(23, 24, -40, 1960, 790, 280, 400);
	const NEAR = [...makeTrees(37, 1, 70, 120, 880, 640, 660), ...makeTrees(41, 1, 1800, 1850, 880, 620, 640)];
	const LEAVES = makeLeaves(5, 22, 0, 1920, 120, 760);
	const GUST = makeGust(9, 18, 900, 380);
	const STARS = makeStars(3, 90, STAGE_W, 520);
	const FLIES = makeFireflies(7, 16, 200, 1750, 560, 860);
	const CLOUDS = [
		{ x: 300, y: 130, s: 1.1, dur: 120 },
		{ x: 1100, y: 90, s: 1.4, dur: 150 },
		{ x: 1600, y: 210, s: 0.9, dur: 130 },
		{ x: 700, y: 250, s: 1.2, dur: 170 },
		{ x: 1350, y: 150, s: 1.6, dur: 160 },
		{ x: 80, y: 220, s: 1.0, dur: 140 }
	];
	const cloudCount = $derived(weather === 'critical' ? 6 : weather === 'warn' ? 4 : 2);
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
	{#if scale > 0}
		<div class="stage" style:transform="translate({offsetX}px, {offsetY}px) scale({scale})">
			<div class="sky"></div>
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}" fill="none">
				<defs>
					<linearGradient id="fw-mist" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--f-mist)" stop-opacity="0" />
						<stop offset=".5" stop-color="var(--f-mist)" />
						<stop offset="1" stop-color="var(--f-mist)" stop-opacity="0" />
					</linearGradient>
					<linearGradient id="fw-water" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--f-water)" />
						<stop offset="1" stop-color="var(--f-water2)" />
					</linearGradient>
					<linearGradient id="fw-shaft" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--f-sun)" stop-opacity=".35" />
						<stop offset="1" stop-color="var(--f-sun)" stop-opacity="0" />
					</linearGradient>
				</defs>

				<g fill="var(--f-star)" class="stars" class:stars--on={starsOn}>
					{#each STARS as star, i (i)}
						<circle cx={star.x} cy={star.y} r={star.r} class:twinkle={star.twinkle} style:animation-delay="-{star.delay}ms" />
					{/each}
				</g>

				<g class="celestial" class:celestial--dim={weather === 'warn'} class:celestial--hidden={weather === 'critical'}>
					<circle cx={body.x} cy={body.y} r="190" fill="var(--f-sun-halo)" opacity=".7" />
					{#if body.body === 'sun'}
						<circle cx={body.x} cy={body.y} r="70" fill="var(--f-sun)" />
					{:else}
						<circle cx={body.x} cy={body.y} r="62" fill="var(--f-moon)" />
						<circle cx={body.x - 18} cy={body.y - 12} r="11" fill="var(--f-sun-halo)" />
						<circle cx={body.x + 20} cy={body.y + 18} r="8" fill="var(--f-sun-halo)" />
					{/if}
				</g>
				{#if shafts}
					<g class="shafts" fill="url(#fw-shaft)">
						<path d="M{body.x - 60} {body.y + 60}L{body.x - 700} 820H{body.x - 560}L{body.x} {body.y + 70}z" />
						<path d="M{body.x} {body.y + 70}L{body.x - 300} 830H{body.x - 160}L{body.x + 40} {body.y + 70}z" />
					</g>
				{/if}

				<g class="clouds">
					{#each CLOUDS.slice(0, cloudCount) as c, i (i)}
						<g transform="translate({c.x} {c.y}) scale({c.s})">
							<g class="cloud" style="animation-duration:{c.dur}s">
								<g fill={weather === 'critical' ? 'var(--f-cloud-storm)' : 'var(--f-cloud)'}>
									<ellipse cx="0" cy="0" rx="150" ry="26" /><ellipse cx="-40" cy="-20" rx="80" ry="30" /><ellipse cx="50" cy="-14" rx="64" ry="26" />
								</g>
							</g>
						</g>
					{/each}
				</g>

				<!-- the wood: far, misty, then close -->
				<g opacity=".5"><ForestTrees trees={FAR} /></g>
				<rect x="0" y="560" width={STAGE_W} height="230" fill="url(#fw-mist)" class="mist" />
				<g opacity=".82"><ForestTrees trees={MID} /></g>
				<rect x="0" y="700" width={STAGE_W} height="160" fill="url(#fw-mist)" class="mist mist--b" />

				<!-- stream, then the bank -->
				<rect x="0" y="880" width={STAGE_W} height="200" fill="url(#fw-water)" />
				<g class="reflect" opacity=".32">
					<rect x="280" y="930" width="38" height="130" rx="19" fill="var(--f-c1)" />
					<rect x="760" y="940" width="30" height="110" rx="15" fill="var(--f-c3)" />
					<rect x="1180" y="934" width="40" height="120" rx="20" fill="var(--f-c2)" />
					<rect x="1640" y="940" width="34" height="118" rx="17" fill="var(--f-c1)" />
				</g>
				<g class="ripples" stroke="var(--f-glint)" stroke-width="3" stroke-linecap="round" opacity=".45">
					<path d="M120 960h140M460 1010h200M900 970h160M1340 1020h210M1560 975h170M640 1050h120" />
				</g>
				<g fill="var(--f-ground2)"><ellipse cx="1020" cy="1000" rx="46" ry="14" /><ellipse cx="1110" cy="1030" rx="30" ry="10" /></g>
				<path d="M0 800C300 784 620 812 960 796S1580 784 1920 802V906C1600 918 1300 892 980 906S300 898 0 910z" fill="var(--f-ground)" />
				<path d="M0 806C300 790 620 818 960 802S1580 790 1920 808" stroke="var(--f-grass)" stroke-width="8" stroke-linecap="round" opacity=".7" />

				<ForestTrees trees={NEAR} />

				<!-- toadstools, fireflies and the stag -->
				<Mushroom x={220} y={890} s={2.2} glow /><Mushroom x={280} y={900} s={1.5} glow /><Mushroom x={1560} y={888} s={2} glow /><Mushroom x={1625} y={900} s={1.3} glow /><Mushroom x={770} y={880} s={1.6} glow />
				<g transform="translate(1360 884) scale(2.1)"><ForestDeer alert={trouble} pulse={gesture} /></g>
				{#if lit}
					<g class="flies" fill="var(--f-fly)">
						{#each FLIES as f, i (i)}
							<circle cx={f.x} cy={f.y} r="3.4" class="fly" style="--dx:{f.dx}px;--dy:{f.dy}px;animation-duration:{f.dur}s;animation-delay:{f.delay}s" />
						{/each}
					</g>
				{/if}

				<ForestFx
					uid="fw"
					x={0}
					y={0}
					w={STAGE_W}
					h={STAGE_H}
					mood={weather}
					pulse={gust}
					leaves={LEAVES}
					gust={GUST}
					gustX={1280}
					gustY={880}
					boltX={1080}
					boltY={0}
					boltH={640}
				/>
				<rect x="0" y="0" width={STAGE_W} height={STAGE_H} fill="#101820" class="veil" class:veil--warn={weather === 'warn'} class:veil--storm={weather === 'critical'} />
			</svg>
		</div>
	{/if}
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: var(--sky-bottom);
	}
	.stage {
		position: absolute;
		left: 0;
		top: 0;
		width: 1920px;
		height: 1080px;
		transform-origin: 0 0;
	}
	.sky {
		position: absolute;
		inset: 0;
		background: linear-gradient(180deg, var(--sky-top), var(--sky-bottom) 72%);
	}
	.layer {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}
	.stars { opacity: 0; transition: opacity 3s ease; }
	.stars--on { opacity: 1; }
	.twinkle { animation: twinkle 4s ease-in-out infinite; }
	.celestial, .veil, .mist, .shafts { transition: opacity 1.5s ease; }
	.celestial--dim { opacity: 0.4; }
	.celestial--hidden { opacity: 0; }
	.veil { opacity: 0; pointer-events: none; }
	.veil--warn { opacity: 0.12; }
	.veil--storm { opacity: 0.3; }
	.mist { opacity: 0.85; }
	.mist--b { animation: drift 50s ease-in-out infinite alternate; }
	.cloud { animation: drift 100s ease-in-out infinite alternate; }
	.ripples { animation: ripple 7s ease-in-out infinite alternate; }
	.reflect { animation: wobble 6s ease-in-out infinite alternate; }
	.fly { animation: fly 6s ease-in-out infinite alternate; opacity: 0; }
	@keyframes drift { from { transform: translateX(-40px); } to { transform: translateX(40px); } }
	@keyframes ripple { from { transform: translateX(-22px); } to { transform: translateX(22px); } }
	@keyframes wobble { from { transform: translateX(-5px); } to { transform: translateX(5px); } }
	@keyframes twinkle { 0%, 100% { opacity: 1; } 50% { opacity: 0.35; } }
	@keyframes fly {
		0% { transform: translate(0, 0); opacity: 0; }
		30%, 60% { opacity: 0.9; }
		100% { transform: translate(var(--dx), var(--dy)); opacity: 0; }
	}
	.scene--paused :global(*) { animation-play-state: paused !important; }
	.scene--still :global(*) { animation: none !important; transition: none !important; }
	.scene--still .fly { opacity: 0.7; }
	@media (prefers-reduced-motion: reduce) {
		.mist--b, .cloud, .ripples, .reflect, .fly, .twinkle { animation: none; }
		.fly { opacity: 0.7; }
	}
</style>
