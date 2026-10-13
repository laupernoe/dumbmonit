<script lang="ts">
	/**
	 * The desert at night — a camel caravan crossing a far dune, a tent and a
	 * campfire in front, a sky full of stars with the Milky Way across it.
	 * Follows the scene contract: fills its positioned parent, the headline
	 * zone (top left) is left clear, the palette lives here.
	 *
	 * `mood` is the weather: ok = clear night, warn = haze (stars dim behind a
	 * veil of dust), critical = sandstorm (a thick veil, gusts of sand).
	 * `pulse` replays a gesture whenever `at` changes: down = the campfire dies
	 * to embers, up = it flares back with a spray of sparks, resolved = a burst
	 * of shooting stars. Reduced motion keeps the scene still.
	 */
	import { reducedMotion } from '#lib/ui/index.js';

	interface Props {
		mood?: 'ok' | 'warn' | 'critical';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}

	let { mood = 'ok', pulse = null }: Props = $props();

	const animated = !reducedMotion();

	// A fixed sky: the same stars on every visit.
	function rng(seed: number) {
		let s = seed;
		return () => {
			s = (s * 1664525 + 1013904223) % 4294967296;
			return s / 4294967296;
		};
	}
	const rand = rng(7);
	const stars = Array.from({ length: 70 }, (_, i) => ({
		x: Math.round(rand() * 1220),
		y: Math.round(rand() * 360),
		r: +(0.8 + rand() * 1.5).toFixed(1),
		twinkle: i < 16,
		delay: +(rand() * 4).toFixed(1)
	}));
	// Denser, finer dust along the band of the Milky Way (rotated -24 deg).
	const dust = Array.from({ length: 90 }, () => {
		const u = (rand() + rand() + rand() - 1.5) * 760;
		const v = (rand() + rand() - 1) * 70;
		const a = (-24 * Math.PI) / 180;
		return {
			x: Math.round(640 + u * Math.cos(a) - v * Math.sin(a)),
			y: Math.round(170 + u * Math.sin(a) + v * Math.cos(a)),
			r: +(0.5 + rand() * 0.9).toFixed(1)
		};
	});
	const gusts = Array.from({ length: 16 }, (_, i) => ({
		y: 120 + Math.round(rand() * 400),
		w: 80 + Math.round(rand() * 180),
		dur: +(1.6 + rand() * 1.8).toFixed(1),
		delay: +(-rand() * 3).toFixed(1),
		o: i
	}));
	const shooters = [
		{ x: 520, y: 70, d: 0 },
		{ x: 760, y: 40, d: 0.35 },
		{ x: 380, y: 150, d: 0.7 },
		{ x: 960, y: 90, d: 1.0 },
		{ x: 640, y: 120, d: 1.4 },
		{ x: 860, y: 20, d: 1.8 }
	];
	const sparks = [-14, -4, 6, 16, 26, -22, 10];
</script>

<div
	class="scene scene-desert"
	class:m-warn={mood === 'warn'}
	class:m-critical={mood === 'critical'}
	aria-hidden="true"
>
	<svg viewBox="0 0 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<radialGradient id="ds-mw" cx=".5" cy=".5" r=".5">
				<stop offset="0" stop-color="var(--mw)" stop-opacity=".5" />
				<stop offset=".55" stop-color="var(--mw)" stop-opacity=".18" />
				<stop offset="1" stop-color="var(--mw)" stop-opacity="0" />
			</radialGradient>
			<radialGradient id="ds-glow" cx=".5" cy=".5" r=".5">
				<stop offset="0" stop-color="var(--fire-glow)" stop-opacity=".75" />
				<stop offset="1" stop-color="var(--fire-glow)" stop-opacity="0" />
			</radialGradient>
			<linearGradient id="ds-veil" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--veil)" stop-opacity=".55" />
				<stop offset="1" stop-color="var(--veil)" stop-opacity="1" />
			</linearGradient>
			<g id="ds-camel">
				<g stroke="var(--camel)" stroke-width="3" stroke-linecap="round"><path d="M-14 -22V0M-6 -22V0M16 -22V0M24 -22V0" /></g>
				<path d="M-22 -24C-22 -34 -14 -34 -10 -40C-6 -46 0 -44 2 -38C6 -44 12 -44 14 -36C18 -30 26 -30 28 -24C28 -20 -22 -18 -22 -24zM24 -30C30 -40 32 -48 36 -54C38 -58 44 -60 48 -57L56 -54L54 -50L44 -50C42 -44 40 -38 34 -28z" fill="var(--camel)" />
			</g>
		</defs>

		<!-- sky: Milky Way and stars -->
		<g class="sky">
			<g transform="rotate(-24 640 170)">
				<ellipse cx="640" cy="170" rx="720" ry="62" fill="url(#ds-mw)" />
				<ellipse cx="560" cy="178" rx="380" ry="12" fill="var(--lane)" opacity=".35" />
			</g>
			<g fill="var(--star)" opacity=".7">
				{#each dust as d, i (i)}<circle cx={d.x} cy={d.y} r={d.r} />{/each}
			</g>
			<g fill="var(--star)">
				{#each stars as s, i (i)}
					<circle class:tw={s.twinkle} cx={s.x} cy={s.y} r={s.r} style:animation-delay="{s.delay}s" />
				{/each}
			</g>
			<g fill="var(--star)" opacity=".9"><path d="M1090 70l3 9 9 3-9 3-3 9-3-9-9-3 9-3z" /><path d="M300 110l2 6 6 2-6 2-2 6-2-6-6-2 6-2z" /></g>
		</g>

		<!-- shooting stars: the incident is resolved -->
		{#if animated && pulse?.kind === 'resolved'}
			{#key pulse.at}
				<g class="shooters">
					{#each shooters as s (s.x)}
						<g transform="translate({s.x} {s.y})">
							<g class="ss-run" style="--d:{s.d}s">
								<path d="M0 0L-96 -34" stroke="var(--star)" stroke-width="2" stroke-linecap="round" opacity=".55" />
								<path d="M0 0L-40 -14" stroke="var(--star)" stroke-width="3" stroke-linecap="round" />
								<circle r="3.4" fill="#fff" />
							</g>
						</g>
					{/each}
				</g>
			{/key}
		{/if}

		<!-- far dunes -->
		<path d="M0 408C140 380 300 378 460 404C600 384 760 384 900 404C1020 388 1140 388 1220 402V430H0z" fill="var(--dune-far)" />
		<!-- caravan on the far ridge -->
		<g class="caravan">
			<g transform="translate(560 398) scale(.5)"><use href="#ds-camel" /></g>
			<g transform="translate(606 396) scale(.5)"><use href="#ds-camel" /></g>
			<g transform="translate(652 398) scale(.5)"><use href="#ds-camel" /></g>
			<g transform="translate(698 400) scale(.5)"><use href="#ds-camel" /></g>
		</g>

		<!-- ground -->
		<rect x="0" y="404" width="1220" height="140" fill="var(--sand1)" />
		<path d="M0 430C160 410 340 414 520 438C700 416 900 420 1060 440C1140 430 1190 430 1220 436V540H0z" fill="var(--sand2)" />
		<path d="M0 492C240 466 520 470 780 496C960 478 1100 480 1220 492V540H0z" fill="var(--sand3)" />
		<g stroke="var(--ripple)" stroke-width="2" stroke-linecap="round" opacity=".5"><path d="M120 456h80M300 470h100M520 452h70M180 512h120M560 520h100M720 506h80" /></g>

		<!-- tent -->
		<g transform="translate(900 486)">
			<path d="M-96 0L0 -84L96 0z" fill="var(--tent)" />
			<path d="M0 -84L96 0H30z" fill="var(--tent-shade)" />
			<path d="M-24 0L0 -50L24 0z" fill="var(--door)" />
			<path d="M0 -84V-98" stroke="var(--tent-shade)" stroke-width="3" stroke-linecap="round" />
			<path d="M0 -98l16 5-16 5z" fill="var(--flag)" />
			<path d="M-96 0L-122 8M96 0L122 8" stroke="var(--tent-shade)" stroke-width="2" stroke-linecap="round" />
		</g>

		<!-- campfire -->
		<g transform="translate(1030 490)">
			{#key pulse?.at ?? 0}
				<g
					class="fire"
					class:f-down={animated && pulse?.kind === 'down'}
					class:f-up={animated && pulse?.kind === 'up'}
				>
					<circle class="glow" cx="0" cy="-26" r="150" fill="url(#ds-glow)" />
					<g class="flame">
						<path d="M0 -70C18 -44 30 -26 24 -8C20 4 -20 4 -24 -8C-30 -26 -14 -40 0 -70z" fill="var(--flame1)" />
						<path d="M0 -52C12 -34 18 -20 14 -8C10 0 -10 0 -14 -8C-18 -20 -8 -32 0 -52z" fill="var(--flame2)" />
						<path d="M0 -30C6 -20 8 -12 5 -6C3 -2 -3 -2 -5 -6C-8 -12 -4 -20 0 -30z" fill="var(--flame3)" />
					</g>
					{#if animated && pulse?.kind === 'up'}
						<g fill="var(--flame3)">
							{#each sparks as dx, i (i)}
								<circle class="spark" cx={dx} cy="-40" r="2.2" style="--dx:{dx * 1.6}px;--d:{i * 0.12}s" />
							{/each}
						</g>
					{/if}
				</g>
			{/key}
			<path d="M-30 4L26 -10M30 4L-26 -10" stroke="var(--log)" stroke-width="9" stroke-linecap="round" />
			<g fill="var(--ember)" class="embers"><circle cx="-8" cy="-6" r="3" /><circle cx="8" cy="-8" r="2.6" /><circle cx="0" cy="-4" r="3.4" /></g>
			<g fill="var(--stone)"><ellipse cx="-44" cy="6" rx="10" ry="6" /><ellipse cx="-24" cy="12" rx="9" ry="5" /><ellipse cx="22" cy="12" rx="9" ry="5" /><ellipse cx="44" cy="6" rx="10" ry="6" /></g>
		</g>

		<!-- weather: haze and sandstorm -->
		<g class="weather">
			<rect class="veil" x="0" y="0" width="1220" height="540" fill="url(#ds-veil)" />
			<g class="gusts" stroke="var(--veil-streak)" stroke-width="3" stroke-linecap="round">
				{#each gusts as g (g.o)}
					<path d="M1300 {g.y}h{g.w}" style="--dur:{g.dur}s;animation-delay:{g.delay}s" />
				{/each}
			</g>
		</g>
	</svg>
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(
			180deg,
			var(--sky1),
			var(--sky2) 73.5%,
			var(--sand1) 74.4%,
			var(--sand1) 80%,
			var(--sand3)
		);
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
	}
	.scene-desert {
		--sky1: #b6aed4;
		--sky2: #f4cfb2;
		--star: #fff8e8;
		--mw: #ffffff;
		--lane: #8f84b0;
		--dune-far: #d6a07a;
		--sand1: #dcae85;
		--sand2: #c99470;
		--sand3: #a9755a;
		--ripple: #8e5f4a;
		--camel: #5a3e30;
		--tent: #9a6c52;
		--tent-shade: #6f4a3a;
		--door: #f6b866;
		--flag: #c8553d;
		--log: #4a3020;
		--stone: #8a7a72;
		--ember: #e8643a;
		--fire-glow: #ff9a3a;
		--flame1: #f2701c;
		--flame2: #ffb43a;
		--flame3: #fff0b0;
		--veil: #e6c08c;
		--veil-streak: #f4dcb0;
		--veil-o: 0;
		--starfade: 1;
	}
	:global(.dark) .scene-desert {
		--sky1: #04071a;
		--sky2: #2a2447;
		--star: #eef2ff;
		--mw: #cfd8ff;
		--lane: #04071a;
		--dune-far: #2a2444;
		--sand1: #2b2543;
		--sand2: #211c36;
		--sand3: #151226;
		--ripple: #3d3659;
		--camel: #09070f;
		--tent: #3c3455;
		--tent-shade: #262038;
		--door: #ffb860;
		--flag: #b2432f;
		--log: #1a110a;
		--stone: #4a4560;
		--veil: #5a4636;
		--veil-streak: #a08462;
	}
	.sky {
		opacity: var(--starfade);
		transition: opacity 1.2s;
	}
	.m-warn {
		--veil-o: 0.3;
		--starfade: 0.55;
	}
	.m-critical {
		--veil-o: 0.7;
		--starfade: 0.15;
	}
	.veil {
		opacity: var(--veil-o);
		transition: opacity 1.2s;
	}
	.gusts {
		opacity: 0;
		transition: opacity 1.2s;
	}
	.m-critical .gusts {
		opacity: 0.55;
	}
	.gusts path {
		animation: gust var(--dur) linear infinite;
	}
	.tw {
		animation: twinkle 3.6s ease-in-out infinite;
	}
	.flame {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: flick 1.3s ease-in-out infinite alternate;
	}
	.glow {
		transform-box: fill-box;
		transform-origin: 50% 50%;
		animation: breathe 2.4s ease-in-out infinite alternate;
	}
	.embers {
		opacity: 0.8;
	}
	.f-down .flame {
		animation: fire-down 7s ease-in-out both;
	}
	.f-down .glow {
		animation: glow-down 7s ease-in-out both;
	}
	.f-up .flame {
		animation:
			fire-up 3.2s ease-out both,
			flick 1.3s ease-in-out 3.2s infinite alternate;
	}
	.f-up .glow {
		animation: glow-up 3.2s ease-out both;
	}
	.spark {
		animation: spark 2s ease-out var(--d) both;
	}
	.ss-run {
		opacity: 0;
		animation: shoot 1.4s ease-in var(--d) both;
	}

	@keyframes twinkle {
		50% {
			opacity: 0.25;
		}
	}
	@keyframes flick {
		from {
			transform: scale(1, 0.94) skewX(-2deg);
		}
		to {
			transform: scale(0.96, 1.06) skewX(2deg);
		}
	}
	@keyframes breathe {
		from {
			opacity: 0.85;
			transform: scale(0.96);
		}
		to {
			opacity: 1;
			transform: scale(1.04);
		}
	}
	@keyframes fire-down {
		0% {
			transform: scale(1);
		}
		18%,
		78% {
			transform: scale(0.1, 0.06);
		}
		100% {
			transform: scale(1);
		}
	}
	@keyframes glow-down {
		0% {
			opacity: 1;
		}
		18%,
		78% {
			opacity: 0.14;
			transform: scale(0.45);
		}
		100% {
			opacity: 1;
			transform: scale(1);
		}
	}
	@keyframes fire-up {
		0% {
			transform: scale(0.1, 0.06);
		}
		35% {
			transform: scale(1.5, 1.9);
		}
		100% {
			transform: scale(1);
		}
	}
	@keyframes glow-up {
		0% {
			opacity: 0.15;
			transform: scale(0.45);
		}
		35% {
			opacity: 1;
			transform: scale(1.5);
		}
		100% {
			opacity: 1;
			transform: scale(1);
		}
	}
	@keyframes spark {
		0% {
			opacity: 1;
			transform: translate(0, 0);
		}
		100% {
			opacity: 0;
			transform: translate(var(--dx), -150px);
		}
	}
	@keyframes shoot {
		0% {
			opacity: 0;
			transform: translate(-60px, -20px);
		}
		15% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(300px, 105px);
		}
	}
	@keyframes gust {
		to {
			transform: translateX(-1700px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.tw,
		.flame,
		.glow,
		.gusts path {
			animation: none;
		}
	}
</style>
