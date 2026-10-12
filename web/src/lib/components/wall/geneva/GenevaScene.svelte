<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop: Lake Geneva at the foot of the Jet d'eau,
	 * the Alps behind, the Bains des Pâquis jetty in front, and a DumbMonit
	 * pigeon on it. A drop-in for the Paris scene (same props).
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky
	 * follows the real hour inside the brightness band of the wall's theme
	 * (see `paris/daylight.ts`); after dark the jet is floodlit, the old town,
	 * the bridge and the jetty light up and the stars come out.
	 *
	 * The network's state is woven in: clouds gather with advisories, a storm
	 * brings a soft drizzle and a darker sky, and the pigeon holds up a sign
	 * with the number of problems. A new problem startles it once.
	 *
	 * Motion is gentle: the jet sways, its water climbs and its spray falls,
	 * ripples glide, a sailing boat crosses the lake, a swan bobs, clouds drift,
	 * and now and then the pigeon glances about and tips its hat. Transform and
	 * opacity only. Hidden tabs pause everything; reduced motion freezes a still
	 * picture.
	 */
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import WallPigeon from '#lib/components/wall/paris/WallPigeon.svelte';
	import { STAGE_W, STAGE_H, STARS, RAIN } from '#lib/components/wall/paris/geometry.js';
	import {
		phaseAt,
		isLit,
		celestialAt,
		sceneStyle,
		type SceneMood,
		type SceneTheme
	} from '#lib/components/wall/paris/daylight.js';
	import {
		JET,
		HORIZON,
		DECK_Y,
		PIGEON_X,
		SPRAY,
		ALPS,
		SHORE,
		WINDOWS,
		RIPPLES,
		PILES
	} from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood: SceneMood;
		/** Problems on the board: the number on the protest sign. */
		problems: number;
		/** Bumped by the wall when a new problem appears: the pigeon startles once. */
		startle?: number;
		/** The tab is hidden: nothing moves, nothing is scheduled. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
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
		class: className = ''
	}: Props = $props();

	const motion = !reducedMotion();

	// --- Stage fit ----------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(
		Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale))
	);
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light --------------------------------------------------------------------

	// Re-read once a minute: nothing in the sky moves faster than that.
	const minute = $derived(new Date(Math.floor(now.getTime() / 60_000) * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	// The water of the jet: white by day, floodlit gold after dark, pale on a dark day.
	const jetColor = $derived(
		theme === 'light' ? '#ffffff' : lit ? '#ffe9b8' : '#c9d6ee'
	);
	const style = $derived(`${sceneStyle(theme, phase, mood)};--jet:${jetColor}`);
	const starsOn = $derived(
		phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn')
	);

	const trouble = $derived(problems > 0);
	const raining = $derived(mood === 'storm');
	const cloudCount = $derived(mood === 'storm' ? 6 : mood === 'clouded' ? 4 : 2);
	const CLOUDS = [
		{ y: 110, s: 1.1, dur: 260, delay: -40 },
		{ y: 290, s: 0.8, dur: 320, delay: -210 },
		{ y: 200, s: 0.95, dur: 290, delay: -150 },
		{ y: 400, s: 1.2, dur: 340, delay: -290 },
		{ y: 60, s: 0.7, dur: 300, delay: -250 },
		{ y: 330, s: 1.0, dur: 380, delay: -60 }
	];

	// --- The pigeon ------------------------------------------------------------------

	let startled = $state(false);
	let tip = $state(false);
	let look = $state(0.5);
	let epoch = 0;
	const between = (a: number, b: number) => a + Math.random() * (b - a);

	// A timer carries the epoch it was set under; a pause or a change of mood
	// bumps it and every stale callback does nothing.
	function later(fn: () => void, ms: number) {
		const mine = epoch;
		setTimeout(() => {
			if (mine === epoch) fn();
		}, ms);
	}

	// A small moment every 40–80 s: a glance at the jet or the lake, a tip of the hat.
	function moment() {
		look = Math.random() < 0.5 ? -0.7 : 0.7;
		if (!trouble && Math.random() < 0.7) {
			tip = true;
			later(() => (tip = false), 1600);
		}
		later(() => (look = trouble ? 0 : 0.5), 3500);
		later(moment, between(40_000, 80_000));
	}

	$effect(() => {
		void trouble;
		const hidden = paused;
		untrack(() => {
			epoch++;
			tip = false;
			look = trouble ? 0 : 0.5;
			if (!hidden && motion) later(moment, between(10_000, 25_000));
		});
		return () => {
			epoch++;
		};
	});

	// A new problem: the pigeon startles for a moment.
	let lastStartle = untrack(() => startle);
	$effect(() => {
		const value = startle;
		if (value === lastStartle) return;
		lastStartle = value;
		startled = true;
		const timer = setTimeout(() => (startled = false), 1800);
		return () => clearTimeout(timer);
	});

	const sign = $derived(
		trouble ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null
	);
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

			<svg class="layer stars" class:stars--on={starsOn} viewBox="0 0 {STAGE_W} {STAGE_H}">
				{#each STARS as star, i (i)}
					<circle
						cx={star.x}
						cy={star.y}
						r={star.r}
						fill="var(--star)"
						class:twinkle={star.twinkle}
						style:animation-delay="-{star.delay}ms"
					/>
				{/each}
			</svg>

			<div
				class="celestial"
				class:celestial--hidden={raining}
				style:transform="translate({body.x - 160}px, {body.y - 160}px)"
			>
				{#if body.body === 'sun'}
					<div class="sun"></div>
				{:else}
					<svg viewBox="0 0 320 320" class="moon">
						<circle cx="160" cy="160" r="110" fill="var(--sun-halo)" opacity="0.5" />
						<path d="M184 104 A64 64 0 1 0 214 206 A50 50 0 1 1 184 104 Z" fill="var(--moon)" />
					</svg>
				{/if}
			</div>

			{#each CLOUDS.slice(0, cloudCount) as cloud, i (i)}
				<div
					class="cloud"
					style:--still-x="{(i * 530 + 380) % 1700}px"
					style:top="{cloud.y}px"
					style:animation-duration="{cloud.dur}s"
					style:animation-delay="{cloud.delay}s"
				>
					<svg viewBox="0 0 300 110" style:width="{300 * cloud.s}px" style:height="{110 * cloud.s}px">
						<path
							d="M30 104 C2 104 0 66 30 62 C24 32 60 18 86 34 C98 6 152 0 170 30 C192 14 234 22 238 54 C270 52 292 78 276 104 Z"
							fill={mood === 'storm' ? 'var(--cloud-storm)' : 'var(--cloud)'}
						/>
					</svg>
				</div>
			{/each}

			<!-- The back: the Alps and Mont-Blanc, the Salève, the old town, the Mont-Blanc bridge, the lake. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<defs>
					<linearGradient id="gv-lake" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--far)" />
						<stop offset="1" stop-color="var(--near)" />
					</linearGradient>
					<linearGradient id="gv-jet" x1="0" y1="1" x2="0" y2="0">
						<stop offset="0" stop-color="var(--jet)" stop-opacity="0.95" />
						<stop offset="1" stop-color="var(--jet)" stop-opacity="0.5" />
					</linearGradient>
				</defs>
				<path d={ALPS.far} fill="var(--far-2)" />
				<path d={ALPS.snow} fill="var(--cloud)" opacity="0.9" />
				<path d={ALPS.saleve} fill="var(--far)" />
				<path d={SHORE.bank} fill="var(--facade-shade)" />
				<g fill="var(--facade)">
					{#each SHORE.blocks as b, i (i)}
						<rect x={b.x} y={HORIZON - b.h} width={b.w} height={b.h} />
					{/each}
				</g>
				<path d={SHORE.spire} fill="var(--roof)" />
				{#if lit}
					<g fill="var(--window-lit)" class="lit">
						{#each WINDOWS as w, i (i)}
							<rect x={w.x} y={HORIZON - w.y} width="5" height="6" />
						{/each}
					</g>
				{/if}
				<rect x="180" y={HORIZON + 14} width="760" height="7" fill="var(--tower)" />
				<g fill="var(--tower)">
					{#each PILES.bridge as x (x)}
						<rect {x} y={HORIZON + 20} width="9" height="14" />
					{/each}
				</g>
				{#if lit}
					<g fill="var(--window-lit)" class="lit">
						{#each PILES.bridge as x (x)}
							<circle cx={x + 4} cy={HORIZON + 10} r="3.2" />
						{/each}
					</g>
				{/if}

				<rect x="0" y={HORIZON + 20} width={STAGE_W} height={STAGE_H - HORIZON - 20} fill="url(#gv-lake)" />
				<g stroke="var(--cloud)" stroke-width="3" stroke-linecap="round" opacity="0.5">
					{#each RIPPLES as r, i (i)}
						<path class="ripple" d="M{r.x} {r.y}h{r.w}" style:animation-delay="-{r.delay}s" />
					{/each}
				</g>
				<path d="M{JET.x - 9} {JET.base + 20}h18l-5 {STAGE_H - JET.base - 140}h-8z" fill="var(--jet)" opacity="0.14" />
			</svg>

			{#if lit}
				<div class="glow flood" style:left="{JET.x - 150}px" style:top="{JET.top - 40}px"></div>
			{/if}

			<!-- The Jet d'eau: a column, a plume and falling spray, a mist at the foot. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<g class="jet">
					<path class="column" d="M{JET.x - 9} {JET.base}L{JET.x - 3} {JET.top}h6L{JET.x + 9} {JET.base}z" fill="url(#gv-jet)" />
					<path
						class="flow"
						d="M{JET.x} {JET.base}V{JET.top}"
						stroke="var(--cloud)"
						stroke-width="4"
						stroke-dasharray="26 22"
						stroke-linecap="round"
						opacity="0.7"
						fill="none"
					/>
					<g class="plume" fill="var(--jet)">
						<ellipse cx={JET.x} cy={JET.top + 6} rx="46" ry="30" opacity="0.5" />
						<ellipse cx={JET.x - 22} cy={JET.top + 30} rx="30" ry="22" opacity="0.4" />
						<ellipse cx={JET.x + 24} cy={JET.top + 34} rx="32" ry="24" opacity="0.4" />
					</g>
					{#each SPRAY as d, i (i)}
						<g class="drop" style:--dx="{d.dx}px" style:--dy="{d.dy}px" style:animation-delay="-{d.delay}s" style:animation-duration="{d.dur}s">
							<circle cx={JET.x} cy={JET.top + 4} r={d.r} fill="var(--jet)" />
						</g>
					{/each}
				</g>
				<ellipse class="mist" cx={JET.x} cy={JET.base + 4} rx="70" ry="12" fill="var(--jet)" opacity="0.55" />
			</svg>

			<!-- Boats: a sailing boat crosses the lake, a swan bobs on the left. -->
			<div class="mover sail">
				<svg viewBox="0 0 120 130" width="120" height="130">
					<path d="M60 8L60 92L14 92Z" fill="var(--cloud)" />
					<path d="M64 24L64 92L100 92Z" fill="var(--cloud)" opacity="0.8" />
					<path d="M6 98h108l-14 14H22z" fill="var(--roof)" />
					{#if lit}<circle cx="60" cy="6" r="3" fill="var(--window-lit)" />{/if}
				</svg>
			</div>
			<div class="swan" style:left="430px" style:top="{HORIZON + 330}px">
				<svg viewBox="0 0 110 80" width="110" height="80">
					<path d="M8 56c0-12 26-12 56-12 20 0 36 0 36 12 0 12-28 16-46 16S8 68 8 56z" fill="var(--cloud)" />
					<path d="M80 48c-10-8-6-30 4-34 6-2 10 2 10 6" fill="none" stroke="var(--cloud)" stroke-width="7" stroke-linecap="round" />
					<path d="M95 18l10 3-10 3z" fill="var(--pot)" />
				</svg>
			</div>

			<!-- The Bains des Pâquis: jetty on posts, a bathhouse, a little lighthouse. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<rect x="1180" y={DECK_Y} width="760" height="22" fill="var(--near-top)" />
				{#each PILES.jetty as x (x)}
					<rect {x} y={DECK_Y + 22} width="12" height="46" fill="var(--near)" />
				{/each}
				<rect x="1180" y={DECK_Y + 22} width="760" height="6" fill="var(--seam)" />
				<rect x="1192" y={DECK_Y - 112} width="22" height="112" fill="var(--facade)" />
				<path d="M1188 {DECK_Y - 112}l15-30 15 30z" fill="var(--brick)" />
				<rect x="1196" y={DECK_Y - 92} width="14" height="12" fill={lit ? 'var(--window-lit)' : 'var(--window)'} />
				<rect x="1700" y={DECK_Y - 84} width="170" height="84" fill="var(--facade)" />
				<path d="M1690 {DECK_Y - 84}h190l-24-26h-142z" fill="var(--brick)" />
				<g fill={lit ? 'var(--window-lit)' : 'var(--window)'}>
					<rect x="1716" y={DECK_Y - 62} width="22" height="30" />
					<rect x="1760" y={DECK_Y - 62} width="22" height="30" />
					<rect x="1804" y={DECK_Y - 62} width="22" height="30" />
				</g>
				<rect x="1230" y={DECK_Y - 8} width="320" height="5" fill="var(--lamp)" />
			</svg>
			{#if lit}
				<div class="glow lamp" style:left="1090px" style:top="{DECK_Y - 170}px"></div>
			{/if}

			<div class="cast" style:left="{PIGEON_X - 75}px" style:top="{DECK_Y - 187}px">
				<WallPigeon
					class="gaston"
					hat
					{tip}
					{sign}
					lookX={look}
					lookY={trouble ? -0.3 : 0}
					eyes={startled ? 'startled' : 'open'}
					blinkOffset={0.7}
				/>
			</div>

			{#if raining}
				<div class="rain">
					<svg viewBox="0 0 1920 1620">
						<path d={RAIN} stroke="var(--rain)" stroke-width="2.4" stroke-linecap="round" />
					</svg>
				</div>
				<div class="veil"></div>
			{/if}
		</div>
	{/if}
</div>

<style>
	@property --sky-top {
		syntax: '<color>';
		inherits: true;
		initial-value: #b8d2ea;
	}
	@property --sky-bottom {
		syntax: '<color>';
		inherits: true;
		initial-value: #eef0ea;
	}

	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
		contain: strict;
		background: var(--sky-bottom);
		/* The hour changes the sky over a minute: a slow fade, never a cut. */
		transition:
			--sky-top 60s linear,
			--sky-bottom 60s linear;
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
		background: linear-gradient(to bottom, var(--sky-top) 0%, var(--sky-bottom) 70%);
	}
	.layer {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}

	.stars {
		opacity: 0;
		transition: opacity 30s linear;
	}
	.stars--on {
		opacity: 1;
	}
	.twinkle {
		animation: twinkle 7s ease-in-out infinite;
	}
	@keyframes twinkle {
		50% {
			opacity: 0.25;
		}
	}

	.celestial {
		position: absolute;
		left: 0;
		top: 0;
		width: 320px;
		height: 320px;
		transition: opacity 4s ease;
	}
	.celestial--hidden {
		opacity: 0;
	}
	.sun {
		position: absolute;
		inset: 0;
		border-radius: 50%;
		background: radial-gradient(circle, var(--sun) 0 22%, var(--sun-halo) 23%, transparent 62%);
	}
	.moon {
		width: 100%;
		height: 100%;
	}

	.cloud {
		position: absolute;
		left: 0;
		animation: drift linear infinite;
		will-change: transform;
	}
	.cloud svg {
		display: block;
	}
	@keyframes drift {
		from {
			transform: translateX(-460px);
		}
		to {
			transform: translateX(2000px);
		}
	}

	.glow {
		position: absolute;
		border-radius: 50%;
		background: radial-gradient(circle, var(--glow) 0%, transparent 70%);
		animation: breathe 7s ease-in-out infinite;
	}
	.flood {
		width: 300px;
		height: 560px;
	}
	.lamp {
		width: 240px;
		height: 240px;
	}
	@keyframes breathe {
		50% {
			opacity: 0.55;
		}
	}
	.lit {
		animation: lamp-on 4s ease both;
	}
	@keyframes lamp-on {
		from {
			opacity: 0;
		}
	}

	/* The jet: the column sways a hair, the water climbs, the plume swells, the spray falls. */
	.jet {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: sway 14s ease-in-out infinite;
	}
	@keyframes sway {
		50% {
			transform: rotate(0.7deg) scaleY(1.015);
		}
	}
	.flow {
		animation: climb 1.6s linear infinite;
	}
	@keyframes climb {
		to {
			stroke-dashoffset: -48;
		}
	}
	.plume {
		transform-box: fill-box;
		transform-origin: 50% 0;
		animation: swell 5s ease-in-out infinite;
	}
	@keyframes swell {
		50% {
			transform: scale(1.08, 1.12);
		}
	}
	.drop {
		opacity: 0;
		animation: spray 3.4s ease-in infinite;
	}
	@keyframes spray {
		0% {
			transform: translate(0, 0);
			opacity: 0.9;
		}
		100% {
			transform: translate(var(--dx), var(--dy));
			opacity: 0;
		}
	}
	.mist {
		animation: breathe 6s ease-in-out infinite;
	}
	.ripple {
		animation: shimmer 6s ease-in-out infinite;
	}
	@keyframes shimmer {
		50% {
			opacity: 0.1;
			transform: translateX(14px);
		}
	}

	.mover {
		position: absolute;
		left: 0;
		top: 0;
		will-change: transform;
	}
	.sail {
		top: 546px;
		animation: cruise 240s linear infinite;
		animation-delay: -90s;
	}
	@keyframes cruise {
		from {
			transform: translateX(-200px);
		}
		to {
			transform: translateX(2000px);
		}
	}
	.swan {
		position: absolute;
		animation: bob 5s ease-in-out infinite;
	}
	@keyframes bob {
		50% {
			transform: translateY(5px) rotate(-1.5deg);
		}
	}

	.cast {
		position: absolute;
	}
	.cast :global(.gaston) {
		display: block;
		width: 150px;
	}

	.rain {
		position: absolute;
		left: 0;
		top: -540px;
		width: 1920px;
		height: 1620px;
		animation: rain 9s linear infinite;
	}
	.rain svg {
		width: 100%;
		height: 100%;
		display: block;
	}
	@keyframes rain {
		to {
			transform: translateY(540px);
		}
	}
	.veil {
		position: absolute;
		inset: 0;
		background: var(--veil);
		animation: veil-in 20s ease both;
	}
	@keyframes veil-in {
		from {
			opacity: 0;
		}
	}

	/* A hidden tab: everything holds its breath. */
	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	/* Reduced motion: a still picture in the right pose. */
	.scene--still :global(*) {
		animation: none !important;
		transition: none !important;
	}
	.scene--still .cloud {
		transform: translateX(var(--still-x));
	}
	.scene--still .sail {
		transform: translateX(1000px);
	}
	.scene--still .drop {
		opacity: 0.8;
		transform: translate(calc(var(--dx) * 0.5), calc(var(--dy) * 0.4));
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
