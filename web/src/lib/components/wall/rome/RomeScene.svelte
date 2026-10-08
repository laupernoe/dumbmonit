<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop: a Roman terracotta rooftop under the dome of
	 * St Peter's and the Colosseum, with the DumbMonit pigeons going about their
	 * day. A drop-in for the Paris scene (same props).
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky
	 * follows the real hour — dawn, day, golden hour, dusk, night, with the sun
	 * or the moon on its arc, lit windows, the floodlit Colosseum and the
	 * lantern of the dome after dark — inside the brightness band of the wall's
	 * theme (see `paris/daylight.ts`).
	 *
	 * The network's state is woven in: clouds gather with advisories, a storm
	 * brings a soft drizzle and a slightly darker sky, the pigeon in the beret
	 * stops strolling and holds up a sign with the number of problems while
	 * looking at the list, the ciabatta pigeon opens an umbrella, the napper
	 * wakes up. A new problem startles everyone once.
	 *
	 * Motion is constant and gentle: drifting clouds, laundry swaying on the
	 * line, chimney smoke, a stroll, pecks, blinks, a snore. Every 40–80 s one
	 * small moment plays (a fly-by, a bread delivery that drops a crumb, a hat
	 * tip, the napper waking for a look around, and, rarely, a flock of swifts
	 * screaming across the sky). All of it is transform and opacity on small
	 * layers; nothing flashes. Hidden tabs pause everything; reduced motion
	 * freezes the scene in a still pose.
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
		FAR_BACK,
		FAR_FRONT,
		PETERS,
		DOME_TOP,
		COLOSSEUM,
		PINES,
		ROW,
		ALTANA,
		LAUNDRY_LINE,
		LAUNDRY_MAST,
		LAUNDRY,
		CLOTH_PATHS,
		TILE_ARCS,
		SWIFTS,
		STARS,
		RAIN,
		litWindows,
		romeStyle
	} from './geometry.js';

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

	// --- Stage fit ----------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(
		Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale))
	);
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light --------------------------------------------------------------------

	// Re-read once a minute: nothing in the sky moves faster than that.
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const style = $derived(`${sceneStyle(theme, phase, mood)};${romeStyle(theme)}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));

	// Windows light up in a different pattern every couple of minutes.
	let windowSeed = $state(1);
	const litSet = $derived(lit ? litWindows(windowSeed, ROW.windows.length, phase === 'night' ? 0.18 : 0.3) : new Set<number>());
	const peterLit = $derived(lit ? litWindows(windowSeed + 7, PETERS.windows.length, 0.55) : new Set<number>());

	const trouble = $derived(problems > 0);
	const raining = $derived(mood === 'storm');
	const cloudCount = $derived(mood === 'storm' ? 7 : mood === 'clouded' ? 5 : 3);
	const CLOUDS = [
		{ y: 120, s: 1.1, dur: 260, delay: -40 },
		{ y: 300, s: 0.8, dur: 320, delay: -210 },
		{ y: 470, s: 0.6, dur: 360, delay: -120 },
		{ y: 200, s: 0.95, dur: 290, delay: -150 },
		{ y: 400, s: 1.25, dur: 340, delay: -290 },
		{ y: 60, s: 0.7, dur: 300, delay: -250 },
		{ y: 540, s: 1.0, dur: 380, delay: -60 }
	];

	// --- The cast -------------------------------------------------------------------

	/** Gaston, in the beret, strolls along the parapet between these points. */
	const STROLL = [890, 1210];
	const SIGN_SPOT = 900;
	let gastonX = $state(1040);
	let gastonWalkMs = $state(0);
	let gastonWalking = $state(false);
	let gastonLook = $state(0);
	let gastonPeck = $state(false);
	let gastonTip = $state(false);

	/** Mimi pecks at her ciabatta by the chimney. */
	let mimiLookY = $state(0);
	let mimiLookX = $state(0.3);

	/** Jules naps on the lamp post. */
	let julesAwake = $state(false);
	let julesLook = $state(0);

	let startled = $state(false);

	/** One-off props for the moments. */
	let flyer = $state<null | { baguette: boolean }>(null);
	let swiftsOn = $state(false);
	let crumbOn = $state(false);
	let feathers = $state<{ id: number; x: number; drift: number; delay: number }[]>([]);
	let flyerEl = $state<HTMLDivElement | null>(null);
	let swiftsEl = $state<HTMLDivElement | null>(null);
	let crumbEl = $state<HTMLDivElement | null>(null);

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
		flyer = null;
		swiftsOn = false;
		crumbOn = false;
		gastonTip = false;
		gastonPeck = false;
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
			// Now and then a peck at something on the stone.
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
		return animation.finished.then(
			() => undefined,
			() => undefined
		);
	}

	type Moment = 'flyby' | 'baguette' | 'swifts' | 'beret' | 'wake';
	let deck: Moment[] = [];
	function nextMoment(): Moment | null {
		const allowed: Moment[] = ['flyby', 'baguette', 'wake'];
		if (!trouble) allowed.push('beret');
		// Swifts are a daytime thing, and a rare one: one draw in three.
		if (phase !== 'night' && Math.random() < 0.34) allowed.push('swifts');
		deck = deck.filter((m) => allowed.includes(m));
		if (deck.length === 0) deck = [...allowed].sort(() => Math.random() - 0.5);
		return deck.shift() ?? null;
	}

	async function play(moment: Moment) {
		const mine = epoch;
		if (moment === 'flyby' || moment === 'baguette') {
			flyer = { baguette: moment === 'baguette' };
			await tick();
			const y = between(230, 380);
			const duration = 19000;
			if (moment === 'baguette') {
				// The crumb leaves the bread right above Mimi, who looks up, then eats it.
				later(() => {
					crumbOn = true;
					mimiLookY = -1;
					void tick().then(() =>
						animate(crumbEl, [
							{ transform: `translate(1600px, ${y + 120}px) rotate(0deg)` },
							{ transform: `translate(1654px, ${LEDGE_Y - 14}px) rotate(220deg)` }
						], { duration: 2600, easing: 'cubic-bezier(0.45, 0, 0.85, 0.6)', fill: 'forwards' })
					);
				}, duration * 0.78);
				later(() => (mimiLookY = 0.7), duration * 0.78 + 2600);
				later(() => {
					crumbOn = false;
					mimiLookY = 0;
				}, duration * 0.78 + 5200);
			}
			await animate(
				flyerEl,
				[
					{ transform: `translate(-260px, ${y + 40}px) rotate(4deg)` },
					{ transform: `translate(500px, ${y - 20}px) rotate(-2deg)`, offset: 0.3 },
					{ transform: `translate(1200px, ${y + 30}px) rotate(3deg)`, offset: 0.62 },
					{ transform: `translate(2160px, ${y - 60}px) rotate(-4deg)` }
				],
				{ duration, easing: 'linear', fill: 'forwards' }
			);
			if (mine === epoch) flyer = null;
		} else if (moment === 'swifts') {
			swiftsOn = true;
			await tick();
			const y = between(120, 300);
			await animate(
				swiftsEl,
				[
					{ transform: `translate(-420px, ${y + 70}px)` },
					{ transform: `translate(900px, ${y - 40}px)`, offset: 0.5 },
					{ transform: `translate(2200px, ${y - 130}px)` }
				],
				{ duration: 15000, easing: 'cubic-bezier(0.4, 0, 0.6, 1)', fill: 'forwards' }
			);
			if (mine === epoch) swiftsOn = false;
		} else if (moment === 'beret') {
			// Stop, face the room, lift the beret — and put it back.
			later(() => {
				gastonLook = 0;
				gastonTip = true;
			}, 200);
			later(() => (gastonTip = false), 2600);
		} else if (moment === 'wake') {
			julesAwake = true;
			julesLook = -0.8;
			later(() => (julesLook = 0.8), 2600);
			later(() => (julesLook = 0), 5000);
			later(() => {
				if (!trouble) julesAwake = false;
			}, 6800);
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
		julesLook = trouble ? -0.8 : 0;
		mimiLookX = trouble ? -0.8 : 0.3;
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

	const gastonSign = $derived(trouble ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null);
	const gastonEyes = $derived(startled ? 'startled' : 'open');
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
		<div
			class="stage"
			style:transform="translate({offsetX}px, {offsetY}px) scale({scale})"
		>
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
				<div class="glow beacon" style:left="{DOME_TOP.x - 60}px" style:top="{DOME_TOP.y - 60}px"></div>
				<div class="glow flood" style:left="1160px" style:top="700px"></div>
			{/if}

			<!-- The city: far roofs, St Peter's, the Colosseum, umbrella pines, the row of houses. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<defs>
					<clipPath id="rome-colosseum"><path d={COLOSSEUM.body} /></clipPath>
				</defs>
				<path d={FAR_BACK} fill="var(--far-2)" />
				<path d={FAR_FRONT} fill="var(--far)" />

				<!-- St Peter's -->
				<path d={PETERS.facade} fill="var(--stone)" />
				<path d={PETERS.smallDomes} fill="var(--stone-shade)" />
				<path d={PETERS.drum} fill="var(--stone)" />
				<path d={PETERS.dome} fill="var(--dome)" />
				<path d={PETERS.domeShade} fill="var(--stone-shade)" opacity="0.45" />
				<path d={PETERS.ribs} fill="none" stroke="var(--stone-shade)" stroke-width="2.4" stroke-linecap="round" />
				<path d={PETERS.lantern} fill="var(--stone)" />
				<path d={PETERS.cross} fill="none" stroke="var(--stone-shade)" stroke-width="3" stroke-linecap="round" />
				<g fill="var(--window)">
					{#each PETERS.windows as w, i (i)}
						<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="3" />
					{/each}
				</g>
				<g fill="var(--window-lit)">
					{#each PETERS.windows as w, i (i)}
						{#if peterLit.has(i)}
							<rect class="lit" x={w.x} y={w.y} width={w.w} height={w.h} rx="3" />
						{/if}
					{/each}
				</g>

				<!-- The Colosseum: whole on the left, broken away on the right. -->
				<path d={COLOSSEUM.body} fill="var(--stone)" />
				<g clip-path="url(#rome-colosseum)">
					<rect x="1380" y="740" width="200" height="120" fill="var(--stone-shade)" opacity="0.4" />
					<path d={COLOSSEUM.cornice} stroke="var(--stone-shade)" stroke-width="3" fill="none" />
				</g>
				<path d={COLOSSEUM.arches} fill="var(--window)" />
				{#if lit}
					<path class="lit" d={COLOSSEUM.arches} fill="var(--window-lit)" opacity="0.62" />
				{/if}

				<!-- Umbrella pines -->
				{#each PINES as pine, i (i)}
					<path d={pine.trunk} fill="var(--trunk)" />
					<path d={pine.canopy} fill="var(--pine)" />
				{/each}

				<!-- Ochre houses under terracotta -->
				<path d={ROW.roofs} fill="var(--tile)" />
				<path d={ROW.chimneys} fill="var(--tile-dark)" />
				<path d={ROW.facadeA} fill="var(--ochre)" />
				<path d={ROW.facadeB} fill="var(--ochre-2)" />
				<path d={ROW.shade} fill="var(--facade-shade)" opacity="0.35" />
				<g fill="var(--window)">
					{#each ROW.windows as w, i (i)}
						<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="7" />
					{/each}
				</g>
				<g fill="var(--window-lit)">
					{#each ROW.windows as w, i (i)}
						{#if litSet.has(i)}
							<rect class="lit" x={w.x} y={w.y} width={w.w} height={w.h} rx="7" />
						{/if}
					{/each}
				</g>

				<!-- The altana, a rooftop loggia -->
				<path d={ALTANA.plinth} fill="var(--ochre)" />
				<path d={ALTANA.back} fill={lit ? 'var(--window-lit)' : 'var(--window)'} opacity={lit ? 0.7 : 1} />
				<path d={ALTANA.roof} fill="var(--tile)" />
				<path d={ALTANA.posts} fill="var(--stone)" />
				<path d={ALTANA.floor} fill="var(--stone-shade)" />
			</svg>

			{#if swiftsOn}
				<div class="mover" bind:this={swiftsEl}>
					{#each SWIFTS as swift, i (i)}
						<svg
							viewBox="-14 -10 76 24"
							class="swift"
							style:left="{swift.x}px"
							style:top="{swift.y}px"
							style:animation-delay="{swift.delay}s"
						>
							<path d="M24 10 C14 8 4 0 -12 -7 C6 -3 14 2 24 6 C34 2 42 -3 60 -7 C44 0 34 8 24 10 Z" fill="var(--swift)" />
							<path d="M20 9 L24 16 L28 9 Z" fill="var(--swift)" />
						</svg>
					{/each}
				</div>
			{/if}

			{#if flyer}
				<div class="mover" bind:this={flyerEl}>
					<WallPigeon flap baguette={flyer.baguette} lookX={0.8} class="flyer" blinkOffset={2} />
				</div>
			{/if}

			{#if lit}
				<div class="glow lamp-glow" style:left="1690px" style:top="668px"></div>
			{/if}

			<!-- Our rooftop: the washing line, chimney, lemon pots, the parapet, the lamp post. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<!-- washing line -->
				<path d={ALTANA.mast} stroke="var(--lamp)" stroke-width="5" stroke-linecap="round" fill="none" />
				<path d={LAUNDRY_MAST} stroke="var(--lamp)" stroke-width="6" stroke-linecap="round" fill="none" />
				<path d={LAUNDRY_LINE} stroke="var(--lamp)" stroke-width="2" fill="none" />
				{#each LAUNDRY as item, i (i)}
					<g transform="translate({item.x} {item.y})">
						<g class="cloth" style:animation-delay="{item.delay}s">
							<path d={CLOTH_PATHS[item.kind]} fill="var(--cloth-{item.tone})" />
							<rect x="-4" y="-5" width="8" height="10" rx="2" fill="var(--tile-dark)" />
						</g>
					</g>
				{/each}

				<!-- chimney stack and its pots -->
				<rect x="1440" y="872" width="84" height="84" fill="var(--brick)" />
				<rect x="1434" y="866" width="96" height="10" fill="var(--tile-dark)" />
				<rect x="1446" y="842" width="18" height="26" rx="3" fill="var(--pot)" />
				<rect x="1472" y="836" width="20" height="32" rx="3" fill="var(--pot)" />
				<rect x="1500" y="846" width="18" height="22" rx="3" fill="var(--pot)" />

				<!-- terracotta pots with lemon trees -->
				{#each [1282, 1326, 1370] as px, i (i)}
					<path d="M{px - 18} {LEDGE_Y - 10} L{px - 14} {LEDGE_Y - 42} H{px + 14} L{px + 18} {LEDGE_Y - 10} Z" fill="var(--pot)" />
					<rect x={px - 17} y={LEDGE_Y - 48} width="34" height="9" rx="3" fill="var(--pot)" />
					<ellipse cx={px} cy={LEDGE_Y - 68 - (i % 2) * 6} rx="24" ry="22" fill="var(--pine)" />
					<g fill="var(--lemon)">
						<circle cx={px - 9} cy={LEDGE_Y - 68 - (i % 2) * 6} r="5" />
						<circle cx={px + 10} cy={LEDGE_Y - 74 - (i % 2) * 6} r="5" />
						<circle cx={px + 2} cy={LEDGE_Y - 58 - (i % 2) * 6} r="4.5" />
					</g>
				{/each}

				<!-- the parapet and the terracotta roof below it -->
				<rect x="0" y={LEDGE_Y - 10} width={STAGE_W} height="16" fill="var(--tile-dark)" />
				<rect x="0" y={LEDGE_Y + 6} width={STAGE_W} height={STAGE_H - LEDGE_Y} fill="var(--tile)" />
				<path d={TILE_ARCS} stroke="var(--tile-dark)" stroke-width="3" fill="none" stroke-linecap="round" />

				<!-- Mimi's ciabatta -->
				<path
					d="M1660 960 C1654 934 1700 920 1722 922 C1752 922 1774 940 1766 960 Z"
					fill="var(--pastry, #e2b36c)"
					stroke="var(--pg-line, #1e2640)"
					stroke-width="4"
					stroke-linejoin="round"
				/>
				<path d="M1688 930 l8 20 M1712 927 l6 24 M1738 930 l4 21" stroke="var(--pastry-crust, #b98240)" stroke-width="4" stroke-linecap="round" />
				<g fill="var(--pastry, #e2b36c)">
					<circle cx="1640" cy="958" r="3.5" />
					<circle cx="1780" cy="957" r="3" />
				</g>

				<!-- lamp post, lantern lit after dark -->
				<path d="M1794 1080 L1796 800 L1804 800 L1806 1080 Z" fill="var(--lamp)" />
				<path d="M1786 806 L1814 806 L1810 796 L1790 796 Z" fill="var(--lamp)" />
				<path d="M1782 796 L1818 796 L1824 758 L1776 758 Z" fill={lit ? 'var(--window-lit)' : 'var(--window)'} stroke="var(--lamp)" stroke-width="5" stroke-linejoin="round" />
				<path d="M1770 760 L1830 760 L1800 744 Z" fill="var(--lamp)" />
			</svg>

			<!-- Chimney smoke, slow and pale. -->
			<div class="smoke" style:left="1472px" style:top="790px">
				<span></span><span></span><span></span>
			</div>

			{#if crumbOn}
				<div class="mover" bind:this={crumbEl}>
					<svg viewBox="0 0 14 14" class="crumb"><circle cx="7" cy="7" r="6" fill="var(--pastry, #e2b36c)" /></svg>
				</div>
			{/if}

			<!-- Mimi, by her ciabatta (an umbrella when it rains). -->
			<div class="cast" style:left="1540px" style:top="{LEDGE_Y + 6 - 175}px">
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

			<!-- Jules, on the lamp: asleep, unless something is wrong. -->
			<div class="cast" style:left="1745px" style:top="{744 + 4 - 137}px">
				<div class="napper" class:napper--asleep={!julesAwake && !startled}>
					<WallPigeon class="jules" eyes={julesAwake || startled ? (startled ? 'startled' : 'open') : 'closed'} lookX={julesLook} lookY={julesAwake ? 0.2 : 0} blinkOffset={5.4} />
				</div>
				{#if !julesAwake && !startled}
					<svg class="zzz" viewBox="0 0 80 120">
						<text x="20" y="100">z</text>
						<text x="20" y="100">z</text>
						<text x="20" y="100">Z</text>
					</svg>
				{/if}
			</div>

			<!-- Gaston, in the beret: strolls, pecks, tips his hat, holds the sign. -->
			<div
				class="cast stroller"
				style:transform="translateX({gastonX - 75}px)"
				style:transition-duration="{gastonWalkMs}ms"
				style:top="{LEDGE_Y + 6 - 187}px"
			>
				<div class="waddle" class:waddle--on={gastonWalking} class:peck--on={gastonPeck}>
					<WallPigeon
						class="gaston"
						hat
						tip={gastonTip}
						sign={gastonSign}
						lookX={gastonLook}
						lookY={trouble ? -0.3 : 0}
						eyes={gastonEyes}
						blinkOffset={0.7}
					/>
				</div>
			</div>

			{#each feathers as feather (feather.id)}
				<div class="feather" style:left="{feather.x}px" style:top="{LEDGE_Y - 150}px" style:--drift="{feather.drift}px" style:animation-delay="{feather.delay}ms">
					<svg viewBox="0 0 20 40"><path d="M10 2 C18 12 18 28 10 38 C2 28 2 12 10 2 Z" fill="var(--pg-body, #6f83a3)" /></svg>
				</div>
			{/each}

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
	}
	.celestial--hidden {
		opacity: 0;
	}
	.beacon {
		animation: breathe 6s ease-in-out infinite;
	}
	.lamp-glow {
		animation: breathe 9s ease-in-out infinite;
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

	.swift {
		position: absolute;
		display: block;
		width: 46px;
		transform-origin: 50% 60%;
		animation: flit 0.55s ease-in-out infinite alternate;
	}
	@keyframes flit {
		to {
			transform: scaleY(0.55) translateY(2px);
		}
	}

	.cloth {
		transform-origin: 0 0;
		animation: sway-cloth 5.5s ease-in-out infinite alternate;
	}
	@keyframes sway-cloth {
		from {
			transform: rotate(-2.5deg);
		}
		to {
			transform: rotate(3deg);
		}
	}

	.flood {
		width: 520px;
		height: 220px;
		border-radius: 50%;
		animation: breathe 11s ease-in-out infinite;
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
	.crumb {
		display: block;
		width: 12px;
	}

	.smoke {
		position: absolute;
		width: 120px;
		height: 60px;
	}
	.smoke span {
		position: absolute;
		left: 0;
		bottom: 0;
		width: 44px;
		height: 44px;
		border-radius: 50%;
		background: radial-gradient(circle, var(--smoke), transparent 70%);
		opacity: 0;
		animation: puff 12s ease-out infinite;
	}
	.smoke span:nth-child(2) {
		animation-delay: -4s;
	}
	.smoke span:nth-child(3) {
		animation-delay: -8s;
	}
	@keyframes puff {
		0% {
			transform: translate(0, 0) scale(0.5);
			opacity: 0;
		}
		20% {
			opacity: 0.8;
		}
		100% {
			transform: translate(46px, -170px) scale(2.2);
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
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
