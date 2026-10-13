<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop: the golden pavilion on its pond at Kyoto, a
	 * torii standing in the water, stone and paper lanterns, and a DumbMonit
	 * pigeon on the near bank. Cherry blossom from February to June, red maples
	 * the rest of the year. A drop-in for the Rome scene (same props), plus
	 * `pulse`.
	 *
	 * Drawn on a 1920 x 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky
	 * follows the real hour inside the brightness band of the wall's theme (see
	 * `paris/daylight.ts`); lanterns and the pavilion's windows light up after
	 * dark.
	 *
	 * Weather is the network's mood: `calm` / `ok` is clear, `clouded` / `warn`
	 * gathers clouds, `storm` / `critical` brings rain and (if motion is
	 * allowed) soft lightning flashes. The pigeon holds up a sign with the
	 * number of problems and opens an umbrella in the rain.
	 *
	 * `pulse` plays a lantern gesture each time `at` changes: `down` snuffs the
	 * lanterns out one after the other, `up` relights them with a flare of
	 * golden light, `resolved` sends a gust of petals (or maple leaves) across
	 * the pond.
	 *
	 * Motion is slow: drifting clouds, falling petals, ripples, a flickering
	 * flame, a koi now and then. Hidden tabs pause everything; reduced motion
	 * freezes the scene in a still pose.
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
		RAIN,
		STARS,
		HORIZON_Y,
		PAVILION,
		TORII,
		PIGEON_FEET,
		TREES_BACK,
		TREES_FRONT,
		FALLING,
		GUST,
		ROPE,
		CHOCHIN,
		TORO,
		RINGS,
		PETAL_PATH,
		LEAF_PATH,
		isSpring,
		kyotoStyle,
		type Tree
	} from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood: SceneMood | 'ok' | 'warn' | 'critical';
		/** Problems on the board: the number on the pigeon's sign. */
		problems: number;
		/** Bumped by the wall when a new problem appears: the pigeon startles once. */
		startle?: number;
		/** The tab is hidden: nothing moves, nothing is scheduled. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		/** A lantern gesture, replayed whenever `at` changes. */
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
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

	const weather: SceneMood = $derived(
		mood === 'ok' ? 'calm' : mood === 'warn' ? 'clouded' : mood === 'critical' ? 'storm' : mood
	);
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const spring = $derived(isSpring(minute));
	const style = $derived(`${sceneStyle(theme, phase, weather)};${kyotoStyle(theme, spring, lit)}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));
	const raining = $derived(weather === 'storm');
	const fallD = $derived(spring ? PETAL_PATH : LEAF_PATH);

	const cloudCount = $derived(weather === 'storm' ? 7 : weather === 'clouded' ? 5 : 2);
	const CLOUDS = [
		{ y: 110, s: 1.1, dur: 260, delay: -40 },
		{ y: 300, s: 0.8, dur: 320, delay: -210 },
		{ y: 470, s: 0.6, dur: 360, delay: -120 },
		{ y: 190, s: 0.95, dur: 290, delay: -150 },
		{ y: 380, s: 1.25, dur: 340, delay: -290 },
		{ y: 60, s: 0.7, dur: 300, delay: -250 },
		{ y: 520, s: 1.0, dur: 380, delay: -60 }
	];
	const WINDOWS = [-68, -15, 38];

	// --- The pigeon -----------------------------------------------------------------

	const trouble = $derived(problems > 0);
	let startled = $state(false);
	let look = $state(0.6);
	let reaction = $state<'down' | 'up' | 'resolved' | null>(null);
	let flash = $state(false);
	let ringKey = $state(0);
	let ringAt = $state(0);

	const sign = $derived(trouble ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null);
	const eyes = $derived(startled || reaction === 'down' ? 'startled' : 'open');
	const lookX = $derived(trouble ? -0.8 : reaction === 'resolved' ? 0.9 : look);
	const lookY = $derived(trouble ? -0.3 : reaction === 'resolved' ? -0.8 : 0);

	// --- Scheduling ------------------------------------------------------------------

	let epoch = 0;
	function later(fn: () => void, ms: number) {
		const mine = epoch;
		setTimeout(() => {
			if (mine === epoch) fn();
		}, ms);
	}
	const between = (a: number, b: number) => a + Math.random() * (b - a);

	/** One small moment every 20-45 s: a koi rings the water, the pigeon looks around. */
	function moments() {
		later(() => {
			if (Math.random() < 0.5) {
				ringAt = Math.floor(Math.random() * RINGS.length);
				ringKey += 1;
			} else {
				look = Math.random() < 0.5 ? -0.8 : 0.9;
				later(() => (look = 0.6), 3200);
			}
			moments();
		}, between(20000, 45000));
	}

	/** Soft lightning in a storm: a double flash every 6-14 s. */
	function lightning() {
		later(() => {
			flash = true;
			later(() => (flash = false), 900);
			lightning();
		}, between(6000, 14000));
	}

	$effect(() => {
		const hidden = paused;
		const storm = raining;
		untrack(() => {
			epoch++;
			flash = false;
			if (hidden || !motion) return;
			moments();
			if (storm) lightning();
		});
		return () => {
			epoch++;
		};
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

	// The pulse: the pigeon reacts for a few seconds, the lanterns do the rest in CSS.
	let lastPulse = untrack(() => pulse?.at ?? 0);
	$effect(() => {
		const current = pulse;
		if (!current || current.at === lastPulse) return;
		lastPulse = current.at;
		untrack(() => {
			reaction = current.kind;
			setTimeout(() => (reaction = null), 4000);
		});
	});

	function treeStyle(tree: Tree): string {
		return `translate(${tree.x} ${tree.y}) scale(${tree.s})`;
	}
</script>

{#snippet crown(tree: Tree)}
	<g transform={treeStyle(tree)}>
		<g class="sway">
			<path d="M0 0V-110M0-60L-44-120M0-76L40-130" stroke="var(--k-trunk)" stroke-width="12" stroke-linecap="round" />
			{#each tree.blobs as blob, i (i)}
				<circle cx={blob.dx} cy={blob.dy} r={blob.r} fill={blob.alt ? 'var(--k-leaf2)' : 'var(--k-leaf)'} />
			{/each}
		</g>
	</g>
{/snippet}

{#snippet toro(x: number, y: number, s: number, i: number)}
	<g transform="translate({x} {y}) scale({s})" style="--i:{i}">
		<circle class="glow" cy="-43" r="34" fill="rgb(255 190 100 / .5)" />
		<rect x="-9" y="-14" width="18" height="14" fill="var(--k-stone)" />
		<rect x="-3.5" y="-36" width="7" height="22" fill="var(--k-stone)" />
		<rect x="-10" y="-54" width="20" height="19" fill="var(--k-lamp)" />
		<rect class="fire" x="-6" y="-50" width="12" height="11" fill="var(--k-fire)" />
		<path d="M-18-54L0-68 18-54Z" fill="var(--k-stone)" />
		<circle cy="-70" r="2.6" fill="var(--k-stone)" />
	</g>
{/snippet}

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

			<!-- The landscape: hills, pond, pavilion and its reflection, torii, trees. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<defs>
					<g id="kyoto-pav">
						<rect x="-122" y="-6" width="244" height="6" fill="var(--k-stone)" />
						<rect x="-104" y="-62" width="208" height="56" fill="var(--k-white)" />
						{#each [0, 1, 2, 3, 4, 5, 6, 7, 8] as p (p)}<rect x={-104 + p * 25.5} y="-62" width="4" height="56" fill="var(--k-wood)" />{/each}
						<rect x="-104" y="-16" width="208" height="4" fill="var(--k-wood)" />
						<path d="M-132-56Q-70-58 -52-84H52Q70-58 132-56Q70-52 0-52Q-70-52 -132-56Z" fill="var(--k-roof)" />
						<rect x="-84" y="-128" width="168" height="44" fill="var(--k-gold)" />
						<rect x="-84" y="-128" width="168" height="6" fill="var(--k-gold2)" />
						{#each WINDOWS as wx (wx)}<rect x={wx} y="-114" width="30" height="22" rx="2" class="pav-win" />{/each}
						<path d="M-106-126Q-56-128 -40-154H40Q56-128 106-126Q56-122 0-122Q-56-122 -106-126Z" fill="var(--k-roof)" />
						<rect x="-58" y="-190" width="116" height="36" fill="var(--k-gold)" />
						<rect x="-58" y="-190" width="116" height="5" fill="var(--k-gold2)" />
						<g class="pav-win"><path d="M-40-148V-170Q-30-180 -20-170V-148Z" /><path d="M-10-148V-170Q0-180 10-170V-148Z" /><path d="M20-148V-170Q30-180 40-170V-148Z" /></g>
						<path d="M-86-188Q-40-190 -20-222L-6-246H6L20-222Q40-190 86-188Q40-184 0-184Q-40-184 -86-188Z" fill="var(--k-roof)" />
						<path d="M0-250L-7-262Q0-272 7-262Z" fill="var(--k-gold)" />
						<path d="M0-250V-246" stroke="var(--k-gold2)" stroke-width="2" />
					</g>
					<g id="kyoto-torii">
						<rect x="-42" y="-118" width="11" height="118" />
						<rect x="31" y="-118" width="11" height="118" />
						<path d="M-66-122Q0-110 66-122L70-136Q0-124 -70-136Z" />
						<rect x="-50" y="-98" width="100" height="8" />
						<rect x="-4" y="-116" width="8" height="18" />
					</g>
					<linearGradient id="kyoto-water" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--k-water1)" />
						<stop offset="1" stop-color="var(--k-water2)" />
					</linearGradient>
				</defs>

				<path d="M0 640C140 590 280 610 420 580C580 545 700 600 860 575C1040 545 1180 600 1340 570C1500 540 1660 590 1920 560V800H0Z" fill="var(--k-hill)" />
				<path d="M0 700C200 660 360 690 560 668C760 645 940 700 1140 676C1360 650 1560 700 1920 676V800H0Z" fill="var(--k-hill2)" />
				{#each TREES_BACK as tree (tree.x)}{@render crown(tree)}{/each}

				<!-- pines either side of the pavilion -->
				<g fill="var(--k-pine)">
					<path d="M1130 772V690" stroke="var(--k-trunk)" stroke-width="10" />
					<ellipse cx="1130" cy="668" rx="80" ry="24" /><ellipse cx="1112" cy="632" rx="58" ry="20" /><ellipse cx="1134" cy="600" rx="36" ry="16" fill="var(--k-pine2)" />
					<path d="M1480 772V700" stroke="var(--k-trunk)" stroke-width="10" />
					<ellipse cx="1480" cy="678" rx="70" ry="22" /><ellipse cx="1494" cy="644" rx="50" ry="18" /><ellipse cx="1478" cy="614" rx="30" ry="14" fill="var(--k-pine2)" />
				</g>

				<!-- the pond -->
				<rect x="0" y={HORIZON_Y} width={STAGE_W} height={STAGE_H - HORIZON_Y} fill="url(#kyoto-water)" />
				<g transform="translate({PAVILION.x} {PAVILION.y}) scale({PAVILION.scale} {-PAVILION.scale})" opacity=".26"><use href="#kyoto-pav" /></g>
				<g transform="translate({TORII.x} {TORII.y}) scale({TORII.scale} {-TORII.scale})" opacity=".24" fill="var(--k-torii)"><use href="#kyoto-torii" /></g>
				<g class="ripples" stroke="var(--k-ripple)" stroke-width="3" stroke-linecap="round">
					<path d="M200 800h160M520 836h200M900 790h180M1200 856h240M1560 810h150M1380 920h220M300 900h180M760 960h200M1000 1030h240M1700 940h160" />
				</g>

				<!-- the pavilion's island and the golden pavilion -->
				<rect x={PAVILION.x - 300} y={PAVILION.y - 10} width="600" height="22" rx="10" fill="var(--k-bank)" />
				<g transform="translate({PAVILION.x} {PAVILION.y}) scale({PAVILION.scale})"><use href="#kyoto-pav" /></g>
				<g transform="translate({TORII.x} {TORII.y}) scale({TORII.scale})" fill="var(--k-torii)">
					<use href="#kyoto-torii" />
					<rect x="-44" y="-8" width="15" height="8" fill="var(--k-lamp)" />
					<rect x="29" y="-8" width="15" height="8" fill="var(--k-lamp)" />
				</g>

				<!-- the near bank -->
				<path d="M0 930C300 904 620 946 940 968C1080 976 1160 996 1220 1080H0Z" fill="var(--k-bank)" />
				<path d="M0 1010C300 990 640 1020 960 1040C1040 1046 1090 1060 1120 1080H0Z" fill="var(--k-bank2)" />
				{#each TREES_FRONT as tree (tree.x)}{@render crown(tree)}{/each}

				<!-- Lanterns. Keyed on `pulse.at`, so the same CSS gesture replays. -->
				{#key pulse?.at ?? 0}
					<g class="lamps" class:lamps-down={pulse?.kind === 'down'} class:lamps-up={pulse?.kind === 'up'} class:lamps-resolved={pulse?.kind === 'resolved'}>
						<path d="M{ROPE.x1} {ROPE.y + 60}V{ROPE.y}M{ROPE.x2} {ROPE.y + 60}V{ROPE.y}" stroke="var(--k-wood)" stroke-width="9" stroke-linecap="round" />
						<path d="M{ROPE.x1} {ROPE.y + 4}Q{(ROPE.x1 + ROPE.x2) / 2} {ROPE.y + 48} {ROPE.x2} {ROPE.y + 4}" stroke="var(--k-wood)" stroke-width="4" />
						{#each CHOCHIN as c (c.i)}
							<g transform="translate({c.x} {c.y}) scale(1.6)" style="--i:{c.i}">
								<circle class="glow" r="30" fill="rgb(255 190 100 / .5)" />
								<ellipse rx="9" ry="12" fill="var(--k-chochin)" />
								<rect class="fire" x="-5" y="-4" width="10" height="8" rx="3" fill="var(--k-fire)" />
								<path d="M-5-12h10M-5 12h10" stroke="var(--k-lamp)" stroke-width="2" />
							</g>
						{/each}
						{#each TORO as t, i (i)}{@render toro(t.x, t.y, t.s, i + 5)}{/each}
						<ellipse class="flare" cx={PAVILION.x} cy={PAVILION.y - 260} rx="320" ry="260" fill="var(--k-gold)" />
						{#if pulse?.kind === 'resolved'}
							{#each GUST as g, i (i)}
								<g transform="translate(0 {g.y})">
									<path class="gust" d={fallD} fill="var(--k-fall)" transform="scale({g.size})" style="--dy:{g.dy}px;animation-delay:{g.delay}ms" />
								</g>
							{/each}
						{/if}
					</g>
				{/key}

				<!-- petals / leaves drifting down -->
				{#each FALLING as f, i (i)}
					<g transform="translate({f.x} 0) scale({f.size})">
						<path class="fall" d={fallD} fill="var(--k-fall)" style="--dx:{f.dx}px;animation-duration:{f.dur}s;animation-delay:{f.delay}s" />
					</g>
				{/each}

				<!-- a koi rings the water now and then -->
				{#key ringKey}
					{#if ringKey > 0}
						<g transform="translate({RINGS[ringAt].x} {RINGS[ringAt].y})">
							<ellipse class="ring" rx="30" ry="8" stroke="var(--k-ripple)" stroke-width="3" />
							<ellipse class="ring ring--late" rx="30" ry="8" stroke="var(--k-ripple)" stroke-width="3" />
						</g>
					{/if}
				{/key}
			</svg>

			<!-- The pigeon, on the near bank. -->
			<div class="cast" style:left="{PIGEON_FEET.x - 75}px" style:top="{PIGEON_FEET.y - 187}px">
				<WallPigeon class="pigeon" {sign} umbrella={raining} {lookX} {lookY} {eyes} blinkOffset={1.6} />
			</div>

			{#if raining}
				<div class="rain">
					<svg viewBox="0 0 1920 1620">
						<path d={RAIN} stroke="var(--rain)" stroke-width="2.4" stroke-linecap="round" />
					</svg>
				</div>
				<div class="veil"></div>
				<div class="flash" class:flash--on={flash}></div>
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
		background: linear-gradient(to bottom, var(--sky-top) 0%, var(--sky-bottom) 70%);
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

	/* The temple at night: lit windows, flickering flames. */
	.layer :global(.pav-win) {
		fill: var(--k-win);
	}
	.glow {
		opacity: var(--k-glow-o);
	}
	.fire {
		animation: flicker 3.4s ease-in-out infinite;
	}
	@keyframes flicker {
		0%, 100% { opacity: 1; }
		40% { opacity: 0.8; }
		70% { opacity: 0.92; }
	}
	.sway {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: sway 9s ease-in-out infinite alternate;
	}
	@keyframes sway {
		to {
			transform: rotate(1.4deg);
		}
	}
	.ripples {
		opacity: 0.8;
		animation: ripples 14s ease-in-out infinite alternate;
	}
	@keyframes ripples {
		to {
			transform: translateX(30px);
		}
	}

	/* Falling petals: each sits in a positioned group, so the keyframes are relative. */
	.fall {
		animation: fall linear infinite;
	}
	@keyframes fall {
		from {
			transform: translate(0, 40px) rotate(0);
			opacity: 0;
		}
		8%,
		92% {
			opacity: 0.85;
		}
		to {
			transform: translate(var(--dx), 1000px) rotate(360deg);
			opacity: 0;
		}
	}
	.ring {
		fill: none;
		transform-box: fill-box;
		transform-origin: center;
		animation: ring 3.6s ease-out both;
	}
	.ring--late {
		animation-delay: 0.7s;
	}
	@keyframes ring {
		from {
			transform: scale(0.2);
			opacity: 0.9;
		}
		to {
			transform: scale(2.2);
			opacity: 0;
		}
	}

	/* Lantern gestures. */
	.flare {
		opacity: 0;
	}
	.lamps :global(.glow),
	.lamps :global(.fire) {
		transform-box: fill-box;
		transform-origin: center;
	}
	.lamps-down :global(.glow),
	.lamps-down :global(.fire) {
		animation: lamp-off 4.4s ease-in-out calc(var(--i, 0) * 0.22s) backwards;
	}
	.lamps-up :global(.glow),
	.lamps-resolved :global(.glow) {
		animation: lamp-on 3s ease-out calc(var(--i, 0) * 0.16s) backwards;
	}
	.lamps-up :global(.fire) {
		animation: lamp-on-fire 3s ease-out calc(var(--i, 0) * 0.16s) backwards;
	}
	.lamps-up .flare {
		animation: flare 3.6s ease-out 1.2s both;
	}
	.lamps-resolved .flare {
		animation: flare 3.2s ease-out 0.4s both;
	}
	@keyframes lamp-off {
		0% {
			opacity: 1;
		}
		12%,
		70% {
			opacity: 0;
		}
		100% {
			opacity: 1;
		}
	}
	@keyframes lamp-on {
		0% {
			opacity: 0;
			transform: scale(0.4);
		}
		35% {
			opacity: 1;
			transform: scale(1.7);
		}
		100% {
			opacity: var(--k-glow-o);
			transform: scale(1);
		}
	}
	@keyframes lamp-on-fire {
		0% {
			opacity: 0;
		}
		35%,
		100% {
			opacity: 1;
		}
	}
	@keyframes flare {
		0% {
			opacity: 0;
		}
		30% {
			opacity: 0.32;
		}
		100% {
			opacity: 0;
		}
	}
	.gust {
		animation: gust 5.5s ease-in-out both;
	}
	@keyframes gust {
		from {
			transform: translate(-60px, 0) rotate(0);
			opacity: 0;
		}
		10% {
			opacity: 1;
		}
		to {
			transform: translate(2000px, var(--dy)) rotate(720deg);
			opacity: 0;
		}
	}

	.cast {
		position: absolute;
	}
	.cast :global(.pigeon) {
		display: block;
		width: 150px;
	}

	/* Rain, veil and lightning. */
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
	}
	.flash--on {
		animation: flash 0.9s linear;
	}
	@keyframes flash {
		0%, 100% { opacity: 0; }
		8% { opacity: 0.22; }
		20% { opacity: 0.03; }
		32% { opacity: 0.16; }
		60% { opacity: 0; }
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
	.scene--still .fall,
	.scene--still .gust {
		display: none;
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
		.fall,
		.gust,
		.flash {
			display: none;
		}
	}
</style>
