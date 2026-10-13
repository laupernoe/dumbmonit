<script lang="ts">
	/**
	 * The wall's living backdrop: a big aquarium with colourful fish, swaying
	 * plants, bubbles and a little treasure chest. A drop-in for the Paris scene
	 * (same props, plus `pulse`).
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The wall theme
	 * sets how bright the water is; the real hour dims the tank light after dark.
	 *
	 * The network's state is the water: calm (or `ok`) is clear with light
	 * shafts, clouded (`warn`) is murky, a storm (`critical`) is dark with a
	 * shark's shadow gliding by. Each problem on the board is a pale fish
	 * floating belly-up at the surface. A new problem startles every fish once.
	 * `pulse` plays a single gesture each time `at` changes: down = the orange
	 * fish turns pale and floats up, up = it revives and darts off, resolved =
	 * the chest opens in a burst of bubbles.
	 *
	 * All motion is transform and opacity; hidden tabs pause it, reduced motion
	 * freezes the scene in a still pose.
	 */
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import { phaseAt, type SceneMood, type SceneTheme } from '#lib/components/wall/paris/daylight.js';
	import {
		STAGE_W,
		STAGE_H,
		FLOOR_Y,
		FISH,
		HERO,
		SCHOOL,
		WEEDS,
		BUBBLES,
		BURST,
		PEBBLES,
		RAYS,
		SHARK_PATH,
		aquariumStyle,
		normMood
	} from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood?: SceneMood | 'ok' | 'warn' | 'critical';
		/** Problems on the board: that many pale fish float at the surface (up to three). */
		problems?: number;
		/** Bumped by the wall when a new problem appears: every fish startles once. */
		startle?: number;
		/** The tab is hidden: nothing moves, nothing is scheduled. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
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

	const motion = !reducedMotion();

	// --- Stage fit ----------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light and weather ----------------------------------------------------------

	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const phase = $derived(phaseAt(new Date(minuteKey * 60_000)));
	const style = $derived(aquariumStyle(theme, phase));
	const tone = $derived(normMood(mood));
	const floaters = $derived(Math.min(3, Math.max(0, Math.floor(problems))));

	// A new problem: everyone darts off for a moment.
	let startled = $state(false);
	let lastStartle = untrack(() => startle);
	$effect(() => {
		const value = startle;
		if (value === lastStartle) return;
		lastStartle = value;
		if (!motion) return;
		startled = true;
		const timer = setTimeout(() => (startled = false), 1600);
		return () => clearTimeout(timer);
	});
</script>

<div
	class="scene tone-{tone} {className}"
	class:scene--paused={paused}
	class:scene--still={!motion}
	{style}
	bind:clientWidth={boxW}
	bind:clientHeight={boxH}
	aria-hidden="true"
>
	{#if scale > 0}
		<div class="stage" style:transform="translate({offsetX}px, {offsetY}px) scale({scale})">
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}" fill="none" focusable="false">
				<defs>
					<g id="aqw-fa">
						<path d="M-26 0L-46-13Q-40 0-46 13Z" fill="currentColor" />
						<path d="M-28 0C-14-16 14-16 30 0C14 16-14 16-28 0Z" fill="currentColor" />
						<path d="M-6-11Q4-26 14-11Z" fill="currentColor" />
						<path d="M-6-11Q4-26 14-11Z" fill="#000" opacity=".15" />
						<path d="M6-14Q12 0 6 14" stroke="#fff" stroke-width="4" opacity=".55" />
						<circle cx="19" cy="-2" r="3.2" fill="#fff" /><circle cx="20" cy="-2" r="1.6" fill="#14202e" />
					</g>
					<g id="aqw-fb">
						<path d="M-22 0L-40-10L-36 0L-40 10Z" fill="currentColor" />
						<ellipse rx="26" ry="17" fill="currentColor" />
						<ellipse cx="-2" cy="5" rx="18" ry="8" fill="#fff" opacity=".25" />
						<path d="M-4-16Q4-24 12-15Z" fill="#000" opacity=".18" />
						<circle cx="14" cy="-4" r="3.4" fill="#fff" /><circle cx="15" cy="-4" r="1.7" fill="#14202e" />
					</g>
					<g id="aqw-fc">
						<path d="M-14 0L-34-10L-30 0L-34 10Z" fill="currentColor" />
						<path d="M-16 0L6-28L28 0L6 28Z" fill="currentColor" />
						<path d="M2-24L-4-40M2 24L-4 40" stroke="currentColor" stroke-width="5" stroke-linecap="round" />
						<path d="M4-20Q10 0 4 20" stroke="#14202e" stroke-width="3" opacity=".4" />
						<circle cx="16" cy="-3" r="3" fill="#fff" /><circle cx="17" cy="-3" r="1.5" fill="#14202e" />
					</g>
					<linearGradient id="aqw-ray" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="#fff" stop-opacity=".45" /><stop offset="1" stop-color="#fff" stop-opacity="0" />
					</linearGradient>
				</defs>

				<g class="rays">
					{#each RAYS as d, i (i)}<path {d} fill="url(#aqw-ray)" />{/each}
				</g>

				<!-- far rocks -->
				<g fill="var(--rock-2)" opacity=".5">
					<path d="M0 {FLOOR_Y + 20}C40 800 160 780 260 830C320 860 360 900 380 {FLOOR_Y + 20}Z" />
					<path d="M1380 {FLOOR_Y + 20}C1420 840 1520 810 1600 850C1660 880 1690 920 1700 {FLOOR_Y + 20}Z" />
				</g>

				<!-- gravel -->
				<path d="M0 {FLOOR_Y + 20}C400 {FLOOR_Y - 20} 900 {FLOOR_Y + 10} 1300 {FLOOR_Y - 10}C1560 {FLOOR_Y - 24} 1760 {FLOOR_Y} 1920 {FLOOR_Y - 6}V1080H0Z" fill="var(--sand)" />
				<path d="M0 {FLOOR_Y + 70}C500 {FLOOR_Y + 40} 1100 {FLOOR_Y + 66} 1920 {FLOOR_Y + 50}V1080H0Z" fill="var(--sand-2)" />
				{#each PEBBLES as p, i (i)}
					<ellipse cx={p.x} cy={p.y} rx={p.rx} ry={p.ry} fill={p.tone === 1 ? 'var(--rock)' : 'var(--rock-2)'} opacity=".6" />
				{/each}
				<g fill="var(--rock)"><path d="M470 {FLOOR_Y + 20}C470 900 540 880 600 896C640 906 664 930 670 {FLOOR_Y + 20}Z" /></g>

				<!-- plants and coral -->
				{#each WEEDS as w, i (i)}
					<path class="weed" style="animation-duration:{w.dur}s;animation-delay:{w.delay}s" d={w.d} stroke={w.color === 1 ? 'var(--plant-1)' : 'var(--plant-2)'} stroke-width={w.width} stroke-linecap="round" />
				{/each}
				<g stroke="var(--coral)" stroke-width="14" stroke-linecap="round" fill="var(--coral)">
					<path d="M960 {FLOOR_Y + 20}v-70M960 {FLOOR_Y - 20}l-30-34M960 {FLOOR_Y - 40}l28-30" fill="none" />
					<circle cx="928" cy={FLOOR_Y - 56} r="9" /><circle cx="990" cy={FLOOR_Y - 74} r="9" /><circle cx="960" cy={FLOOR_Y - 54} r="9" />
				</g>

				<!-- air stone -->
				<rect x="288" y={FLOOR_Y + 4} width="34" height="14" rx="6" fill="var(--rock-2)" />

				<!-- treasure chest -->
				{#key pulse?.at}
					<g transform="translate(1620 {FLOOR_Y + 40}) scale(2.4)" class="chest" class:open={pulse?.kind === 'resolved'}>
						<circle class="glow" cx="0" cy="-30" r="34" fill="#ffd45a" />
						<rect x="-27" y="-26" width="54" height="26" rx="3" fill="var(--wood)" />
						<rect x="-27" y="-14" width="54" height="4" fill="var(--gold)" />
						<g class="coins" fill="var(--gold)"><circle cx="-10" cy="-27" r="5" /><circle cx="2" cy="-30" r="5" /><circle cx="12" cy="-27" r="5" /></g>
						<path class="lid" d="M-27-26Q-27-48 0-48Q27-48 27-26Z" fill="var(--wood-2)" />
						<rect x="-4" y="-22" width="8" height="10" rx="2" fill="var(--gold)" />
						{#if pulse?.kind === 'resolved'}
							{#each BURST as b, i (i)}
								<circle class="burst" style="--dx:{b.dx}px;animation-delay:{b.delay}s" cx={b.x} cy="-34" r={b.r} stroke="#fff" stroke-width="1.5" fill="#fff" fill-opacity=".25" />
							{/each}
						{/if}
					</g>
				{/key}

				<!-- fish -->
				{#each FISH as f, i (i)}
					<g transform="translate({f.x} {f.y}) scale({f.scale})">
						<g class="fx" class:fx--dart={startled} style="animation-delay:{i * 0.05}s">
							<g class="swim" style="--d:{f.d}px;animation-duration:{f.dur}s;animation-delay:{f.delay}s">
								<use href="#aqw-f{f.kind}" style="color:{f.color}" />
							</g>
						</g>
					</g>
				{/each}
				{#each SCHOOL as s, i (i)}
					<g transform="translate({s.x} {s.y}) scale(.6)">
						<g class="fx" class:fx--dart={startled}>
							<g class="swim" style="--d:420px;animation-duration:22s;animation-delay:{s.delay}s">
								<use href="#aqw-fa" style="color:#8fd3ee" />
							</g>
						</g>
					</g>
				{/each}

				<!-- one pale fish per problem, belly-up at the surface -->
				{#each Array.from({ length: floaters }, (_, k) => k) as k (k)}
					<g transform="translate({760 + k * 200} 120) scale(1.5)">
						<g class="floater" style="animation-delay:{-k * 1.3}s">
							<g transform="scale(1 -1)" style="filter:grayscale(1) brightness(1.3)">
								<use href="#aqw-fa" style="color:#f2c230" />
								<path d="M16-5l7 7M23-5l-7 7" stroke="#14202e" stroke-width="2" stroke-linecap="round" />
							</g>
						</g>
					</g>
				{/each}

				<!-- the orange fish: the gesture -->
				<g transform="translate({HERO.x} {HERO.y}) scale({HERO.scale})">
					<g class="fx" class:fx--dart={startled}>
						<g class="swim" style="--d:{HERO.d}px;animation-duration:{HERO.dur}s">
							{#key pulse?.at}
								<g class="hero" class:hero-down={pulse?.kind === 'down'} class:hero-up={pulse?.kind === 'up'}>
									<use href="#aqw-fa" style="color:{HERO.color}" />
									<path class="xeye" d="M16-5l7 7M23-5l-7 7" stroke="#14202e" stroke-width="2" stroke-linecap="round" />
								</g>
							{/key}
						</g>
					</g>
				</g>

				<!-- bubbles: the air stone and the chest's own trickle -->
				{#each BUBBLES as b, i (i)}
					<circle class="bub" style="--dx:{b.dx}px;animation-duration:{b.dur}s;animation-delay:{b.delay}s" cx={b.x} cy={FLOOR_Y} r={b.r} stroke="#fff" stroke-width="2" fill="#fff" fill-opacity=".18" />
				{/each}

				<!-- the shark's shadow -->
				<g class="shark"><path d={SHARK_PATH} /></g>
			</svg>

			<div class="veil murk"></div>
			<div class="veil deep"></div>
			<div class="veil night"></div>
		</div>
	{/if}
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
		contain: strict;
		background: var(--w-bottom);
	}
	.stage {
		position: absolute;
		left: 0;
		top: 0;
		width: 1920px;
		height: 1080px;
		transform-origin: 0 0;
		background: linear-gradient(to bottom, var(--w-top), var(--w-bottom));
	}
	.layer {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}
	.veil {
		position: absolute;
		inset: 0;
		opacity: 0;
		transition: opacity 2s ease;
	}
	.murk {
		background: var(--murk);
	}
	.deep {
		background: #020c16;
	}
	.night {
		background: #020a18;
		opacity: var(--night, 0);
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
		transition: opacity 2s ease;
		animation: aqw-sway 16s ease-in-out infinite alternate;
	}
	.tone-warn .rays {
		opacity: 0.12;
	}
	.tone-critical .rays {
		opacity: 0;
	}
	@keyframes aqw-sway {
		to {
			transform: translateX(60px);
		}
	}

	.swim,
	.fx,
	.floater,
	.hero,
	.weed,
	.burst,
	.lid {
		transform-box: fill-box;
		transform-origin: center;
	}
	.swim {
		animation: aqw-swim 20s linear infinite;
	}
	@keyframes aqw-swim {
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
	.fx--dart {
		animation: aqw-dart 1.6s ease-in-out both;
	}
	@keyframes aqw-dart {
		0%,
		100% {
			transform: translate(0, 0);
		}
		30% {
			transform: translate(60px, -26px);
		}
		65% {
			transform: translate(-30px, 14px);
		}
	}
	.floater {
		animation: aqw-bob 4s ease-in-out infinite alternate;
	}
	@keyframes aqw-bob {
		from {
			transform: translateY(0) rotate(-4deg);
		}
		to {
			transform: translateY(10px) rotate(5deg);
		}
	}
	.weed {
		transform-origin: 50% 100%;
		animation: aqw-weed 6s ease-in-out infinite alternate;
	}
	@keyframes aqw-weed {
		from {
			transform: rotate(-4deg);
		}
		to {
			transform: rotate(4deg);
		}
	}
	.bub {
		opacity: 0;
		animation: aqw-rise 10s linear infinite;
	}
	@keyframes aqw-rise {
		0% {
			transform: translate(0, 0);
			opacity: 0;
		}
		8% {
			opacity: 0.8;
		}
		100% {
			transform: translate(var(--dx), -900px);
			opacity: 0;
		}
	}

	.shark {
		fill: #010a12;
		opacity: 0;
		transition: opacity 2s ease;
		transform: translate(-600px, 380px) scale(2.4);
	}
	.tone-critical .shark {
		opacity: 0.55;
		animation: aqw-shark 46s linear infinite;
	}
	@keyframes aqw-shark {
		from {
			transform: translate(-900px, 400px) scale(2.4);
		}
		to {
			transform: translate(2000px, 340px) scale(2.4);
		}
	}

	/* The orange fish. */
	.xeye {
		opacity: 0;
	}
	.hero-down {
		animation: aqw-down 4.5s ease-in-out both;
	}
	.hero-down .xeye {
		animation: aqw-xeye 4.5s step-end both;
	}
	@keyframes aqw-down {
		0% {
			transform: translateY(0) scaleY(1);
			filter: none;
		}
		35%,
		80% {
			transform: translateY(-300px) scaleY(-1) rotate(8deg);
			filter: grayscale(1) brightness(1.35);
		}
		100% {
			transform: translateY(0) scaleY(1);
			filter: none;
		}
	}
	@keyframes aqw-xeye {
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
		animation: aqw-up 3.6s ease-in-out both;
	}
	@keyframes aqw-up {
		0% {
			transform: translate(0, -240px) scaleY(-1);
			filter: grayscale(1) brightness(1.35);
		}
		22% {
			transform: translate(0, -170px) scaleY(1) rotate(-12deg);
			filter: none;
		}
		32% {
			transform: translate(-30px, -170px) rotate(8deg);
		}
		55% {
			transform: translate(1100px, -260px);
			opacity: 1;
		}
		56% {
			transform: translate(1100px, -260px);
			opacity: 0;
		}
		57% {
			transform: translate(-900px, 0);
			opacity: 0;
		}
		100% {
			transform: translate(0, 0);
			opacity: 1;
		}
	}

	/* The chest. */
	.glow,
	.coins {
		opacity: 0;
	}
	.lid {
		transform-origin: 100% 100%;
	}
	.open .lid {
		animation: aqw-lid 5s ease-in-out both;
	}
	.open .glow,
	.open .coins {
		animation: aqw-glow 5s ease-in-out both;
	}
	@keyframes aqw-lid {
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
	@keyframes aqw-glow {
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
		animation: aqw-burst 2.6s ease-out both;
	}
	@keyframes aqw-burst {
		0% {
			transform: translate(0, 0) scale(0.3);
			opacity: 0;
		}
		15% {
			opacity: 0.95;
		}
		100% {
			transform: translate(var(--dx), -420px) scale(1);
			opacity: 0;
		}
	}

	/* A hidden tab: everything holds its breath. */
	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	/* Reduced motion: a still picture. */
	.scene--still :global(*) {
		animation: none !important;
		transition: none !important;
	}
	.scene--still .bub {
		opacity: 0.5;
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
