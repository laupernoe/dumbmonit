<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop: Marrakech at the edge of Jemaa el-Fna — the
	 * Koutoubia minaret over the mosque wings, palms, red ramparts, the snowy
	 * Atlas behind, striped market stalls, grill smoke, and strings of brass
	 * lanterns that come up as the evening does. One DumbMonit pigeon stands on
	 * the terrace wall. A drop-in for the Rome scene (same props), plus `pulse`.
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky
	 * follows the real hour, inside the brightness band of the wall's theme
	 * (see `paris/daylight.ts`).
	 *
	 * The network's state is woven in. `mood` is the weather: calm, clouds with
	 * advisories ('warn'), a storm with rain and lightning when critical. With
	 * problems on the board the pigeon holds up a sign. A new problem startles
	 * it once.
	 *
	 * `pulse` is the lanterns' gesture, replayed whenever `at` changes: on
	 * `down` they go out one after another along the square, on `up` they
	 * relight in the same wave with a bloom, on `resolved` a stork glides over.
	 *
	 * Motion is gentle (clouds, smoke, a stroll, a peck, lantern flicker), all
	 * transform and opacity. Hidden tabs pause; reduced motion freezes it in a
	 * still pose (lantern gesture reduced to a dimming).
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
		LEDGE_Y,
		ATLAS_FAR,
		ATLAS_NEAR,
		SNOW_FAR,
		SNOW_NEAR,
		KOUTOUBIA,
		KOUTOUBIA_TOP,
		MOSQUE,
		RAMPART,
		PALMS,
		AWNING_STRIPES,
		AWNING_VALANCE,
		GOODS,
		STALLS,
		SMOKES,
		POLES,
		STRINGS,
		LANTERNS,
		BOLT,
		STARS,
		RAIN,
		litWindows,
		marrakechStyle
	} from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		/** Weather: the wall's own moods, or the status vocabulary (ok / warn / critical). */
		mood?: SceneMood | 'ok' | 'warn' | 'critical';
		/** Problems on the board: the number on the protest sign. */
		problems: number;
		/** Bumped by the wall when a new problem appears: the pigeon startles once. */
		startle?: number;
		/** The tab is hidden: nothing moves, nothing is scheduled. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		/** A gesture of the lanterns, replayed whenever `at` changes. */
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
		class?: string;
	}

	let {
		theme,
		now,
		mood = 'ok',
		problems,
		startle = 0,
		paused = false,
		focus = 0,
		pulse = null,
		class: className = ''
	}: Props = $props();

	const motion = !reducedMotion();
	const weather = $derived<SceneMood>(
		mood === 'critical' || mood === 'storm' ? 'storm' : mood === 'warn' || mood === 'clouded' ? 'clouded' : 'calm'
	);

	// --- Stage fit ----------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light --------------------------------------------------------------------

	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const style = $derived(`${sceneStyle(theme, phase, weather)};${marrakechStyle(theme, lit)}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));

	let windowSeed = $state(1);
	const litSet = $derived(lit ? litWindows(windowSeed, KOUTOUBIA.windows.length, 0.5) : new Set<number>());

	const trouble = $derived(problems > 0);
	const raining = $derived(weather === 'storm');
	const cloudCount = $derived(weather === 'storm' ? 7 : weather === 'clouded' ? 5 : 3);
	const CLOUDS = [
		{ y: 120, s: 1.1, dur: 260, delay: -40 },
		{ y: 300, s: 0.8, dur: 320, delay: -210 },
		{ y: 470, s: 0.6, dur: 360, delay: -120 },
		{ y: 200, s: 0.95, dur: 290, delay: -150 },
		{ y: 400, s: 1.25, dur: 340, delay: -290 },
		{ y: 60, s: 0.7, dur: 300, delay: -250 },
		{ y: 540, s: 1.0, dur: 380, delay: -60 }
	];

	// --- The pigeon ----------------------------------------------------------------

	const STROLL = [880, 1240];
	const SIGN_SPOT = 900;
	let pigeonX = $state(1040);
	let walkMs = $state(0);
	let walking = $state(false);
	let look = $state(0);
	let pecking = $state(false);
	let startled = $state(false);

	let epoch = 0;
	function later(fn: () => void, ms: number) {
		const mine = epoch;
		setTimeout(() => {
			if (mine === epoch) fn();
		}, ms);
	}
	const between = (a: number, b: number) => a + Math.random() * (b - a);

	function stroll() {
		if (trouble) return;
		const target = Math.round(between(STROLL[0], STROLL[1]));
		const distance = Math.abs(target - pigeonX);
		if (distance < 60) {
			later(stroll, 1500);
			return;
		}
		look = target > pigeonX ? 0.7 : -0.7;
		walkMs = Math.round((distance / 34) * 1000);
		walking = true;
		pigeonX = target;
		later(() => {
			walking = false;
			look = 0;
			if (Math.random() < 0.5) {
				later(() => (pecking = true), 1200);
				later(() => (pecking = false), 2600);
			}
			later(stroll, between(5000, 11000));
		}, walkMs);
	}

	function toSignSpot() {
		const distance = Math.abs(SIGN_SPOT - pigeonX);
		look = -0.8;
		walkMs = Math.round((distance / 60) * 1000);
		walking = distance > 4;
		pigeonX = SIGN_SPOT;
		later(() => (walking = false), walkMs);
	}

	// (Re)start the cast whenever the trouble flag or the pause changes.
	$effect(() => {
		void trouble;
		const hidden = paused;
		untrack(() => {
			epoch++;
			pecking = false;
			if (hidden) return;
			if (trouble) toSignSpot();
			else if (motion) later(stroll, between(1500, 4000));
		});
		return () => untrack(() => epoch++);
	});

	// Lit windows change now and then while the city is lit.
	$effect(() => {
		if (!lit || paused || !motion) return;
		const timer = setInterval(() => (windowSeed += 1), 120_000);
		return () => clearInterval(timer);
	});

	let lastStartle = untrack(() => startle);
	$effect(() => {
		const value = startle;
		if (value === lastStartle) return;
		lastStartle = value;
		untrack(() => flinch());
	});
	function flinch() {
		startled = true;
		setTimeout(() => (startled = false), 1800);
	}

	// --- The lanterns' gesture ------------------------------------------------------

	let pulseKind = $state<'down' | 'up' | 'resolved' | null>(null);
	let pulseKey = $state(0);
	let stork = $state(false);
	/** Reduced motion: the lanterns just dim for a few seconds. */
	let dimmed = $state(false);

	let lastAt = untrack(() => pulse?.at);
	$effect(() => {
		const current = pulse;
		if (!current || current.at === lastAt) return;
		lastAt = current.at;
		untrack(() => {
			pulseKind = current.kind;
			pulseKey += 1;
			if (current.kind === 'down') {
				flinch();
				if (!motion) {
					dimmed = true;
					setTimeout(() => (dimmed = false), 6000);
				}
			}
			if (current.kind === 'resolved' && motion) {
				stork = true;
				setTimeout(() => (stork = false), 13000);
			}
		});
	});

	const sign = $derived(trouble ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null);
	const eyes = $derived(startled ? 'startled' : 'open');
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

			{#if lit}
				<div class="glow beacon" style:left="{KOUTOUBIA_TOP.x - 60}px" style:top="{KOUTOUBIA_TOP.y - 40}px"></div>
				<div class="glow square" style:left="560px" style:top="760px"></div>
				<div class="glow square" style:left="1240px" style:top="760px"></div>
			{/if}

			<!-- The city: the Atlas, ramparts, palms, the mosque and its minaret. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<defs>
					<clipPath id="mk-far"><path d={ATLAS_FAR} /></clipPath>
					<clipPath id="mk-near"><path d={ATLAS_NEAR} /></clipPath>
					<radialGradient id="mk-bloom">
						<stop offset="0" stop-color="var(--lamp-on)" stop-opacity="0.9" />
						<stop offset="0.35" stop-color="var(--lamp-on)" stop-opacity="0.3" />
						<stop offset="1" stop-color="var(--lamp-on)" stop-opacity="0" />
					</radialGradient>
				</defs>

				<path d={ATLAS_FAR} fill="var(--mount-far)" />
				<g clip-path="url(#mk-far)"><path d={SNOW_FAR} fill="var(--snow)" opacity="0.9" /></g>
				<path d={ATLAS_NEAR} fill="var(--mount)" />
				<g clip-path="url(#mk-near)"><path d={SNOW_NEAR} fill="var(--snow)" opacity="0.8" /></g>
				<rect x="0" y="780" width={STAGE_W} height="60" fill="var(--sand)" opacity="0.5" />

				<!-- ramparts -->
				<path d={RAMPART.body} fill="var(--stone-shade)" />
				<path d={RAMPART.merlons} fill="var(--stone-shade)" />
				<path d={RAMPART.towers} fill="var(--stone)" />

				<!-- mosque wings -->
				<path d={MOSQUE.body} fill="var(--stone)" />
				<path d={MOSQUE.merlons} fill="var(--stone)" />
				<path d={MOSQUE.arches} fill="var(--stone-shade)" />

				<!-- the Koutoubia -->
				<path d={KOUTOUBIA.shaft} fill="var(--stone)" />
				<path d={KOUTOUBIA.shade} fill="var(--stone-shade)" opacity="0.55" />
				<path d={KOUTOUBIA.merlons} fill="var(--stone)" />
				<path d={KOUTOUBIA.band} fill="none" stroke="var(--stone-shade)" stroke-width="2" />
				<path d={KOUTOUBIA.tower} fill="var(--stone)" />
				<path d={KOUTOUBIA.towerMerlons} fill="var(--stone)" />
				<path d={KOUTOUBIA.cupola} fill="var(--stone-shade)" />
				<path d={KOUTOUBIA.finial} stroke="var(--gold)" stroke-width="2.4" />
				{#each KOUTOUBIA.balls as ball (ball.y)}
					<circle cx={KOUTOUBIA_TOP.x} cy={ball.y} r={ball.r} fill="var(--gold)" />
				{/each}
				<g fill="var(--stone-shade)">
					{#each KOUTOUBIA.windows as w, i (i)}
						<rect x={w.x} y={w.y} width={w.w} height={w.h} rx={w.w / 2} />
					{/each}
					<path d={KOUTOUBIA.door} />
				</g>
				<g fill="var(--lamp-on)">
					{#each KOUTOUBIA.windows as w, i (i)}
						{#if litSet.has(i)}
							<rect class="lit" x={w.x + 3} y={w.y + 3} width={w.w - 6} height={w.h - 4} rx={(w.w - 6) / 2} opacity="0.85" />
						{/if}
					{/each}
				</g>

				<!-- palms -->
				{#each PALMS as p, i (i)}
					<path d={p.trunk} fill="var(--trunk)" />
					<path d={p.fronds} fill="var(--palm)" />
				{/each}

				<!-- the square -->
				<rect x="0" y="800" width={STAGE_W} height="280" fill="var(--sand)" />
				<path d="M0 840 C400 826 900 832 1300 842 C1600 828 1800 832 1920 840 V1080 H0Z" fill="var(--sand-2)" opacity="0.7" />

				<!-- stalls, far row first -->
				{#each [...STALLS].sort((a, b) => a.y - b.y) as s, i (i)}
					<g transform="translate({s.x} {s.y}) scale({s.s})" style:--a="var(--awn-{s.c})">
						<path d="M6 0V-100M114 0V-100" stroke="var(--wood)" stroke-width="5" />
						<path d="M0 -34h120V0H0Z" fill="var(--wood)" />
						<g fill="var(--goods)">
							{#each GOODS as g, j (j)}<circle cx={g.x} cy={g.y} r="6.5" opacity={j % 2 ? 0.8 : 1} />{/each}
						</g>
						<path d="M-6 -100H126L138 -72H-18Z" fill="var(--a)" />
						<path d={AWNING_STRIPES} fill="var(--stripe)" />
						<path d={AWNING_VALANCE} fill="var(--a)" />
					</g>
				{/each}

				<!-- grill smoke -->
				{#each SMOKES as s, i (i)}
					<g transform="translate({s.x} {s.y})">
						{#each [0, 1, 2] as k (k)}
							<circle class="puff" r="18" fill="var(--smoke)" style:animation-delay="-{k * 3.4 + i}s" />
						{/each}
					</g>
				{/each}

				<!-- bloom and strings of lanterns -->
				{#each POLES as pole (pole.x)}
					<path d="M{pole.x} {pole.top - 6}V965" stroke="var(--wood)" stroke-width="7" stroke-linecap="round" />
				{/each}
				<path d={STRINGS.join(' ')} stroke="var(--brass)" stroke-width="2.4" fill="none" />
				{#key pulseKey}
					{#each LANTERNS as l (l.rank)}
						<g transform="translate({l.x} {l.y}) rotate({l.tilt})">
							<path d="M0 0V8M-9 8h18l-3 6h-12zM-9 14h18l4 8v14l-4 8h-18l-4-8v-14zM-5 52h10l-5 8z" fill="var(--brass)" transform="translate(0 0)" />
							<circle
								class="bulb {pulseKind ? `bulb--${pulseKind}` : ''}"
								class:bulb--dim={dimmed}
								cx="0"
								cy="30"
								r="9"
								fill="var(--lamp-on)"
								style:--rank="{l.rank}"
							/>
							<circle
								class="bulb halo {pulseKind ? `bulb--${pulseKind}` : ''}"
								class:bulb--dim={dimmed}
								cx="0"
								cy="30"
								r="40"
								fill="url(#mk-bloom)"
								style:--rank="{l.rank}"
							/>
						</g>
					{/each}
				{/key}
			</svg>

			<!-- The terrace wall the pigeon stands on. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<rect x="0" y={LEDGE_Y + 6} width={STAGE_W} height={STAGE_H - LEDGE_Y} fill="var(--near)" />
				<rect x="0" y={LEDGE_Y + 6} width={STAGE_W} height="12" fill="var(--near-top)" />
			</svg>

			<div class="cast stroller" style:transform="translateX({pigeonX - 75}px)" style:transition-duration="{walkMs}ms" style:top="{LEDGE_Y + 6 - 187}px">
				<div class="waddle" class:waddle--on={walking} class:peck--on={pecking}>
					<WallPigeon class="gaston" {sign} lookX={look} lookY={trouble ? -0.3 : 0} {eyes} blinkOffset={0.7} />
				</div>
			</div>

			{#if stork}
				<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
					<g class="stork">
						<path d="M-30 4L-72 12M-30 6L-70 20" stroke="var(--stork-leg, #c8452f)" stroke-width="2.4" stroke-linecap="round" />
						<ellipse cx="0" cy="0" rx="32" ry="9" fill="#f6f2ea" />
						<path d="M26 -2Q40 -12 46 -8" stroke="#f6f2ea" stroke-width="5" stroke-linecap="round" fill="none" />
						<circle cx="47" cy="-8" r="4.4" fill="#f6f2ea" />
						<path d="M50 -9L80 -5L50 -4Z" fill="#c8452f" />
						<g class="wing">
							<path d="M-10 -4Q-6 -34 24 -48Q10 -24 16 -4Z" fill="#f6f2ea" />
							<path d="M12 -34Q20 -42 24 -48Q16 -26 17 -14Z" fill="#232433" />
						</g>
					</g>
				</svg>
			{/if}

			{#if raining}
				<div class="rain">
					<svg viewBox="0 0 1920 1620"><path d={RAIN} stroke="var(--rain)" stroke-width="2.4" stroke-linecap="round" /></svg>
				</div>
				<div class="veil"></div>
				<svg class="layer flash" viewBox="0 0 {STAGE_W} {STAGE_H}">
					<path d={BOLT} fill="#fff7d6" />
					<rect width={STAGE_W} height={STAGE_H} fill="#fff7d6" opacity="0.22" />
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
		width: 120px;
		height: 120px;
		border-radius: 50%;
		background: radial-gradient(circle, var(--glow) 0%, transparent 70%);
	}
	.square {
		width: 520px;
		height: 220px;
		animation: breathe 11s ease-in-out infinite;
	}
	.beacon {
		animation: breathe 6s ease-in-out infinite;
	}
	@keyframes breathe {
		50% {
			opacity: 0.55;
		}
	}
	.lit {
		animation: lamp-on 4s ease both;
	}
	@keyframes lamp-on {
		from {
			opacity: 0;
		}
	}

	/* Grill smoke: puffs that swell and drift up. */
	.puff {
		transform-box: fill-box;
		transform-origin: center;
		opacity: 0;
		animation: puff 10s ease-out infinite;
	}
	@keyframes puff {
		0% {
			transform: translate(0, 0) scale(0.4);
			opacity: 0;
		}
		20% {
			opacity: 0.8;
		}
		100% {
			transform: translate(40px, -150px) scale(2.2);
			opacity: 0;
		}
	}

	/* Lanterns: lit as the evening is, with a slow flicker. */
	.bulb {
		transform-box: fill-box;
		transform-origin: center;
		opacity: var(--lan-on);
		animation: flicker 5s ease-in-out infinite;
		animation-delay: calc(var(--rank) * -0.7s);
	}
	.halo {
		opacity: calc(var(--lan-on) * 0.8);
	}
	@keyframes flicker {
		45% {
			opacity: calc(var(--lan-on) * 0.82);
		}
	}
	/* down: they go out in a wave along the square, stay dark a while, and come back. */
	.bulb--down {
		animation: lan-down 9s ease both;
		animation-delay: calc(var(--rank) * 140ms);
	}
	@keyframes lan-down {
		0% {
			opacity: var(--lan-on);
		}
		10%,
		84% {
			opacity: 0;
		}
		100% {
			opacity: var(--lan-on);
		}
	}
	/* up: dark, then each one relights in the same wave and blooms. */
	.bulb--up {
		animation: lan-up 3.4s ease-out both;
		animation-delay: calc(var(--rank) * 140ms);
	}
	@keyframes lan-up {
		0% {
			opacity: 0;
			transform: scale(0.6);
		}
		35% {
			opacity: 1;
			transform: scale(1.7);
		}
		100% {
			opacity: var(--lan-on);
			transform: scale(1);
		}
	}
	.bulb--dim {
		opacity: 0;
	}

	.stork {
		animation: glide 12s linear forwards;
	}
	.wing {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: flap 0.9s ease-in-out infinite alternate;
	}
	@keyframes glide {
		from {
			transform: translate(-160px, 330px);
		}
		50% {
			transform: translate(960px, 280px);
		}
		to {
			transform: translate(2100px, 330px);
		}
	}
	@keyframes flap {
		to {
			transform: scaleY(-0.55);
		}
	}

	.cast {
		position: absolute;
		left: 0;
	}
	.cast :global(.gaston) {
		display: block;
		width: 150px;
	}
	.stroller {
		transition-property: transform;
		transition-timing-function: linear;
	}
	.waddle {
		transform-origin: 50% 100%;
	}
	.waddle--on {
		animation: waddle 1.1s ease-in-out infinite;
	}
	@keyframes waddle {
		0%,
		100% {
			transform: rotate(-4deg) translateY(0);
		}
		25%,
		75% {
			transform: rotate(0deg) translateY(-5px);
		}
		50% {
			transform: rotate(4deg) translateY(0);
		}
	}
	.peck--on {
		animation: peck 1.4s ease-in-out;
	}
	@keyframes peck {
		30%,
		60% {
			transform: rotate(16deg) translateY(10px);
		}
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
	/* Two quick flashes now and then; never a strobe. */
	.flash {
		opacity: 0;
		animation: lightning 11s ease-out infinite;
	}
	@keyframes lightning {
		0%,
		100% {
			opacity: 0;
		}
		3% {
			opacity: 0.9;
		}
		6% {
			opacity: 0.1;
		}
		9% {
			opacity: 0.7;
		}
		20% {
			opacity: 0;
		}
	}

	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	.scene--still :global(*) {
		animation: none !important;
		transition: none !important;
	}
	.scene--still .cloud {
		transform: translateX(var(--still-x));
	}
	.scene--still .flash {
		display: none;
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
		.flash {
			display: none;
		}
	}
</style>
