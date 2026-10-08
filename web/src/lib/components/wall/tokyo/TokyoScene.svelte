<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop, Tokyo edition: a low-rise rooftop under power
	 * lines, Tokyo Tower and the Skytree over a dense sea of small buildings,
	 * Mount Fuji faint in the haze, and the DumbMonit pigeons going about
	 * their day. A drop-in twin of the Paris scene (same props).
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky
	 * follows the real hour inside the brightness band of the wall's theme
	 * (see `../paris/daylight.ts`); after dark the windows come on a few at a
	 * time, the neon signs fade in, Tokyo Tower turns orange-gold and the
	 * Skytree blue, and the tower sparkles for the first five minutes of the
	 * hour.
	 *
	 * The network's state is woven in: clouds gather with advisories, a storm
	 * brings a soft drizzle and a darker sky and stops the blossom, the pigeon
	 * on the roof stops strolling and holds up a sign with the number of
	 * problems while looking at the list, the pigeon on the wire opens an
	 * umbrella, the napper by the water tank wakes up. A new problem startles
	 * everyone once.
	 *
	 * Motion is constant and gentle: drifting clouds and cherry-blossom
	 * petals, a stroll, pecks, blinks, a snore. Every 40–80 s one small moment
	 * plays (a fly-by, a shinkansen sliding along the viaduct, a gust of
	 * petals, the napper waking). All of it is transform and opacity on small
	 * layers; nothing flashes. Hidden tabs pause everything; reduced motion
	 * freezes the scene in a still pose.
	 */
	import { tick, untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import WallPigeon from '#lib/components/wall/paris/WallPigeon.svelte';
	import {
		STAGE_W,
		STAGE_H,
		LEDGE_Y,
		FUJI,
		FUJI_SNOW,
		FAR_BACK,
		FAR_FRONT,
		TOKYO_TOWER,
		TOKYO_TOWER_LATTICE,
		TT_SPARKS,
		TT_TOP,
		TT_DECKS,
		SKYTREE,
		SKYTREE_LATTICE,
		ST_TOP,
		ST_DECKS,
		VIADUCT,
		VIADUCT_Y,
		CITY,
		ZINC_SEAMS,
		POLES,
		WIRES,
		PERCH,
		wirePath,
		PETALS,
		STARS,
		RAIN,
		litWindows
	} from './geometry';
	import {
		phaseAt,
		isLit,
		celestialAt,
		isSparkleTime,
		sceneStyle,
		type SceneMood,
		type SceneTheme
	} from '#lib/components/wall/paris/daylight.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood: SceneMood;
		/** Problems on the board: the number on the protest sign. */
		problems: number;
		/** Bumped by the wall when a new problem appears: everyone startles once. */
		startle?: number;
		/** The tab is hidden: nothing moves, nothing is scheduled. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
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
		class: className = ''
	}: Props = $props();

	const motion = !reducedMotion();

	// --- Tokyo's own palette --------------------------------------------------------

	/** Tower paint, Skytree steel, neon, Fuji, train, blossom: [day, lit] per theme. */
	const PALETTE: Record<SceneTheme, Record<string, [string, string]>> = {
		light: {
			'--tt-red': ['#e4572e', '#f08a3c'],
			'--tt-white': ['#f6f1e7', '#ffe9c4'],
			'--st': ['#e9eef5', '#e9eef5'],
			'--st-line': ['#9fb0c8', '#4cb4e8'],
			'--neon-a': ['#e8628f', '#ff4d8d'],
			'--neon-b': ['#58b8d4', '#2fc4ea'],
			'--neon-c': ['#e8ac4c', '#ffb23c'],
			'--fuji': ['#b2bdd2', '#aab3cc'],
			'--fuji-snow': ['#f7f9fc', '#e8e8f4'],
			'--train': ['#f4f6fa', '#eceff6'],
			'--train-stripe': ['#2a6fd6', '#2a6fd6'],
			'--train-win': ['#8d9ab2', '#f2cf86'],
			'--petal': ['#f4b6c8', '#f0aac0']
		},
		dark: {
			'--tt-red': ['#a24c36', '#f2893a'],
			'--tt-white': ['#b9b4ab', '#ffd9a0'],
			'--st': ['#33435f', '#2f4a6e'],
			'--st-line': ['#4a5d82', '#7fd4ff'],
			'--neon-a': ['#d84a82', '#ff4d8d'],
			'--neon-b': ['#36b4d6', '#3cc8e8'],
			'--neon-c': ['#dc9a38', '#ffb23c'],
			'--fuji': ['#17233f', '#141d36'],
			'--fuji-snow': ['#8497bd', '#6c7ca4'],
			'--train': ['#aab6cf', '#8e9bb8'],
			'--train-stripe': ['#2a5fb0', '#2a5fb0'],
			'--train-win': ['#36456a', '#f5b542'],
			'--petal': ['#d99ab4', '#c88aa6']
		},
		oled: {
			'--tt-red': ['#2a1510', '#7a431a'],
			'--tt-white': ['#2c2c2e', '#8a6a3c'],
			'--st': ['#0e131c', '#0e131c'],
			'--st-line': ['#1b2433', '#2a5b73'],
			'--neon-a': ['#5a2038', '#7a2646'],
			'--neon-b': ['#184a5c', '#1e6074'],
			'--neon-c': ['#5c4218', '#7a5520'],
			'--fuji': ['#06080c', '#05070b'],
			'--fuji-snow': ['#1d2431', '#161c27'],
			'--train': ['#3a4150', '#2c323f'],
			'--train-stripe': ['#14305f', '#14305f'],
			'--train-win': ['#1a2030', '#7a5a1f'],
			'--petal': ['#6b4452', '#583846']
		}
	};

	// --- Stage fit ----------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light --------------------------------------------------------------------

	// Re-read once a minute: nothing in the sky moves faster than that.
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const style = $derived(
		sceneStyle(theme, phase, mood) +
			';' +
			Object.entries(PALETTE[theme])
				.map(([key, [day, night]]) => `${key}:${lit ? night : day}`)
				.join(';')
	);
	const starsOn = $derived(
		phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn')
	);

	// Windows light up in a different pattern every couple of minutes.
	let windowSeed = $state(1);
	const litSet = $derived(
		lit ? litWindows(windowSeed, CITY.windows.length, phase === 'night' ? 0.2 : 0.32) : new Set<number>()
	);

	const trouble = $derived(problems > 0);
	const raining = $derived(mood === 'storm');
	const cloudCount = $derived(mood === 'storm' ? 7 : mood === 'clouded' ? 5 : 3);
	const CLOUDS = [
		{ y: 120, s: 1.1, dur: 260, delay: -40 },
		{ y: 300, s: 0.8, dur: 320, delay: -210 },
		{ y: 440, s: 0.6, dur: 360, delay: -120 },
		{ y: 200, s: 0.95, dur: 290, delay: -150 },
		{ y: 380, s: 1.25, dur: 340, delay: -290 },
		{ y: 60, s: 0.7, dur: 300, delay: -250 },
		{ y: 500, s: 1.0, dur: 380, delay: -60 }
	];
	const petalCount = $derived(raining ? 0 : mood === 'clouded' ? 14 : PETALS.length);

	// --- The cast -------------------------------------------------------------------

	/** Gaston strolls along the parapet between these points. */
	const STROLL = [890, 1210];
	const SIGN_SPOT = 900;
	let gastonX = $state(1040);
	let gastonWalkMs = $state(0);
	let gastonWalking = $state(false);
	let gastonLook = $state(0);
	let gastonPeck = $state(false);

	/** Mimi, on the wire. */
	let mimiLookY = $state(0);
	let mimiLookX = $state(-0.3);

	/** Jules naps on the water tank. */
	let julesAwake = $state(false);
	let julesLook = $state(0);

	let startled = $state(false);
	let sparkleMoment = $state(false);
	const sparkling = $derived(lit && (sparkleMoment || isSparkleTime(minute)));

	/** One-off props for the moments. */
	let flyer = $state(false);
	let trainOn = $state(false);
	let gust = $state(false);
	let feathers = $state<{ id: number; x: number; drift: number; delay: number }[]>([]);
	let flyerEl = $state<HTMLDivElement | null>(null);
	let trainEl = $state<HTMLDivElement | null>(null);

	// --- Scheduling ------------------------------------------------------------------

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
		gust = false;
		gastonPeck = false;
		sparkleMoment = false;
		julesAwake = trouble;
		mimiLookY = 0;
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

	function animate(
		el: HTMLElement | null,
		frames: Keyframe[],
		options: KeyframeAnimationOptions
	): Promise<void> {
		if (!el || !('animate' in el)) return Promise.resolve();
		const animation = el.animate(frames, options);
		running.push(animation);
		return animation.finished.then(
			() => undefined,
			() => undefined
		);
	}

	type Moment = 'flyby' | 'shinkansen' | 'gust' | 'wake' | 'sparkle';
	let deck: Moment[] = [];
	function nextMoment(): Moment | null {
		const allowed: Moment[] = ['flyby', 'shinkansen', 'wake'];
		if (!raining) allowed.push('gust');
		if (lit) allowed.push('sparkle');
		deck = deck.filter((m) => allowed.includes(m));
		if (deck.length === 0) deck = [...allowed].sort(() => Math.random() - 0.5);
		return deck.shift() ?? null;
	}

	async function play(moment: Moment) {
		const mine = epoch;
		if (moment === 'flyby') {
			flyer = true;
			await tick();
			const y = between(230, 380);
			await animate(
				flyerEl,
				[
					{ transform: `translate(-260px, ${y + 40}px) rotate(4deg)` },
					{ transform: `translate(500px, ${y - 20}px) rotate(-2deg)`, offset: 0.3 },
					{ transform: `translate(1200px, ${y + 30}px) rotate(3deg)`, offset: 0.62 },
					{ transform: `translate(2160px, ${y - 60}px) rotate(-4deg)` }
				],
				{ duration: 19000, easing: 'linear', fill: 'forwards' }
			);
			if (mine === epoch) flyer = false;
		} else if (moment === 'shinkansen') {
			// A white streak along the viaduct: quick, quiet, gone.
			trainOn = true;
			await tick();
			await animate(
				trainEl,
				[{ transform: 'translateX(-780px)' }, { transform: 'translateX(1960px)' }],
				{ duration: 6500, easing: 'cubic-bezier(0.3, 0, 0.7, 1)', fill: 'forwards' }
			);
			if (mine === epoch) trainOn = false;
		} else if (moment === 'gust') {
			gust = true;
			later(() => (gust = false), 16000);
		} else if (moment === 'wake') {
			julesAwake = true;
			julesLook = -0.8;
			later(() => (julesLook = 0.8), 2600);
			later(() => (julesLook = 0), 5000);
			later(() => {
				if (!trouble) julesAwake = false;
			}, 6800);
		} else if (moment === 'sparkle') {
			sparkleMoment = true;
			later(() => (sparkleMoment = false), 24000);
		}
	}

	function scheduleMoments(first = false) {
		later(
			() => {
				const moment = nextMoment();
				if (moment) void play(moment);
				scheduleMoments();
			},
			first ? between(9000, 16000) : between(40000, 80000)
		);
	}

	function startAll() {
		julesAwake = trouble;
		julesLook = trouble ? 0.8 : 0;
		mimiLookX = trouble ? -0.8 : -0.3;
		if (!motion) {
			gastonX = trouble ? SIGN_SPOT : 1040;
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

	// Lit windows change now and then while the city is lit.
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

	const gastonSign = $derived(
		trouble ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null
	);
	const gastonEyes = $derived(startled ? 'startled' : 'open');
	const wires = WIRES.map((w) => wirePath(w));
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
							fill={mood === 'storm' ? 'var(--cloud-storm)' : 'var(--cloud)'}
						/>
					</svg>
				</div>
			{/each}

			{#if lit}
				<div class="glow beacon" style:left="{TT_TOP.x - 60}px" style:top="{TT_TOP.y - 60}px"></div>
				<div class="glow beacon beacon--slow" style:left="{ST_TOP.x - 60}px" style:top="{ST_TOP.y - 60}px"></div>
			{/if}

			<!-- Far: Fuji in the haze, two layers of distant rooftops, the two towers. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<g class="fuji">
					<path d={FUJI} fill="var(--fuji)" />
					<path d={FUJI_SNOW} fill="var(--fuji-snow)" />
				</g>
				<path d={FAR_BACK.path} fill="var(--far-2)" />
				<g fill="var(--window-lit)" class="far-lights" class:far-lights--on={lit}>
					{#each FAR_BACK.lights as p, i (i)}
						<circle cx={p.x} cy={p.y} r="1.5" />
					{/each}
				</g>

				<!-- Skytree -->
				<path d={SKYTREE} fill="var(--st)" />
				<path d={SKYTREE_LATTICE} fill="none" stroke="var(--st-line)" stroke-width="1.8" stroke-linecap="round" />
				{#if lit}
					<g class="deck-lights">
						{#each ST_DECKS as d, i (i)}
							<ellipse cx={d.x} cy={d.y} rx={d.w} ry="6" fill="var(--st-line)" opacity="0.5" />
						{/each}
					</g>
				{/if}

				<!-- Tokyo Tower -->
				<path d={TOKYO_TOWER} fill="var(--tt-red)" fill-rule="evenodd" />
				<path d={TOKYO_TOWER_LATTICE} fill="none" stroke="var(--tt-white)" stroke-width="2.2" stroke-linecap="round" />
				{#if lit}
					<g class="deck-lights">
						{#each TT_DECKS as d, i (i)}
							<ellipse cx={d.x} cy={d.y} rx={d.w} ry="7" fill="var(--tt-white)" opacity="0.45" />
						{/each}
						<circle class="aviation" cx={TT_TOP.x} cy={TT_TOP.y + 2} r="3.6" fill="#ff5a4a" />
					</g>
				{/if}
				{#if sparkling}
					<g class="sparkle">
						{#each TT_SPARKS as spark, i (i)}
							<circle cx={spark.x} cy={spark.y} r="3" fill="#fff6dc" style:animation-delay="{spark.delay}ms" />
						{/each}
					</g>
				{/if}

				<path d={FAR_FRONT.path} fill="var(--far)" />
				<g fill="var(--window-lit)" class="far-lights" class:far-lights--on={lit}>
					{#each FAR_FRONT.lights as p, i (i)}
						<circle cx={p.x} cy={p.y} r="1.7" />
					{/each}
				</g>

				<!-- The shinkansen viaduct, behind the nearest roofs. -->
				<path d={VIADUCT} fill="var(--far-2)" />
			</svg>

			{#if trainOn}
				<div class="mover train" bind:this={trainEl} style:top="{VIADUCT_Y - 30}px">
					<svg viewBox="0 0 740 32" width="740" height="32">
						{#each [0, 1, 2, 3, 4, 5] as car (car)}
							<rect x={car * 118} y="4" width="116" height="24" rx="3" fill="var(--train)" />
							<rect x={car * 118} y="19" width="116" height="4" fill="var(--train-stripe)" />
							<rect x={car * 118 + 8} y="9" width="100" height="5" rx="2" fill="var(--train-win)" />
						{/each}
						<path d="M708 4 L716 4 C730 8 740 20 740 28 L708 28 Z" fill="var(--train)" />
						<path d="M708 19 L738 19 L740 23 L708 23 Z" fill="var(--train-stripe)" />
						<path d="M722 9 L733 15 L722 15 Z" fill="var(--train-win)" />
					</svg>
				</div>
			{/if}

			<!-- The dense low city, neon after dark. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<path d={CITY.roof} fill="var(--roof)" />
				<path d={CITY.facade} fill="var(--facade)" />
				<path d={CITY.shade} fill="var(--facade-shade)" />
				<g fill="var(--window)">
					{#each CITY.windows as w, i (i)}
						<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="1.5" />
					{/each}
				</g>
				<g fill="var(--window-lit)">
					{#each CITY.windows as w, i (i)}
						{#if litSet.has(i)}
							<rect class="lit" x={w.x} y={w.y} width={w.w} height={w.h} rx="1.5" />
						{/if}
					{/each}
				</g>
				<!-- signs: dull panels by day... -->
				<g fill="var(--facade-shade)" stroke="var(--roof)" stroke-width="2">
					{#each CITY.signs as s, i (i)}
						<rect x={s.x} y={s.y} width={s.w} height={s.h} rx="2" />
					{/each}
				</g>
				<!-- ...lit neon once the lights are on -->
				<g class="neon" class:neon--on={lit}>
					{#each CITY.signs as s, i (i)}
						<g class="neon-sign" style:animation-delay="-{(i * 1700) % 9000}ms">
							<rect x={s.x - 6} y={s.y - 6} width={s.w + 12} height={s.h + 12} rx="8" fill="var(--neon-{s.c})" opacity="0.18" />
							<rect x={s.x + 2} y={s.y + 2} width={s.w - 4} height={s.h - 4} rx="2" fill="var(--neon-{s.c})" />
						</g>
					{/each}
				</g>
			</svg>

			{#if flyer}
				<div class="mover" bind:this={flyerEl}>
					<WallPigeon flap lookX={0.8} class="flyer" blinkOffset={2} />
				</div>
			{/if}

			<!-- Our rooftop: the water tank, the poles and wires, the parapet. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<!-- water tank on its stand -->
				<path d="M564 962 L568 898 L572 898 L576 962 Z M664 962 L668 898 L672 898 L676 962 Z" fill="var(--lamp)" />
				<rect x="556" y="846" width="128" height="58" rx="6" fill="var(--pot)" />
				<rect x="552" y="838" width="136" height="12" rx="4" fill="var(--near-top)" />
				<path d="M556 868 L684 868 M556 884 L684 884" stroke="var(--near)" stroke-width="3" opacity="0.6" />

				<!-- utility poles -->
				{#each POLES as pole (pole.x)}
					<rect x={pole.x - 4} y={pole.top} width="8" height={STAGE_H - pole.top} fill="var(--lamp)" />
					<rect x={pole.x - 34} y={pole.top - 2} width="68" height="6" rx="2" fill="var(--lamp)" />
					<rect x={pole.x - 28} y={pole.top + 18} width="56" height="5" rx="2" fill="var(--lamp)" />
					<rect x={pole.x - 12} y={pole.top + 42} width="24" height="38" rx="5" fill="var(--lamp)" />
				{/each}
				<g fill="none" stroke="var(--lamp)" stroke-width="2.2" stroke-linecap="round">
					{#each wires as d, i (i)}
						<path {d} />
					{/each}
				</g>

				<!-- the parapet and the metal roof below it -->
				<rect x="0" y={LEDGE_Y - 10} width={STAGE_W} height="16" fill="var(--near-top)" />
				<rect x="0" y={LEDGE_Y + 6} width={STAGE_W} height={STAGE_H - LEDGE_Y} fill="var(--near)" />
				<path d={ZINC_SEAMS} stroke="var(--seam)" stroke-width="3" />

				<!-- a rooftop shrine lantern, warm after dark -->
				<path d="M1470 962 L1470 906 M1470 906 L1494 906" stroke="var(--lamp)" stroke-width="4" fill="none" stroke-linecap="round" />
				<rect x="1486" y="906" width="16" height="30" rx="7" fill={lit ? 'var(--window-lit)' : 'var(--brick)'} stroke="var(--lamp)" stroke-width="3" />
			</svg>
			{#if lit}
				<div class="glow lamp-glow" style:left="1394px" style:top="864px"></div>
			{/if}

			<!-- Mimi, on the wire (an umbrella when it rains). -->
			<div class="cast" style:left="{PERCH.x - 70}px" style:top="{PERCH.y + 3 - 175}px">
				<div class="pecker" class:pecker--busy={!trouble && mimiLookY === 0}>
					<WallPigeon
						class="mimi"
						lookX={mimiLookX}
						lookY={mimiLookY}
						eyes={startled ? 'startled' : 'open'}
						umbrella={raining}
						blinkOffset={3.1}
					/>
				</div>
			</div>

			<!-- Jules, on the water tank: asleep, unless something is wrong. -->
			<div class="cast" style:left="565px" style:top="{838 + 4 - 137}px">
				<div class="napper" class:napper--asleep={!julesAwake && !startled}>
					<WallPigeon
						class="jules"
						eyes={julesAwake || startled ? (startled ? 'startled' : 'open') : 'closed'}
						lookX={julesLook}
						lookY={julesAwake ? 0.2 : 0}
						blinkOffset={5.4}
					/>
				</div>
				{#if !julesAwake && !startled}
					<svg class="zzz" viewBox="0 0 80 120">
						<text x="20" y="100">z</text>
						<text x="20" y="100">z</text>
						<text x="20" y="100">Z</text>
					</svg>
				{/if}
			</div>

			<!-- Gaston: strolls, pecks, holds the sign. -->
			<div
				class="cast stroller"
				style:transform="translateX({gastonX - 75}px)"
				style:transition-duration="{gastonWalkMs}ms"
				style:top="{LEDGE_Y + 6 - 187}px"
			>
				<div class="waddle" class:waddle--on={gastonWalking} class:peck--on={gastonPeck}>
					<WallPigeon
						class="gaston"
						sign={gastonSign}
						lookX={gastonLook}
						lookY={trouble ? -0.3 : 0}
						eyes={gastonEyes}
						blinkOffset={0.7}
					/>
				</div>
			</div>

			{#each feathers as feather (feather.id)}
				<div
					class="feather"
					style:left="{feather.x}px"
					style:top="{LEDGE_Y - 150}px"
					style:--drift="{feather.drift}px"
					style:animation-delay="{feather.delay}ms"
				>
					<svg viewBox="0 0 20 40"><path d="M10 2 C18 12 18 28 10 38 C2 28 2 12 10 2 Z" fill="var(--pg-body, #6f83a3)" /></svg>
				</div>
			{/each}

			<!-- Cherry blossom, drifting across everything. -->
			{#each PETALS.slice(0, petalCount) as p, i (i)}
				<div
					class="petal"
					style:left="{p.x}px"
					style:top="{p.y0}px"
					style:width="{p.size}px"
					style:--dx="{p.dx}px"
					style:--dy="{p.dy}px"
					style:--rot="{p.rot}deg"
					style:--still-x="{p.stillX}px"
					style:--still-y="{p.stillY + 40}px"
					style:animation-duration="{p.dur}s"
					style:animation-delay="{p.delay}s"
				>
					<svg
						viewBox="0 0 14 14"
						class="petal-sway"
						style:animation-duration="{p.sway}s"
						style:animation-direction={p.alt ? 'alternate-reverse' : 'alternate'}
					>
						<path d="M7 1 C10 1 13 5 11 9 L7 13 L3 9 C1 5 4 1 7 1 Z" fill="var(--petal)" />
					</svg>
				</div>
			{/each}
			{#if gust && !raining}
				{#each PETALS as p, i (i)}
					<div
						class="petal petal--gust"
						style:left="{(p.x * 0.5) % 900}px"
						style:top="{200 + ((i * 97) % 520)}px"
						style:width="{p.size}px"
						style:--rot="{p.rot}deg"
						style:animation-delay="{i * 180}ms"
					>
						<svg viewBox="0 0 14 14" class="petal-sway" style:animation-duration="{p.sway}s">
							<path d="M7 1 C10 1 13 5 11 9 L7 13 L3 9 C1 5 4 1 7 1 Z" fill="var(--petal)" />
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

	.fuji {
		opacity: 0.55;
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
		animation-name: drift;
		animation-timing-function: linear;
		animation-iteration-count: infinite;
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
	.lamp-glow {
		width: 220px;
		height: 220px;
		animation: breathe 9s ease-in-out infinite;
	}
	.beacon {
		animation: breathe 6s ease-in-out infinite;
	}
	.beacon--slow {
		animation-duration: 8s;
		animation-delay: -3s;
	}
	@keyframes breathe {
		50% {
			opacity: 0.55;
		}
	}
	.deck-lights,
	.aviation {
		animation: lamp-on 5s ease both;
	}
	.aviation {
		animation: aviation 4s ease-in-out infinite;
	}
	@keyframes aviation {
		0%,
		100% {
			opacity: 0.25;
		}
		50% {
			opacity: 0.95;
		}
	}
	.sparkle circle {
		opacity: 0;
		animation: sparkle 2.6s ease-in-out infinite;
	}
	@keyframes sparkle {
		0%,
		100% {
			opacity: 0;
		}
		50% {
			opacity: 0.85;
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

	.far-lights {
		opacity: 0;
		transition: opacity 20s ease;
	}
	.far-lights--on {
		opacity: 0.7;
	}

	/* Neon: fades in after dark and breathes very slowly; never blinks. */
	.neon {
		opacity: 0;
		transition: opacity 8s ease;
	}
	.neon--on {
		opacity: 1;
	}
	.neon-sign {
		animation: neon 9s ease-in-out infinite;
	}
	@keyframes neon {
		50% {
			opacity: 0.78;
		}
	}

	.mover {
		position: absolute;
		left: 0;
		top: 0;
		will-change: transform;
	}
	.mover :global(.flyer) {
		display: block;
		width: 120px;
	}
	.train {
		left: 0;
		transform: translateX(-780px);
	}
	.train svg {
		display: block;
	}

	.petal {
		position: absolute;
		opacity: 0.8;
		animation: petal linear infinite;
		will-change: transform;
	}
	.petal-sway {
		display: block;
		width: 100%;
		animation: sway ease-in-out infinite alternate;
	}
	@keyframes petal {
		0% {
			transform: translate(0, 0) rotate(0deg);
			opacity: 0;
		}
		6% {
			opacity: 0.85;
		}
		90% {
			opacity: 0.7;
		}
		100% {
			transform: translate(var(--dx), var(--dy)) rotate(var(--rot));
			opacity: 0;
		}
	}
	@keyframes sway {
		from {
			transform: translateX(-26px) rotate(-30deg);
		}
		to {
			transform: translateX(26px) rotate(30deg);
		}
	}
	.petal--gust {
		animation: gust 11s ease-in forwards;
		opacity: 0;
	}
	@keyframes gust {
		0% {
			transform: translate(0, 0) rotate(0deg);
			opacity: 0;
		}
		12% {
			opacity: 0.85;
		}
		85% {
			opacity: 0.7;
		}
		100% {
			transform: translate(1500px, 160px) rotate(var(--rot));
			opacity: 0;
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
	.cast :global(.mimi) {
		display: block;
		width: 140px;
	}
	.cast :global(.jules) {
		display: block;
		width: 110px;
	}
	.stroller {
		transition-property: transform;
		transition-timing-function: linear;
	}
	.waddle,
	.pecker,
	.napper {
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
		25% {
			transform: rotate(0deg) translateY(-5px);
		}
		50% {
			transform: rotate(4deg) translateY(0);
		}
		75% {
			transform: rotate(0deg) translateY(-5px);
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
		68% {
			transform: rotate(18deg) translate(8px, 12px);
		}
		73% {
			transform: none;
		}
		79% {
			transform: rotate(18deg) translate(8px, 12px);
		}
		84% {
			transform: none;
		}
	}
	/* Asleep: a slow breath, the chest rising. */
	.napper--asleep {
		animation: snore 4.5s ease-in-out infinite;
	}
	@keyframes snore {
		50% {
			transform: scale(1.04, 0.97);
		}
	}
	.zzz {
		position: absolute;
		left: 66px;
		top: -30px;
		width: 80px;
		overflow: visible;
	}
	.zzz text {
		font-size: 30px;
		font-weight: 700;
		fill: var(--c-ink-2);
		opacity: 0;
		animation: zzz 7.5s ease-out infinite;
	}
	.zzz text:nth-child(2) {
		animation-delay: -2.5s;
	}
	.zzz text:nth-child(3) {
		animation-delay: -5s;
	}
	@keyframes zzz {
		0% {
			transform: translate(0, 0) scale(0.6);
			opacity: 0;
		}
		25% {
			opacity: 0.75;
		}
		100% {
			transform: translate(34px, -80px) scale(1.15);
			opacity: 0;
		}
	}

	.feather {
		position: absolute;
		width: 14px;
		opacity: 0;
		animation: feather 6s ease-in-out forwards;
	}
	@keyframes feather {
		0% {
			opacity: 0;
			transform: translate(0, 0) rotate(0deg);
		}
		10% {
			opacity: 0.9;
		}
		100% {
			opacity: 0;
			transform: translate(var(--drift), 150px) rotate(160deg);
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
	.scene--still .petal {
		transform: translate(var(--still-x), var(--still-y));
	}
	.scene--still .petal--gust,
	.scene--still .train {
		display: none;
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
