<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop: Reykjavik. Hallgrimskirkja on its hill above
	 * rows of colourful corrugated-iron houses, the harbour with its boats,
	 * Esja across the bay and, after dark, the northern lights. A drop-in for
	 * the Paris scene (same props), plus `pulse`.
	 *
	 * Drawn on a 1920 x 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky
	 * follows the real hour inside the brightness band of the wall's theme (see
	 * `paris/daylight.ts`); windows and the church floodlights come on after
	 * dark, and so do the aurora curtains (in the light theme they only appear
	 * as part of a pulse).
	 *
	 * Mood is the weather: calm is clear, clouded brings clouds and dims the
	 * aurora, storm brings rain, a dark sky and lightning flashes. `ok`, `warn`
	 * and `critical` are accepted as synonyms. A DumbMonit pigeon stands on the
	 * roof ridge: with problems it holds up a sign, in a storm it opens an
	 * umbrella, a new problem startles it. Now and then a puffin flies past.
	 *
	 * `pulse`, replayed whenever `at` changes:
	 *  - down: the aurora collapses into a thin line and the church and the
	 *    windows go dark, flickering back after a few seconds;
	 *  - up: the aurora surges, curtains swelling and brightening;
	 *  - resolved: rings of light ripple out from the spire while the aurora
	 *    sweeps through violet.
	 *
	 * Hidden tabs pause everything; reduced motion freezes a still picture
	 * (no pulse, no lightning).
	 */
	import { tick, untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import WallPigeon from '#lib/components/wall/paris/WallPigeon.svelte';
	import {
		phaseAt,
		isLit,
		celestialAt,
		sceneStyle,
		type SceneTheme
	} from '#lib/components/wall/paris/daylight.js';
	import {
		STAGE_W,
		STAGE_H,
		LEDGE_Y,
		PIGEON_X,
		ESJA,
		ESJA_FRONT,
		SNOW,
		HILL,
		CHURCH,
		FAR_ROW,
		BACK_ROW,
		FRONT_ROW,
		WINDOW_COUNT,
		CURTAINS,
		STARS,
		RAIN,
		litWindows,
		reykjavikStyle
	} from './geometry.js';

	type Mood = 'calm' | 'clouded' | 'storm' | 'ok' | 'warn' | 'critical';
	interface Props {
		theme: SceneTheme;
		now: Date;
		mood?: Mood;
		/** Problems on the board: the number on the protest sign. */
		problems?: number;
		/** Bumped by the wall when a new problem appears: the pigeon startles once. */
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

	// --- Light and weather ------------------------------------------------------------

	const level = $derived(mood === 'clouded' || mood === 'warn' ? 1 : mood === 'storm' || mood === 'critical' ? 2 : 0);
	const raining = $derived(level === 2);

	// Re-read once a minute: nothing in the sky moves faster than that.
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));
	// The aurora shows in the dark themes after dark; the light theme only borrows it for a pulse.
	const auroraOp = $derived(theme !== 'light' && lit ? (theme === 'oled' ? 0.6 : 0.9) * [1, 0.5, 0.12][level] : 0);
	const style = $derived(
		`${sceneStyle(theme, phase, level === 2 ? 'storm' : level === 1 ? 'clouded' : 'calm')};${reykjavikStyle(theme)};--aur-op:${auroraOp}`
	);

	const CLOUDS = [
		{ y: 150, s: 1.1, dur: 260, delay: -40 },
		{ y: 330, s: 0.8, dur: 320, delay: -210 },
		{ y: 90, s: 0.7, dur: 300, delay: -250 },
		{ y: 240, s: 0.95, dur: 290, delay: -150 },
		{ y: 420, s: 1.25, dur: 340, delay: -290 },
		{ y: 40, s: 1.0, dur: 360, delay: -60 },
		{ y: 520, s: 0.6, dur: 380, delay: -120 }
	];
	const cloudCount = $derived([2, 5, 7][level]);

	let windowSeed = $state(1);
	const litSet = $derived(lit ? litWindows(windowSeed, WINDOW_COUNT, phase === 'night' ? 0.22 : 0.34) : new Set<number>());

	// --- Pulse -----------------------------------------------------------------------

	let playing = $state<'down' | 'up' | 'resolved' | null>(null);
	$effect(() => {
		const p = pulse;
		playing = null;
		if (!p || !motion || paused) return;
		void p.at;
		const begin = requestAnimationFrame(() => (playing = p.kind));
		const end = setTimeout(() => (playing = null), 5600);
		return () => {
			cancelAnimationFrame(begin);
			clearTimeout(end);
		};
	});

	// --- The pigeon -------------------------------------------------------------------

	const trouble = $derived(problems > 0);
	let lookX = $state(0);
	let startled = $state(false);
	const lookY = $derived(playing === 'up' || playing === 'resolved' ? -0.9 : trouble ? -0.3 : 0);
	const eyes = $derived(startled || playing === 'down' ? 'startled' : 'open');
	const sign = $derived(trouble ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null);

	$effect(() => {
		if (paused || !motion) {
			lookX = trouble ? -0.8 : 0;
			return;
		}
		if (trouble) {
			lookX = -0.8;
			return;
		}
		const timer = setInterval(() => (lookX = Math.round((Math.random() * 1.4 - 0.7) * 10) / 10), 6500);
		return () => clearInterval(timer);
	});

	let lastStartle = untrack(() => startle);
	$effect(() => {
		const value = startle;
		if (value === lastStartle) return;
		lastStartle = value;
		startled = true;
		const timer = setTimeout(() => (startled = false), 1800);
		return () => clearTimeout(timer);
	});

	// Lit windows change now and then while the town is lit.
	$effect(() => {
		if (!lit || paused || !motion) return;
		const timer = setInterval(() => (windowSeed += 1), 120_000);
		return () => clearInterval(timer);
	});

	// --- Lightning, puffin -------------------------------------------------------------

	let strike = $state(0);
	let boltX = $state(700);
	$effect(() => {
		if (!raining || paused || !motion) return;
		let timer: ReturnType<typeof setTimeout>;
		const next = () => {
			timer = setTimeout(() => {
				boltX = Math.round(300 + Math.random() * 1000);
				strike += 1;
				next();
			}, 7000 + Math.random() * 9000);
		};
		next();
		return () => clearTimeout(timer);
	});

	let puffin = $state(false);
	let puffinEl = $state<HTMLDivElement | null>(null);
	$effect(() => {
		if (paused || !motion || phase === 'night') return;
		let timer: ReturnType<typeof setTimeout>;
		let flight: Animation | undefined;
		const schedule = (first: boolean) => {
			timer = setTimeout(
				async () => {
					puffin = true;
					await tick();
					const y = 380 + Math.random() * 140;
					flight = puffinEl?.animate(
						[
							{ transform: `translate(-120px, ${y + 30}px)` },
							{ transform: `translate(900px, ${y - 20}px)`, offset: 0.5 },
							{ transform: `translate(2040px, ${y + 20}px)` }
						],
						{ duration: 16000, easing: 'linear', fill: 'forwards' }
					);
					await flight?.finished.catch(() => undefined);
					puffin = false;
					schedule(false);
				},
				first ? 9000 + Math.random() * 7000 : 40000 + Math.random() * 40000
			);
		};
		schedule(true);
		return () => {
			clearTimeout(timer);
			flight?.cancel();
			puffin = false;
		};
	});

	const CORRUGATION = (() => {
		let d = '';
		for (let x = 0; x <= STAGE_W; x += 26) d += `M${x} ${LEDGE_Y + 8}V${STAGE_H}`;
		return d;
	})();
</script>

<div
	class="scene {className}"
	class:scene--paused={paused}
	class:scene--still={!motion}
	class:p-down={playing === 'down'}
	class:p-up={playing === 'up'}
	class:p-resolved={playing === 'resolved'}
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

			<!-- Northern lights: curtains with rays, and their glow on the bay. -->
			<svg class="layer aurora" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<defs>
					{#each CURTAINS as c, i (i)}
						<linearGradient id="rey-w-{i}" gradientUnits="userSpaceOnUse" x1="0" y1={c.y0} x2="0" y2={c.y1}>
							<stop offset="0" stop-color="var(--rey-aur{c.tone})" stop-opacity="0" />
							<stop offset="0.8" stop-color="var(--rey-aur{c.tone})" stop-opacity="0.8" />
							<stop offset="1" stop-color="var(--rey-aur{c.tone})" stop-opacity="0.2" />
						</linearGradient>
					{/each}
					<linearGradient id="rey-w-refl" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--rey-aur1)" stop-opacity="0.32" />
						<stop offset="1" stop-color="var(--rey-aur1)" stop-opacity="0" />
					</linearGradient>
					<filter id="rey-w-blur" x="-5%" y="-30%" width="110%" height="160%"><feGaussianBlur stdDeviation="9" /></filter>
				</defs>
				{#each CURTAINS as c, i (i)}
					<g class="sway sway-{i}">
						<path d={c.d} fill="url(#rey-w-{i})" filter="url(#rey-w-blur)" />
						<path d={c.rays} stroke="url(#rey-w-{i})" stroke-width="3" stroke-linecap="round" />
					</g>
				{/each}
				<rect x="220" y="660" width="860" height="260" fill="url(#rey-w-refl)" />
			</svg>

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

			<!-- The town -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<path d={ESJA} fill="var(--rey-mtn)" />
				<path d={SNOW} fill="var(--rey-snow)" opacity="0.8" />
				<path d={ESJA_FRONT} fill="var(--rey-mtn2)" />
				<rect x="0" y="640" width={STAGE_W} height={LEDGE_Y - 640} fill="var(--rey-water1)" />
				<rect x="0" y="780" width={STAGE_W} height={LEDGE_Y - 780} fill="var(--rey-water2)" opacity="0.55" />

				<!-- far shore -->
				{#each FAR_ROW as h, i (i)}
					<rect x={h.x} y={h.base - h.h} width={h.w} height={h.h} fill="var(--rey-h{h.wall})" opacity="0.75" />
					<path d="M{h.x - 1} {h.base - h.h}L{h.x + h.w / 2} {h.base - h.h - h.w * 0.36}L{h.x + h.w + 1} {h.base - h.h}z" fill="var(--rey-r{h.roof})" opacity="0.75" />
				{/each}

				<!-- water: ripples, boats, the quay -->
				<g stroke="var(--rey-ripple)" stroke-width="3" stroke-linecap="round" opacity="0.4" class="ripples">
					<path d="M80 700h140M360 730h180M640 690h120M200 800h200M520 840h160M820 780h150M120 900h180M760 910h200" />
				</g>
				<g class="boat boat-a">
					<g transform="translate(300 770)">
						<path d="M0 0H170L148 28H22z" fill="var(--rey-hull)" /><rect x="92" y="-38" width="46" height="38" fill="var(--rey-r1)" /><rect x="100" y="-28" width="30" height="14" fill="var(--window-lit)" opacity="0.85" />
						<path d="M44 0V-76M44-76L112-12" stroke="var(--rey-mast)" stroke-width="3.5" fill="none" />
					</g>
				</g>
				<g class="boat boat-b"><g transform="translate(640 730)"><path d="M0 0H96L84 18H12z" fill="var(--rey-hull)" /><path d="M48 0V-92L92-12z" fill="var(--rey-sail)" /></g></g>
				<rect x="0" y="918" width="1120" height="14" fill="var(--rey-quay)" />
				<g fill="var(--lamp)"><rect x="180" y="896" width="8" height="26" /><rect x="520" y="896" width="8" height="26" /><rect x="860" y="896" width="8" height="26" /></g>

				<!-- the hill and Hallgrimskirkja, floodlit after dark -->
				<path d={HILL} fill="var(--rey-ground)" />
				<g transform="translate({CHURCH.x} {CHURCH.y}) scale({CHURCH.s})">
					<path class="chlit" d="M-170 0L-18 -330H18L170 0z" fill="var(--rey-beam)" />
					<ellipse class="chlit" cy="-6" rx="200" ry="44" fill="var(--glow)" />
					<rect x="-156" y="-30" width="52" height="30" fill="var(--rey-church2)" /><rect x="104" y="-30" width="52" height="30" fill="var(--rey-church2)" />
					{#each CHURCH.cols as h, i (i)}
						<rect x={-20 - 13 * (i + 1)} y={-h} width="13" height={h} fill="var(--rey-church)" stroke="var(--rey-church2)" stroke-width="1.4" />
						<rect x={20 + 13 * i} y={-h} width="13" height={h} fill="var(--rey-church)" stroke="var(--rey-church2)" stroke-width="1.4" />
					{/each}
					<rect x="-20" y="-238" width="40" height="238" fill="var(--rey-church)" /><rect x="2" y="-238" width="18" height="238" fill="var(--rey-church2)" opacity="0.55" />
					<path d="M-10 -238V0M0 -238V0M10 -238V0" stroke="var(--rey-church2)" stroke-width="1.2" opacity="0.6" />
					<rect x="-27" y="-272" width="54" height="36" fill="var(--rey-church)" /><rect x="-30" y="-276" width="60" height="6" fill="var(--rey-church2)" />
					<path d="M-27 -272L0 -344L27 -272z" fill="var(--rey-church)" /><path d="M0 -344L27 -272H8z" fill="var(--rey-church2)" opacity="0.55" />
					<path d="M0 -344V-364M-6 -354H6" stroke="var(--rey-church2)" stroke-width="2.4" />
					<g class="chlit" fill="var(--window-lit)"><path d="M-14 -262v-18h6v18zM-3 -262v-18h6v18zM8 -262v-18h6v18zM-4 -200v-40h8v40zM-4 -150v-30h8v30z" /></g>
					<g class="rings" stroke="var(--rey-aur1)" stroke-width="3" fill="none"><circle cy="-330" r="40" /><circle cy="-330" r="40" /><circle cy="-330" r="40" /></g>
				</g>

				<!-- colourful houses, back row then front row -->
				{#each [BACK_ROW, FRONT_ROW] as rowOf, k (k)}
					{#each rowOf as h, i (i)}
						<rect x={h.x} y={h.base - h.h} width={h.w} height={h.h} fill="var(--rey-h{h.wall})" />
						<path d="M{h.x - 3} {h.base - h.h}L{h.x + h.w / 2} {h.base - h.h - h.w * 0.34}L{h.x + h.w + 3} {h.base - h.h}z" fill="var(--rey-r{h.roof})" />
						{#each h.wins as w (w.i)}
							<rect class="win" x={w.x} y={w.y} width={w.w} height={w.h} rx="1.5" fill={litSet.has(w.i) ? 'var(--window-lit)' : 'var(--rey-win)'} />
						{/each}
					{/each}
				{/each}
			</svg>

			{#if puffin}
				<div class="mover" bind:this={puffinEl}>
					<svg viewBox="0 0 60 40" class="puffin">
						<path class="pwing" d="M26 18C18 4 8 2 2 4C12 10 20 18 26 24z" fill="#1d2433" />
						<ellipse cx="34" cy="22" rx="18" ry="10" fill="#1d2433" /><ellipse cx="36" cy="26" rx="12" ry="6" fill="#fbf7ee" />
						<circle cx="48" cy="18" r="7" fill="#fbf7ee" /><circle cx="50" cy="17" r="1.6" fill="#1d2433" />
						<path d="M54 15L62 19L54 23z" fill="#f08a2c" /><path d="M54 18H60" stroke="#c9302c" stroke-width="1.6" />
						<path d="M18 26l-10 3M20 28l-10 6" stroke="#f08a2c" stroke-width="2" stroke-linecap="round" />
					</svg>
				</div>
			{/if}

			<!-- Our roof ridge, in corrugated iron, and the pigeon on it. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<rect x="0" y={LEDGE_Y + 6} width={STAGE_W} height={STAGE_H - LEDGE_Y} fill="var(--rey-roof)" />
				<path d={CORRUGATION} stroke="var(--rey-roof-line)" stroke-width="3" opacity="0.7" />
				<rect x="0" y={LEDGE_Y + 2} width={STAGE_W} height="14" fill="var(--rey-roof-line)" />
			</svg>
			<div class="cast" style:left="{PIGEON_X - 75}px" style:top="{LEDGE_Y + 6 - 187}px">
				<WallPigeon class="gaston" {sign} {lookX} {lookY} {eyes} umbrella={raining} blinkOffset={0.7} />
			</div>

			{#if raining}
				<div class="rain">
					<svg viewBox="0 0 1920 1620"><path d={RAIN} stroke="var(--rain)" stroke-width="2.4" stroke-linecap="round" /></svg>
				</div>
				<div class="veil"></div>
			{/if}
			{#key strike}
				{#if strike > 0}
					<div class="flash"></div>
					<svg class="layer bolt" viewBox="0 0 {STAGE_W} {STAGE_H}">
						<path d="M{boltX} 0L{boltX - 40} 230L{boltX + 10} 224L{boltX - 70} 470" stroke="#fff6c8" stroke-width="7" stroke-linejoin="round" fill="none" />
					</svg>
				{/if}
			{/key}
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
		background: linear-gradient(to bottom, var(--sky-top) 0%, var(--sky-bottom) 60%);
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

	/* The aurora: dark themes after dark; a pulse borrows it in the light one. */
	.aurora {
		opacity: var(--aur-op);
		transition: opacity 20s ease;
		transform-box: fill-box;
		transform-origin: 50% 70%;
	}
	.sway {
		animation: sway 22s ease-in-out infinite alternate;
		transform-box: fill-box;
		transform-origin: 50% 100%;
	}
	.sway-1 {
		animation-duration: 29s;
		animation-direction: alternate-reverse;
	}
	.sway-2 {
		animation-duration: 17s;
	}
	@keyframes sway {
		from {
			transform: translateX(-26px) skewX(-2deg);
		}
		to {
			transform: translateX(26px) skewX(2deg) scaleY(1.1);
		}
	}

	.win {
		transition: fill 4s ease;
	}
	.ripples {
		animation: shimmer 8s ease-in-out infinite;
	}
	@keyframes shimmer {
		50% {
			opacity: 0.15;
		}
	}
	.boat {
		animation: bob 6s ease-in-out infinite alternate;
	}
	.boat-b {
		animation-duration: 8s;
		animation-delay: -3s;
	}
	@keyframes bob {
		from {
			transform: translate(-14px, -2px);
		}
		to {
			transform: translate(14px, 3px);
		}
	}

	.mover {
		position: absolute;
		left: 0;
		top: 0;
		will-change: transform;
	}
	.puffin {
		display: block;
		width: 64px;
	}
	.pwing {
		transform-box: fill-box;
		transform-origin: 90% 90%;
		animation: flap 0.16s ease-in-out infinite alternate;
	}
	@keyframes flap {
		to {
			transform: scaleY(-0.7);
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
		from {
			transform: translateY(0);
		}
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
	.flash {
		position: absolute;
		inset: 0;
		background: #fff;
		opacity: 0;
		animation: flash 0.9s linear both;
	}
	.bolt {
		animation: bolt 0.9s linear both;
	}
	@keyframes flash {
		0%,
		100% {
			opacity: 0;
		}
		8% {
			opacity: 0.5;
		}
		18% {
			opacity: 0.1;
		}
		28% {
			opacity: 0.38;
		}
		50% {
			opacity: 0;
		}
	}
	@keyframes bolt {
		0%,
		40%,
		100% {
			opacity: 0;
		}
		5%,
		30% {
			opacity: 1;
		}
		15% {
			opacity: 0.2;
		}
	}

	/* Pulse down: the lights go out and flicker back. */
	.p-down .aurora {
		animation: a-out 5.4s ease both;
	}
	.p-down :global(.chlit),
	.p-down .win {
		animation: l-out 5.4s steps(1, end) both;
	}
	@keyframes a-out {
		0% {
			opacity: var(--aur-op);
		}
		14%,
		78% {
			opacity: 0;
			transform: scaleY(0.12);
		}
		100% {
			opacity: var(--aur-op);
		}
	}
	@keyframes l-out {
		0%,
		8% {
			opacity: 1;
		}
		9%,
		70% {
			opacity: 0;
		}
		74% {
			opacity: 1;
		}
		77% {
			opacity: 0;
		}
		82%,
		100% {
			opacity: 1;
		}
	}
	/* Pulse up: the aurora surges. */
	.p-up .aurora {
		animation: a-up 5s ease-in-out both;
	}
	@keyframes a-up {
		0%,
		100% {
			opacity: var(--aur-op);
		}
		35% {
			opacity: var(--rey-aur-peak);
			transform: scaleY(1.4);
		}
	}
	/* Pulse resolved: rings from the spire, the aurora through violet. */
	.p-resolved .aurora {
		animation: a-res 5.4s ease-in-out both;
	}
	@keyframes a-res {
		0%,
		100% {
			opacity: var(--aur-op);
			filter: hue-rotate(0);
		}
		40% {
			opacity: var(--rey-aur-peak);
			filter: hue-rotate(75deg);
		}
	}
	.rings {
		display: none;
	}
	.p-resolved .rings {
		display: block;
	}
	.rings circle {
		transform-box: fill-box;
		transform-origin: center;
		opacity: 0;
		animation: ring 3.6s ease-out both;
	}
	.rings circle:nth-child(2) {
		animation-delay: 0.7s;
	}
	.rings circle:nth-child(3) {
		animation-delay: 1.4s;
	}
	@keyframes ring {
		0% {
			transform: scale(0.15);
			opacity: 0.9;
		}
		100% {
			transform: scale(3.2);
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
