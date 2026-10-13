<script lang="ts">
	/**
	 * Retro pixel arcade: rows of little invaders marching over a starfield, a
	 * bunker line and a player cannon, under CRT scanlines. Every sprite is a
	 * run of SVG rects (one merged path), drawn with crisp edges.
	 *
	 * `mood` is the weather: ok = a slow march, warn = the fleet is lower and
	 * marches faster, critical = a red alert border flashes. `pulse` replays a
	 * gesture on the hero invader (the lone crab on the right): down = it turns
	 * red and drops, up = the cannon slides under it and zaps, it respawns,
	 * resolved = "HIGH SCORE" in pixel text and confetti.
	 */
	import {
		BUNKER,
		BURST,
		CANNON,
		CRAB_A,
		CRAB_B,
		OCTO_A,
		OCTO_B,
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
	} from '#lib/components/wall/arcade/sprites.js';

	interface Props {
		mood?: Mood;
		pulse?: Pulse | null;
	}

	let { mood = 'ok', pulse = null }: Props = $props();
	const level = $derived(levelOf(mood));

	const P = 6;
	const COLS = 6;
	const PITCH = 16; // pixel units between two invaders of a row
	const FLEET_X = 420;
	const ROWS: { y: number; a: Sprite; b: Sprite; tone: number }[] = [
		{ y: 100, a: SQUID_A, b: SQUID_B, tone: 1 },
		{ y: 165, a: CRAB_A, b: CRAB_B, tone: 2 },
		{ y: 230, a: OCTO_A, b: OCTO_B, tone: 3 }
	];

	function rowPath(sprite: Sprite): string {
		const w = sprite[0].length;
		let d = '';
		for (let c = 0; c < COLS; c++) d += spritePath(sprite, c * PITCH + Math.floor((PITCH - w) / 2));
		return d;
	}
	const fleet = ROWS.map((r) => ({ y: r.y, tone: r.tone, a: rowPath(r.a), b: rowPath(r.b) }));

	const crabA = spritePath(CRAB_A);
	const crabB = spritePath(CRAB_B);
	const burst = spritePath(BURST);
	const cannon = spritePath(CANNON);
	const bunker = spritePath(BUNKER);

	// The hero crab, its drop to the ground, the cannon's slide under it.
	const HERO = { x: 1100, y: 230 };
	const GROUND = 500;
	const DROP = GROUND - (HERO.y + 8 * P);
	const CANNON_X = 698;
	const SLIDE = HERO.x + (11 * P) / 2 - (CANNON_X + (13 * P) / 2);
	const BUNKERS = [480, 660, 840, 1020];

	const STARS = Array.from({ length: 44 }, (_, i) => ({
		x: Math.round(rnd(i, 1) * 1200),
		y: Math.round(rnd(i, 2) * 430),
		big: rnd(i, 3) > 0.8,
		delay: -rnd(i, 4) * 4
	}));

	const TEXT = 'HIGH SCORE';
	const TEXT_PX = 8;
	const textD = textPath(TEXT);
	const textX = 840 - (textWidth(TEXT) * TEXT_PX) / 2;
	const CONFETTI = Array.from({ length: 30 }, (_, i) => ({
		x: Math.round(300 + rnd(i, 5) * 900),
		sway: Math.round((rnd(i, 6) - 0.5) * 90),
		delay: Math.round(rnd(i, 7) * 900),
		tone: i % 5
	}));
</script>

<div
	class="scene scene-arcade lv-{level}"
	style="--drop: {DROP}px; --slide: {SLIDE}px"
	aria-hidden="true"
>
	<svg viewBox="0 0 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" shape-rendering="crispEdges">
		<g fill="var(--star)">
			{#each STARS as star, i (i)}
				<rect
					class="star"
					x={star.x}
					y={star.y}
					width={star.big ? 6 : 3}
					height={star.big ? 6 : 3}
					style:animation-delay="{star.delay}s"
				/>
			{/each}
		</g>

		<!-- the fleet: lowered and faster with the mood -->
		<g class="fleet">
			<g class="march">
				{#each fleet as row (row.y)}
					<g transform="translate({FLEET_X} {row.y})" style:fill="var(--c{row.tone})">
						<path class="fa" d={row.a} transform="scale({P})" />
						<path class="fb" d={row.b} transform="scale({P})" />
					</g>
				{/each}
			</g>
		</g>

		<g fill="var(--bunker)">
			{#each BUNKERS as x (x)}
				<path d={bunker} transform="translate({x} 392) scale({P})" />
			{/each}
		</g>
		<rect x="0" y={GROUND} width="1220" height="6" fill="var(--ground)" />

		{#key pulse?.at}
			<g class="gesture k-{pulse?.kind ?? 'none'}">
				<!-- hero crab: green, red when it is in trouble -->
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

				<g class="cannon" fill="var(--cannon)">
					<path d={cannon} transform="translate({CANNON_X} 452) scale({P})" />
				</g>
				<rect class="laser" x={HERO.x + (11 * P) / 2 - 3} y="428" width="6" height="22" fill="var(--spark)" />

				<g class="score">
					<path d={textD} transform="translate({textX + 5} 39) scale({TEXT_PX})" fill="var(--shadow)" />
					<path d={textD} transform="translate({textX} 34) scale({TEXT_PX})" fill="var(--spark)" />
				</g>
				{#each CONFETTI as piece, i (i)}
					<rect
						class="confetti"
						x={piece.x}
						y="-16"
						width="8"
						height="8"
						style:--sway="{piece.sway}px"
						style:animation-delay="{piece.delay}ms"
						fill={['var(--c1)', 'var(--c2)', 'var(--c3)', 'var(--hot)', 'var(--spark)'][piece.tone]}
					/>
				{/each}
			</g>
		{/key}
	</svg>
	<div class="scan"></div>
	<div class="alert"></div>
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(180deg, var(--sky1), var(--sky2));
	}
	.scene svg {
		display: block;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
	}
	.scene-arcade {
		--sky1: #d9d3ff;
		--sky2: #f1edff;
		--star: #7a6fd0;
		--c1: #b02a9a;
		--c2: #0b7e96;
		--c3: #2a8a2a;
		--cannon: #1f7a33;
		--bunker: #3f9a52;
		--ground: #1f7a33;
		--hot: #d81f32;
		--spark: #b8860b;
		--shadow: #e7b8ff;
		--scan: rgba(60, 40, 140, 0.1);
		--beat: 1s;
		--march: 6s;
		--lower: 0px;
	}
	:global(.dark) .scene-arcade {
		--sky1: #07041a;
		--sky2: #170c46;
		--star: #fff;
		--c1: #ff7ae1;
		--c2: #5ff3ff;
		--c3: #8dff6b;
		--cannon: #8dff6b;
		--bunker: #2fae52;
		--ground: #8dff6b;
		--hot: #ff3b4d;
		--spark: #fff6a8;
		--shadow: #ff3bb0;
		--scan: rgba(0, 0, 0, 0.22);
	}
	.scene-arcade:global(.lv-warn) {
		--beat: 0.55s;
		--march: 3s;
		--lower: 36px;
	}
	.scene-arcade:global(.lv-critical) {
		--beat: 0.3s;
		--march: 1.7s;
		--lower: 72px;
		--c1: #ff5a6e;
		--c2: #ff8a5a;
		--c3: #ffb04a;
	}

	.scan {
		position: absolute;
		inset: 0;
		pointer-events: none;
		background: repeating-linear-gradient(180deg, var(--scan) 0 1px, transparent 1px 3px);
	}
	.alert {
		position: absolute;
		inset: 0;
		pointer-events: none;
		opacity: 0;
		border: 5px solid var(--hot);
		box-shadow: inset 0 0 40px color-mix(in srgb, var(--hot) 55%, transparent);
	}
	:global(.lv-critical) .alert {
		opacity: 1;
		animation: blink 0.9s step-end infinite;
	}

	.fleet {
		translate: 0 var(--lower);
		transition: translate 1.2s steps(6);
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
	.star {
		animation: twinkle 3s step-end infinite;
	}

	/* idle state of everything the gestures drive */
	.hero-red,
	.burst,
	.laser,
	.score,
	.confetti {
		opacity: 0;
	}

	/* down: red blinks, the crab falls to the ground and bursts, then respawns */
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

	/* up: the cannon slides under the red crab, zaps it, it respawns */
	:global(.k-up) .hero-ok {
		animation: u-ok 3s step-end;
	}
	:global(.k-up) .hero-red {
		animation: u-red 3s step-end;
	}
	:global(.k-up) .cannon {
		animation: u-cannon 3s linear;
	}
	:global(.k-up) .laser {
		animation: u-laser 3s linear;
	}
	:global(.k-up) .burst-hero {
		animation: u-burst 3s step-end;
	}

	/* resolved: HIGH SCORE and confetti */
	:global(.k-resolved) .score {
		animation: score 4.2s step-end;
	}
	:global(.k-resolved) .confetti {
		animation: fall 2.6s linear both;
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
	@keyframes march {
		0% {
			translate: 0 0;
			animation-timing-function: steps(8);
		}
		45% {
			translate: 48px 0;
		}
		50% {
			translate: 48px 0;
			animation-timing-function: steps(8);
		}
		95% {
			translate: 0 0;
		}
		100% {
			translate: 0 0;
		}
	}

	@keyframes d-move {
		0%,
		30% {
			translate: 0 0;
			animation-timing-function: steps(8);
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
			translate: 0 0;
			animation-timing-function: steps(11);
		}
		33% {
			translate: var(--slide) 0;
		}
		72% {
			translate: var(--slide) 0;
			animation-timing-function: steps(11);
		}
		100% {
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
			animation-timing-function: steps(7);
		}
		46% {
			opacity: 1;
			translate: 0 -160px;
		}
		47%,
		100% {
			opacity: 0;
			translate: 0 -160px;
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
			animation-timing-function: steps(16);
		}
		100% {
			opacity: 1;
			translate: var(--sway) 580px;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
		.fb {
			opacity: 0;
		}
		:global(.lv-critical) .alert {
			opacity: 1;
		}
	}
</style>
