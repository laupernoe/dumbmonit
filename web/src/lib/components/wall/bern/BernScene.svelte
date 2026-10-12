<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop: Bern. The Zytglogge clock tower with its
	 * blue-and-gold dial showing the real time, sandstone arcades under red
	 * tiles, the Aare looping through the old town, the Bundeshaus dome and the
	 * Alps behind, a Swiss flag in the breeze. A drop-in for the Paris scene
	 * (same props), with one DumbMonit pigeon on the parapet.
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky
	 * follows the real hour inside the brightness band of the wall's theme (see
	 * `paris/daylight.ts`); windows, the clock face and the dome light up after
	 * dark. Clouds gather with advisories, a storm brings rain, and with
	 * problems the pigeon stops strolling and holds up a sign with their number.
	 * A new problem startles it once.
	 *
	 * Motion is gentle and cheap: drifting clouds, the flag, the river's
	 * shimmer, a stroll, and every 40–80 s a fly-by or a hat tip. Hidden tabs
	 * pause everything; reduced motion freezes the scene in a still pose.
	 */
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import WallPigeon from '#lib/components/wall/paris/WallPigeon.svelte';
	import {
		phaseAt,
		isLit,
		celestialAt,
		sceneStyle,
		type SceneMood,
		type SceneTheme
	} from '#lib/components/wall/paris/daylight.js';
	import {
		STAGE_W,
		STAGE_H,
		LEDGE_Y,
		TOWER,
		ROW,
		ALPS,
		BUNDES,
		AARE,
		ZYT,
		STARS,
		RAIN,
		TILE_ARCS,
		litWindows,
		bernStyle
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
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light --------------------------------------------------------------------

	// Re-read once a minute: nothing in the sky moves faster than that.
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const style = $derived(`${sceneStyle(theme, phase, mood)};${bernStyle(theme)}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));

	// The Zytglogge tells the time: hands from the minute, in the browser's zone.
	const minuteAngle = $derived(minute.getMinutes() * 6);
	const hourAngle = $derived((minute.getHours() % 12) * 30 + minute.getMinutes() * 0.5);

	// Windows light up in a different pattern every couple of minutes.
	let windowSeed = $state(1);
	const litSet = $derived(
		lit ? litWindows(windowSeed, ROW.windows.length, phase === 'night' ? 0.18 : 0.3) : new Set<number>()
	);

	const trouble = $derived(problems > 0);
	const raining = $derived(mood === 'storm');
	const cloudCount = $derived(mood === 'storm' ? 6 : mood === 'clouded' ? 4 : 2);
	const CLOUDS = [
		{ y: 120, s: 1.1, dur: 260, delay: -40 },
		{ y: 300, s: 0.8, dur: 320, delay: -210 },
		{ y: 200, s: 0.95, dur: 290, delay: -150 },
		{ y: 420, s: 1.2, dur: 340, delay: -290 },
		{ y: 60, s: 0.7, dur: 300, delay: -250 },
		{ y: 520, s: 1.0, dur: 380, delay: -60 }
	];

	// --- The pigeon -----------------------------------------------------------------

	/** Gaston, in the beret, strolls along the parapet between these points. */
	const STROLL = [800, 1150];
	const SIGN_SPOT = 900;
	let gastonX = $state(1000);
	let gastonWalkMs = $state(0);
	let gastonWalking = $state(false);
	let gastonLook = $state(0);
	let gastonTip = $state(false);
	let startled = $state(false);
	let flyer = $state(false);
	let flyerEl = $state<HTMLDivElement | null>(null);

	// Every timer carries the epoch it was set under; a pause, a change of
	// mood or a teardown bumps it and every stale callback does nothing.
	let epoch = 0;
	const running: Animation[] = [];
	function later(fn: () => void, ms: number) {
		const mine = epoch;
		setTimeout(() => {
			if (mine === epoch) fn();
		}, ms);
	}
	const between = (a: number, b: number) => a + Math.random() * (b - a);

	function stopAll() {
		epoch++;
		for (const animation of running.splice(0)) animation.cancel();
		flyer = false;
		gastonTip = false;
		gastonWalking = false;
	}

	function walkTo(target: number, speed: number, look: number) {
		const distance = Math.abs(target - gastonX);
		gastonLook = look;
		gastonWalkMs = Math.round((distance / speed) * 1000);
		gastonWalking = distance > 4;
		gastonX = target;
		later(() => {
			gastonWalking = false;
			gastonLook = 0;
		}, gastonWalkMs);
	}

	function stroll() {
		if (trouble) return;
		const target = Math.round(between(STROLL[0], STROLL[1]));
		if (Math.abs(target - gastonX) < 60) return void later(stroll, 1500);
		walkTo(target, 34, target > gastonX ? 0.7 : -0.7);
		later(stroll, gastonWalkMs + between(5000, 11000));
	}

	async function flyby() {
		const mine = epoch;
		flyer = true;
		// Let the flyer mount (bind:this) before animating it.
		await new Promise((r) => setTimeout(r, 30));
		if (mine !== epoch) return;
		const y = between(200, 360);
		const animation = flyerEl?.animate(
			[
				{ transform: `translate(-260px, ${y + 40}px) rotate(4deg)` },
				{ transform: `translate(900px, ${y - 20}px) rotate(-2deg)`, offset: 0.5 },
				{ transform: `translate(2160px, ${y - 60}px) rotate(-4deg)` }
			],
			{ duration: 19000, easing: 'linear', fill: 'forwards' }
		);
		if (!animation) return;
		running.push(animation);
		await animation.finished.catch(() => undefined);
		if (mine === epoch) flyer = false;
	}

	function scheduleMoments(first = false) {
		later(
			() => {
				// A hat tip when all is well, otherwise (or half the time) a fly-by.
				if (!trouble && Math.random() < 0.5) {
					gastonLook = 0;
					gastonTip = true;
					later(() => (gastonTip = false), 2400);
				} else void flyby();
				scheduleMoments();
			},
			first ? between(9000, 16000) : between(40000, 80000)
		);
	}

	// (Re)start the cast whenever the trouble flag or the pause changes.
	$effect(() => {
		void trouble;
		const hidden = paused;
		untrack(() => {
			stopAll();
			if (hidden) return;
			if (!motion) {
				gastonX = trouble ? SIGN_SPOT : 1000;
				gastonLook = trouble ? -0.8 : 0;
				return;
			}
			if (trouble) walkTo(SIGN_SPOT, 60, -0.8);
			else later(stroll, 2000);
			scheduleMoments(true);
		});
		return () => untrack(stopAll);
	});

	// Lit windows change now and then while the city is lit.
	$effect(() => {
		if (!lit || paused || !motion) return;
		const timer = setInterval(() => (windowSeed += 1), 120_000);
		return () => clearInterval(timer);
	});

	// A new problem: the pigeon startles once.
	let lastStartle = untrack(() => startle);
	$effect(() => {
		const value = startle;
		if (value === lastStartle) return;
		lastStartle = value;
		untrack(() => {
			startled = true;
			setTimeout(() => (startled = false), 1800);
		});
	});

	const sign = $derived(
		trouble ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null
	);
	const flagX = TOWER.x + 66;
	const TICKS = Array.from({ length: 12 }, (_, i) => i * 30);
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
					<circle cx={star.x} cy={star.y} r={star.r} fill="var(--star)" class:twinkle={star.twinkle} style:animation-delay="-{star.delay}ms" />
				{/each}
			</svg>

			<div class="celestial" class:celestial--hidden={raining} style:transform="translate({body.x - 160}px, {body.y - 160}px)">
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
						<path d="M30 104 C2 104 0 66 30 62 C24 32 60 18 86 34 C98 6 152 0 170 30 C192 14 234 22 238 54 C270 52 292 78 276 104 Z" fill={mood === 'storm' ? 'var(--cloud-storm)' : 'var(--cloud)'} />
					</svg>
				</div>
			{/each}

			{#if lit}
				<div class="glow face-glow" style:left="{TOWER.x - 130}px" style:top="{TOWER.face - 130}px"></div>
			{/if}

			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<defs>
					<clipPath id="bernw-gap"><path d={AARE.bank} /></clipPath>
				</defs>

				<!-- Far: the Alps and the Bundeshaus on its hill. -->
				<path d={ALPS.ridge} fill="var(--alps)" />
				<path d={ALPS.snow} fill="var(--snow)" />
				<path d={BUNDES.body} fill="var(--far)" />
				<path d={BUNDES.drum} fill="var(--far-2)" />
				<path d={BUNDES.dome} fill="var(--copper)" />
				<path d={BUNDES.lantern} fill="var(--copper)" />
				{#if lit}<path class="lit" d={BUNDES.drum} fill="var(--window-lit)" opacity="0.5" />{/if}

				<!-- The Aare loops through the gap in the old town. -->
				<path d={AARE.bank} fill="var(--grass)" />
				<g clip-path="url(#bernw-gap)">
					<path d={AARE.river} stroke="var(--aare)" stroke-width="64" stroke-linecap="round" fill="none" />
					<path class="shimmer" d={AARE.river} stroke="var(--aare-hi)" stroke-width="3" stroke-linecap="round" stroke-dasharray="14 46" fill="none" opacity="0.5" />
					<g fill="var(--tree)"><circle cx="730" cy="800" r="24" /><circle cx="1000" cy="790" r="28" /><circle cx="1020" cy="900" r="20" /></g>
				</g>

				<!-- Arcaded old town under red tiles. -->
				<path d={ROW.roofs} fill="var(--tile)" />
				<path d={ROW.facadeA} fill="var(--sand)" />
				<path d={ROW.facadeB} fill="var(--sand-2)" />
				<path d={ROW.arches} fill="var(--arch)" />
				<g fill="var(--window)">
					{#each ROW.windows as w, i (i)}
						<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="3" />
					{/each}
				</g>
				<g fill="var(--window-lit)">
					{#each ROW.windows as w, i (i)}
						{#if litSet.has(i)}<rect class="lit" x={w.x} y={w.y} width={w.w} height={w.h} rx="3" />{/if}
					{/each}
				</g>

				<!-- The Zytglogge. -->
				<path d={ZYT.shaft} fill="var(--stone)" />
				<path d={ZYT.shade} fill="var(--stone-shade)" opacity="0.7" />
				<path d={ZYT.eaves} fill="var(--tile-dark)" />
				<path d={ZYT.gate} fill="var(--arch)" />
				<path d={ZYT.upper} fill="var(--stone)" />
				<path d={ZYT.lantern} fill="var(--stone-shade)" />
				<path d={ZYT.openings} fill={lit ? 'var(--window-lit)' : 'var(--window)'} />
				<path d={ZYT.turrets} fill="var(--tile)" />
				<path d={ZYT.roof} fill="var(--tile)" />
				<path d={ZYT.spire} fill="var(--gold)" />
				<circle cx={TOWER.x} cy="44" r="6" fill="var(--gold)" />

				<!-- Big dial: gold ring, blue face, twelve ticks, hands from the real time. -->
				<circle cx={TOWER.x} cy={TOWER.face} r="58" fill="var(--gold)" />
				<circle cx={TOWER.x} cy={TOWER.face} r="50" fill="var(--dial)" />
				<g stroke="var(--gold)" stroke-width="3" stroke-linecap="round">
					{#each TICKS as a (a)}
						<path d="M{TOWER.x} {TOWER.face - 46}v7" transform="rotate({a} {TOWER.x} {TOWER.face})" />
					{/each}
				</g>
				<g stroke="var(--hand)" stroke-linecap="round">
					<path d="M{TOWER.x} {TOWER.face}V{TOWER.face - 24}" stroke-width="6" transform="rotate({hourAngle} {TOWER.x} {TOWER.face})" />
					<path d="M{TOWER.x} {TOWER.face}V{TOWER.face - 38}" stroke-width="4" transform="rotate({minuteAngle} {TOWER.x} {TOWER.face})" />
				</g>
				<circle cx={TOWER.x} cy={TOWER.face} r="5" fill="var(--gold)" />
				{#if lit}<circle class="lit" cx={TOWER.x} cy={TOWER.face} r="50" fill="var(--window-lit)" opacity="0.28" />{/if}

				<!-- Astronomical dial below: rings, cross, a sun pointer that follows the hours. -->
				<circle cx={TOWER.x} cy={TOWER.dial} r="38" fill="var(--gold)" />
				<circle cx={TOWER.x} cy={TOWER.dial} r="32" fill="var(--dial)" />
				<circle cx={TOWER.x} cy={TOWER.dial} r="20" fill="none" stroke="var(--gold)" stroke-width="2" />
				<path d="M{TOWER.x} {TOWER.dial - 32}V{TOWER.dial + 32}M{TOWER.x - 32} {TOWER.dial}H{TOWER.x + 32}" stroke="var(--gold)" stroke-width="1.5" />
				<circle cx={TOWER.x + 20} cy={TOWER.dial} r="5" fill="var(--gold)" transform="rotate({hourAngle * 2 - 90} {TOWER.x} {TOWER.dial})" />

				<!-- The Swiss flag on the turret pole. -->
				<rect x={flagX - 1} y="100" width="2.5" height="52" fill="var(--arch)" />
				<g transform="translate({flagX + 1} 100)">
					<g class="flag">
						<rect width="40" height="40" fill="var(--flag)" />
						<path d="M17 8h6v9h9v6h-9v9h-6v-9H8v-6h9z" fill="var(--flag-cross)" />
					</g>
				</g>
			</svg>

			{#if flyer}
				<div class="mover" bind:this={flyerEl}>
					<WallPigeon flap lookX={0.8} class="flyer" blinkOffset={2} />
				</div>
			{/if}

			<!-- Our terrace: the parapet and the tiled roof below it. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<rect x="0" y={LEDGE_Y - 10} width={STAGE_W} height="16" fill="var(--tile-dark)" />
				<rect x="0" y={LEDGE_Y + 6} width={STAGE_W} height={STAGE_H - LEDGE_Y} fill="var(--tile)" />
				<path d={TILE_ARCS} stroke="var(--tile-dark)" stroke-width="3" fill="none" stroke-linecap="round" />
			</svg>

			<!-- Gaston, in the beret: strolls, tips his hat, holds the sign. -->
			<div
				class="cast stroller"
				style:transform="translateX({gastonX - 75}px)"
				style:transition-duration="{gastonWalkMs}ms"
				style:top="{LEDGE_Y + 6 - 187}px"
			>
				<div class="waddle" class:waddle--on={gastonWalking}>
					<WallPigeon
						class="gaston"
						hat
						tip={gastonTip}
						{sign}
						lookX={gastonLook}
						lookY={trouble ? -0.3 : 0}
						eyes={startled ? 'startled' : 'open'}
						blinkOffset={0.7}
					/>
				</div>
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
		background: linear-gradient(to bottom, var(--sky-top) 0%, var(--sky-bottom) 78%);
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
		transition:
			transform 60s linear,
			opacity 20s ease;
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
	}
	.face-glow {
		width: 260px;
		height: 260px;
		animation: breathe 9s ease-in-out infinite;
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

	.flag {
		transform-box: fill-box;
		transform-origin: 0 50%;
		animation: wave 2.6s ease-in-out infinite alternate;
	}
	@keyframes wave {
		from {
			transform: skewY(-4deg) scaleX(0.94);
		}
		to {
			transform: skewY(5deg) scaleX(1);
		}
	}
	.shimmer {
		animation: flow 6s linear infinite;
	}
	@keyframes flow {
		to {
			stroke-dashoffset: -120;
		}
	}

	.mover {
		position: absolute;
		left: 0;
		top: 0;
		will-change: transform;
	}
	.mover :global(.flyer) {
		display: block;
		width: 120px;
	}

	.cast {
		position: absolute;
		left: 0;
	}
	.cast :global(.gaston) {
		display: block;
		width: 150px;
	}
	.stroller {
		transition-property: transform;
		transition-timing-function: linear;
	}
	.waddle {
		transform-origin: 50% 100%;
	}
	.waddle--on {
		animation: waddle 1.1s ease-in-out infinite;
	}
	@keyframes waddle {
		0%,
		100% {
			transform: rotate(-4deg) translateY(0);
		}
		25%,
		75% {
			transform: rotate(0deg) translateY(-5px);
		}
		50% {
			transform: rotate(4deg) translateY(0);
		}
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
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
