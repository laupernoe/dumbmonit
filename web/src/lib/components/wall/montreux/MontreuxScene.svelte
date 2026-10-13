<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's Swiss backdrop: the quay at Montreux on Lake Geneva, Chillon on
	 * its rock, the Dents du Midi across the water, the vineyard terraces of
	 * Lavaux and a paddle steamer crossing, with the DumbMonit pigeons on the
	 * quay. A drop-in for the Paris scene (same props).
	 *
	 * Drawn on a 1920 x 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky follows
	 * the real hour (see `paris/daylight.ts`): sun or moon on its arc, stars,
	 * lit castle windows, shore lights and a lamp after dark. The network's state
	 * is woven in: clouds gather with advisories, a storm brings drizzle, the
	 * pigeon in the beret holds up a sign with the number of problems, the
	 * napper wakes, the quay pigeon opens an umbrella. A new problem startles
	 * everyone once.
	 *
	 * Motion is gentle: drifting clouds, the steamer's slow crossing and its
	 * smoke, ripples, a stroll, pecks, a snore; now and then a fly-by or a hat
	 * tip. All transform and opacity. Hidden tabs pause everything; reduced
	 * motion freezes the scene in a still pose.
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
		LAMP,
		SHORE_Y,
		PEAKS,
		PEAK_SHADE,
		SNOW,
		FAR_SHORE,
		LAVAUX,
		TERRACES,
		HOUSES,
		CASTLE,
		CASTLE_WINDOWS,
		SHORE_LIGHTS,
		CLOUDS,
		STARS,
		RAIN,
		litWindows,
		montreuxStyle
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

	// --- Stage fit ----------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light --------------------------------------------------------------------

	// Re-read once a minute: nothing in the sky moves faster than that.
	const minute = $derived(new Date(Math.floor(now.getTime() / 60_000) * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const style = $derived(`${sceneStyle(theme, phase, mood)};${montreuxStyle(theme)}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));

	// Castle windows light up in a different pattern every couple of minutes.
	let windowSeed = $state(1);
	const litSet = $derived(
		lit ? litWindows(windowSeed, CASTLE_WINDOWS.length, 0.6) : new Set<number>()
	);

	const trouble = $derived(problems > 0);
	const raining = $derived(mood === 'storm');
	const cloudCount = $derived(mood === 'storm' ? 5 : mood === 'clouded' ? 4 : 2);

	// --- The cast -------------------------------------------------------------------

	/** Gaston, in the beret, strolls along the quay between these points. */
	const STROLL = [760, 1120];
	const SIGN_SPOT = 920;
	let gastonX = $state(960);
	let gastonWalkMs = $state(0);
	let gastonWalking = $state(false);
	let gastonLook = $state(0);
	let gastonPeck = $state(false);
	let gastonTip = $state(false);
	let julesAwake = $state(false);
	let julesLook = $state(0);
	let startled = $state(false);
	let flyer = $state(false);
	let flyerEl = $state<HTMLDivElement | null>(null);

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
		gastonTip = false;
		gastonPeck = false;
		julesAwake = trouble;
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

	type Moment = 'flyby' | 'beret' | 'wake';
	let deck: Moment[] = [];
	async function play(moment: Moment) {
		const mine = epoch;
		if (moment === 'flyby') {
			flyer = true;
			await tick();
			const y = between(300, 520);
			const el = flyerEl;
			if (!el || !('animate' in el)) return;
			const animation = el.animate(
				[
					{ transform: `translate(-260px, ${y + 40}px) rotate(4deg)` },
					{ transform: `translate(900px, ${y - 20}px) rotate(-2deg)`, offset: 0.5 },
					{ transform: `translate(2160px, ${y - 60}px) rotate(-4deg)` }
				],
				{ duration: 19000, easing: 'linear', fill: 'forwards' }
			);
			running.push(animation);
			await animation.finished.catch(() => undefined);
			if (mine === epoch) flyer = false;
		} else if (moment === 'beret') {
			// Stop, face the room, lift the beret, put it back.
			later(() => {
				gastonLook = 0;
				gastonTip = true;
			}, 200);
			later(() => (gastonTip = false), 2600);
		} else {
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
				const allowed: Moment[] = trouble ? ['flyby', 'wake'] : ['flyby', 'wake', 'beret'];
				deck = deck.filter((x) => allowed.includes(x));
				if (deck.length === 0) deck = [...allowed].sort(() => Math.random() - 0.5);
				const moment = deck.shift();
				if (moment) void play(moment);
				scheduleMoments();
			},
			first ? between(9000, 16000) : between(40000, 80000)
		);
	}

	function startAll() {
		julesAwake = trouble;
		julesLook = trouble ? -0.8 : 0;
		if (!motion) {
			gastonX = trouble ? SIGN_SPOT : 960;
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

	// Lit windows change now and then while the castle is lit.
	$effect(() => {
		if (!lit || paused || !motion) return;
		const timer = setInterval(() => (windowSeed += 1), 120_000);
		return () => clearInterval(timer);
	});

	// A new problem: everyone startles once.
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

	const gastonSign = $derived(
		trouble ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null
	);
	const eyes = $derived(startled ? 'startled' : 'open');
	const box = `0 0 ${STAGE_W} ${STAGE_H}`;
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

			<svg class="layer stars" class:stars--on={starsOn} viewBox={box}>
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

			<!-- The land and the lake: Dents du Midi, Lavaux, Chillon, shore lights. -->
			<svg class="layer" viewBox={box}>
				<defs>
					<clipPath id="mt-peaks"><path d={PEAKS} /></clipPath>
					<clipPath id="mt-lavaux"><path d={LAVAUX} /></clipPath>
					<linearGradient id="mt-lake" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--mt-water-1)" />
						<stop offset="1" stop-color="var(--mt-water-2)" />
					</linearGradient>
				</defs>
				<path d={PEAKS} fill="var(--mt-peak)" />
				<g clip-path="url(#mt-peaks)">
					<path d={PEAK_SHADE} fill="var(--mt-peak-2)" />
					<path d={SNOW} fill="var(--mt-snow)" />
				</g>
				<rect x="0" y="520" width={STAGE_W} height="190" fill="var(--mt-haze)" />
				<path d={FAR_SHORE} fill="var(--mt-hill)" />
				<path d={LAVAUX} fill="var(--mt-vine)" />
				<path d={TERRACES} stroke="var(--mt-vine-2)" stroke-width="5" clip-path="url(#mt-lavaux)" />
				{#each HOUSES as h, i (i)}
					<rect x={h.x} y={h.y} width="22" height="14" fill="var(--mt-wall)" />
					<path d="M{h.x - 3} {h.y}l14 -11 14 11z" fill="var(--mt-roof)" />
					{#if lit}<rect x={h.x + 8} y={h.y + 5} width="5" height="5" fill="var(--window-lit)" />{/if}
				{/each}

				<rect x="0" y={SHORE_Y} width={STAGE_W} height={LEDGE_Y - SHORE_Y} fill="url(#mt-lake)" />

				<!-- Chillon, with its reflection -->
				<g id="mt-castle" transform="translate({CASTLE.x} {CASTLE.y}) scale({CASTLE.scale})">
					<path d="M-24 10Q-12 -8 12 -8H318Q342 -8 354 10Z" fill="var(--mt-rock)" />
					<rect x="150" y="-170" width="54" height="112" fill="var(--mt-wall)" />
					<rect x="190" y="-170" width="14" height="112" fill="var(--mt-wall-2)" />
					<path d="M144-170L177-216L210-170Z" fill="var(--mt-roof)" />
					<rect x="44" y="-100" width="96" height="44" fill="var(--mt-wall)" />
					<path d="M38-100L92-148L146-100Z" fill="var(--mt-roof-2)" />
					<rect x="236" y="-134" width="44" height="80" fill="var(--mt-wall)" />
					<rect x="268" y="-134" width="12" height="80" fill="var(--mt-wall-2)" />
					<path d="M230-134L258-192L286-134Z" fill="var(--mt-roof)" />
					<rect x="0" y="-62" width="330" height="62" fill="var(--mt-wall)" />
					<rect x="0" y="-14" width="330" height="14" fill="var(--mt-wall-2)" />
					<rect x="0" y="-68" width="330" height="8" fill="var(--mt-wood)" />
					<path d="M0-74H330" stroke="var(--mt-wall)" stroke-width="8" stroke-dasharray="9 7" />
					<rect x="-8" y="-122" width="40" height="122" fill="var(--mt-wall)" />
					<rect x="22" y="-122" width="10" height="122" fill="var(--mt-wall-2)" />
					<path d="M-14-122L12-180L38-122Z" fill="var(--mt-roof)" />
					<rect x="296" y="-106" width="40" height="106" fill="var(--mt-wall)" />
					<rect x="326" y="-106" width="10" height="106" fill="var(--mt-wall-2)" />
					<path d="M290-106L316-160L342-106Z" fill="var(--mt-roof)" />
					{#each CASTLE_WINDOWS as w, i (i)}
						<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="2" fill={litSet.has(i) ? 'var(--window-lit)' : 'var(--mt-dark)'} opacity={litSet.has(i) ? 1 : 0.55} />
					{/each}
				</g>
				<g opacity="0.22" transform="translate(0 {CASTLE.y * 2}) scale(1 -1)"><use href="#mt-castle" /></g>
				{#if lit}
					{#each SHORE_LIGHTS as l, i (i)}
						<circle cx={l.x} cy={l.y} r="3" fill="var(--window-lit)" />
						<path d="M{l.x} {l.y + 5}v26" stroke="var(--window-lit)" stroke-width="3" opacity="0.25" />
					{/each}
				{/if}
			</svg>

			{#if lit}
				<div class="glow flood" style:left="{CASTLE.x - 60}px" style:top="{CASTLE.y - 200}px"></div>
			{/if}

			<!-- The gesture: Chillon sinks into silhouette (down) or is floodlit gold (up). -->
			{#if active === 'down' || active === 'up'}
				{#key playKey}
					<svg class="layer" viewBox={box}>
						<use href="#mt-castle" class={active === 'down' ? 'mt-dim' : 'mt-flood'} />
					</svg>
				{/key}
			{/if}

			<!-- Ripples on the lake -->
			<svg class="layer ripples" viewBox={box}>
				<g class="rip rip-a" stroke="var(--cloud)" stroke-width="3" stroke-linecap="round" opacity="0.5">
					<path d="M120 760h90M520 800h120M900 740h80M1260 820h110M1640 770h100M300 880h130M760 910h90M1420 900h120M1780 850h90" />
				</g>
				<g class="rip rip-b" stroke="var(--cloud)" stroke-width="3" stroke-linecap="round" opacity="0.35">
					<path d="M60 840h70M420 760h60M700 860h110M1100 790h90M1500 830h80M1000 930h120M200 930h90M1700 920h110" />
				</g>
			</svg>

			<!-- The paddle steamer, crossing at its own pace. -->
			<div class="steamer" style:top="{SHORE_Y + 82}px">
				<svg viewBox="-60 -125 260 150" width="468" height="270">
					<path d="M-10 -16H186L172 2H6Z" fill="var(--mt-hull)" />
					<path d="M-8 -10H184L180 -6H-4Z" fill="var(--mt-red)" />
					<path d="M70 -16a22 22 0 0 1 44 0Z" fill="var(--mt-hull-2)" />
					<rect x="16" y="-34" width="150" height="18" fill="var(--mt-hull)" />
					<rect x="30" y="-48" width="110" height="14" fill="var(--mt-hull)" />
					<rect x="26" y="-51" width="118" height="4" fill="var(--mt-hull-2)" />
					<path d="M22 -26H160M38 -41H132" stroke={lit ? 'var(--window-lit)' : 'var(--mt-dark)'} stroke-width="6" stroke-dasharray="6 5" opacity={lit ? 1 : 0.8} />
					<rect x="46" y="-76" width="16" height="26" fill="var(--mt-red)" />
					<rect x="116" y="-76" width="16" height="26" fill="var(--mt-red)" />
					<rect x="46" y="-80" width="16" height="6" fill="#1d2230" />
					<rect x="116" y="-80" width="16" height="6" fill="#1d2230" />
					<path d="M176 -16V-72" stroke="var(--mt-dark)" stroke-width="2" />
					<rect x="177" y="-74" width="14" height="14" fill="#d52b1e" />
					<path d="M181 -67h6M184 -70v6" stroke="#fff" stroke-width="2.4" />
					<path d="M-30 4h30M-50 10h44M-26 16h30" stroke="var(--cloud)" stroke-width="2.4" stroke-linecap="round" opacity="0.7" />
				</svg>
				<span class="puff" style="left:100px"></span>
				<span class="puff p2" style="left:100px"></span>
				<span class="puff" style="left:226px"></span>
				<span class="puff p2" style="left:226px"></span>
				{#if active === 'resolved'}
					{#key playKey}
						{#each [0, 1, 2] as k (k)}
							<span class="toot" style="left:170px" style:animation-delay="{k * 1.5}s"></span>
							<span class="toot" style="left:296px" style:animation-delay="{k * 1.5 + 0.2}s"></span>
						{/each}
					{/key}
				{/if}
			</div>

			{#if flyer}
				<div class="mover" bind:this={flyerEl}>
					<WallPigeon flap lookX={0.8} class="flyer" blinkOffset={2} />
				</div>
			{/if}

			<!-- The quay: stone, seams, the lamp post. -->
			<svg class="layer" viewBox={box}>
				<rect x="0" y={LEDGE_Y} width={STAGE_W} height={STAGE_H - LEDGE_Y} fill="var(--mt-quay)" />
				<rect x="0" y={LEDGE_Y} width={STAGE_W} height="12" fill="var(--mt-quay-2)" />
				<path d="M0 1020H1920M0 1060H1920" stroke="var(--mt-quay-2)" stroke-width="3" />
				<path d={Array.from({ length: 14 }, (_, i) => `M${i * 150 + 40} 974V1020M${i * 150 + 115} 1020V1060`).join('')} stroke="var(--mt-quay-2)" stroke-width="3" />
				<path d="M{LAMP.x} {LAMP.y}V{LEDGE_Y}" stroke="var(--lamp)" stroke-width="9" />
				<rect x={LAMP.x - 30} y={LAMP.y + 4} width="60" height="8" rx="3" fill="var(--lamp)" />
				<circle cx={LAMP.x} cy={LAMP.y - 18} r="16" fill={lit ? 'var(--window-lit)' : 'var(--mt-wall)'} stroke="var(--lamp)" stroke-width="4" />
			</svg>
			{#if lit}
				<div class="glow lamp-glow" style:left="{LAMP.x - 110}px" style:top="{LAMP.y - 128}px"></div>
			{/if}

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
						{eyes}
						blinkOffset={0.7}
					/>
				</div>
			</div>

			<!-- Mimi pecks at the cobbles, and opens her umbrella when it rains. -->
			<div class="cast" style:transform="translateX(1370px)" style:top="{LEDGE_Y + 6 - 175}px">
				<div class="pecker" class:pecker--busy={!trouble}>
					<WallPigeon class="mimi" lookX={trouble ? -0.8 : 0.3} {eyes} umbrella={raining} blinkOffset={3.1} />
				</div>
			</div>

			<!-- Jules, on the lamp: asleep, unless something is wrong. -->
			<div class="cast" style:transform="translateX({LAMP.x - 55}px)" style:top="{LAMP.y + 4 - 137}px">
				<div class="napper" class:napper--asleep={!julesAwake && !startled}>
					<WallPigeon class="jules" eyes={julesAwake || startled ? eyes : 'closed'} lookX={julesLook} lookY={julesAwake ? 0.2 : 0} blinkOffset={5.4} />
				</div>
			</div>

			{#if raining}
				<div class="rain">
					<svg viewBox="0 0 1920 1620">
						<path d={RAIN} stroke="var(--rain)" stroke-width="2.4" stroke-linecap="round" />
					</svg>
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
	.glow {
		position: absolute;
		border-radius: 50%;
		background: radial-gradient(circle, var(--glow) 0%, transparent 70%);
		animation: breathe 9s ease-in-out infinite;
	}
	.lamp-glow {
		width: 220px;
		height: 220px;
	}
	.flood {
		width: 520px;
		height: 260px;
		animation-duration: 11s;
	}
	@keyframes breathe {
		50% {
			opacity: 0.55;
		}
	}

	.rip {
		animation: ripple 14s ease-in-out infinite alternate;
	}
	.rip-b {
		animation-duration: 19s;
		animation-direction: alternate-reverse;
	}
	@keyframes ripple {
		to {
			transform: translateX(40px);
		}
	}

	.steamer {
		position: absolute;
		left: 0;
		width: 468px;
		animation: cruise 170s linear infinite;
		will-change: transform;
	}
	.steamer svg {
		display: block;
	}
	@keyframes cruise {
		from {
			transform: translateX(-560px);
		}
		to {
			transform: translateX(2000px);
		}
	}
	.puff {
		position: absolute;
		top: 40px;
		width: 44px;
		height: 44px;
		border-radius: 50%;
		background: radial-gradient(circle, var(--smoke), transparent 70%);
		opacity: 0;
		animation: puff 9s ease-out infinite;
	}
	.p2 {
		animation-delay: -4.5s;
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
			transform: translate(-110px, -90px) scale(2.2);
			opacity: 0;
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
		79% {
			transform: rotate(18deg) translate(8px, 12px);
		}
		73%,
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

	/* The gesture: Chillon goes dark or is floodlit; the steamer toots (resolved). */
	.mt-dim {
		filter: brightness(0.1);
		opacity: 0;
		animation: mt-dim 5.2s ease-in-out both;
	}
	.mt-flood {
		filter: brightness(1.7) sepia(1) saturate(3.5);
		opacity: 0;
		animation: mt-flood 5.2s ease-in-out both;
	}
	.toot {
		position: absolute;
		top: 30px;
		width: 70px;
		height: 70px;
		border-radius: 50%;
		background: radial-gradient(circle, var(--smoke), transparent 70%);
		opacity: 0;
		animation: toot 2.2s ease-out both;
	}
	@keyframes mt-dim {
		12%,
		80% {
			opacity: 0.8;
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes mt-flood {
		15% {
			opacity: 0.9;
		}
		30% {
			opacity: 0.5;
		}
		45%,
		75% {
			opacity: 0.85;
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes toot {
		0% {
			transform: translate(0, 0) scale(0.4);
			opacity: 0;
		}
		15% {
			opacity: 0.95;
		}
		100% {
			transform: translate(-50px, -150px) scale(2.8);
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
	.scene--still .steamer {
		transform: translateX(520px);
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
		.steamer {
			transform: translateX(520px);
		}
	}
</style>
