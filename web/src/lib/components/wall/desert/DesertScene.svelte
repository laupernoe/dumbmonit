<script lang="ts">
	/**
	 * The wall's backdrop: the desert at night. A camel caravan crosses a far
	 * dune, a tent and a campfire sit in the foreground, the sky is full of
	 * stars with the Milky Way across it. A drop-in for the Rome scene (same
	 * props, plus `pulse`).
	 *
	 * Drawn on a 1920 x 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks the part kept in view on a narrow screen). It is night at
	 * every hour; the wall theme sets the brightness band: Day a pale twilight
	 * (navy ink reads on it), Night a deep blue sky, OLED black with faint land.
	 *
	 * The network is the weather. Calm: a clear sky. Advisories (`warn` or
	 * `clouded`): a haze of dust dims the stars. A storm (`critical` or
	 * `storm`): a sandstorm, with a thick veil and gusts of sand. Problems on
	 * the board make the fire sit a little lower; a new one (`startle`) makes it
	 * flinch once.
	 *
	 * `pulse` replays a gesture whenever its `at` changes: `down` the fire dies
	 * to embers, `up` it flares back with a spray of sparks, `resolved` a burst
	 * of shooting stars. Calmly, one shooting star crosses every 35-75 s in the
	 * dark themes. All motion is transform and opacity; hidden tabs pause it and
	 * reduced motion freezes the scene in a still pose.
	 */
	import { reducedMotion } from '#lib/ui/index.js';
	import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

	type Mood = 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';

	interface Props {
		theme: SceneTheme;
		now?: Date;
		mood?: Mood;
		problems?: number;
		/** Bumped by the wall when a new problem appears: the fire flinches once. */
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
		mood = 'ok',
		problems = 0,
		startle = 0,
		paused = false,
		focus = 0,
		pulse = null,
		class: className = ''
	}: Props = $props();

	const motion = !reducedMotion();
	const STAGE_W = 1920;
	const STAGE_H = 1080;

	// --- Stage fit ----------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Weather ------------------------------------------------------------------

	/** 0 clear, 1 haze, 2 sandstorm. */
	const level = $derived(
		mood === 'warn' || mood === 'clouded' ? 1 : mood === 'critical' || mood === 'storm' ? 2 : 0
	);
	const dark = $derived(theme !== 'light');

	// --- Palette (inline custom properties) -----------------------------------------

	const PALETTES: Record<SceneTheme, Record<string, string>> = {
		light: {
			'--sky-top': '#aeb0d6',
			'--sky-bottom': '#f4cdb0',
			'--star': '#fff8e8',
			'--mw': '#ffffff',
			'--lane': '#8f84b0',
			'--far': '#d6a07a',
			'--mid': '#c99470',
			'--near': '#a9755a',
			'--ripple': '#8e5f4a',
			'--camel': '#5a3e30',
			'--tent': '#9a6c52',
			'--tent-shade': '#6f4a3a',
			'--door': '#f6b866',
			'--flag': '#c8553d',
			'--log': '#4a3020',
			'--stone': '#8a7a72',
			'--figure': '#3b2a26',
			'--veil': '#b98552',
			'--veil-streak': '#f0cf9c'
		},
		dark: {
			'--sky-top': '#03061a',
			'--sky-bottom': '#2a2447',
			'--star': '#eef2ff',
			'--mw': '#cfd8ff',
			'--lane': '#03061a',
			'--far': '#2a2444',
			'--mid': '#211c36',
			'--near': '#151226',
			'--ripple': '#3d3659',
			'--camel': '#09070f',
			'--tent': '#3c3455',
			'--tent-shade': '#262038',
			'--door': '#ffb860',
			'--flag': '#b2432f',
			'--log': '#1a110a',
			'--stone': '#4a4560',
			'--figure': '#07050d',
			'--veil': '#4a3a30',
			'--veil-streak': '#8e7656'
		},
		oled: {
			'--sky-top': '#000000',
			'--sky-bottom': '#05060e',
			'--star': '#e4e9fa',
			'--mw': '#b8c4f0',
			'--lane': '#000000',
			'--far': '#0b0a14',
			'--mid': '#08070f',
			'--near': '#050409',
			'--ripple': '#1a1727',
			'--camel': '#000000',
			'--tent': '#1b1728',
			'--tent-shade': '#0f0c18',
			'--door': '#ffa84c',
			'--flag': '#8c3626',
			'--log': '#0c0705',
			'--stone': '#252233',
			'--figure': '#000000',
			'--veil': '#2a2018',
			'--veil-streak': '#5c4a34'
		}
	};
	const FIRE = {
		'--fire-glow': '#ff9a3a',
		'--flame1': '#f2701c',
		'--flame2': '#ffb43a',
		'--flame3': '#fff0b0',
		'--ember': '#e8643a'
	};
	const style = $derived(
		Object.entries({ ...PALETTES[theme], ...FIRE })
			.map(([key, value]) => `${key}:${value}`)
			.join(';')
	);

	// --- The sky --------------------------------------------------------------------

	function rng(seed: number) {
		let s = seed;
		return () => {
			s = (s * 1664525 + 1013904223) % 4294967296;
			return s / 4294967296;
		};
	}
	const rand = rng(11);
	const STARS = Array.from({ length: 230 }, (_, i) => ({
		x: Math.round(rand() * STAGE_W),
		y: Math.round(rand() * 700),
		r: +(0.8 + rand() * 1.9).toFixed(1),
		twinkle: i < 40,
		delay: +(rand() * 5).toFixed(1)
	}));
	// Fine dust along the band of the Milky Way, which runs from lower left to upper right.
	const DUST = Array.from({ length: 260 }, () => {
		const u = (rand() + rand() + rand() - 1.5) * 1400;
		const v = (rand() + rand() - 1) * 150;
		const a = (-22 * Math.PI) / 180;
		return {
			x: Math.round(1000 + u * Math.cos(a) - v * Math.sin(a)),
			y: Math.round(330 + u * Math.sin(a) + v * Math.cos(a)),
			r: +(0.5 + rand() * 1.1).toFixed(1)
		};
	});
	const GUSTS = Array.from({ length: 34 }, (_, i) => ({
		id: i,
		y: 200 + Math.round(rand() * 860),
		w: 120 + Math.round(rand() * 260),
		dur: +(1.8 + rand() * 2.2).toFixed(1),
		delay: +(-rand() * 4).toFixed(1)
	}));
	const SHOOTERS = [
		{ x: 760, y: 90, d: 0 },
		{ x: 1180, y: 50, d: 0.3 },
		{ x: 520, y: 260, d: 0.6 },
		{ x: 1500, y: 150, d: 0.9 },
		{ x: 940, y: 220, d: 1.3 },
		{ x: 1320, y: 330, d: 1.6 },
		{ x: 300, y: 120, d: 1.9 },
		{ x: 1700, y: 60, d: 2.2 }
	];
	const SPARKS = [-20, -8, 6, 18, 30, -30, 12, -2, 24];
	const CAMELS = [0, 1, 2, 3, 4].map((i) => ({ x: i * 112, y: i % 2 ? 2 : 0, delay: -i * 0.35 }));

	// A calm shooting star now and then, in the dark themes.
	let ambient = $state(0);
	$effect(() => {
		if (!motion || paused || !dark) return;
		let timer: ReturnType<typeof setTimeout>;
		const schedule = () => {
			timer = setTimeout(
				() => {
					ambient += 1;
					schedule();
				},
				35_000 + Math.random() * 40_000
			);
		};
		schedule();
		return () => clearTimeout(timer);
	});
	const ambientAt = $derived(ambient);
	const ambientX = $derived(300 + ((ambient * 397) % 1100));
	const ambientY = $derived(60 + ((ambient * 131) % 240));

	const gesture = $derived(motion && pulse ? pulse : null);
	const lowFire = $derived(problems > 0);
</script>

<div
	class="scene {className}"
	class:scene--paused={paused}
	class:scene--still={!motion}
	class:lvl-1={level === 1}
	class:lvl-2={level === 2}
	{style}
	bind:clientWidth={boxW}
	bind:clientHeight={boxH}
	aria-hidden="true"
>
	{#if scale > 0}
		<div class="stage" style:transform="translate({offsetX}px, {offsetY}px) scale({scale})">
			<div class="sky"></div>
			<svg viewBox="0 0 {STAGE_W} {STAGE_H}" fill="none" focusable="false">
				<defs>
					<radialGradient id="wd-mw" cx=".5" cy=".5" r=".5">
						<stop offset="0" stop-color="var(--mw)" stop-opacity=".48" />
						<stop offset=".55" stop-color="var(--mw)" stop-opacity=".16" />
						<stop offset="1" stop-color="var(--mw)" stop-opacity="0" />
					</radialGradient>
					<radialGradient id="wd-glow" cx=".5" cy=".5" r=".5">
						<stop offset="0" stop-color="var(--fire-glow)" stop-opacity=".7" />
						<stop offset="1" stop-color="var(--fire-glow)" stop-opacity="0" />
					</radialGradient>
					<linearGradient id="wd-veil" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--veil)" stop-opacity=".75" />
						<stop offset="1" stop-color="var(--veil)" stop-opacity="1" />
					</linearGradient>
					<g id="wd-camel">
						<g stroke="var(--camel)" stroke-width="3" stroke-linecap="round"><path d="M-14 -22V0M-6 -22V0M16 -22V0M24 -22V0" /></g>
						<path d="M-22 -24C-22 -34 -14 -34 -10 -40C-6 -46 0 -44 2 -38C6 -44 12 -44 14 -36C18 -30 26 -30 28 -24C28 -20 -22 -18 -22 -24zM24 -30C30 -40 32 -48 36 -54C38 -58 44 -60 48 -57L56 -54L54 -50L44 -50C42 -44 40 -38 34 -28z" fill="var(--camel)" />
					</g>
				</defs>

				<!-- stars and the Milky Way -->
				<g class="stars">
					<g transform="rotate(-22 1000 330)">
						<ellipse cx="1000" cy="330" rx="1300" ry="120" fill="url(#wd-mw)" />
						<ellipse cx="880" cy="344" rx="700" ry="22" fill="var(--lane)" opacity=".35" />
					</g>
					<g fill="var(--star)" opacity=".7">
						{#each DUST as d, i (i)}<circle cx={d.x} cy={d.y} r={d.r} />{/each}
					</g>
					<g fill="var(--star)">
						{#each STARS as s, i (i)}
							<circle class:tw={s.twinkle} cx={s.x} cy={s.y} r={s.r} style:animation-delay="{s.delay}s" />
						{/each}
					</g>
				</g>

				<!-- shooting stars: now and then, and when an incident resolves -->
				{#if dark && motion && ambientAt > 0}
					{#key ambientAt}
						<g transform="translate({ambientX} {ambientY})">
							<g class="ss-run" style="--d:0s">
								<path d="M0 0L-170 -60" stroke="var(--star)" stroke-width="2.5" stroke-linecap="round" opacity=".5" />
								<path d="M0 0L-70 -25" stroke="var(--star)" stroke-width="4" stroke-linecap="round" />
								<circle r="5" fill="#fff" />
							</g>
						</g>
					{/key}
				{/if}
				{#if gesture?.kind === 'resolved'}
					{#key gesture.at}
						{#each SHOOTERS as s (s.x)}
							<g transform="translate({s.x} {s.y})">
								<g class="ss-run" style="--d:{s.d}s">
									<path d="M0 0L-170 -60" stroke="var(--star)" stroke-width="2.5" stroke-linecap="round" opacity=".5" />
									<path d="M0 0L-70 -25" stroke="var(--star)" stroke-width="4" stroke-linecap="round" />
									<circle r="5" fill="#fff" />
								</g>
							</g>
						{/each}
					{/key}
				{/if}

				<!-- far dune and the caravan -->
				<path d="M0 770C300 740 600 744 900 768C1200 746 1600 742 1920 764V1080H0z" fill="var(--far)" />
				<g class="caravan">
					<g class="walk">
						{#each CAMELS as c, i (i)}
							<g transform="translate({c.x} {c.y})">
								<g class="bob" style:animation-delay="{c.delay}s">
									<g transform="translate(0 776) scale(.8)"><use href="#wd-camel" /></g>
								</g>
							</g>
						{/each}
						<path d="M0 744Q56 736 112 744T224 744T336 744T448 744" stroke="var(--camel)" stroke-width="1.5" opacity=".6" />
					</g>
				</g>

				<!-- mid and near dunes -->
				<path d="M0 840C400 800 800 810 1200 846C1500 820 1750 826 1920 840V1080H0z" fill="var(--mid)" />
				<path d="M0 960C500 920 1000 930 1400 972C1650 950 1800 952 1920 962V1080H0z" fill="var(--near)" />
				<g stroke="var(--ripple)" stroke-width="3" stroke-linecap="round" opacity=".5">
					<path d="M200 880h140M560 905h180M900 870h120M1560 890h160M300 1020h200M820 1040h160M1100 1015h140" />
				</g>

				<!-- tent -->
				<g transform="translate(1210 955) scale(1.5)">
					<path d="M-96 0L0 -84L96 0z" fill="var(--tent)" />
					<path d="M0 -84L96 0H30z" fill="var(--tent-shade)" />
					<path d="M-24 0L0 -50L24 0z" fill="var(--door)" />
					<path d="M0 -84V-98" stroke="var(--tent-shade)" stroke-width="3" stroke-linecap="round" />
					<path class="flag" d="M0 -98l16 5-16 5z" fill="var(--flag)" />
					<path d="M-96 0L-122 8M96 0L122 8" stroke="var(--tent-shade)" stroke-width="2" stroke-linecap="round" />
				</g>

				<!-- the traveller by the fire -->
				<g fill="var(--figure)" transform="translate(1568 972)">
					<circle cx="0" cy="-52" r="10" />
					<path d="M-16 0C-18 -22 -12 -40 0 -40C12 -40 18 -22 16 0z" />
				</g>

				<!-- campfire -->
				<g transform="translate(1430 972) scale(1.6)">
					{#key gesture?.at ?? 0}
						<g
							class="fire"
							class:f-low={lowFire}
							class:f-down={gesture?.kind === 'down'}
							class:f-up={gesture?.kind === 'up'}
						>
							{#key startle}
								<g class="flinch" class:flinch--on={startle > 0 && motion}>
									<ellipse class="glow" cx="0" cy="2" rx="190" ry="50" fill="url(#wd-glow)" />
									<circle class="glow" cx="0" cy="-26" r="150" fill="url(#wd-glow)" />
									<g class="flame">
										<path d="M0 -70C18 -44 30 -26 24 -8C20 4 -20 4 -24 -8C-30 -26 -14 -40 0 -70z" fill="var(--flame1)" />
										<path d="M0 -52C12 -34 18 -20 14 -8C10 0 -10 0 -14 -8C-18 -20 -8 -32 0 -52z" fill="var(--flame2)" />
										<path d="M0 -30C6 -20 8 -12 5 -6C3 -2 -3 -2 -5 -6C-8 -12 -4 -20 0 -30z" fill="var(--flame3)" />
									</g>
								</g>
							{/key}
							<g fill="var(--flame3)">
								{#each SPARKS as dx, i (i)}
									<circle class="spark" class:spark--burst={gesture?.kind === 'up'} cx={dx} cy="-40" r="2.2" style="--dx:{dx * 1.6}px;--d:{i * 0.9}s" />
								{/each}
							</g>
						</g>
					{/key}
					<path d="M-30 4L26 -10M30 4L-26 -10" stroke="var(--log)" stroke-width="9" stroke-linecap="round" />
					<g fill="var(--ember)" class="embers"><circle cx="-8" cy="-6" r="3" /><circle cx="8" cy="-8" r="2.6" /><circle cx="0" cy="-4" r="3.4" /></g>
					<g fill="var(--stone)"><ellipse cx="-44" cy="6" rx="10" ry="6" /><ellipse cx="-24" cy="12" rx="9" ry="5" /><ellipse cx="22" cy="12" rx="9" ry="5" /><ellipse cx="44" cy="6" rx="10" ry="6" /></g>
				</g>

				<!-- weather: haze and sandstorm -->
				<rect class="veil" width={STAGE_W} height={STAGE_H} fill="url(#wd-veil)" />
				<g class="gusts" stroke="var(--veil-streak)" stroke-width="6" stroke-linecap="round">
					{#each GUSTS as g (g.id)}
						<path d="M2100 {g.y}h{g.w}" style="--dur:{g.dur}s;animation-delay:{g.delay}s" />
					{/each}
				</g>
			</svg>
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
		background: var(--sky-bottom);
		--veil-o: 0;
		--starfade: 1;
	}
	.lvl-1 {
		--veil-o: 0.3;
		--starfade: 0.55;
	}
	.lvl-2 {
		--veil-o: 0.88;
		--starfade: 0.1;
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
		background: linear-gradient(to bottom, var(--sky-top) 0%, var(--sky-bottom) 72%);
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}
	.stars {
		opacity: var(--starfade);
		transition: opacity 1.5s;
	}
	.veil {
		opacity: var(--veil-o);
		transition: opacity 1.5s;
	}
	.gusts {
		opacity: 0;
		transition: opacity 1.5s;
	}
	.lvl-2 .gusts {
		opacity: 0.85;
	}
	.gusts path {
		animation: gust var(--dur) linear infinite;
	}
	.tw {
		animation: twinkle 3.8s ease-in-out infinite;
	}

	/* the caravan: a slow crossing of the far dune, then round again */
	.walk {
		animation: cross 320s linear infinite;
	}
	.bob {
		animation: bob 1.8s ease-in-out infinite;
	}
	.flag {
		transform-box: fill-box;
		transform-origin: 0 50%;
		animation: wave 2.6s ease-in-out infinite alternate;
	}

	/* the fire */
	.flame {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: flick 1.3s ease-in-out infinite alternate;
	}
	.lvl-2 .flame {
		animation-duration: 0.7s;
	}
	.glow {
		transform-box: fill-box;
		transform-origin: 50% 70%;
		animation: breathe 2.4s ease-in-out infinite alternate;
	}
	.f-low .flame,
	.f-low .glow {
		scale: 0.85;
	}
	.embers {
		opacity: 0.8;
	}
	.spark {
		opacity: 0;
		animation: spark 5s ease-out var(--d) infinite;
	}
	.fire:not(.f-up) .spark:nth-child(n + 4) {
		display: none;
	}
	.f-down .spark {
		display: none;
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
	.spark--burst {
		animation: spark 2s ease-out calc(var(--d) / 7) both;
	}
	.flinch--on .flame {
		animation: flinch 0.9s ease-out 1, flick 1.3s ease-in-out 0.9s infinite alternate;
	}
	.ss-run {
		opacity: 0;
		animation: shoot 1.5s ease-in var(--d) both;
	}

	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	.scene--still :global(*) {
		animation: none !important;
	}

	@keyframes twinkle {
		50% {
			opacity: 0.25;
		}
	}
	@keyframes cross {
		from {
			transform: translateX(-560px);
		}
		to {
			transform: translateX(2000px);
		}
	}
	@keyframes bob {
		50% {
			transform: translateY(-3px);
		}
	}
	@keyframes wave {
		to {
			transform: skewY(8deg);
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
	@keyframes flinch {
		30% {
			transform: scale(0.7, 0.55);
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
			transform: translate(-100px, -35px);
		}
		15% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(520px, 182px);
		}
	}
	@keyframes gust {
		to {
			transform: translateX(-2500px);
		}
	}
</style>
