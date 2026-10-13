<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop: the Matterhorn over Zermatt, its snow, the
	 * chalets and the church, the Gornergrat train climbing the meadow, and the
	 * DumbMonit pigeons on the village wall. A drop-in for the Paris scene
	 * (same props).
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky
	 * follows the real hour inside the brightness band of the wall's theme (see
	 * `paris/daylight.ts`); the snow turns pink at dawn and dusk, the windows
	 * and the train light up after dark.
	 *
	 * The network's state is woven in: clouds gather with advisories, a storm
	 * brings drizzle and a darker sky, the pigeon in the hat holds up a sign
	 * with the number of problems, the second pigeon opens an umbrella. A new
	 * problem startles everyone once.
	 *
	 * Motion is gentle: drifting clouds, the flag, the plume of snow off the
	 * summit, a stroll, pecks. Every 40-80 s one small moment plays (the train
	 * goes by, a fly-by, a hat tip). Hidden tabs pause everything; reduced
	 * motion freezes the scene in a still pose.
	 */
	import { tick, untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import WallPigeon from '#lib/components/wall/paris/WallPigeon.svelte';
	import {
		phaseAt,
		isLit,
		celestialAt,
		type SceneMood,
		type SceneTheme
	} from '#lib/components/wall/paris/daylight.js';
	import {
		STAGE_W, STAGE_H, LEDGE_Y, PEAK, MOUNTAIN, MOUNTAIN_SHADE, SNOW, SNOW_SHADE, RIDGES,
		FAR_RANGE, FAR_RIGHT, MEADOW, CHALETS, CHALET_WINDOWS, CHURCH, FLAG, PINES, TRACK,
		TRAIN_ANGLE, trackY, STARS, RAIN, litWindows, zermattStyle
	} from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood?: SceneMood | 'ok' | 'warn' | 'critical';
		/** Problems on the board: the number on the protest sign. */
		problems: number;
		/** Bumped by the wall when a new problem appears: everyone startles once. */
		startle?: number;
		/** The tab is hidden: nothing moves, nothing is scheduled. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		/** A short landmark gesture, replayed whenever `at` changes. */
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
		class?: string;
	}

	let {
		theme,
		now,
		mood: moodIn = 'calm',
		problems,
		startle = 0,
		paused = false,
		focus = 0,
		pulse = null,
		class: className = ''
	}: Props = $props();

	const motion = !reducedMotion();

	// The wall's moods and the status page's ones are the same weather.
	const mood = $derived<SceneMood>(
		moodIn === 'storm' || moodIn === 'critical'
			? 'storm'
			: moodIn === 'clouded' || moodIn === 'warn'
				? 'clouded'
				: 'calm'
	);

	// --- The gesture ---------------------------------------------------------------

	// A short landmark gesture, replayed whenever `pulse.at` changes. Reduced
	// motion skips it: the scene holds its still pose.
	let active = $state<'down' | 'up' | 'resolved' | null>(null);
	let playKey = $state(0);
	let seen: number | null = untrack(() => pulse?.at ?? null);
	let gestureTimer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const p = pulse;
		if (!p || p.at === seen) return;
		seen = p.at;
		active = null;
		clearTimeout(gestureTimer);
		if (!motion) return;
		void tick().then(() => {
			active = p.kind;
			playKey++;
			gestureTimer = setTimeout(() => (active = null), p.kind === 'resolved' ? 7000 : 5200);
		});
	});
	$effect(() => () => clearTimeout(gestureTimer));

	// --- Stage fit ---------------------------------------------------------------
	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light -------------------------------------------------------------------
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const style = $derived(zermattStyle(theme, phase, mood));
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));

	let windowSeed = $state(1);
	const litSet = $derived(lit ? litWindows(windowSeed, CHALET_WINDOWS.length, phase === 'night' ? 0.25 : 0.4) : new Set<number>());

	const trouble = $derived(problems > 0);
	const raining = $derived(mood === 'storm');
	const cloudCount = $derived(mood === 'storm' ? 6 : mood === 'clouded' ? 4 : 2);
	const CLOUDS = [
		{ y: 110, s: 1.1, dur: 260, delay: -40 }, { y: 300, s: 0.8, dur: 320, delay: -210 },
		{ y: 450, s: 0.6, dur: 360, delay: -120 }, { y: 200, s: 0.95, dur: 290, delay: -150 },
		{ y: 380, s: 1.25, dur: 340, delay: -290 }, { y: 60, s: 0.7, dur: 300, delay: -250 }
	];

	// --- The cast ----------------------------------------------------------------
	const STROLL = [780, 1120];
	const SIGN_SPOT = 800;
	let gastonX = $state(900);
	let gastonWalkMs = $state(0);
	let gastonWalking = $state(false);
	let gastonLook = $state(0);
	let gastonPeck = $state(false);
	let gastonTip = $state(false);
	let startled = $state(false);

	let flyer = $state(false);
	let trainOn = $state(false);
	let feathers = $state<{ id: number; x: number; drift: number; delay: number }[]>([]);
	let flyerEl = $state<HTMLDivElement | null>(null);
	let trainEl = $state<HTMLDivElement | null>(null);

	// Every timer carries the epoch it was set under; a pause, a change of
	// mood or a teardown bumps it and every stale callback does nothing.
	let epoch = 0;
	const running: Animation[] = [];
	function later(fn: () => void, ms: number) {
		const mine = epoch;
		setTimeout(() => {
			if (mine === epoch) fn();
		}, ms);
	}
	const between = (a: number, b: number) => a + Math.random() * (b - a);

	function stopAll() {
		epoch++;
		for (const animation of running.splice(0)) animation.cancel();
		flyer = false;
		trainOn = false;
		gastonTip = false;
		gastonPeck = false;
	}

	function stroll() {
		if (trouble) return;
		const target = Math.round(between(STROLL[0], STROLL[1]));
		const distance = Math.abs(target - gastonX);
		if (distance < 60) {
			later(stroll, 1500);
			return;
		}
		gastonLook = target > gastonX ? 0.7 : -0.7;
		gastonWalkMs = Math.round((distance / 34) * 1000);
		gastonWalking = true;
		gastonX = target;
		later(() => {
			gastonWalking = false;
			gastonLook = 0;
			if (Math.random() < 0.5) {
				later(() => (gastonPeck = true), 1200);
				later(() => (gastonPeck = false), 2600);
			}
			later(stroll, between(5000, 11000));
		}, gastonWalkMs);
	}

	function toSignSpot() {
		const distance = Math.abs(SIGN_SPOT - gastonX);
		gastonLook = -0.8;
		gastonWalkMs = Math.round((distance / 60) * 1000);
		gastonWalking = distance > 4;
		gastonX = SIGN_SPOT;
		later(() => (gastonWalking = false), gastonWalkMs);
	}

	function animate(el: HTMLElement | null, frames: Keyframe[], options: KeyframeAnimationOptions): Promise<void> {
		if (!el || !('animate' in el)) return Promise.resolve();
		const animation = el.animate(frames, options);
		running.push(animation);
		return animation.finished.then(() => undefined, () => undefined);
	}

	type Moment = 'train' | 'flyby' | 'hat';
	let deck: Moment[] = [];
	function nextMoment(): Moment {
		const allowed: Moment[] = ['train', 'flyby'];
		if (!trouble) allowed.push('hat');
		deck = deck.filter((x) => allowed.includes(x));
		if (deck.length === 0) deck = [...allowed].sort(() => Math.random() - 0.5);
		return deck.shift() ?? 'train';
	}

	async function play(moment: Moment) {
		const mine = epoch;
		if (moment === 'train') {
			trainOn = true;
			await tick();
			// Down the valley or up to the Gornergrat, at random.
			const [a, b] = Math.random() < 0.5 ? [-380, 2000] : [2000, -380];
			const at = (x: number) => `translate(${x}px, ${trackY(x) - 64}px) rotate(${TRAIN_ANGLE}deg)`;
			await animate(trainEl, [{ transform: at(a) }, { transform: at(b) }], {
				duration: 30000, easing: 'linear', fill: 'forwards'
			});
			if (mine === epoch) trainOn = false;
		} else if (moment === 'flyby') {
			flyer = true;
			await tick();
			const y = between(230, 380);
			await animate(
				flyerEl,
				[
					{ transform: `translate(-260px, ${y + 40}px) rotate(4deg)` },
					{ transform: `translate(900px, ${y - 20}px) rotate(-2deg)`, offset: 0.5 },
					{ transform: `translate(2160px, ${y - 60}px) rotate(-4deg)` }
				],
				{ duration: 19000, easing: 'linear', fill: 'forwards' }
			);
			if (mine === epoch) flyer = false;
		} else {
			later(() => {
				gastonLook = 0;
				gastonTip = true;
			}, 200);
			later(() => (gastonTip = false), 2600);
		}
	}

	function scheduleMoments(first = false) {
		later(() => {
			void play(nextMoment());
			scheduleMoments();
		}, first ? between(6000, 12000) : between(40000, 80000));
	}

	function startAll() {
		if (!motion) {
			gastonX = trouble ? SIGN_SPOT : 900;
			gastonLook = trouble ? -0.8 : 0;
			return;
		}
		if (trouble) toSignSpot();
		else later(stroll, 2000);
		scheduleMoments(true);
	}

	// (Re)start the cast whenever the trouble flag or the pause changes.
	$effect(() => {
		void trouble;
		const hidden = paused;
		untrack(() => {
			stopAll();
			if (!hidden) startAll();
		});
		return () => untrack(stopAll);
	});

	// Lit windows change now and then while the village is lit.
	$effect(() => {
		if (!lit || paused || !motion) return;
		const timer = setInterval(() => (windowSeed += 1), 120_000);
		return () => clearInterval(timer);
	});

	// A new problem: everyone startles, and a few feathers drift down.
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
			feathers = Array.from({ length: 4 }, (_, i) => ({
				id: featherId++,
				x: gastonX + (i - 1.5) * 26,
				drift: Math.round(between(-40, 40)),
				delay: i * 260
			}));
			setTimeout(() => (feathers = []), 7000);
		});
	});

	const gastonSign = $derived(trouble ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null);
</script>

<div
	class="scene {className}"
	class:scene--paused={paused}
	class:scene--still={!motion}
	class:p-down={active === 'down'}
	class:p-up={active === 'up'}
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
						<path d="M30 104 C2 104 0 66 30 62 C24 32 60 18 86 34 C98 6 152 0 170 30 C192 14 234 22 238 54 C270 52 292 78 276 104 Z" fill={mood === 'storm' ? 'var(--cloud-storm)' : 'var(--cloud)'} />
					</svg>
				</div>
			{/each}

			<!-- The mountains: far ranges, the Matterhorn with its snow, the meadow and the pines. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<defs><clipPath id="zm-clip"><path d={MOUNTAIN} /></clipPath></defs>
				<path d={FAR_RANGE} fill="var(--mh-far)" />
				<path d={FAR_RIGHT} fill="var(--mh-far)" />
				<path d={MOUNTAIN} fill="var(--mh-rock)" />
				<g clip-path="url(#zm-clip)">
					<path d={MOUNTAIN_SHADE} fill="var(--mh-rock2)" />
					<path d={SNOW} fill="var(--mh-snow)" />
					<path d={SNOW_SHADE} fill="var(--mh-snow-shade)" />
					<path d={RIDGES} stroke="var(--mh-rock2)" stroke-width="5" stroke-linecap="round" opacity="0.6" />
				</g>
				<path d={MEADOW} fill="var(--mh-meadow)" />
				<path d={TRACK} stroke="var(--mh-rail)" stroke-width="5" />
				<path d={PINES} fill="var(--mh-pine)" />
			</svg>

			<!-- The gesture: the Matterhorn goes dark (down), blushes with alpenglow (up), lights its Swiss flag (resolved). -->
			{#if active}
				{#key playKey}
					<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
						<g clip-path="url(#zm-clip)">
							{#if active === 'down'}
								<rect class="zm-shade" x="800" y="190" width="1120" height="650" fill="#0d1526" />
							{:else if active === 'up'}
								<rect class="zm-glow" x="800" y="190" width="1120" height="650" fill="#ff9a6b" />
							{:else}
								<g class="zm-show">
									<rect x="1335" y="360" width="120" height="120" fill="#d52b1e" />
									<path d="M1385 378h20v32h32v20h-32v32h-20v-32h-32v-20h32z" fill="#fff" />
								</g>
							{/if}
						</g>
					</svg>
				{/key}
			{/if}

			<!-- The plume of snow the wind pulls off the summit. -->
			<svg class="layer plume" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<g fill="var(--mh-plume)">
					<ellipse cx={PEAK.x + 70} cy={PEAK.y + 6} rx="90" ry="9" />
					<ellipse cx={PEAK.x + 150} cy={PEAK.y - 4} rx="60" ry="7" />
					<ellipse cx={PEAK.x + 210} cy={PEAK.y - 12} rx="34" ry="5" />
				</g>
			</svg>

			{#if trainOn}
				<div class="mover" bind:this={trainEl}>
					<svg class="train" viewBox="0 0 330 64">
						<g fill="var(--mh-train)">
							<rect x="0" y="12" width="102" height="44" rx="6" /><rect x="108" y="12" width="102" height="44" rx="6" /><rect x="216" y="12" width="102" height="44" rx="6" />
						</g>
						<path d="M0 40H318" stroke="#fbf7ee" stroke-width="4" />
						<g fill={lit ? 'var(--window-lit)' : 'var(--window)'}>
							{#each [0, 108, 216] as x (x)}
								<rect x={x + 10} y="20" width="18" height="14" rx="2" /><rect x={x + 38} y="20" width="18" height="14" rx="2" /><rect x={x + 66} y="20" width="18" height="14" rx="2" />
							{/each}
						</g>
						<path d="M10 56v6M92 56v6M118 56v6M200 56v6M226 56v6M308 56v6" stroke="var(--mh-rail)" stroke-width="6" />
					</svg>
				</div>
			{/if}

			{#if flyer}
				<div class="mover" bind:this={flyerEl}>
					<WallPigeon flap lookX={0.8} class="flyer" blinkOffset={2} />
				</div>
			{/if}

			<!-- The village: chalets, the church, the flag. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<path d={CHURCH.body} fill="var(--mh-stone)" />
				<path d={CHURCH.tower} fill="var(--mh-stone)" />
				<path d={CHURCH.spire} fill="var(--mh-spire)" />
				<circle cx={CHURCH.clock.x} cy={CHURCH.clock.y} r={CHURCH.clock.r} fill="var(--lamp)" />
				<g fill={lit ? 'var(--window-lit)' : 'var(--window)'}>
					{#each CHURCH.windows as w, i (i)}<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="10" />{/each}
				</g>
				{#each CHALETS as c, i (i)}
					<rect x={c.x} y={LEDGE_Y - c.h} width={c.w} height={c.h} fill="var(--mh-wood)" />
					<path d="M{c.x} {LEDGE_Y - c.h * 0.66}h{c.w}M{c.x} {LEDGE_Y - c.h * 0.33}h{c.w}" stroke="var(--mh-wood2)" stroke-width="3" />
					<path d="M{c.x - 16} {LEDGE_Y - c.h}L{c.x + c.w / 2} {LEDGE_Y - c.h - 62}L{c.x + c.w + 16} {LEDGE_Y - c.h}Z" fill="var(--mh-roof)" />
					<path d="M{c.x - 16} {LEDGE_Y - c.h}L{c.x + c.w / 2} {LEDGE_Y - c.h - 62}L{c.x + c.w + 16} {LEDGE_Y - c.h}L{c.x + c.w + 6} {LEDGE_Y - c.h}L{c.x + c.w / 2} {LEDGE_Y - c.h - 50}L{c.x - 6} {LEDGE_Y - c.h}Z" fill="var(--mh-snow)" />
				{/each}
				<g>
					{#each CHALET_WINDOWS as w, i (i)}
						<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="2" fill="var(--window)" />
						{#if litSet.has(i)}<rect class="lit" x={w.x} y={w.y} width={w.w} height={w.h} rx="2" fill="var(--window-lit)" />{/if}
					{/each}
				</g>
				<path d={FLAG.pole} stroke="var(--lamp)" stroke-width="4" />
				<g transform="translate({FLAG.x} {FLAG.y})">
					<g class="flag">
						<rect x="2" y="0" width="34" height="34" fill="var(--mh-flag)" />
						<path d="M17 7h6v8h8v6h-8v8h-6v-8H9v-6h8z" fill="var(--mh-flag-cross)" />
					</g>
				</g>
			</svg>

			<!-- Our wall: the pigeons stand on it. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<rect x="0" y={LEDGE_Y} width={STAGE_W} height={STAGE_H - LEDGE_Y} fill="var(--mh-wall)" />
				<rect x="0" y={LEDGE_Y} width={STAGE_W} height="14" fill="var(--mh-wall-top)" />
				<path d="M0 {LEDGE_Y + 60}H{STAGE_W}M0 {LEDGE_Y + 104}H{STAGE_W}" stroke="var(--mh-wall-top)" stroke-width="2" opacity="0.6" />
			</svg>

			<!-- Mimi, by the flag: the umbrella goes up in the rain. -->
			<div class="cast" style:left="1520px" style:top="{LEDGE_Y + 6 - 175}px">
				<div class="pecker" class:pecker--busy={!trouble}>
					<WallPigeon class="mimi" lookX={trouble ? -0.8 : 0.3} eyes={startled ? 'startled' : 'open'} umbrella={raining} blinkOffset={3.1} />
				</div>
			</div>

			<!-- Gaston: strolls, pecks, tips his hat, holds the sign. -->
			<div class="cast stroller" style:transform="translateX({gastonX - 75}px)" style:transition-duration="{gastonWalkMs}ms" style:top="{LEDGE_Y + 6 - 187}px">
				<div class="waddle" class:waddle--on={gastonWalking} class:peck--on={gastonPeck}>
					<WallPigeon class="gaston" hat tip={gastonTip} sign={gastonSign} lookX={gastonLook} lookY={trouble ? -0.3 : 0} eyes={startled ? 'startled' : 'open'} blinkOffset={0.7} />
				</div>
			</div>

			{#each feathers as feather (feather.id)}
				<div class="feather" style:left="{feather.x}px" style:top="{LEDGE_Y - 150}px" style:--drift="{feather.drift}px" style:animation-delay="{feather.delay}ms">
					<svg viewBox="0 0 20 40"><path d="M10 2 C18 12 18 28 10 38 C2 28 2 12 10 2 Z" fill="var(--pg-body, #6f83a3)" /></svg>
				</div>
			{/each}

			{#if raining}
				<div class="rain">
					<svg viewBox="0 0 1920 1620"><path d={RAIN} stroke="var(--rain)" stroke-width="2.4" stroke-linecap="round" /></svg>
				</div>
				<div class="veil"></div>
				<svg class="layer bolt-layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
					<path class="bolt" d="M1180 0l-70 250 60-10-90 330 160-360-70 10z" fill="#fff" />
				</svg>
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
		background: var(--sky-bottom);
		/* The hour changes the sky over a minute: a slow fade, never a cut. */
		transition: --sky-top 60s linear, --sky-bottom 60s linear;
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
		transition: transform 60s linear, opacity 20s ease;
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
	.plume {
		animation: plume 14s ease-in-out infinite alternate;
		transform-origin: 1390px 215px;
	}
	@keyframes plume {
		from {
			transform: scaleX(0.85);
			opacity: 0.7;
		}
		to {
			transform: scaleX(1.1);
			opacity: 1;
		}
	}
	.flag {
		transform-origin: 2px 0;
		animation: wave 3s ease-in-out infinite alternate;
	}
	@keyframes wave {
		from {
			transform: skewY(-4deg) scaleX(0.96);
		}
		to {
			transform: skewY(4deg) scaleX(1);
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
	.mover {
		position: absolute;
		left: 0;
		top: 0;
		transform-origin: 0 0;
		will-change: transform;
	}
	.mover :global(.flyer) {
		display: block;
		width: 130px;
	}
	.train {
		display: block;
		width: 330px;
		height: 64px;
	}

	.cast {
		position: absolute;
		left: 0;
	}
	.cast :global(.gaston) {
		display: block;
		width: 150px;
	}
	.cast :global(.mimi) {
		display: block;
		width: 140px;
	}
	.stroller {
		transition-property: transform;
		transition-timing-function: linear;
	}
	.waddle,
	.pecker {
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
	.pecker--busy {
		animation: peck-loop 8s ease-in-out infinite;
	}
	@keyframes peck {
		30%,
		60% {
			transform: rotate(16deg) translateY(10px);
		}
	}
	@keyframes peck-loop {
		0%,
		62%,
		100% {
			transform: none;
		}
		68%,
		74% {
			transform: rotate(14deg) translateY(8px);
		}
		80% {
			transform: none;
		}
	}
	.feather {
		position: absolute;
		width: 20px;
		opacity: 0;
		animation: feather 6s ease-in forwards;
	}
	@keyframes feather {
		0% {
			opacity: 0.9;
			transform: translate(0, 0) rotate(0deg);
		}
		100% {
			opacity: 0;
			transform: translate(var(--drift), 220px) rotate(200deg);
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

	.flash {
		position: absolute;
		inset: 0;
		background: #d6defc;
		opacity: 0;
		animation: flash 11s linear infinite;
	}
	.bolt {
		opacity: 0;
		animation: bolt 11s linear infinite;
	}
	@keyframes flash {
		0%,
		90%,
		100% {
			opacity: 0;
		}
		91% {
			opacity: 0.5;
		}
		92% {
			opacity: 0.04;
		}
		93.5% {
			opacity: 0.38;
		}
		96% {
			opacity: 0;
		}
	}
	@keyframes bolt {
		0%,
		90.5%,
		94%,
		100% {
			opacity: 0;
		}
		91%,
		93.5% {
			opacity: 0.9;
		}
	}

	/* The gesture: the summit goes dark and its plume stops (down), the plume gusts and the
	   peak blushes (up), the mountain lights the Swiss flag (resolved). */
	.zm-shade {
		opacity: 0;
		animation: zm-shade 5.2s ease-in-out both;
	}
	.zm-glow {
		opacity: 0;
		mix-blend-mode: soft-light;
		animation: zm-glow 5.2s ease-in-out both;
	}
	.zm-show {
		opacity: 0;
		animation: zm-show 7s ease-in-out both;
	}
	.p-down .plume {
		animation: plume-down 5.2s ease-in-out both;
	}
	.p-up .plume {
		animation: plume-up 5.2s ease-out both;
	}
	@keyframes zm-shade {
		20%,
		75% {
			opacity: 0.6;
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes zm-glow {
		30%,
		60% {
			opacity: 0.9;
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes zm-show {
		10%,
		30% {
			opacity: 0.95;
		}
		22% {
			opacity: 0.55;
		}
		80% {
			opacity: 0.95;
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes plume-down {
		15%,
		75% {
			opacity: 0;
			transform: scaleX(0.3);
		}
	}
	@keyframes plume-up {
		0% {
			opacity: 0.2;
			transform: scaleX(0.4);
		}
		30% {
			opacity: 1;
			transform: scaleX(1.9);
		}
		100% {
			opacity: 0.85;
			transform: scaleX(1);
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
	.scene--still .flash,
	.scene--still .bolt {
		display: none;
	}
	@media (prefers-reduced-motion: reduce) {
		.flash,
		.bolt {
			display: none;
		}
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
