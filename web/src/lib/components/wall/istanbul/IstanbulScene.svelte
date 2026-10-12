<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop: Istanbul across the Bosphorus. A drop-in for
	 * the Paris scene (same props). Galata tower on its hill, an imperial
	 * mosque with four minarets and the lamps (mahya) strung between the tall
	 * two, a second mosque, shore houses, the Asian side in haze, a ferry
	 * crossing, gulls gliding, and the DumbMonit pigeon on the quay railing.
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view). The sky follows the real hour
	 * inside the brightness band of the wall's theme (see `paris/daylight.ts`);
	 * after dark the mosque is floodlit, the lamps and windows glow.
	 *
	 * Weather: `mood` accepts the wall's `calm | clouded | storm` or the
	 * status vocabulary `ok | warn | critical`; a storm brings rain, a dark
	 * sky and, unless motion is reduced, a few lightning flashes.
	 *
	 * `pulse` plays a short gesture whenever its `at` changes: on `down` the
	 * mahya lamps go dark one after the other from the middle out; on `up`
	 * they relight in a sweep from left to right; on `resolved` the ferry
	 * blows its horn in puffs of steam and a flock of gulls bursts skyward.
	 *
	 * Everything moves with transform and opacity only. Hidden tabs pause,
	 * reduced motion freezes the scene in a still pose.
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
		SHORE,
		QUAY_Y,
		ASIAN_BACK,
		ASIAN_FRONT,
		FAR_MINARETS,
		MX,
		SX,
		GX,
		MOSQUE,
		MOSQUE2,
		GALATA,
		HOUSES,
		MAHYA,
		MAHYA_WIRE,
		GLINTS,
		RAILING,
		STARS,
		RAIN,
		litWindows,
		istanbulStyle
	} from './geometry.js';

	type Weather = SceneMood | 'ok' | 'warn' | 'critical';
	type PulseKind = 'down' | 'up' | 'resolved';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood?: Weather;
		/** Problems on the board: the number on the pigeon's sign. */
		problems?: number;
		/** Bumped by the wall when a new problem appears: the pigeon startles once. */
		startle?: number;
		/** The tab is hidden: nothing moves. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		/** A short gesture, replayed whenever `at` changes. */
		pulse?: { kind: PulseKind; at: number } | null;
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

	// --- Stage fit ---------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light and weather --------------------------------------------------------

	const weather = $derived<SceneMood>(
		mood === 'storm' || mood === 'critical'
			? 'storm'
			: mood === 'clouded' || mood === 'warn'
				? 'clouded'
				: 'calm'
	);
	const raining = $derived(weather === 'storm');
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const style = $derived(`${sceneStyle(theme, phase, weather)};${istanbulStyle(theme)}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));

	let windowSeed = $state(1);
	const mosqueLit = $derived(lit ? litWindows(windowSeed, MOSQUE.windows.length, 0.7) : new Set<number>());
	const secondLit = $derived(lit ? litWindows(windowSeed + 3, MOSQUE2.windows.length, 0.6) : new Set<number>());
	$effect(() => {
		if (!lit || paused || !motion) return;
		const timer = setInterval(() => (windowSeed += 1), 120_000);
		return () => clearInterval(timer);
	});

	const cloudCount = $derived(weather === 'storm' ? 7 : weather === 'clouded' ? 5 : 2);
	const CLOUDS = [
		{ y: 120, s: 1.1, dur: 260, delay: -40 },
		{ y: 300, s: 0.8, dur: 320, delay: -210 },
		{ y: 470, s: 0.6, dur: 360, delay: -120 },
		{ y: 200, s: 0.95, dur: 290, delay: -150 },
		{ y: 400, s: 1.25, dur: 340, delay: -290 },
		{ y: 60, s: 0.7, dur: 300, delay: -250 },
		{ y: 540, s: 1.0, dur: 380, delay: -60 }
	];

	// --- The pigeon --------------------------------------------------------------

	const trouble = $derived(problems > 0);
	const PIGEON_X = 1470;
	let look = $state(0);
	let startled = $state(false);
	let spooked = $state(false);

	// A glance around the harbour now and then.
	$effect(() => {
		if (paused || !motion) {
			look = trouble ? -0.8 : 0;
			return;
		}
		if (trouble) {
			look = -0.8;
			return;
		}
		const timer = setInterval(() => {
			look = [-0.8, 0.6, 0.1, -0.4][Math.floor(Math.random() * 4)];
		}, 6500);
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

	const sign = $derived(
		trouble ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null
	);

	// --- The pulse ---------------------------------------------------------------

	let active = $state<PulseKind | null>(null);
	let lastAt = untrack(() => pulse?.at ?? null);
	let frame = 0;
	let done: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const p = pulse;
		if (!p || p.at === lastAt) return;
		lastAt = p.at;
		if (!motion) return;
		active = null;
		spooked = p.kind === 'down';
		cancelAnimationFrame(frame);
		clearTimeout(done);
		frame = requestAnimationFrame(() => (active = p.kind));
		done = setTimeout(() => {
			active = null;
			spooked = false;
		}, 7500);
	});

	/** Gulls of the resolved flock: direction and delay of each. */
	const FLOCK = [
		{ dx: -620, dy: -520, delay: 0, s: 1 },
		{ dx: -380, dy: -640, delay: 120, s: 0.9 },
		{ dx: -860, dy: -380, delay: 220, s: 1.1 },
		{ dx: -140, dy: -700, delay: 300, s: 0.8 },
		{ dx: -1000, dy: -560, delay: 420, s: 0.95 },
		{ dx: -520, dy: -330, delay: 520, s: 0.85 },
		{ dx: -250, dy: -460, delay: 640, s: 1 }
	];

	/** Gulls on patrol: height, size, crossing time, start offset, direction. */
	const GULLS = [
		{ y: 330, s: 1, dur: 46, delay: -12, flip: false },
		{ y: 520, s: 0.8, dur: 62, delay: -40, flip: true },
		{ y: 420, s: 0.65, dur: 74, delay: -58, flip: false }
	];
</script>

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
					<circle
						cx={star.x}
						cy={star.y}
						r={star.r}
						fill="var(--star)"
						class:twinkle={star.twinkle}
						style:animation-delay="-{star.delay}ms"
					/>
				{/each}
			</svg>

			<div
				class="celestial"
				class:celestial--hidden={raining}
				style:transform="translate({body.x - 160}px, {body.y - 160}px)"
			>
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

			{#if lit}
				<div class="glow flood" style:left="{MX - 330}px" style:top="{SHORE - 280}px"></div>
				<div class="glow flood flood-b" style:left="{SX - 170}px" style:top="{SHORE - 150}px"></div>
			{/if}

			<!-- The shores, the city, the water. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<defs>
					<clipPath id="ist-water"><rect x="0" y={SHORE + 14} width={STAGE_W} height={QUAY_Y - SHORE - 14} /></clipPath>
					<linearGradient id="ist-sea" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--water)" />
						<stop offset="1" stop-color="var(--water2)" />
					</linearGradient>
					<g id="ist-city">
						<!-- Galata tower on its hill -->
						<path d={GALATA.hill} fill="var(--hill)" />
						<path d={GALATA.shaft} fill="var(--wall)" />
						<path d={GALATA.drum} fill="var(--wall2)" />
						<path d={GALATA.gallery} fill="var(--wall)" />
						<path d={GALATA.cone} fill="var(--cone)" />
						<path d={GALATA.spire} fill="var(--cone)" />
						<g fill={lit ? 'var(--window-lit)' : 'var(--window)'}>
							{#each GALATA.windows as w, i (i)}
								<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="2" />
							{/each}
						</g>

						<!-- The imperial mosque -->
						<path d="M{MX - 400} {SHORE + 14} Q{MX - 200} {SHORE - 30} {MX} {SHORE - 40} Q{MX + 220} {SHORE - 30} {MX + 400} {SHORE + 14} Z" fill="var(--hill)" />
						<path d={MOSQUE.body} fill="var(--wall)" />
						<path d={MOSQUE.arcade} fill="var(--wall2)" />
						<path d={MOSQUE.halves} fill="var(--lead2)" />
						<path d={MOSQUE.dome} fill="var(--lead)" />
						<path d={MOSQUE.domeShade} fill="var(--wall2)" opacity="0.35" />
						<path d={MOSQUE.finial} fill="var(--gold)" />
						{#each [...MOSQUE.tall, ...MOSQUE.short] as mi (mi.x)}
							<path d={mi.shaft} fill="var(--wall)" />
							<path d={mi.balconies} fill="var(--wall2)" />
							<path d={mi.cap} fill="var(--cone)" />
							<path d={mi.crescent} fill="var(--gold)" />
						{/each}
						<g fill="var(--window)">
							{#each MOSQUE.windows as w, i (i)}
								<rect x={w.x} y={w.y} width={w.w} height={w.h} rx={w.w / 2} />
							{/each}
						</g>
						<g fill="var(--window-lit)">
							{#each MOSQUE.windows as w, i (i)}
								{#if mosqueLit.has(i)}<rect x={w.x} y={w.y} width={w.w} height={w.h} rx={w.w / 2} />{/if}
							{/each}
						</g>

						<!-- The second mosque -->
						<path d={MOSQUE2.body} fill="var(--wall)" />
						<path d={MOSQUE2.halves} fill="var(--lead2)" />
						<path d={MOSQUE2.dome} fill="var(--lead)" />
						<path d={MOSQUE2.finial} fill="var(--gold)" />
						{#each MOSQUE2.minarets as mi (mi.x)}
							<path d={mi.shaft} fill="var(--wall)" />
							<path d={mi.balconies} fill="var(--wall2)" />
							<path d={mi.cap} fill="var(--cone)" />
							<path d={mi.crescent} fill="var(--gold)" />
						{/each}
						<g fill="var(--window)">
							{#each MOSQUE2.windows as w, i (i)}
								<rect x={w.x} y={w.y} width={w.w} height={w.h} rx={w.w / 2} />
							{/each}
						</g>
						<g fill="var(--window-lit)">
							{#each MOSQUE2.windows as w, i (i)}
								{#if secondLit.has(i)}<rect x={w.x} y={w.y} width={w.w} height={w.h} rx={w.w / 2} />{/if}
							{/each}
						</g>

						<!-- Shore houses -->
						{#each HOUSES.walls as house, i (i)}
							<path d={house.d} fill={['var(--h1)', 'var(--h2)', 'var(--h3)'][house.tone]} />
						{/each}
						<path d={HOUSES.roofs} fill="var(--roofh)" />
					</g>
				</defs>

				<!-- the Asian shore in haze -->
				<path d={ASIAN_BACK} fill="var(--haze2)" />
				<g fill="var(--haze2)">
					{#each FAR_MINARETS as [x, base, h], i (i)}
						<path d="M{x - 3} {base} L{x - 2} {base - h} L{x} {base - h - 16} L{x + 2} {base - h} L{x + 3} {base} Z" />
					{/each}
				</g>
				<path d={ASIAN_FRONT} fill="var(--haze)" />

				<use href="#ist-city" />

				<!-- the mahya: lamps strung between the tall minarets -->
				<path d={MAHYA_WIRE} stroke="var(--wall2)" stroke-width="1.6" fill="none" opacity="0.7" />
				<g class="mahya" class:mahya--lit={lit} class:pulse-down={active === 'down'} class:pulse-up={active === 'up'}>
					{#each MAHYA as lamp (lamp.k)}
						<g class="bulb" style="--dd:{lamp.ring * 110}ms;--du:{lamp.k * 95}ms">
							{#if lit}<circle cx={lamp.x} cy={lamp.y} r="13" fill="var(--glow)" />{/if}
							<circle cx={lamp.x} cy={lamp.y} r="4.2" fill={lit ? 'var(--bulb)' : 'var(--bulb-off)'} />
						</g>
					{/each}
				</g>

				<!-- the water -->
				<rect x="0" y={SHORE + 14} width={STAGE_W} height={STAGE_H - SHORE - 14} fill="url(#ist-sea)" />
				<g clip-path="url(#ist-water)">
					<use href="#ist-city" transform="translate(0 {2 * (SHORE + 14)}) scale(1 -1)" opacity="0.2" />
				</g>
				<g stroke="var(--cloud)" stroke-width="3" stroke-linecap="round">
					{#each GLINTS as g, i (i)}
						<path d="M{g.x} {g.y} h{g.w}" class="glint" style:animation-delay="-{g.delay}ms" />
					{/each}
				</g>
			</svg>

			<!-- The ferry, crossing and crossing back. -->
			<div class="ferry" style:top="{SHORE + 70}px">
				<svg viewBox="0 0 320 120">
					<path d="M6 70 H312 L292 104 H34 Z" fill="var(--hull)" />
					<path d="M20 90 H302 L296 98 H28 Z" fill="var(--hull2)" />
					<path d="M60 70 V44 H258 V70 Z" fill="var(--hull)" />
					<path d="M92 44 V28 H210 V44 Z" fill="var(--hull)" />
					<g fill={lit ? 'var(--window-lit)' : 'var(--window)'}>
						{#each [0, 1, 2, 3, 4, 5, 6, 7, 8] as i (i)}
							<rect x={72 + i * 20} y="52" width="12" height="9" rx="2" />
						{/each}
						{#each [0, 1, 2, 3, 4] as i (i)}
							<rect x={102 + i * 20} y="34" width="12" height="7" rx="2" />
						{/each}
					</g>
					<rect x="146" y="6" width="22" height="24" fill="var(--hull2)" />
					<rect x="146" y="12" width="22" height="5" fill="var(--hull)" />
					<path d="M232 28 V8 l22 5 l-22 5" fill="var(--hull2)" />
				</svg>
				{#if active === 'resolved'}
					<div class="steam">
						{#each [0, 1, 2] as i (i)}
							<span style="animation-delay:{i * 420}ms"></span>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Gulls on patrol. -->
			{#each GULLS as gull, i (i)}
				<div
					class="gull"
					class:gull--flip={gull.flip}
					style:--still-x="{300 + i * 520}px"
					style:top="{gull.y}px"
					style:width="{64 * gull.s}px"
					style:animation-duration="{gull.dur}s"
					style:animation-delay="{gull.delay}s"
				>
					<svg viewBox="0 0 64 30" class="gull-bob" style:animation-delay="{i * -1.3}s">
						<path class="wings" d="M2 18 Q16 0 32 15 Q48 0 62 18 Q48 11 32 22 Q16 11 2 18 Z" fill="var(--gull)" stroke="var(--gull-line)" stroke-width="1.2" stroke-linejoin="round" />
					</svg>
				</div>
			{/each}

			<!-- The quay and its railing. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<rect x="0" y={QUAY_Y} width={STAGE_W} height={STAGE_H - QUAY_Y} fill="var(--quay)" />
				<rect x="0" y={QUAY_Y} width={STAGE_W} height="12" fill="var(--quay2)" />
				<path d={RAILING} stroke="var(--quay2)" stroke-width="5" stroke-linecap="round" />
				<rect x="0" y={QUAY_Y - 10} width={STAGE_W} height="7" rx="3" fill="var(--quay2)" />
				<path d="M{PIGEON_X + 150} {QUAY_Y + 4} L{PIGEON_X + 154} {QUAY_Y - 30} H{PIGEON_X + 182} L{PIGEON_X + 186} {QUAY_Y + 4} Z" fill="var(--lead)" />
				<rect x={PIGEON_X + 148} y={QUAY_Y - 36} width="40" height="8" rx="4" fill="var(--lead2)" />
			</svg>

			<div class="cast" style:left="{PIGEON_X - 75}px" style:top="{QUAY_Y + 4 - 187}px">
				<WallPigeon
					class="pigeon"
					{sign}
					lookX={look}
					lookY={trouble ? -0.3 : 0}
					eyes={startled || spooked ? 'startled' : 'open'}
					blinkOffset={1.4}
				/>
			</div>

			<!-- The flock that rises when an incident is resolved. -->
			{#if active === 'resolved'}
				{#each FLOCK as bird, i (i)}
					<div
						class="gull burst"
						style:left="{PIGEON_X - 220}px"
						style:top="{QUAY_Y - 20}px"
						style:width="{56 * bird.s}px"
						style:--dx="{bird.dx}px"
						style:--dy="{bird.dy}px"
						style:animation-delay="{bird.delay}ms"
					>
						<svg viewBox="0 0 64 30">
							<path class="wings" d="M2 18 Q16 0 32 15 Q48 0 62 18 Q48 11 32 22 Q16 11 2 18 Z" fill="var(--gull)" stroke="var(--gull-line)" stroke-width="1.2" stroke-linejoin="round" />
						</svg>
					</div>
				{/each}
			{/if}

			{#if raining}
				<div class="rain">
					<svg viewBox="0 0 1920 1620">
						<path d={RAIN} stroke="var(--rain)" stroke-width="2.4" stroke-linecap="round" />
					</svg>
				</div>
				<div class="veil"></div>
				<div class="flash"></div>
				<svg class="bolt" viewBox="0 0 1920 1080">
					<path d="M{GX + 880} 40 L{GX + 840} 250 L{GX + 884} 244 L{GX + 820} 470 L{GX + 930} 220 L{GX + 886} 226 Z" fill="#fff" />
				</svg>
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

	.glow {
		position: absolute;
		border-radius: 50%;
		background: radial-gradient(circle, var(--glow) 0%, transparent 70%);
	}
	.flood {
		width: 660px;
		height: 420px;
		animation: breathe 11s ease-in-out infinite;
	}
	.flood-b {
		width: 340px;
		height: 260px;
	}
	@keyframes breathe {
		50% {
			opacity: 0.7;
		}
	}

	.glint {
		opacity: 0.25;
		animation: glint 5s ease-in-out infinite;
	}
	@keyframes glint {
		50% {
			opacity: 0.02;
		}
	}

	/* The mahya. Unlit by day it is a faint garland; lit at night it glows. */
	.bulb {
		--base: 0.55;
		opacity: var(--base);
		transform-box: fill-box;
		transform-origin: center;
	}
	.mahya--lit .bulb {
		--base: 1;
	}
	.pulse-down .bulb {
		animation: lamp-down 5.6s var(--dd) both;
	}
	.pulse-up .bulb {
		animation: lamp-up 3.4s var(--du) both;
	}
	@keyframes lamp-down {
		0% {
			opacity: var(--base);
		}
		10%,
		78% {
			opacity: 0.04;
		}
		100% {
			opacity: var(--base);
		}
	}
	@keyframes lamp-up {
		0%,
		5% {
			opacity: 0.04;
			transform: scale(1);
		}
		18% {
			opacity: 1;
			transform: scale(2.1);
		}
		45%,
		100% {
			opacity: var(--base);
			transform: scale(1);
		}
	}

	.ferry {
		position: absolute;
		left: 0;
		width: 320px;
		animation: cross 95s linear infinite;
		animation-delay: -30s;
		will-change: transform;
	}
	.ferry svg {
		display: block;
		width: 100%;
	}
	@keyframes cross {
		from {
			transform: translateX(-380px);
		}
		to {
			transform: translateX(2000px);
		}
	}
	.steam {
		position: absolute;
		left: 150px;
		top: -4px;
	}
	.steam span {
		position: absolute;
		width: 26px;
		height: 26px;
		margin-left: -13px;
		border-radius: 50%;
		background: var(--cloud);
		opacity: 0;
		animation: puff 2.4s ease-out 2 both;
	}
	@keyframes puff {
		0% {
			opacity: 0.9;
			transform: translate(0, 0) scale(0.5);
		}
		100% {
			opacity: 0;
			transform: translate(24px, -120px) scale(3);
		}
	}

	.gull {
		position: absolute;
		left: 0;
		animation: glide linear infinite;
		will-change: transform;
	}
	.gull--flip {
		animation-direction: reverse;
	}
	.gull svg {
		display: block;
		width: 100%;
	}
	.gull-bob {
		animation: bob 5.5s ease-in-out infinite;
	}
	.gull .wings {
		transform-box: fill-box;
		transform-origin: center;
		animation: flap 1.7s ease-in-out infinite;
	}
	@keyframes glide {
		from {
			transform: translateX(-120px);
		}
		to {
			transform: translateX(2040px);
		}
	}
	@keyframes bob {
		50% {
			transform: translateY(-26px);
		}
	}
	@keyframes flap {
		50% {
			transform: scaleY(0.45);
		}
	}
	.burst {
		animation: burst 4.6s cubic-bezier(0.3, 0.1, 0.5, 1) both;
	}
	.burst .wings {
		animation-duration: 0.45s;
	}
	@keyframes burst {
		0% {
			opacity: 0;
			transform: translate(0, 0);
		}
		8% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(var(--dx), var(--dy));
		}
	}

	.cast {
		position: absolute;
	}
	.cast :global(.pigeon) {
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
	.flash,
	.bolt {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		opacity: 0;
		animation: flash 12s linear infinite;
	}
	.flash {
		background: rgb(235 240 255 / 0.5);
	}
	.bolt {
		animation-name: flash-bolt;
	}
	@keyframes flash {
		0%,
		90% {
			opacity: 0;
		}
		91% {
			opacity: 0.8;
		}
		93% {
			opacity: 0.1;
		}
		94% {
			opacity: 0.6;
		}
		98%,
		100% {
			opacity: 0;
		}
	}
	@keyframes flash-bolt {
		0%,
		90.5% {
			opacity: 0;
		}
		91% {
			opacity: 0.95;
		}
		92.5% {
			opacity: 0;
		}
		94% {
			opacity: 0.7;
		}
		95%,
		100% {
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
	.scene--still .cloud,
	.scene--still .gull {
		transform: translateX(var(--still-x));
	}
	.scene--still .ferry {
		transform: translateX(700px);
	}
	.scene--still .flash,
	.scene--still .bolt {
		opacity: 0;
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
		.flash,
		.bolt {
			opacity: 0;
		}
		.cloud,
		.gull {
			transform: translateX(var(--still-x, 400px));
		}
		.ferry {
			transform: translateX(700px);
		}
	}
</style>
