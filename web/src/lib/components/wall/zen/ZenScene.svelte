<script lang="ts">
	/**
	 * The wall's living backdrop: a zen garden — raked sand rippling round
	 * stones, a koi pond with lily pads, a shishi-odoshi, a maple, a bamboo grove
	 * and a stone lantern. A drop-in for the Paris scene (same props, plus `pulse`).
	 *
	 * Drawn on a 1920 x 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky follows
	 * the real hour inside the brightness band of the wall's theme (see
	 * `paris/daylight.ts`); the lantern is lit after dark.
	 *
	 * The network's state is the weather: clouds gather with advisories, a storm
	 * brings rain, lightning and a darker sky; the koi grow shy while problems
	 * stand. A new problem (`startle`) or a `down` pulse sends the koi diving, `up`
	 * has one leap, `resolved` tips the shishi-odoshi with a clack.
	 *
	 * Motion is slow: koi, water dripping into the bamboo tube that tips now and
	 * then, leaves falling, bamboo swaying, clouds drifting. Transform and
	 * opacity only; hidden tabs pause everything; reduced motion freezes the
	 * scene in a still pose (no lightning flashes).
	 */
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
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
		SAND_Y,
		POND,
		HILLS_BACK,
		HILLS_FRONT,
		HEDGE,
		STONE_GROUPS,
		WAVES,
		RIM,
		BAMBOO,
		KOI,
		KOI_CSS,
		STARS,
		RAIN,
		zenStyle
	} from './geometry.js';

	type Gesture = { kind: 'down' | 'up' | 'resolved'; at: number };

	interface Props {
		theme: SceneTheme;
		now: Date;
		/** `calm | clouded | storm`, or `ok | warn | critical`. */
		mood: SceneMood | 'ok' | 'warn' | 'critical';
		/** Problems on the board: the koi are shy while there are some. */
		problems: number;
		/** Bumped by the wall when a new problem appears: the koi dive once. */
		startle?: number;
		/** The tab is hidden: nothing moves. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		/** A gesture, replayed whenever `at` changes. */
		pulse?: Gesture | null;
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

	// --- Stage fit ----------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light and weather ------------------------------------------------------------

	const skyMood = $derived<SceneMood>(
		mood === 'storm' || mood === 'critical' ? 'storm' : mood === 'clouded' || mood === 'warn' ? 'clouded' : 'calm'
	);
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const style = $derived(`${sceneStyle(theme, phase, skyMood)};${zenStyle(theme)}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));
	const raining = $derived(skyMood === 'storm');
	const cloudCount = $derived(skyMood === 'storm' ? 7 : skyMood === 'clouded' ? 5 : 3);
	const CLOUDS = [
		{ y: 120, s: 1.1, dur: 260, delay: -40 },
		{ y: 300, s: 0.8, dur: 320, delay: -210 },
		{ y: 440, s: 0.6, dur: 360, delay: -120 },
		{ y: 200, s: 0.95, dur: 290, delay: -150 },
		{ y: 370, s: 1.25, dur: 340, delay: -290 },
		{ y: 60, s: 0.7, dur: 300, delay: -250 },
		{ y: 500, s: 1.0, dur: 380, delay: -60 }
	];

	// --- Gestures ---------------------------------------------------------------------

	/** The last gesture: the pulse, or a dive when the wall bumps `startle`. */
	let gesture = $state<Gesture | null>(null);
	let lastPulse = untrack(() => pulse?.at ?? null);
	let lastStartle = untrack(() => startle);
	$effect(() => {
		const at = pulse?.at ?? null;
		const current = pulse;
		untrack(() => {
			if (at !== lastPulse) {
				lastPulse = at;
				gesture = current;
			}
		});
	});
	$effect(() => {
		const value = startle;
		untrack(() => {
			if (value !== lastStartle) {
				lastStartle = value;
				gesture = { kind: 'down', at: Date.now() };
			}
		});
	});
	const stamp = $derived(gesture ? gesture.at : 0);
	const kind = $derived(gesture ? gesture.kind : 'none');
	const shy = $derived(problems > 0);
</script>

{@html `<style>${KOI_CSS}</style>`}

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
						<path
							d="M30 104 C2 104 0 66 30 62 C24 32 60 18 86 34 C98 6 152 0 170 30 C192 14 234 22 238 54 C270 52 292 78 276 104 Z"
							fill={raining ? 'var(--cloud-storm)' : 'var(--cloud)'}
						/>
					</svg>
				</div>
			{/each}

			<!-- The garden. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}" fill="none">
				<defs>
					<linearGradient id="zen-w-pond" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--pond1)" /><stop offset="1" stop-color="var(--pond2)" />
					</linearGradient>
					<linearGradient id="zen-w-sand" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--sand2)" /><stop offset="1" stop-color="var(--sand1)" />
					</linearGradient>
					<clipPath id="zen-w-clip"><rect x="0" y={SAND_Y} width={STAGE_W} height={STAGE_H - SAND_Y} /></clipPath>
					<g id="zen-w-koi">
						<path d="M17 0C9-8-6-7-14-1L-24-8-21 0-24 8-14 1C-6 7 9 8 17 0z" fill="var(--k)" />
						<path d="M6-6C1-8-5-6-8-2-4 0 2-2 6-6zM-2 5c4 2 8 1 11-2" fill="var(--kp)" opacity=".9" />
						<path d="M2-6l-4-5 8 3zM2 6l-4 5 8-3z" fill="var(--k)" opacity=".7" />
					</g>
					<path id="zen-w-leaf" d="M0-6l3 5 6-1-3 5 4 5-6-1-4 4-4-4-6 1 4-5-3-5 6 1z" />
				</defs>

				<path d={HILLS_BACK} fill="var(--hill2)" />
				<path d={HILLS_FRONT} fill="var(--hill1)" />
				<rect x="0" y={SAND_Y - 12} width={STAGE_W} height="30" fill="var(--hedge)" />
				<g fill="var(--hedge2)">
					{#each HEDGE as b, i (i)}<ellipse cx={b.x} cy={b.y} rx={b.rx} ry={b.ry} />{/each}
				</g>

				<!-- sand and rake lines -->
				<rect x="0" y={SAND_Y} width={STAGE_W} height={STAGE_H - SAND_Y} fill="url(#zen-w-sand)" />
				<g clip-path="url(#zen-w-clip)" stroke="var(--rake)" stroke-width="2.4" stroke-linecap="round">
					{#each WAVES as w, i (i)}<path d={w} opacity={0.8 - i * 0.06} />{/each}
					{#each STONE_GROUPS as g, gi (gi)}
						<g transform="translate({g.x} {g.y})">
							{#each g.rings as r, i (i)}<ellipse rx={r.rx} ry={r.ry} opacity={0.85 - i * 0.04} />{/each}
						</g>
					{/each}
				</g>

				<!-- stones in the sand -->
				<ellipse cx="420" cy="896" rx="80" ry="24" fill="var(--shadow)" />
				<path d="M350 896c-10-56 34-96 78-92 44 4 76 40 68 92z" fill="var(--stone)" />
				<path d="M366 862c18-26 48-38 74-30-26 4-48 18-62 44z" fill="var(--stone-hi)" />
				<path d="M388 818c6-10 14-14 24-14" stroke="var(--moss)" stroke-width="9" stroke-linecap="round" />
				<ellipse cx="540" cy="900" rx="36" ry="12" fill="var(--shadow)" />
				<path d="M512 900c-4-24 14-38 32-37 18 1 28 16 26 37z" fill="var(--stone2)" />
				<ellipse cx="860" cy="980" rx="50" ry="16" fill="var(--shadow)" />
				<path d="M818 980c-6-34 20-58 46-56 26 2 42 26 38 56z" fill="var(--stone)" />
				<path d="M830 958c10-14 28-20 44-16-16 2-30 10-38 26z" fill="var(--stone-hi)" />
				<ellipse cx="150" cy="1018" rx="44" ry="14" fill="var(--shadow)" />
				<path d="M112 1018c-4-26 14-42 36-41 22 1 34 18 32 41z" fill="var(--stone2)" />

				<!-- stone lantern -->
				<g transform="translate(700 800) scale(1.5)">
					<ellipse cx="0" cy="6" rx="26" ry="7" fill="var(--shadow)" />
					<path d="M-16 4h32v-8h-32zM-6-4h12V-34h-12z" fill="var(--stone)" />
					<path d="M-18-34h36v-6h-36z" fill="var(--stone2)" />
					<rect x="-12" y="-62" width="24" height="22" fill="var(--stone)" />
					<rect x="-7" y="-58" width="14" height="14" fill="var(--lantern)" opacity={lit ? 1 : 0.35} />
					<path d="M-26-62h52l-10-14h-32zM-5-76h10v-8h-10z" fill="var(--stone2)" />
				</g>
				{#if lit}<circle class="lantern-glow" cx="700" cy="708" r="110" fill="var(--glow)" />{/if}

				<!-- pond -->
				<ellipse cx={POND.x} cy={POND.y + 12} rx={POND.rx + 22} ry={POND.ry + 18} fill="var(--shadow)" />
				<ellipse cx={POND.x} cy={POND.y} rx={POND.rx} ry={POND.ry} fill="url(#zen-w-pond)" />
				<g class="shimmer" stroke="var(--glint)" stroke-width="2.4" stroke-linecap="round">
					<path d="M1180 900h60M1520 890h80M1330 972h90M1620 960h60M1260 936h40" />
				</g>
				<g class="koi" class:koi--shy={shy}>
					{#key stamp}
						<g class="veil v-{kind}">
							{#each KOI as k (k.id)}
								<g class={k.id} transform={k.first}>
									<g style="--k:{k.body};--kp:{k.patch}" transform="scale({k.s})"><use href="#zen-w-koi" /></g>
								</g>
							{/each}
						</g>
					{/key}
				</g>
				<g fill="var(--pad)">
					<ellipse cx="1220" cy="944" rx="40" ry="11" /><ellipse cx="1280" cy="968" rx="28" ry="8" /><ellipse cx="1590" cy="912" rx="36" ry="10" /><ellipse cx="1650" cy="940" rx="24" ry="7" />
				</g>
				<g fill="var(--lotus)"><path d="M1220 938l-9-13 9 5 9-5z" /><path d="M1590 906l-8-12 8 5 8-5z" /></g>
				{#key stamp}
					<g class="effects e-{kind}">
						<g class="leap"><g class="leap-r"><g transform="scale(1.7)"><use href="#zen-w-koi" style="--k:var(--koi1);--kp:var(--koiw)" /></g></g></g>
						<ellipse class="ring r1" cx={POND.x} cy={POND.y} rx="230" ry="60" stroke="var(--glint)" stroke-width="4" />
						<ellipse class="ring r2" cx={POND.x} cy={POND.y} rx="230" ry="60" stroke="var(--glint)" stroke-width="3" />
						<ellipse class="ring r3" cx={POND.x} cy={POND.y} rx="230" ry="60" stroke="var(--glint)" stroke-width="2" />
					</g>
				{/key}
				{#each RIM as s, i (i)}
					<ellipse cx={s.x} cy={s.y} rx={s.rx} ry={s.ry} fill={s.alt ? 'var(--stone2)' : 'var(--stone)'} />
				{/each}

				<!-- shishi-odoshi, facing the pond -->
				<g transform="translate(1090 860) scale(-1.6 1.6)">
					<path d="M-6 0v64M6 0v64M-14 64h28" stroke="var(--wood)" stroke-width="6" stroke-linecap="round" />
					<path d="M-14-24C-40-50-100-70-150-72" stroke="var(--bamboo2)" stroke-width="8" stroke-linecap="round" />
					<path d="M-14-24C-40-50-100-70-150-72" stroke="var(--bamboo)" stroke-width="3" stroke-linecap="round" />
					<path class="stream" d="M-62-54v46" stroke="var(--glint)" stroke-width="2.4" stroke-linecap="round" stroke-dasharray="4 5" />
					<ellipse cx="46" cy="30" rx="22" ry="8" fill="var(--stone)" /><path d="M28 30c-2-10 8-16 18-16s18 6 16 16z" fill="var(--stone2)" />
					<g class="tube">
						{#key stamp}
							<g class="tilt t-{kind}">
								<rect x="-62" y="-8" width="104" height="16" rx="6" fill="var(--bamboo2)" />
								<rect x="-62" y="-8" width="104" height="6" rx="3" fill="var(--bamboo)" />
								<path d="M-32-8v16M10-8v16" stroke="var(--bamboo2)" stroke-width="2.4" />
								<path class="fill" d="M-60-2h26" stroke="var(--glint)" stroke-width="4" stroke-linecap="round" />
							</g>
						{/key}
						<circle r="4.4" fill="var(--wood)" />
					</g>
					{#key stamp}
						<g transform="translate(46 12)">
							<ellipse class="cr c1 c-{kind}" rx="12" ry="5" stroke="var(--glint)" stroke-width="2" />
							<ellipse class="cr c2 c-{kind}" rx="12" ry="5" stroke="var(--glint)" stroke-width="1.6" />
						</g>
					{/key}
					<path class="spill" d="M-62 8c-3 22-4 44-8 70" stroke="var(--glint)" stroke-width="3.4" stroke-linecap="round" />
				</g>

				<!-- maple -->
				<path d="M1560 744c-6-70 6-130-10-200M1556 600c40-24 60-56 74-100M1556 560c-44-22-66-50-76-92" stroke="var(--trunk)" stroke-width="20" stroke-linecap="round" />
				<g>
					<ellipse cx="1546" cy="420" rx="190" ry="92" fill="var(--maple2)" />
					<ellipse cx="1440" cy="468" rx="120" ry="58" fill="var(--maple1)" />
					<ellipse cx="1660" cy="476" rx="110" ry="56" fill="var(--maple3)" />
					<ellipse cx="1530" cy="372" rx="120" ry="52" fill="var(--maple1)" />
					<ellipse cx="1590" cy="438" rx="90" ry="38" fill="var(--maple3)" opacity=".85" />
				</g>
				<g class="leaf l1" fill="var(--maple1)"><use href="#zen-w-leaf" transform="scale(1.6)" /></g>
				<g class="leaf l2" fill="var(--maple3)"><use href="#zen-w-leaf" transform="scale(1.6)" /></g>
				<g class="leaf l3" fill="var(--maple2)"><use href="#zen-w-leaf" transform="scale(1.6)" /></g>
				<g class="leaf l4" fill="var(--maple1)"><use href="#zen-w-leaf" transform="scale(1.6)" /></g>
				<g fill="var(--maple1)" opacity=".8"><ellipse cx="1470" cy="1020" rx="10" ry="4" /><ellipse cx="1560" cy="1040" rx="8" ry="3" /><ellipse cx="1380" cy="1048" rx="9" ry="3.5" /></g>

				<!-- bamboo grove -->
				{#each BAMBOO as b, i (i)}
					<g class="stalk" style:animation-delay="-{i * 1.7}s">
						<path d="M{b.x} {SAND_Y + 20}L{b.x + b.lean} 0" stroke="var(--bamboo2)" stroke-width={b.w} stroke-linecap="round" />
						<path d="M{b.x} {SAND_Y + 20}L{b.x + b.lean} 0" stroke="var(--bamboo)" stroke-width={b.w / 3} stroke-linecap="round" />
						{#each [0.18, 0.36, 0.55, 0.74] as f (f)}
							<path d="M{b.x + b.lean * (1 - f) - b.w / 2} {SAND_Y * (1 - f)}h{b.w}" stroke="var(--dark)" stroke-width="2.4" opacity=".45" />
						{/each}
					</g>
				{/each}
				<g class="sway" fill="var(--leaf)">
					<path d="M1822 150c-50-8-86 12-110 36 40-8 76-10 110-36zM1858 190c46-10 84 4 110 26-42-4-78-6-110-26zM1892 120c-44-6-76 10-96 30 36-6 66-8 96-30zM1822 260c40-8 70 6 92 24-36-2-64-6-92-24zM1858 300c-44-6-76 10-96 28 36-4 66-6 96-28z" />
				</g>

				<!-- the storm's bolt -->
				<path class="bolt" d="M960 40l-34 120h28l-40 140 82-160h-34l36-100z" fill="var(--bolt)" />
			</svg>

			{#if raining}
				<div class="veil-storm"></div>
				<div class="rain">
					<svg viewBox="0 0 1920 1620">
						<path d={RAIN} stroke="var(--rain)" stroke-width="2.4" stroke-linecap="round" />
					</svg>
				</div>
				<div class="flash"></div>
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
		--bolt: #f4f7ff;
		background: var(--sky-bottom);
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
	.lantern-glow {
		opacity: 0.5;
	}

	/* Storm */
	.veil-storm {
		position: absolute;
		inset: 0;
		background: var(--veil);
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
	.flash {
		position: absolute;
		inset: 0;
		background: var(--bolt);
		opacity: 0;
		animation: flash 11s infinite;
	}
	.bolt {
		opacity: 0;
	}
	@keyframes flash {
		0%,
		60%,
		66%,
		100% {
			opacity: 0;
		}
		61% {
			opacity: 0.28;
		}
		63% {
			opacity: 0.05;
		}
		64% {
			opacity: 0.2;
		}
	}
	@keyframes bolt {
		0%,
		60.5%,
		65%,
		100% {
			opacity: 0;
		}
		61%,
		64.5% {
			opacity: 1;
		}
		63% {
			opacity: 0.2;
		}
	}
	.scene:has(.rain) .bolt {
		animation: bolt 11s infinite;
	}

	/* Ambient life */
	.stream {
		animation: drip 0.5s linear infinite;
	}
	@keyframes drip {
		to {
			stroke-dashoffset: -9;
		}
	}
	.shimmer {
		animation: shimmer 6s ease-in-out infinite;
	}
	@keyframes shimmer {
		50% {
			opacity: 0.3;
		}
	}
	.koi {
		transition: opacity 3s;
	}
	.koi--shy {
		opacity: 0.55;
	}
	.sway {
		transform-box: fill-box;
		transform-origin: 100% 100%;
		animation: sway 8s ease-in-out infinite;
	}
	.stalk {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: sway 9s ease-in-out infinite;
	}
	@keyframes sway {
		50% {
			transform: rotate(1.2deg);
		}
	}
	.leaf {
		opacity: 0;
		animation: fall 16s linear infinite;
	}
	.l1 {
		animation-delay: -2s;
	}
	.l2 {
		animation-delay: -6s;
		animation-duration: 19s;
	}
	.l3 {
		animation-delay: -10s;
		animation-duration: 14s;
	}
	.l4 {
		animation-delay: -13s;
		animation-duration: 18s;
	}
	@keyframes fall {
		0% {
			opacity: 0;
			transform: translate(1560px, 480px) rotate(0);
		}
		8% {
			opacity: 1;
		}
		50% {
			transform: translate(1480px, 800px) rotate(220deg);
		}
		92% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(1400px, 1030px) rotate(440deg);
		}
	}
	.tube {
		transform: rotate(16deg);
		animation: tipping 18s ease-in-out infinite;
	}
	@keyframes tipping {
		0%,
		70% {
			transform: rotate(16deg);
		}
		74%,
		78% {
			transform: rotate(-26deg);
		}
		83% {
			transform: rotate(18deg);
		}
		85% {
			transform: rotate(13deg);
		}
		88%,
		100% {
			transform: rotate(16deg);
		}
	}
	.spill {
		opacity: 0;
		animation: spill 18s linear infinite;
	}
	@keyframes spill {
		0%,
		73%,
		80%,
		100% {
			opacity: 0;
		}
		75%,
		78% {
			opacity: 0.9;
		}
	}
	.fill {
		opacity: 0;
		animation: fillup 18s linear infinite;
	}
	@keyframes fillup {
		0% {
			opacity: 0;
		}
		60%,
		72% {
			opacity: 0.85;
		}
		75%,
		100% {
			opacity: 0;
		}
	}
	.cr,
	.ring {
		opacity: 0;
		transform-box: fill-box;
		transform-origin: center;
	}
	.leap {
		opacity: 0;
	}

	/* down: the koi dive and hide, then drift back. */
	.v-down {
		animation: hide 6s ease-in-out both;
	}
	@keyframes hide {
		0% {
			opacity: 1;
		}
		14%,
		78% {
			opacity: 0;
		}
		100% {
			opacity: 1;
		}
	}
	.e-down .ring {
		animation: ring-in 1.6s ease-in both;
	}
	.e-down .r2 {
		animation-delay: 0.35s;
	}
	@keyframes ring-in {
		0% {
			opacity: 0.8;
			transform: scale(1);
		}
		100% {
			opacity: 0;
			transform: scale(0.15);
		}
	}
	/* up: one koi leaps out of the pond, rings spread where it lands. */
	.e-up .leap {
		animation: leap 2.2s ease-out both;
	}
	.e-up .leap-r {
		animation: leap-turn 2.2s linear both;
	}
	@keyframes leap {
		0% {
			opacity: 0;
			transform: translate(1260px, 930px);
		}
		10% {
			opacity: 1;
		}
		50% {
			transform: translate(1380px, 760px);
		}
		88% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(1500px, 930px);
		}
	}
	@keyframes leap-turn {
		0% {
			transform: rotate(-60deg);
		}
		50% {
			transform: rotate(0deg);
		}
		100% {
			transform: rotate(60deg);
		}
	}
	.e-up .ring {
		animation: ring-out 2.6s ease-out 1.8s both;
	}
	.e-up .r2 {
		animation-delay: 2.1s;
	}
	.e-up .r3 {
		animation-delay: 2.4s;
	}
	@keyframes ring-out {
		0% {
			opacity: 0.85;
			transform: scale(0.1);
		}
		100% {
			opacity: 0;
			transform: scale(1);
		}
	}
	/* resolved: the shishi-odoshi tips and clacks on its stone. */
	.t-resolved {
		animation: tip-now 3s ease-in-out both;
	}
	@keyframes tip-now {
		0%,
		12% {
			transform: rotate(0deg);
		}
		30%,
		50% {
			transform: rotate(-44deg);
		}
		62% {
			transform: rotate(8deg);
		}
		68% {
			transform: rotate(-3deg);
		}
		100% {
			transform: rotate(0deg);
		}
	}
	.cr.c-resolved {
		animation: clack 1.6s ease-out 1.85s both;
	}
	.cr.c2.c-resolved {
		animation-delay: 2.15s;
	}
	@keyframes clack {
		0% {
			opacity: 0.95;
			transform: scale(0.3);
		}
		100% {
			opacity: 0;
			transform: scale(3);
		}
	}

	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	/* Reduced motion: a still picture. */
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
		.scene .cloud {
			transform: translateX(var(--still-x));
		}
	}
</style>
