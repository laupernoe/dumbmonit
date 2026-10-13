<script lang="ts">
	/**
	 * A big aquarium — colourful fish, swaying plants, bubbles from an air
	 * stone, a little treasure chest, a DumbMonit pigeon sitting on the gravel
	 * and a filter whose LED takes the page's status colour (`--scene-tone`).
	 * Follows the scene contract: fills its positioned parent, the headline
	 * zone (top left) is left clear, the palette lives here.
	 *
	 * `mood` is the weather: ok = clear water, warn = murky water, critical =
	 * dark water with a shark's shadow gliding past (`calm`, `clouded` and
	 * `storm` are accepted as aliases). `pulse` plays one gesture each time
	 * `at` changes: down = the orange fish turns pale and floats up, up = it
	 * revives and darts off, resolved = the chest opens in a burst of bubbles.
	 * Reduced motion freezes the picture.
	 */
	interface Props {
		mood?: 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}
	let { mood = 'ok', pulse = null }: Props = $props();

	const tone = $derived(
		mood === 'warn' || mood === 'clouded'
			? 'warn'
			: mood === 'critical' || mood === 'storm'
				? 'critical'
				: 'ok'
	);

	type Fish = { x: number; y: number; t: 'a' | 'b' | 'c'; c: string; s: number; d: number; dur: number; delay: number };
	const FISH: Fish[] = [
		{ x: 760, y: 330, t: 'a', c: '#f2c230', s: 1, d: 180, dur: 14, delay: 0 },
		{ x: 1000, y: 200, t: 'c', c: '#f2d14a', s: 1.1, d: 140, dur: 17, delay: -5 },
		{ x: 1250, y: 340, t: 'b', c: '#4aa8e8', s: 0.9, d: 200, dur: 12, delay: -3 },
		{ x: 900, y: 420, t: 'a', c: '#e85d8a', s: 0.8, d: 120, dur: 11, delay: -7 },
		{ x: 1480, y: 210, t: 'a', c: '#5ed1b0', s: 0.7, d: 100, dur: 9, delay: -2 },
		{ x: 700, y: 200, t: 'b', c: '#a678e0', s: 0.8, d: 150, dur: 15, delay: -9 },
		{ x: 1560, y: 360, t: 'c', c: '#ff9f5a', s: 0.9, d: 160, dur: 13, delay: -4 }
	];
	const SCHOOL = [0, 1, 2, 3, 4].map((i) => ({ x: 1330 + i * 16, y: 110 + (i % 2) * 14 + i * 3, delay: -i * 0.6 }));
	const BUBBLES = [0, 1, 2, 3, 4, 5, 6, 7].map((i) => ({
		x: 800 + (i % 3) * 5,
		r: 2.5 + (i % 4) * 1.2,
		dur: 5 + (i % 5) * 0.9,
		delay: -i * 0.8,
		dx: (i % 2 ? 1 : -1) * (6 + i * 2)
	}));
	const WEEDS = [
		{ x: 620, h: 150, a: 14, c: 1, dur: 5 },
		{ x: 646, h: 110, a: -12, c: 2, dur: 6 },
		{ x: 960, h: 90, a: 10, c: 2, dur: 5.5 },
		{ x: 1250, h: 140, a: -14, c: 1, dur: 6.5 },
		{ x: 1272, h: 100, a: 12, c: 2, dur: 5 },
		{ x: 1560, h: 160, a: 14, c: 1, dur: 7 },
		{ x: 1584, h: 110, a: -10, c: 2, dur: 5.8 },
		{ x: 1660, h: 120, a: 12, c: 1, dur: 6.2 }
	];
	const BURST = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map((i) => ({
		x: -18 + i * 4,
		r: 3 + (i % 4) * 1.5,
		dx: (i - 4.5) * 12,
		delay: i * 0.09
	}));
</script>

<div class="scene scene-aquarium tone-{tone}" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<g id="aq-fa">
				<path d="M-26 0L-46-13Q-40 0-46 13Z" fill="currentColor"/>
				<path d="M-28 0C-14-16 14-16 30 0C14 16-14 16-28 0Z" fill="currentColor"/>
				<path d="M-6-11Q4-26 14-11Z" fill="currentColor"/>
				<path d="M-6-11Q4-26 14-11Z" fill="#000" opacity=".15"/>
				<path d="M6-14Q12 0 6 14" stroke="#fff" stroke-width="4" opacity=".55"/>
				<circle cx="19" cy="-2" r="3.2" fill="#fff"/><circle cx="20" cy="-2" r="1.6" fill="#14202e"/>
			</g>
			<g id="aq-fb">
				<path d="M-22 0L-40-10L-36 0L-40 10Z" fill="currentColor"/>
				<ellipse rx="26" ry="17" fill="currentColor"/>
				<ellipse cx="-2" cy="5" rx="18" ry="8" fill="#fff" opacity=".25"/>
				<path d="M-4-16Q4-24 12-15Z" fill="#000" opacity=".18"/>
				<circle cx="14" cy="-4" r="3.4" fill="#fff"/><circle cx="15" cy="-4" r="1.7" fill="#14202e"/>
			</g>
			<g id="aq-fc">
				<path d="M-14 0L-34-10L-30 0L-34 10Z" fill="currentColor"/>
				<path d="M-16 0L6-28L28 0L6 28Z" fill="currentColor"/>
				<path d="M2-24L-4-40M2 24L-4 40" stroke="currentColor" stroke-width="5" stroke-linecap="round"/>
				<path d="M4-20Q10 0 4 20" stroke="#14202e" stroke-width="3" opacity=".4"/>
				<circle cx="16" cy="-3" r="3" fill="#fff"/><circle cx="17" cy="-3" r="1.5" fill="#14202e"/>
			</g>
			<linearGradient id="aq-ray" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="#fff" stop-opacity=".5"/><stop offset="1" stop-color="#fff" stop-opacity="0"/>
			</linearGradient>
		</defs>

		<!-- light shafts -->
		<g class="rays">
			<path d="M900 80L960 80L890 500L760 500Z" fill="url(#aq-ray)"/>
			<path d="M1200 80L1240 80L1220 500L1120 500Z" fill="url(#aq-ray)"/>
			<path d="M1520 80L1580 80L1630 500L1490 500Z" fill="url(#aq-ray)"/>
		</g>

		<!-- gravel and rocks -->
		<path d="M560 520C760 490 960 500 1160 494C1360 488 1540 492 1780 500V620H560Z" fill="var(--sand)"/>
		<path d="M560 548C860 530 1260 540 1560 532V620H560Z" fill="var(--sand2)"/>
		<g fill="var(--rock)"><path d="M860 500C860 470 900 462 930 470C952 476 962 490 966 502Z"/><path d="M1400 500C1406 478 1436 472 1458 482C1470 488 1476 496 1478 502Z"/></g>
		<g fill="var(--rock2)" opacity=".7"><path d="M880 500C884 484 906 480 920 486C930 490 936 496 938 502Z"/></g>

		<!-- plants -->
		{#each WEEDS as w, i (i)}
			<path class="weed" style="animation-duration:{w.dur}s" d="M{w.x} 502q{w.a} {-w.h / 4} 0 {-w.h / 2}t0 {-w.h / 2}" stroke={w.c === 1 ? 'var(--plant1)' : 'var(--plant2)'} stroke-width="9" stroke-linecap="round"/>
		{/each}
		<g class="coral"><path d="M1040 504v-26M1040 490l-12-14M1040 482l12-12" stroke="var(--coral)" stroke-width="8" stroke-linecap="round"/><circle cx="1028" cy="474" r="5" fill="var(--coral)"/><circle cx="1052" cy="468" r="5" fill="var(--coral)"/><circle cx="1040" cy="474" r="5" fill="var(--coral)"/></g>

		<!-- filter with the status LED -->
		<rect x="1722" y="82" width="26" height="150" rx="4" fill="var(--rock2)" opacity=".8"/>
		<circle cx="1735" cy="100" r="5" fill="var(--scene-tone, var(--c-signal))"/>
		<circle cx="1735" cy="100" r="13" fill="var(--scene-tone, var(--c-signal))" opacity=".25"/>

		<!-- the pigeon on the gravel -->
		<use href="#dm-mascot" transform="translate(1340 468) scale(.1)"/>

		<!-- treasure chest -->
		{#key pulse?.at}
			<g transform="translate(1160 502)" class="chest" class:open={pulse?.kind === 'resolved'}>
				<circle class="glow" cx="0" cy="-30" r="30" fill="#ffd45a"/>
				<rect x="-27" y="-26" width="54" height="26" rx="3" fill="var(--wood)"/>
				<rect x="-27" y="-14" width="54" height="4" fill="var(--gold)"/>
				<g class="coins" fill="var(--gold)"><circle cx="-10" cy="-27" r="5"/><circle cx="2" cy="-30" r="5"/><circle cx="12" cy="-27" r="5"/></g>
				<path class="lid" d="M-27-26Q-27-48 0-48Q27-48 27-26Z" fill="var(--wood2)"/>
				<rect x="-4" y="-22" width="8" height="10" rx="2" fill="var(--gold)"/>
				{#if pulse?.kind === 'resolved'}
					{#each BURST as b, i (i)}
						<circle class="burst" style="--dx:{b.dx}px;animation-delay:{b.delay}s" cx={b.x} cy="-34" r={b.r} stroke="#fff" stroke-width="1.5" fill="#fff" fill-opacity=".25"/>
					{/each}
				{/if}
			</g>
		{/key}

		<!-- fish -->
		{#each FISH as f, i (i)}
			<g transform="translate({f.x} {f.y}) scale({f.s})">
				<g class="swim" style="--d:{f.d}px;animation-duration:{f.dur}s;animation-delay:{f.delay}s">
					<use href="#aq-f{f.t}" style="color:{f.c}"/>
				</g>
			</g>
		{/each}
		{#each SCHOOL as s, i (i)}
			<g transform="translate({s.x} {s.y}) scale(.3)">
				<g class="swim" style="--d:260px;animation-duration:16s;animation-delay:{s.delay}s">
					<use href="#aq-fa" style="color:#8fd3ee"/>
				</g>
			</g>
		{/each}

		<!-- the orange fish: the gesture -->
		<g transform="translate(1080 330) scale(1.25)">
			<g class="swim" style="--d:90px;animation-duration:10s">
				{#key pulse?.at}
					<g class="hero" class:hero-down={pulse?.kind === 'down'} class:hero-up={pulse?.kind === 'up'}>
						<use href="#aq-fa" style="color:#ff7a2f"/>
						<path class="xeye" d="M16-5l7 7M23-5l-7 7" stroke="#14202e" stroke-width="2" stroke-linecap="round"/>
					</g>
				{/key}
			</g>
		</g>

		<!-- bubbles from the air stone -->
		{#each BUBBLES as b, i (i)}
			<circle class="bub" style="--dx:{b.dx}px;animation-duration:{b.dur}s;animation-delay:{b.delay}s" cx={b.x} cy="498" r={b.r} stroke="#fff" stroke-width="1.2" fill="#fff" fill-opacity=".2"/>
		{/each}

		<!-- the shark's shadow -->
		<g class="shark">
			<path d="M0 0C60-30 160-44 250-20C290-8 320-30 350-60C346-30 352-6 372 20C340 10 320 14 300 12C230 40 120 40 40 22Z M150-34L178-78L204-28Z M180 24L150 70L210 34Z"/>
		</g>
	</svg>
	<div class="murk"></div>
	<div class="deep"></div>
</div>

<style>
	.scene-aquarium {
		--w1: #6fd0e4;
		--w2: #1f86b0;
		--sand: #e8d3a0;
		--sand2: #d6bb82;
		--rock: #8a93a0;
		--rock2: #6e7886;
		--plant1: #3f9a58;
		--plant2: #6bbf6c;
		--coral: #f06a7a;
		--wood: #9a5b2b;
		--wood2: #b06a34;
		--gold: #f0c04a;
		--murk: #7a8a4a;
	}
	:global(.dark) .scene-aquarium {
		--w1: #134a6b;
		--w2: #071a2e;
		--sand: #5a5240;
		--sand2: #453f31;
		--rock: #3b4552;
		--rock2: #2c3541;
		--plant1: #1f6a3c;
		--plant2: #2f8a50;
		--coral: #b04456;
		--wood: #6d4020;
		--wood2: #82502a;
		--gold: #c99a2e;
		--murk: #3d4a22;
	}
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(180deg, var(--w1), var(--w2));
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
	}
	.murk,
	.deep {
		position: absolute;
		inset: 0;
		pointer-events: none;
		opacity: 0;
		transition: opacity 1.6s ease;
	}
	.murk {
		background: var(--murk);
	}
	.deep {
		background: #020c16;
	}
	.tone-warn .murk {
		opacity: 0.4;
	}
	.tone-warn .deep {
		opacity: 0.1;
	}
	.tone-critical .murk {
		opacity: 0.25;
	}
	.tone-critical .deep {
		opacity: 0.55;
	}
	.rays {
		opacity: 0.4;
		transition: opacity 1.6s ease;
		animation: aq-sway 12s ease-in-out infinite alternate;
	}
	.tone-warn .rays {
		opacity: 0.12;
	}
	.tone-critical .rays {
		opacity: 0;
	}
	@keyframes aq-sway {
		to {
			transform: translateX(26px);
		}
	}

	.swim {
		transform-box: fill-box;
		transform-origin: center;
		animation: aq-swim 12s linear infinite;
	}
	@keyframes aq-swim {
		0% {
			transform: translateX(0) scaleX(1);
		}
		49% {
			transform: translateX(var(--d)) scaleX(1);
		}
		50% {
			transform: translateX(var(--d)) scaleX(-1);
		}
		99% {
			transform: translateX(0) scaleX(-1);
		}
		100% {
			transform: translateX(0) scaleX(1);
		}
	}
	.weed {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: aq-weed 5s ease-in-out infinite alternate;
	}
	@keyframes aq-weed {
		from {
			transform: rotate(-4deg);
		}
		to {
			transform: rotate(4deg);
		}
	}
	.bub {
		opacity: 0;
		animation: aq-rise 6s linear infinite;
	}
	@keyframes aq-rise {
		0% {
			transform: translate(0, 0);
			opacity: 0;
		}
		10% {
			opacity: 0.8;
		}
		100% {
			transform: translate(var(--dx), -410px);
			opacity: 0;
		}
	}

	.shark {
		fill: #010a12;
		opacity: 0;
		transition: opacity 1.6s ease;
		transform: translate(560px, 250px) scale(1.4);
	}
	.tone-critical .shark {
		opacity: 0.55;
		animation: aq-shark 38s linear infinite;
	}
	@keyframes aq-shark {
		from {
			transform: translate(560px, 250px) scale(1.4);
		}
		to {
			transform: translate(1900px, 210px) scale(1.4);
		}
	}

	/* The orange fish. */
	.hero {
		transform-box: fill-box;
		transform-origin: center;
	}
	.xeye {
		opacity: 0;
	}
	.hero-down {
		animation: aq-down 4.5s ease-in-out both;
	}
	.hero-down .xeye {
		animation: aq-xeye 4.5s step-end both;
	}
	@keyframes aq-down {
		0% {
			transform: translateY(0) scaleY(1);
			filter: none;
		}
		35%,
		80% {
			transform: translateY(-110px) scaleY(-1) rotate(8deg);
			filter: grayscale(1) brightness(1.35);
		}
		100% {
			transform: translateY(0) scaleY(1);
			filter: none;
		}
	}
	@keyframes aq-xeye {
		0%,
		100% {
			opacity: 0;
		}
		20%,
		85% {
			opacity: 1;
		}
	}
	.hero-up {
		animation: aq-up 3.6s ease-in-out both;
	}
	@keyframes aq-up {
		0% {
			transform: translate(0, -100px) scaleY(-1);
			filter: grayscale(1) brightness(1.35);
		}
		22% {
			transform: translate(0, -70px) scaleY(1) rotate(-12deg);
			filter: none;
		}
		32% {
			transform: translate(-16px, -70px) rotate(8deg);
		}
		55% {
			transform: translate(620px, -110px);
			opacity: 1;
		}
		56% {
			transform: translate(620px, -110px);
			opacity: 0;
		}
		57% {
			transform: translate(-420px, 0);
			opacity: 0;
		}
		100% {
			transform: translate(0, 0);
			opacity: 1;
		}
	}

	/* The chest. */
	.glow {
		opacity: 0;
	}
	.lid {
		transform-box: fill-box;
		transform-origin: 100% 100%;
	}
	.coins {
		opacity: 0;
	}
	.open .lid {
		animation: aq-lid 5s ease-in-out both;
	}
	.open .glow,
	.open .coins {
		animation: aq-glow 5s ease-in-out both;
	}
	@keyframes aq-lid {
		0% {
			transform: rotate(0);
		}
		15%,
		75% {
			transform: rotate(68deg);
		}
		100% {
			transform: rotate(0);
		}
	}
	@keyframes aq-glow {
		0%,
		100% {
			opacity: 0;
		}
		20%,
		70% {
			opacity: 0.9;
		}
	}
	.burst {
		opacity: 0;
		transform-box: fill-box;
		transform-origin: center;
		animation: aq-burst 2.4s ease-out both;
	}
	@keyframes aq-burst {
		0% {
			transform: translate(0, 0) scale(0.3);
			opacity: 0;
		}
		15% {
			opacity: 0.95;
		}
		100% {
			transform: translate(var(--dx), -270px) scale(1);
			opacity: 0;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
