<script lang="ts">
	/**
	 * The night every city shares: a near-black sky with its stars (a few of
	 * them twinkle), the moon and its halo, the glow of the city on the
	 * horizon, the water with the moon's path on it and the reflection of the
	 * skyline. The city itself comes in `children` (everything standing on the
	 * horizon, reflected) and `front` (boats, quays, trees: in front of the
	 * water, not reflected); both receive this scene's unique id prefix.
	 *
	 * The weather is the state of the page: clear when all is well, moonlit
	 * clouds drifting over the stars on a warning, a storm (heavy clouds, rain,
	 * a distant lightning flash every few seconds) when something is down. It
	 * is never the only sign: the page says it in words.
	 *
	 * Landmark gestures (`pulse`): on `down` what a city marks `.lights` goes
	 * out and `.windows` dim, on `up` the lights come back, on `resolved` the
	 * `.sparkle` points glitter. `sparkle` keeps them glittering (the wall, on
	 * the hour). Under `prefers-reduced-motion` nothing moves and no gesture
	 * plays; `paused` holds every animation where it is.
	 */
	import type { Snippet } from 'svelte';
	import {
		BANNER_FRAME,
		HORIZON,
		MOON,
		STAGE_FRAME,
		WORLD_BOTTOM,
		WORLD_X0,
		WORLD_X1,
		fitViewBox,
		makeStars,
		moodLevel,
		type SceneFrame,
		type SceneMood,
		type ScenePulse
	} from './nightScene';
	import { createPulse } from './scenePulse.svelte.js';

	interface Props {
		mood?: SceneMood;
		pulse?: ScenePulse;
		/** `banner`: the status page head; `stage`: the wall's full screen. */
		frame?: 'banner' | 'stage';
		/** World x kept in view on a narrow box (overrides the frame's). */
		focus?: number;
		paused?: boolean;
		sparkle?: boolean;
		/** Colour of the city's glow on the horizon. */
		glow?: string;
		/** Seed of the starfield, so each city has its own sky. */
		seed?: number;
		children: Snippet<[string]>;
		front?: Snippet<[string]>;
	}

	let {
		mood = 'ok',
		pulse = null,
		frame = 'banner',
		focus,
		paused = false,
		sparkle = false,
		glow = '#3a3f6b',
		seed = 11,
		children,
		front
	}: Props = $props();

	const uid = $props.id();
	const level = $derived(moodLevel(mood));
	const pl = createPulse(() => pulse);

	let boxW = $state(0);
	let boxH = $state(0);
	const shown = $derived<SceneFrame>({
		...(frame === 'stage' ? STAGE_FRAME : BANNER_FRAME),
		...(focus !== undefined ? { focus } : {})
	});
	const viewBox = $derived(fitViewBox(boxW, boxH, shown));

	const W0 = WORLD_X0;
	const WW = WORLD_X1 - WORLD_X0;
	const stars = $derived(makeStars(seed));

	// Moonlit clouds: [x, y, width]; the first six come with a warning, all with a storm.
	const CLOUDS: [number, number, number][] = [
		[1480, 330, 260], [980, 260, 300], [1900, 250, 280], [560, 360, 240], [2300, 380, 260],
		[180, 250, 300], [1240, 420, 320], [1700, 470, 300], [760, 470, 280], [-200, 400, 320],
		[2150, 520, 300], [1350, 230, 360]
	];
	function cloudPath(x: number, y: number, w: number): string {
		const h = w * 0.16;
		return (
			`M${x - w / 2} ${y}` +
			`a${w * 0.2} ${h * 0.9} 0 0 1 ${w * 0.22} ${-h * 0.7}` +
			`a${w * 0.22} ${h * 1.3} 0 0 1 ${w * 0.34} ${-h * 0.5}` +
			`a${w * 0.2} ${h} 0 0 1 ${w * 0.3} ${h * 0.55}` +
			`a${w * 0.12} ${h * 0.7} 0 0 1 ${w * 0.14} ${h * 0.65}z`
		);
	}

	// The moon's path on the water: short strokes, narrower and fainter far off.
	const GLADE = (() => {
		let d = '';
		for (let i = 0; i < 18; i++) {
			const y = HORIZON + 8 + i * 12 + i * i * 0.5;
			const half = 10 + i * 5;
			const jitter = ((i * 37) % 11) - 5;
			d += `M${MOON.x - half + jitter} ${Math.round(y)}h${half * (0.7 + ((i * 13) % 5) / 10)}`;
			d += `M${MOON.x + half * 0.3 + jitter} ${Math.round(y + 4)}h${half * 0.6}`;
		}
		return d;
	})();

	const RAIN = (() => {
		let d = '';
		let s = 7;
		for (let i = 0; i < 160; i++) {
			s = (s * 1664525 + 1013904223) % 4294967296;
			const x = W0 + Math.round((s / 4294967296) * WW);
			s = (s * 1664525 + 1013904223) % 4294967296;
			const y = Math.round((s / 4294967296) * 1060);
			d += `M${x} ${y}l-7 24`;
		}
		return d;
	})();
</script>

<div
	class="night l{level}"
	class:p-down={pl.active === 'down'}
	class:p-up={pl.active === 'up'}
	class:p-ok={pl.active === 'resolved'}
	class:paused
	class:sparkling={sparkle}
	bind:clientWidth={boxW}
	bind:clientHeight={boxH}
	aria-hidden="true"
>
	<svg {viewBox} preserveAspectRatio="xMidYMax slice" fill="none">
		<defs>
			<linearGradient id="{uid}-sky" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2={HORIZON}>
				<stop offset="0" stop-color="#010207" />
				<stop offset=".5" stop-color="#03060f" />
				<stop offset=".86" stop-color="#080e1f" />
				<stop offset="1" stop-color="#10172d" />
			</linearGradient>
			<linearGradient id="{uid}-glow" gradientUnits="userSpaceOnUse" x1="0" y1="560" x2="0" y2={HORIZON}>
				<stop offset="0" stop-color={glow} stop-opacity="0" />
				<stop offset="1" stop-color={glow} stop-opacity=".42" />
			</linearGradient>
			<radialGradient id="{uid}-halo" gradientUnits="userSpaceOnUse" cx={MOON.x} cy={MOON.y} r="260">
				<stop offset="0" stop-color="#b9c8ef" stop-opacity=".34" />
				<stop offset=".18" stop-color="#8fa3d6" stop-opacity=".16" />
				<stop offset="1" stop-color="#5a6ea8" stop-opacity="0" />
			</radialGradient>
			<!-- Moonlight across the skyline: a soft wash, stronger near the moon. -->
			<radialGradient id="{uid}-wash" gradientUnits="userSpaceOnUse" cx={MOON.x} cy={MOON.y + 120} r="900">
				<stop offset="0" stop-color="#9db2e6" stop-opacity=".16" />
				<stop offset="1" stop-color="#9db2e6" stop-opacity="0" />
			</radialGradient>
			<linearGradient id="{uid}-water" gradientUnits="userSpaceOnUse" x1="0" y1={HORIZON} x2="0" y2={WORLD_BOTTOM}>
				<stop offset="0" stop-color="#0b1226" />
				<stop offset="1" stop-color="#02040a" />
			</linearGradient>
			<linearGradient id="{uid}-fadeout" gradientUnits="userSpaceOnUse" x1="0" y1={HORIZON} x2="0" y2={HORIZON + 210}>
				<stop offset="0" stop-color="#fff" stop-opacity=".9" />
				<stop offset="1" stop-color="#fff" stop-opacity="0" />
			</linearGradient>
			<mask id="{uid}-refl" maskUnits="userSpaceOnUse" x={W0} y={HORIZON} width={WW} height={WORLD_BOTTOM - HORIZON}>
				<rect x={W0} y={HORIZON} width={WW} height={WORLD_BOTTOM - HORIZON} fill="url(#{uid}-fadeout)" />
			</mask>
			<clipPath id="{uid}-below">
				<rect x={W0} y={HORIZON} width={WW} height={WORLD_BOTTOM - HORIZON} />
			</clipPath>
		</defs>

		<!-- sky, stars, moon -->
		<rect x={W0} y="0" width={WW} height={HORIZON} fill="url(#{uid}-sky)" />
		<g class="stars">
			<path d={stars.dots[0]} stroke-width="1.5" opacity=".5" />
			<path d={stars.dots[1]} stroke-width="2.2" opacity=".72" />
			<path d={stars.dots[2]} stroke-width="3" opacity=".92" />
			{#each stars.twinkle as s, i (i)}
				<circle class="tw" cx={s.x} cy={s.y} r={s.r} style="animation-delay:-{s.d}ms" />
			{/each}
		</g>
		<circle cx={MOON.x} cy={MOON.y} r="260" fill="url(#{uid}-halo)" />
		<g class="moon">
			<circle cx={MOON.x} cy={MOON.y} r={MOON.r} fill="#eef2fb" />
			<g fill="#c9d3e8">
				<circle cx={MOON.x - 9} cy={MOON.y - 7} r="7" opacity=".7" />
				<circle cx={MOON.x + 8} cy={MOON.y + 6} r="9" opacity=".55" />
				<circle cx={MOON.x - 4} cy={MOON.y + 14} r="4" opacity=".6" />
				<circle cx={MOON.x + 12} cy={MOON.y - 12} r="3.5" opacity=".5" />
			</g>
		</g>
		<rect class="skyveil" x={W0} y="0" width={WW} height={HORIZON} />

		<!-- weather in the sky: moonlit clouds -->
		<g class="clouds">
			{#each CLOUDS as [x, y, w], i (i)}
				<g class="cloud" class:storm-only={i >= 6} style="animation-delay:-{i * 4}s">
					<path d={cloudPath(x + 3, y - 4, w)} class="cloud-rim" />
					<path d={cloudPath(x, y, w)} class="cloud-body" />
				</g>
			{/each}
		</g>

		<!-- the city's light on the horizon -->
		<rect x={W0} y="560" width={WW} height={HORIZON - 560} fill="url(#{uid}-glow)" />

		<!-- the water, the reflection, the moon's path -->
		<rect x={W0} y={HORIZON} width={WW} height={WORLD_BOTTOM - HORIZON} fill="url(#{uid}-water)" />
		<g clip-path="url(#{uid}-below)">
			<g mask="url(#{uid}-refl)" opacity=".42">
				<use href="#{uid}-city" transform="matrix(1 0 0 -1 0 {HORIZON * 2})" />
			</g>
		</g>
		<path class="glade" d={GLADE} />

		<g id="{uid}-city">
			{@render children(uid)}
		</g>
		<rect x={W0} y="0" width={WW} height={HORIZON} fill="url(#{uid}-wash)" pointer-events="none" />

		{#if front}{@render front(uid)}{/if}

		<!-- the storm: rain over everything, a distant flash -->
		<path class="rain" d={RAIN} />
		<path class="bolt" d="M1820 300l-26 92 22-4-30 112 56-122-26 4z" />
		<rect class="flash" x={W0} y="0" width={WW} height={WORLD_BOTTOM} />
	</svg>
</div>

<style>
	.night {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: #02040a;
	}
	.night svg {
		display: block;
		width: 100%;
		height: 100%;
	}

	/* --- sky ------------------------------------------------------------- */
	.stars path {
		stroke: #dfe7ff;
		stroke-linecap: round;
	}
	.tw {
		fill: #f4f7ff;
		opacity: 0.85;
	}
	.skyveil {
		fill: #01030a;
		opacity: 0;
		transition: opacity 2s;
	}
	.l1 .skyveil {
		opacity: 0.3;
	}
	.l2 .skyveil {
		opacity: 0.62;
	}

	/* --- clouds: hidden when clear, lit from the moon's side --------------- */
	.cloud {
		opacity: 0;
		transition: opacity 2s;
	}
	.cloud-body {
		fill: #141b30;
	}
	.cloud-rim {
		fill: #5b6d96;
	}
	.l1 .cloud:not(.storm-only) {
		opacity: 0.92;
	}
	.l2 .cloud {
		opacity: 0.97;
	}
	.l2 .cloud-body {
		fill: #0c1222;
	}
	.l2 .cloud-rim {
		fill: #33415f;
	}

	/* --- water ------------------------------------------------------------ */
	.glade {
		stroke: #d5def5;
		stroke-width: 2.2;
		stroke-linecap: round;
		opacity: 0.5;
	}
	.l2 .glade {
		opacity: 0.12;
	}

	/* --- storm ------------------------------------------------------------ */
	.rain {
		stroke: #a9bbe2;
		stroke-width: 1.5;
		stroke-linecap: round;
		opacity: 0;
		transition: opacity 2s;
	}
	.l2 .rain {
		opacity: 0.32;
	}
	.bolt,
	.flash {
		opacity: 0;
	}
	.bolt {
		fill: #f1f4ff;
	}
	.flash {
		fill: #c9d6ff;
	}

	/* --- gestures: what a city marks .lights, .windows, .sparkle ---------- */
	.night :global(.sparkle) {
		opacity: 0;
	}

	@media (prefers-reduced-motion: no-preference) {
		.tw {
			animation: n-twinkle 5.5s ease-in-out infinite;
		}
		.cloud {
			animation: n-drift 60s ease-in-out infinite alternate;
		}
		.glade {
			animation: n-shimmer 4s ease-in-out infinite alternate;
		}
		.l2 .rain {
			animation: n-fall 0.75s linear infinite;
		}
		.l2 .flash {
			animation: n-flash 9s linear infinite;
		}
		.l2 .bolt {
			animation: n-bolt 9s linear infinite;
		}
		.p-down :global(.lights) {
			animation: n-out 6s ease-out both;
		}
		.p-down :global(.windows) {
			animation: n-dim 6s ease-out both;
		}
		.p-up :global(.lights) {
			animation: n-on 4.5s ease-out both;
		}
		.p-ok :global(.sparkle) {
			animation: n-sparkle 1.2s ease-in-out 5 both;
			animation-delay: var(--d, 0ms);
		}
		.sparkling :global(.sparkle) {
			animation: n-sparkle 1.2s ease-in-out infinite;
			animation-delay: var(--d, 0ms);
		}
		.paused,
		.paused :global(*) {
			animation-play-state: paused !important;
		}
	}

	@keyframes n-twinkle {
		0%, 100% { opacity: 0.85; }
		45% { opacity: 0.25; }
		55% { opacity: 0.95; }
	}
	@keyframes n-drift {
		to { transform: translateX(70px); }
	}
	@keyframes n-shimmer {
		from { opacity: 0.35; transform: translateX(-4px); }
		to { opacity: 0.55; transform: translateX(4px); }
	}
	@keyframes n-fall {
		to { transform: translate(-14px, 48px); }
	}
	/* One distant flash every 9 s: far below three per second. */
	@keyframes n-flash {
		0%, 90%, 100% { opacity: 0; }
		91% { opacity: 0.28; }
		92% { opacity: 0.04; }
		93% { opacity: 0.2; }
		95% { opacity: 0; }
	}
	@keyframes n-bolt {
		0%, 90.5%, 93.5%, 100% { opacity: 0; }
		91%, 93% { opacity: 0.9; }
	}
	@keyframes n-out {
		0% { opacity: 1; }
		8% { opacity: 0.15; }
		13% { opacity: 0.8; }
		20%, 84% { opacity: 0; }
		100% { opacity: 1; }
	}
	@keyframes n-dim {
		0%, 100% { opacity: 1; }
		20%, 84% { opacity: 0.4; }
	}
	@keyframes n-on {
		0% { opacity: 0; }
		25% { opacity: 1; }
		36% { opacity: 0.25; }
		55%, 100% { opacity: 1; }
	}
	@keyframes n-sparkle {
		0%, 100% { opacity: 0; }
		45% { opacity: 1; }
		60% { opacity: 0.2; }
	}
</style>
