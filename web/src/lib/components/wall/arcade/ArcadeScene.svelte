<script lang="ts">
	/**
	 * The wall's living backdrop, arcade edition: a fleet of pixel invaders
	 * (stand-ins for the devices) marching over a drifting starfield, a bunker
	 * line, a cannon patrolling the ground and firing, a mystery ship now and
	 * then, CRT scanlines over it all. A drop-in for the Paris scene (same
	 * props), drawn on a 1920 x 1080 stage scaled to cover its box.
	 *
	 * The sky follows the real hour and the theme (a pixel sun by day, moon by
	 * night; pure black on OLED). The network's state is woven in: clouded =
	 * the fleet sits lower and marches faster; storm = red alert (a flashing
	 * border and a red wash); each problem on the board turns one invader red.
	 * `startle` (a new problem) and `pulse` replay a gesture on the hero crab:
	 * down = it turns red and drops, up = the cannon zaps it and it respawns,
	 * resolved = "HIGH SCORE" and confetti.
	 *
	 * All motion is stepped transform/opacity. Hidden tabs pause it; reduced
	 * motion freezes a still picture (the alert border stays on, solid).
	 */
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import {
		phaseAt,
		celestialAt,
		type SceneMood,
		type SceneTheme,
		type ScenePhase
	} from '#lib/components/wall/paris/daylight.js';
	import {
		BUNKER,
		BURST,
		CANNON,
		CRAB_A,
		CRAB_B,
		OCTO_A,
		OCTO_B,
		ORB,
		SQUID_A,
		SQUID_B,
		level as levelOf,
		rnd,
		spritePath,
		textPath,
		textWidth,
		type Mood,
		type Pulse,
		type Sprite
	} from './sprites.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood: SceneMood | Mood;
		/** Problems on the board: that many invaders turn red. */
		problems: number;
		/** Bumped by the wall when a new problem appears: the hero crab goes down once. */
		startle?: number;
		/** The tab is hidden: nothing moves. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		pulse?: Pulse | null;
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
	const level = $derived(levelOf(mood));

	// --- Stage fit -----------------------------------------------------------------

	const STAGE_W = 1920;
	const STAGE_H = 1080;
	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Sky -------------------------------------------------------------------------

	const SKIES: Record<SceneTheme, Record<ScenePhase, [string, string]>> = {
		light: {
			day: ['#cfd6ff', '#efeaff'],
			golden: ['#ffd9c9', '#fff0e0'],
			dawn: ['#cdbfff', '#e8defa'],
			dusk: ['#cdbfff', '#e8defa'],
			night: ['#9aa0e0', '#c9c4f0']
		},
		dark: {
			day: ['#1b1470', '#3a1f8f'],
			golden: ['#2a1478', '#5a2a8a'],
			dawn: ['#140a45', '#3a1a6a'],
			dusk: ['#140a45', '#3a1a6a'],
			night: ['#05030f', '#120a3a']
		},
		oled: {
			day: ['#000', '#000'],
			golden: ['#000', '#000'],
			dawn: ['#000', '#000'],
			dusk: ['#000', '#000'],
			night: ['#000', '#000']
		}
	};
	const P = 8;
	const GROUND = 972;
	const HERO = { x: 1680, y: 520 };
	const DROP = GROUND - (HERO.y + 8 * P);
	const GC_X = 1000; // the gesture cannon's rest column
	const SLIDE = HERO.x + (11 * P) / 2 - (GC_X + (13 * P) / 2);

	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const body = $derived(celestialAt(minute));
	const sky = $derived(SKIES[theme][phase]);
	const style = $derived(
		`--sky1:${sky[0]};--sky2:${sky[1]};--drop:${DROP}px;--slide:${SLIDE}px`
	);
	const bodyColor = $derived(body.body === 'sun' ? 'var(--sun)' : 'var(--moon)');

	// --- Sprites ---------------------------------------------------------------------

	const COLS = 8;
	const PITCH = 16; // pixel units between two invaders of a row
	const FLEET_X = 420;
	const ROWS: { y: number; a: Sprite; b: Sprite; tone: number }[] = [
		{ y: 200, a: SQUID_A, b: SQUID_B, tone: 1 },
		{ y: 288, a: CRAB_A, b: CRAB_B, tone: 2 },
		{ y: 376, a: CRAB_A, b: CRAB_B, tone: 2 },
		{ y: 464, a: OCTO_A, b: OCTO_B, tone: 3 },
		{ y: 552, a: OCTO_A, b: OCTO_B, tone: 3 }
	];
	const cell = (s: Sprite, c: number) => c * PITCH + Math.floor((PITCH - s[0].length) / 2);
	function rowPath(sprite: Sprite): string {
		let d = '';
		for (let c = 0; c < COLS; c++) d += spritePath(sprite, cell(sprite, c));
		return d;
	}
	const fleet = ROWS.map((r) => ({ y: r.y, tone: r.tone, a: rowPath(r.a), b: rowPath(r.b) }));

	// One red invader per problem, spread over the grid.
	const TROUBLE: [row: number, col: number][] = [
		[1, 3],
		[3, 6],
		[0, 1],
		[4, 2],
		[2, 5],
		[3, 0],
		[1, 7],
		[4, 5]
	];
	const reds = $derived(
		TROUBLE.slice(0, Math.min(problems, TROUBLE.length)).map(([r, c]) => ({
			x: cell(ROWS[r].a, c) * P,
			y: ROWS[r].y,
			a: spritePath(ROWS[r].a),
			b: spritePath(ROWS[r].b)
		}))
	);

	const crabA = spritePath(CRAB_A);
	const crabB = spritePath(CRAB_B);
	const burst = spritePath(BURST);
	const cannon = spritePath(CANNON);
	const bunker = spritePath(BUNKER);
	const orb = spritePath(ORB);
	const ufo = spritePath([
		'.....XXXXXX.....',
		'...XXXXXXXXXX...',
		'..XXXXXXXXXXXX..',
		'.XX.XX.XX.XX.XX.',
		'XXXXXXXXXXXXXXXX',
		'..XXX..XX..XXX..',
		'...X........X...'
	]);

	const BUNKERS = [360, 640, 920, 1200];

	const STARS = Array.from({ length: 70 }, (_, i) => ({
		x: Math.round(rnd(i, 1) * 1900),
		y: Math.round(rnd(i, 2) * 1060),
		big: rnd(i, 3) > 0.85,
		delay: -rnd(i, 4) * 4
	}));

	const TEXT = 'HIGH SCORE';
	const TEXT_PX = 16;
	const textD = textPath(TEXT);
	const textX = 960 - (textWidth(TEXT) * TEXT_PX) / 2;
	const TONES = ['var(--c1)', 'var(--c2)', 'var(--c3)', 'var(--hot)', 'var(--spark)'];
	const CONFETTI = Array.from({ length: 40 }, (_, i) => ({
		x: Math.round(rnd(i, 5) * 1880),
		sway: Math.round((rnd(i, 6) - 0.5) * 140),
		delay: Math.round(rnd(i, 7) * 1100),
		tone: i % 5
	}));

	// --- Gestures --------------------------------------------------------------------

	let gesture = $state<{ kind: Pulse['kind']; key: number } | null>(null);
	let seq = 0;

	$effect(() => {
		const p = pulse;
		if (p) untrack(() => (gesture = { kind: p.kind, key: ++seq }));
	});

	let lastStartle = untrack(() => startle);
	$effect(() => {
		const s = startle;
		untrack(() => {
			if (s !== lastStartle) gesture = { kind: 'down', key: ++seq };
			lastStartle = s;
		});
	});
</script>

<div
	class="scene scene-arcade {className} th-{theme} lv-{level}"
	class:scene--paused={paused}
	class:scene--still={!motion}
	{style}
	style:--bodyc={bodyColor}
	bind:clientWidth={boxW}
	bind:clientHeight={boxH}
	aria-hidden="true"
>
	{#if scale > 0}
		<div class="stage" style:transform="translate({offsetX}px, {offsetY}px) scale({scale})">
			<svg viewBox="0 0 {STAGE_W} {STAGE_H}" fill="none" shape-rendering="crispEdges">
				<g class="drift" fill="var(--star)">
					{#each [0, -STAGE_H] as shift (shift)}
						<g transform="translate(0 {shift})">
							{#each STARS as star, i (i)}
								<rect
									class="star"
									x={star.x}
									y={star.y}
									width={star.big ? 8 : 4}
									height={star.big ? 8 : 4}
									style:animation-delay="{star.delay}s"
								/>
							{/each}
						</g>
					{/each}
				</g>

				<path
					class="orb"
					d={orb}
					fill="var(--bodyc)"
					transform="translate({body.x - 55} {body.y - 55}) scale(10)"
				/>

				<g class="ufo" fill="var(--hot)">
					<path d={ufo} transform="translate(0 100) scale({P})" />
				</g>

				<g class="fleet">
					<g class="march">
						{#each fleet as row (row.y)}
							<g transform="translate({FLEET_X} {row.y})" style:fill="var(--c{row.tone})">
								<path class="fa" d={row.a} transform="scale({P})" />
								<path class="fb" d={row.b} transform="scale({P})" />
							</g>
						{/each}
						{#each reds as red, i (i)}
							<g class="red" transform="translate({FLEET_X + red.x} {red.y})" fill="var(--hot)">
								<path class="fa" d={red.a} transform="scale({P})" />
								<path class="fb" d={red.b} transform="scale({P})" />
							</g>
						{/each}
					</g>
				</g>

				<g fill="var(--bunker)">
					{#each BUNKERS as x (x)}
						<path d={bunker} transform="translate({x} 780) scale({P})" />
					{/each}
				</g>
				<rect x="0" y={GROUND} width={STAGE_W} height="8" fill="var(--ground)" />

				<g class="patrol" fill="var(--cannon)">
					<path d={cannon} transform="translate(0 900) scale({P})" />
					<rect class="shot" x="48" y="870" width="8" height="28" fill="var(--spark)" />
				</g>

				{#key gesture?.key}
					<g class="gesture k-{gesture?.kind ?? 'none'}">
						<g transform="translate({HERO.x} {HERO.y})">
							<g class="hero-move">
								<g class="hero-ok" style:fill="var(--c2)">
									<path class="fa" d={crabA} transform="scale({P})" />
									<path class="fb" d={crabB} transform="scale({P})" />
								</g>
								<path class="hero-red" d={crabA} transform="scale({P})" fill="var(--hot)" />
							</g>
						</g>
						<path
							class="burst burst-hero"
							d={burst}
							transform="translate({HERO.x} {HERO.y}) scale({P})"
							fill="var(--spark)"
						/>
						<path
							class="burst burst-ground"
							d={burst}
							transform="translate({HERO.x} {GROUND - 8 * P}) scale({P})"
							fill="var(--spark)"
						/>
						<g class="gcannon" fill="var(--cannon)">
							<path d={cannon} transform="translate({GC_X} 900) scale({P})" />
						</g>
						<rect
							class="laser"
							x={HERO.x + (11 * P) / 2 - 4}
							y="870"
							width="8"
							height="30"
							fill="var(--spark)"
						/>
						<g class="score">
							<path d={textD} transform="translate({textX + 8} 68) scale({TEXT_PX})" fill="var(--shadow)" />
							<path d={textD} transform="translate({textX} 60) scale({TEXT_PX})" fill="var(--spark)" />
						</g>
						{#each CONFETTI as piece, i (i)}
							<rect
								class="confetti"
								x={piece.x}
								y="-20"
								width="12"
								height="12"
								style:--sway="{piece.sway}px"
								style:animation-delay="{piece.delay}ms"
								fill={TONES[piece.tone]}
							/>
						{/each}
					</g>
				{/key}
			</svg>
		</div>
	{/if}
	<div class="wash"></div>
	<div class="scan"></div>
	<div class="alert"></div>
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
		contain: strict;
		background: linear-gradient(180deg, var(--sky1), var(--sky2));
		--star: #7a6fd0;
		--sun: #d99a00;
		--moon: #6f6fb8;
		--c1: #b02a9a;
		--c2: #0b7e96;
		--c3: #2a8a2a;
		--cannon: #1f7a33;
		--bunker: #3f9a52;
		--ground: #1f7a33;
		--hot: #d81f32;
		--spark: #b8860b;
		--shadow: #e7b8ff;
		--scan: rgba(60, 40, 140, 0.09);
		--beat: 1s;
		--march: 8s;
		--lower: 0px;
	}
	.scene:global(.th-dark),
	.scene:global(.th-oled) {
		--star: #fff;
		--sun: #ffd84a;
		--moon: #e8eaff;
		--c1: #ff7ae1;
		--c2: #5ff3ff;
		--c3: #8dff6b;
		--cannon: #8dff6b;
		--bunker: #2fae52;
		--ground: #8dff6b;
		--hot: #ff3b4d;
		--spark: #fff6a8;
		--shadow: #ff3bb0;
		--scan: rgba(0, 0, 0, 0.24);
	}
	.scene:global(.lv-warn) {
		--beat: 0.6s;
		--march: 4s;
		--lower: 48px;
	}
	.scene:global(.lv-critical) {
		--beat: 0.32s;
		--march: 2.2s;
		--lower: 96px;
	}
	.stage {
		position: absolute;
		left: 0;
		top: 0;
		width: 1920px;
		height: 1080px;
		transform-origin: 0 0;
	}
	svg {
		display: block;
		width: 100%;
		height: 100%;
	}
	.scan {
		position: absolute;
		inset: 0;
		background: repeating-linear-gradient(180deg, var(--scan) 0 1px, transparent 1px 3px);
	}
	.wash {
		position: absolute;
		inset: 0;
		opacity: 0;
		background: radial-gradient(ellipse at center, transparent 40%, var(--hot) 140%);
	}
	.alert {
		position: absolute;
		inset: 0;
		opacity: 0;
		border: 6px solid var(--hot);
		box-shadow: inset 0 0 60px color-mix(in srgb, var(--hot) 60%, transparent);
	}
	:global(.lv-critical) .alert {
		opacity: 1;
		animation: blink 1s step-end infinite;
	}
	:global(.lv-critical) .wash {
		opacity: 0.35;
	}
	.orb {
		opacity: 0.55;
	}
	:global(.lv-critical) .orb {
		opacity: 0.2;
	}

	.drift {
		animation: drift 160s steps(135) infinite;
	}
	.star {
		animation: twinkle 3s step-end infinite;
	}
	.fleet {
		translate: 0 var(--lower);
		transition: translate 1.5s steps(8);
	}
	.march {
		animation: march var(--march) infinite;
	}
	.fa,
	.fb {
		animation: flip var(--beat) step-end infinite;
	}
	.fb {
		animation-name: flop;
	}
	.red {
		animation: blink 0.8s step-end infinite;
	}
	.ufo {
		animation: ufo 34s steps(120) 6s infinite;
		translate: -200px 0;
	}
	.patrol {
		animation: patrol 18s steps(26) infinite alternate;
		translate: 200px 0;
	}
	.shot {
		opacity: 0;
		animation: shoot 3.6s steps(14) infinite;
	}

	.hero-red,
	.burst,
	.laser,
	.score,
	.confetti,
	.gcannon {
		opacity: 0;
	}
	:global(.k-down) .hero-move {
		animation: d-move 2.8s linear;
	}
	:global(.k-down) .hero-ok {
		animation: d-ok 2.8s step-end;
	}
	:global(.k-down) .hero-red {
		animation: d-red 2.8s step-end;
	}
	:global(.k-down) .burst-ground {
		animation: d-burst 2.8s step-end;
	}
	:global(.k-up) .hero-ok {
		animation: u-ok 3.2s step-end;
	}
	:global(.k-up) .hero-red {
		animation: u-red 3.2s step-end;
	}
	:global(.k-up) .gcannon {
		animation: u-cannon 3.2s linear;
	}
	:global(.k-up) .laser {
		animation: u-laser 3.2s linear;
	}
	:global(.k-up) .burst-hero {
		animation: u-burst 3.2s step-end;
	}
	:global(.k-resolved) .score {
		animation: score 4.4s step-end;
	}
	:global(.k-resolved) .confetti {
		animation: fall 2.8s linear both;
	}

	@keyframes blink {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.15;
		}
	}
	@keyframes flip {
		0% {
			opacity: 1;
		}
		50% {
			opacity: 0;
		}
	}
	@keyframes flop {
		0% {
			opacity: 0;
		}
		50% {
			opacity: 1;
		}
	}
	@keyframes twinkle {
		0% {
			opacity: 0.9;
		}
		60% {
			opacity: 0.25;
		}
	}
	@keyframes drift {
		to {
			translate: 0 1080px;
		}
	}
	@keyframes march {
		0% {
			translate: 0 0;
			animation-timing-function: steps(12);
		}
		45% {
			translate: 96px 0;
		}
		50% {
			translate: 96px 0;
			animation-timing-function: steps(12);
		}
		95%,
		100% {
			translate: 0 0;
		}
	}
	@keyframes ufo {
		0% {
			translate: -200px 0;
		}
		100% {
			translate: 2100px 0;
		}
	}
	@keyframes patrol {
		to {
			translate: 1500px 0;
		}
	}
	@keyframes shoot {
		0%,
		55% {
			opacity: 0;
			translate: 0 0;
		}
		56% {
			opacity: 1;
			translate: 0 0;
		}
		100% {
			opacity: 1;
			translate: 0 -520px;
		}
	}

	@keyframes d-move {
		0%,
		30% {
			translate: 0 0;
			animation-timing-function: steps(12);
		}
		66% {
			translate: 0 var(--drop);
		}
		100% {
			translate: 0 0;
		}
	}
	@keyframes d-ok {
		0% {
			opacity: 1;
		}
		1%,
		91% {
			opacity: 0;
		}
		92%,
		96% {
			opacity: 1;
		}
		97% {
			opacity: 0;
		}
		98%,
		100% {
			opacity: 1;
		}
	}
	@keyframes d-red {
		0%,
		3%,
		14%,
		25% {
			opacity: 0;
		}
		1%,
		8%,
		19%,
		30%,
		31% {
			opacity: 1;
		}
		66% {
			opacity: 1;
		}
		67%,
		100% {
			opacity: 0;
		}
	}
	@keyframes d-burst {
		0%,
		66% {
			opacity: 0;
		}
		67%,
		72% {
			opacity: 1;
		}
		73%,
		75% {
			opacity: 0;
		}
		76%,
		82% {
			opacity: 1;
		}
		83%,
		100% {
			opacity: 0;
		}
	}
	@keyframes u-red {
		0%,
		46% {
			opacity: 1;
		}
		47%,
		100% {
			opacity: 0;
		}
	}
	@keyframes u-ok {
		0%,
		60% {
			opacity: 0;
		}
		61%,
		65% {
			opacity: 1;
		}
		66%,
		69% {
			opacity: 0;
		}
		70%,
		100% {
			opacity: 1;
		}
	}
	@keyframes u-cannon {
		0% {
			opacity: 1;
			translate: 0 0;
			animation-timing-function: steps(14);
		}
		33% {
			opacity: 1;
			translate: var(--slide) 0;
		}
		72% {
			opacity: 1;
			translate: var(--slide) 0;
			animation-timing-function: steps(14);
		}
		99% {
			opacity: 1;
			translate: 0 0;
		}
		100% {
			opacity: 0;
			translate: 0 0;
		}
	}
	@keyframes u-laser {
		0%,
		33% {
			opacity: 0;
			translate: 0 0;
		}
		34% {
			opacity: 1;
			translate: 0 0;
			animation-timing-function: steps(8);
		}
		46% {
			opacity: 1;
			translate: 0 -310px;
		}
		47%,
		100% {
			opacity: 0;
			translate: 0 -310px;
		}
	}
	@keyframes u-burst {
		0%,
		46% {
			opacity: 0;
		}
		47%,
		52% {
			opacity: 1;
		}
		53%,
		55% {
			opacity: 0;
		}
		56%,
		60% {
			opacity: 1;
		}
		61%,
		100% {
			opacity: 0;
		}
	}
	@keyframes score {
		0% {
			opacity: 0;
		}
		5% {
			opacity: 1;
		}
		11% {
			opacity: 0;
		}
		17% {
			opacity: 1;
		}
		23% {
			opacity: 0;
		}
		29%,
		84% {
			opacity: 1;
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes fall {
		0% {
			opacity: 1;
			translate: 0 0;
			animation-timing-function: steps(18);
		}
		100% {
			opacity: 1;
			translate: var(--sway) 1120px;
		}
	}

	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	/* Reduced motion: a still picture; the red alert stays on, solid. */
	.scene--still :global(*) {
		animation: none !important;
		transition: none !important;
	}
	.scene--still .fb {
		opacity: 0;
	}
	.scene--still .patrol {
		translate: 700px 0;
	}
	.scene--still .ufo {
		translate: -400px 0;
	}
	.scene--still:global(.lv-critical) .alert {
		opacity: 1;
	}
</style>
