<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop, Rio de Janeiro: Christ the Redeemer far away
	 * on Corcovado, the Sugarloaf and Morro da Urca with their cable cars,
	 * the bay, and Copacabana's sand and wave-patterned promenade, where a
	 * DumbMonit pigeon looks out to sea. A drop-in for the Rome scene (same
	 * props).
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` keeps a stage x in view on a narrow screen). The sky follows the
	 * real hour inside the brightness band of the wall's theme (see
	 * `paris/daylight.ts`); after dark the town glows and Christ is floodlit.
	 *
	 * Mood is weather: calm is clear, clouded brings the mist that really does
	 * cap Corcovado, a storm brings rain and slow lightning (never in reduced
	 * motion). `mood` takes the wall's names (calm / clouded / storm) or the
	 * status page's (ok / warn / critical).
	 *
	 * `pulse` is the cable car's gesture: when a device goes down both cabins
	 * stop dead on their cable, swing and turn red; when it comes back they
	 * jolt into motion with a ring of light; when an incident resolves Christ
	 * opens his arms in a burst of golden rays.
	 *
	 * Motion is gentle: sea glints, foam, drifting clouds, cabins crossing the
	 * bay, swaying palms, and, now and then, a hang glider. Hidden tabs pause
	 * everything; reduced motion freezes a still picture.
	 */
	import { tick, untrack } from 'svelte';
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
		HORIZON,
		FAR_RIDGE,
		FAR_RIDGE_2,
		CORCOVADO,
		SUGARLOAF,
		TOWN,
		TOWN_WINDOWS,
		BOATS,
		SEA_GLINTS,
		SHORE,
		FOAM,
		WAVE_BANDS,
		UMBRELLAS,
		PALMS,
		STARS,
		RAIN,
		litWindows,
		rioStyle
	} from './geometry.js';

	type Mood = SceneMood | 'ok' | 'warn' | 'critical';
	interface Props {
		theme: SceneTheme;
		now: Date;
		mood: Mood;
		/** Problems on the board: the number on the protest sign. */
		problems: number;
		/** Bumped by the wall when a new problem appears: the pigeon startles once. */
		startle?: number;
		/** The tab is hidden: nothing moves, nothing is scheduled. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		/** A short gesture, replayed whenever `at` changes. */
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

	const weather = $derived<SceneMood>(
		mood === 'storm' || mood === 'critical' ? 'storm' : mood === 'clouded' || mood === 'warn' ? 'clouded' : 'calm'
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
	const style = $derived(`${sceneStyle(theme, phase, weather)};${rioStyle(theme)}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));

	let windowSeed = $state(1);
	const litSet = $derived(lit ? litWindows(windowSeed, TOWN_WINDOWS.length, phase === 'night' ? 0.2 : 0.32) : new Set<number>());

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

	// --- Cable cars and Christ: the pulse ---------------------------------------------

	const DX = SUGARLOAF.to.x - SUGARLOAF.from.x;
	const DY = SUGARLOAF.to.y - SUGARLOAF.from.y;
	const RAYS = Array.from({ length: 14 }, (_, i) => (i / 14) * 360);

	let gesture = $state<null | 'down' | 'up' | 'resolved'>(null);
	let gestureTimer: ReturnType<typeof setTimeout> | undefined;
	let lastPulse = untrack(() => pulse?.at);
	$effect(() => {
		const current = pulse;
		if (!current || current.at === lastPulse) return;
		lastPulse = current.at;
		untrack(() => {
			clearTimeout(gestureTimer);
			if (!motion) return;
			gesture = null;
			void tick().then(() => {
				gesture = current.kind;
				gestureTimer = setTimeout(() => (gesture = null), current.kind === 'down' ? 6500 : 4200);
			});
		});
		return () => clearTimeout(gestureTimer);
	});

	// --- The pigeon -------------------------------------------------------------------

	let startled = $state(false);
	let look = $state(-0.5);
	let feathers = $state<{ id: number; drift: number; delay: number }[]>([]);
	const pigeonEyes = $derived(startled || gesture === 'down' ? 'startled' : 'open');
	const sign = $derived(trouble ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null);

	// --- Scheduling ---------------------------------------------------------------------

	// Every timer carries the epoch it was set under; a pause or teardown bumps
	// it and every stale callback does nothing.
	let epoch = 0;
	const running: Animation[] = [];
	function later(fn: () => void, ms: number) {
		const mine = epoch;
		setTimeout(() => {
			if (mine === epoch) fn();
		}, ms);
	}
	const between = (a: number, b: number) => a + Math.random() * (b - a);
	let glider = $state(false);
	let gliderEl = $state<HTMLDivElement | null>(null);

	function stopAll() {
		epoch++;
		for (const animation of running.splice(0)) animation.cancel();
		glider = false;
	}

	/** The pigeon watches the sea, now and then glances along the beach. */
	function wander() {
		look = trouble ? -0.3 : [-0.8, -0.3, 0.5, 0.9][Math.floor(Math.random() * 4)];
		later(wander, between(4000, 9000));
	}

	async function hangGlide() {
		const mine = epoch;
		glider = true;
		await tick();
		if (!gliderEl || !('animate' in gliderEl)) return;
		const y = between(180, 330);
		const animation = gliderEl.animate(
			[
				{ transform: `translate(2100px, ${y - 90}px) rotate(-6deg)` },
				{ transform: `translate(1100px, ${y + 70}px) rotate(5deg)`, offset: 0.5 },
				{ transform: `translate(-300px, ${y + 190}px) rotate(-4deg)` }
			],
			{ duration: 28000, easing: 'linear', fill: 'forwards' }
		);
		running.push(animation);
		await animation.finished.then(() => undefined, () => undefined);
		if (mine === epoch) glider = false;
	}

	function scheduleGlider(first = false) {
		later(
			() => {
				// A glider only flies in daylight and fair weather.
				if (!lit && !raining) void hangGlide();
				scheduleGlider();
			},
			first ? between(8000, 15000) : between(50000, 90000)
		);
	}

	$effect(() => {
		void trouble;
		const hidden = paused;
		untrack(() => {
			stopAll();
			look = trouble ? -0.3 : -0.5;
			if (!hidden && motion) {
				later(wander, 3000);
				scheduleGlider(true);
			}
		});
		return () => untrack(stopAll);
	});

	$effect(() => {
		if (!lit || paused || !motion) return;
		const timer = setInterval(() => (windowSeed += 1), 120_000);
		return () => clearInterval(timer);
	});

	// A new problem: the pigeon startles and a few feathers drift down.
	let lastStartle = untrack(() => startle);
	let featherId = 0;
	$effect(() => {
		const value = startle;
		if (value === lastStartle) return;
		lastStartle = value;
		untrack(() => {
			startled = true;
			setTimeout(() => (startled = false), 1800);
			if (!motion) return;
			feathers = Array.from({ length: 4 }, (_, i) => ({ id: featherId++, drift: Math.round(between(-40, 40)), delay: i * 260 }));
			setTimeout(() => (feathers = []), 7000);
		});
	});
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
						<path d="M30 104 C2 104 0 66 30 62 C24 32 60 18 86 34 C98 6 152 0 170 30 C192 14 234 22 238 54 C270 52 292 78 276 104 Z" fill={raining ? 'var(--cloud-storm)' : 'var(--cloud)'} />
					</svg>
				</div>
			{/each}

			{#if lit}
				<div class="glow flood" style:left="{CORCOVADO.top.x - 110}px" style:top="{CORCOVADO.top.y - 120}px"></div>
			{/if}

			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<defs>
					<!-- voilier : origine = ligne de flottaison -->
					<g id="rio-boat">
						<path d="M-22 0H24L16 8H-14z" fill="var(--hull)" />
						<path d="M0-2V-44L22-6z" fill="var(--sail)" />
						<path d="M-3-2V-34L-20-6z" fill="var(--sail)" opacity="0.75" />
					</g>
					<!-- palmier : origine = pied -->
					<g id="rio-palm">
						<path d="M0 0C6-40 2-84 14-124" stroke="var(--trunk)" stroke-width="7" stroke-linecap="round" />
						<g fill="var(--palm)">
							<path d="M14-124C-8-150-44-142-66-118C-34-130-12-130 14-124z" />
							<path d="M14-124C36-152 74-144 96-116C68-130 40-130 14-124z" />
							<path d="M14-124C0-158 12-182 30-194C28-166 24-146 14-124z" />
							<path d="M14-124C-18-128-52-98-66-80C-38-98-18-108 14-124z" fill="var(--palm-shade)" />
							<path d="M14-124C46-128 80-98 94-78C66-98 40-108 14-124z" fill="var(--palm-shade)" />
						</g>
					</g>
				</defs>

				<!-- far ridges -->
				<path d={FAR_RIDGE} fill="var(--ridge)" />
				<path d={FAR_RIDGE_2} fill="var(--ridge)" />

				<!-- Corcovado and Christ, far away -->
				<path d={CORCOVADO.hill} fill="var(--hill-far)" />
				<path d={CORCOVADO.shade} fill="var(--hill-far-shade)" />
				<g transform="translate({CORCOVADO.top.x} {CORCOVADO.top.y}) scale(1.4)">
					<path d="M-9-2L-7-14H7L9-2z" fill="var(--christ-shade)" />
					<path d="M-5.2-14L-4-44H4L5.2-14z" fill="var(--christ)" />
					<path d="M0-44V-14H5.2L4-44z" fill="var(--christ-shade)" />
					<path d="M-24-43L-4-44.5V-39.5L-24-38.5zM24-43L4-44.5V-39.5L24-38.5z" fill="var(--christ)" />
					<circle cx="0" cy="-49" r="3.4" fill="var(--christ)" />
					{#if gesture === 'resolved'}
						<g class="rays" transform="translate(0 -38)">
							{#each RAYS as angle, i (i)}
								<path d="M0 -26 L0 -{i % 2 ? 70 : 96}" stroke="var(--ray)" stroke-width={i % 2 ? 2.5 : 4} stroke-linecap="round" transform="rotate({angle})" />
							{/each}
							<circle r="30" fill="var(--ray)" class="rays-halo" />
						</g>
					{/if}
				</g>

				<!-- the town at the foot of the hills -->
				<g fill="var(--town)">
					{#each TOWN as b, i (i)}
						<rect x={b.x} y={b.y} width={b.w} height={b.h} />
					{/each}
				</g>
				<g fill="var(--town-roof)">
					{#each TOWN as b, i (i)}
						{#if b.roof}<rect x={b.x - 1} y={b.y - 3} width={b.w + 2} height="4" />{/if}
					{/each}
				</g>
				<g fill="var(--window-lit)">
					{#each TOWN_WINDOWS as w, i (i)}
						{#if litSet.has(i)}<rect x={w.x} y={w.y} width="6" height="7" rx="1" />{/if}
					{/each}
				</g>

				<!-- Morro da Urca and the Sugarloaf -->
				<path d={SUGARLOAF.urca} fill="var(--rock-shade)" />
				<path d={SUGARLOAF.urcaForest} fill="var(--forest-shade)" />
				<path d={SUGARLOAF.loaf} fill="var(--rock)" />
				<path d={SUGARLOAF.shade} fill="var(--rock-shade)" opacity="0.6" />
				<path d={SUGARLOAF.forest} fill="var(--forest)" />
				<rect x={SUGARLOAF.from.x - 12} y={SUGARLOAF.from.y - 4} width="24" height="10" rx="2" fill="var(--cable)" />
				<rect x={SUGARLOAF.to.x - 12} y={SUGARLOAF.to.y - 4} width="24" height="10" rx="2" fill="var(--cable)" />

				<!-- the cables, and the two cabins that pass each other -->
				{#each [0, 1] as k (k)}
					<line
						x1={SUGARLOAF.from.x + k * 16}
						y1={SUGARLOAF.from.y + k * 8}
						x2={SUGARLOAF.to.x + k * 16}
						y2={SUGARLOAF.to.y + k * 8}
						stroke="var(--cable)"
						stroke-width="1.8"
					/>
				{/each}
				{#each [0, 1] as k (k)}
					<g transform="translate({SUGARLOAF.from.x + k * 16} {SUGARLOAF.from.y + k * 8})">
						<g
							class="cabin"
							class:cabin--back={k === 1}
							class:cabin--halt={gesture === 'down'}
							class:cabin--jolt={gesture === 'up'}
							style:--dx="{DX}px"
							style:--dy="{DY}px"
						>
							<g class="hang" class:hang--swing={gesture === 'down'}>
								<g class="car" class:cabin-red={gesture === 'down'}>
									<path d="M0 0V10" stroke="var(--cable)" stroke-width="2" />
									<rect class="cab-body" x="-11" y="10" width="22" height="15" rx="3.5" fill="var(--cab)" />
									<rect class="cab-lamp" x="-8" y="13" width="16" height="6" rx="2" fill="var(--cab-lamp)" />
								</g>
							</g>
							{#if gesture === 'up'}
								<circle class="ring" cy="18" r="10" fill="none" stroke="var(--cab-lamp)" stroke-width="3" />
								<circle class="ring ring--late" cy="18" r="10" fill="none" stroke="var(--cab-lamp)" stroke-width="2" />
							{/if}
						</g>
					</g>
				{/each}

				<!-- the bay -->
				<rect x="0" y={HORIZON} width={STAGE_W} height="170" fill="var(--sea)" />
				<rect x="0" y={HORIZON + 80} width={STAGE_W} height="120" fill="var(--sea-deep)" opacity="0.55" />
				<path class="glints" d={SEA_GLINTS} stroke="var(--glint)" stroke-width="3" stroke-linecap="round" />
				{#each BOATS as boat, i (i)}
					<g transform="translate({boat.x} {boat.y}) scale({boat.s})"><g class="bob" style:animation-delay="-{i * 1.7}s"><use href="#rio-boat" /></g></g>
				{/each}

				<!-- Copacabana: sand, foam, umbrellas -->
				<path d={SHORE} fill="var(--sand)" />
				<path class="foam" d={FOAM} stroke="var(--foam)" stroke-width="7" stroke-linecap="round" />
				{#each UMBRELLAS as u, i (i)}
					<g transform="translate({u.x} {u.y}) scale({u.s})">
						<path d="M0 0V-50" stroke="var(--trunk)" stroke-width="3" />
						<path d="M-34-50C-30-78 30-78 34-50C22-56 12-56 0-50C-12-56-22-56-34-50z" fill="var(--umb-{u.c})" />
					</g>
				{/each}

				<!-- the promenade, with its wave pavement -->
				<rect x="0" y={LEDGE_Y - 10} width={STAGE_W} height="10" fill="var(--curb)" />
				<rect x="0" y={LEDGE_Y} width={STAGE_W} height={STAGE_H - LEDGE_Y} fill="var(--stone)" />
				{#each WAVE_BANDS as d, i (i)}
					<path {d} stroke="var(--stone-wave)" stroke-width="12" stroke-linecap="round" fill="none" />
				{/each}

				{#each PALMS as p, i (i)}
					<g transform="translate({p.x} {p.y}) scale({p.s})"><g class="sway" style:animation-delay="-{p.d}s"><use href="#rio-palm" /></g></g>
				{/each}
			</svg>

			<!-- A mist cap on Corcovado when the weather turns. -->
			{#if weather !== 'calm'}
				<div class="mist" style:left="{CORCOVADO.top.x - 190}px" style:top="{CORCOVADO.top.y - 40}px">
					<svg viewBox="0 0 380 120"><ellipse cx="190" cy="60" rx="180" ry="36" fill="var(--cloud)" opacity="0.85" /><ellipse cx="120" cy="38" rx="90" ry="26" fill="var(--cloud)" opacity="0.8" /><ellipse cx="270" cy="76" rx="100" ry="26" fill="var(--cloud)" opacity="0.8" /></svg>
				</div>
			{/if}

			{#if glider}
				<div class="glider" bind:this={gliderEl}>
					<svg viewBox="0 0 120 70" width="120" height="70">
						<path d="M60 6L4 38L60 28L116 38Z" fill="var(--umb-a)" /><path d="M60 6L60 28" stroke="var(--cable)" stroke-width="2" />
						<path d="M60 28V50M52 56L60 50L68 56" stroke="var(--cable)" stroke-width="3" stroke-linecap="round" fill="none" />
					</svg>
				</div>
			{/if}

			<!-- Zé, looking out to sea (an umbrella when it rains, a sign when something is wrong). -->
			<div class="cast" style:left="1010px" style:top="{LEDGE_Y + 6 - 187}px">
				<WallPigeon class="ze" lookX={look} lookY={trouble ? -0.3 : 0} eyes={pigeonEyes} umbrella={raining} {sign} blinkOffset={1.9} />
			</div>
			{#each feathers as feather (feather.id)}
				<div class="feather" style:left="{1060 + feather.id % 4 * 20}px" style:top="{LEDGE_Y - 150}px" style:--drift="{feather.drift}px" style:animation-delay="{feather.delay}ms">
					<svg viewBox="0 0 20 40"><path d="M10 2 C18 12 18 28 10 38 C2 28 2 12 10 2 Z" fill="var(--pg-body, #6f83a3)" /></svg>
				</div>
			{/each}

			{#if raining}
				<div class="rain">
					<svg viewBox="0 0 1920 1620"><path d={RAIN} stroke="var(--rain)" stroke-width="2.4" stroke-linecap="round" /></svg>
				</div>
				<div class="veil"></div>
				<div class="lightning">
					<svg viewBox="0 0 {STAGE_W} {STAGE_H}"><path d="M1290 120 L1262 300 L1300 296 L1250 470 L1340 270 L1296 274 Z" fill="var(--bolt)" /></svg>
				</div>
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
		background: linear-gradient(to bottom, var(--sky-top) 0%, var(--sky-bottom) 66%);
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
		width: 220px;
		height: 260px;
		border-radius: 50%;
		background: radial-gradient(circle, var(--glow) 0%, transparent 70%);
		animation: breathe 7s ease-in-out infinite;
	}
	@keyframes breathe {
		50% {
			opacity: 0.55;
		}
	}

	.mist {
		position: absolute;
		width: 380px;
		height: 120px;
		animation: mist 24s ease-in-out infinite alternate;
	}
	.mist svg {
		display: block;
		width: 100%;
		height: 100%;
	}
	@keyframes mist {
		to {
			transform: translateX(70px);
		}
	}

	/* Sea and sand */
	.glints {
		animation: glint 6s ease-in-out infinite;
	}
	@keyframes glint {
		50% {
			opacity: 0.35;
		}
	}
	.foam {
		animation: lap 7s ease-in-out infinite;
	}
	@keyframes lap {
		50% {
			transform: translateY(-9px);
			opacity: 0.55;
		}
	}
	.bob {
		animation: bob 5s ease-in-out infinite;
	}
	@keyframes bob {
		50% {
			transform: translateY(-3px) rotate(1.5deg);
		}
	}
	.sway {
		transform-origin: 0 0;
		animation: sway 7s ease-in-out infinite;
	}
	@keyframes sway {
		50% {
			transform: rotate(2.4deg);
		}
	}

	/* The cable cars: one up, one down, forever. */
	.cabin {
		animation: ride 52s ease-in-out infinite alternate;
	}
	.cabin--back {
		animation-direction: alternate-reverse;
	}
	@keyframes ride {
		from {
			transform: translate(0, 0);
		}
		to {
			transform: translate(var(--dx), var(--dy));
		}
	}
	/* Down: stop dead on the cable, swing, and the lamp goes red. */
	.cabin--halt {
		animation-play-state: paused;
	}
	.hang {
		transform-origin: 0 0;
	}
	.hang--swing {
		animation: swing 1.7s ease-in-out infinite;
	}
	@keyframes swing {
		0%,
		100% {
			transform: rotate(-9deg);
		}
		50% {
			transform: rotate(9deg);
		}
	}
	.cabin-red .cab-body {
		fill: #b3362a;
	}
	.cabin-red .cab-lamp {
		fill: #ff4d3d;
		animation: alarm 0.7s steps(2, jump-none) infinite;
	}
	@keyframes alarm {
		50% {
			opacity: 0.2;
		}
	}
	/* Up: a jolt into motion, and a ring of light. */
	.cabin--jolt .hang {
		animation: jolt 0.9s cubic-bezier(0.3, 1.6, 0.5, 1) both;
	}
	@keyframes jolt {
		from {
			transform: translate(-6px, 6px) rotate(-12deg);
		}
	}
	.ring {
		transform-box: fill-box;
		transform-origin: center;
		animation: ring 1.8s ease-out both;
	}
	.ring--late {
		animation-delay: 0.5s;
	}
	@keyframes ring {
		from {
			opacity: 0.9;
			transform: scale(0.5);
		}
		to {
			opacity: 0;
			transform: scale(5);
		}
	}
	/* Resolved: Christ opens his arms on a burst of golden light. */
	.rays {
		transform-box: fill-box;
		transform-origin: center;
		animation: rays 4s ease-out both;
	}
	.rays-halo {
		opacity: 0.5;
	}
	@keyframes rays {
		0% {
			opacity: 0;
			transform: translate(0, -38px) scale(0.4) rotate(0deg);
		}
		25% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(0, -38px) scale(1.5) rotate(25deg);
		}
	}

	.glider {
		position: absolute;
		left: 0;
		top: 0;
		will-change: transform;
	}

	.cast {
		position: absolute;
	}
	.cast :global(.ze) {
		display: block;
		width: 150px;
	}
	.feather {
		position: absolute;
		width: 20px;
		height: 40px;
		animation: fall 6s ease-in both;
	}
	@keyframes fall {
		from {
			transform: translate(0, 0) rotate(0deg);
			opacity: 1;
		}
		to {
			transform: translate(var(--drift), 140px) rotate(160deg);
			opacity: 0;
		}
	}

	/* Weather */
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
	/* Two quick flashes every eleven seconds; never a strobe. */
	.lightning {
		position: absolute;
		inset: 0;
		opacity: 0;
		background: rgb(255 255 255 / 0.22);
		animation: flash 11s linear infinite;
	}
	.lightning svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}
	@keyframes flash {
		0%,
		58%,
		63%,
		100% {
			opacity: 0;
		}
		59% {
			opacity: 1;
		}
		60.5% {
			opacity: 0.15;
		}
		61.5% {
			opacity: 0.7;
		}
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
	.scene--still .lightning {
		display: none;
	}
	.scene--still .cabin {
		transform: translate(calc(var(--dx) * 0.4), calc(var(--dy) * 0.4));
	}
	.scene--still .cabin--back {
		transform: translate(calc(var(--dx) * 0.6), calc(var(--dy) * 0.6));
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
		.lightning {
			display: none;
		}
	}
</style>
