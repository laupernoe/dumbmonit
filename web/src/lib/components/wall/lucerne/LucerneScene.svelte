<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop, Swiss edition: Lucerne. The Kapellbrücke, the
	 * old covered wooden bridge with its flower boxes and the stone Wasserturm,
	 * crosses the Reuss; Pilatus stands behind the old town; two swans drift by.
	 * A drop-in for the Paris and Rome scenes (same props).
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky follows
	 * the real hour inside the brightness band of the wall's theme (see
	 * `paris/daylight.ts`): sun or moon on its arc, stars, lit windows and
	 * bridge lamps after dark, their glow on the river.
	 *
	 * The network's state is woven in: clouds gather with advisories, a storm
	 * brings drizzle and a darker sky, and the pigeon in the beret holds up a
	 * sign with the number of problems. A new problem startles him once.
	 *
	 * Motion is gentle: drifting clouds, shimmering water, swans, a stroll, and
	 * every 40-80 s a moment (a fly-by, a hat tip). Hidden tabs pause
	 * everything; reduced motion freezes the scene in a still pose.
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
		WATER_Y,
		DECK_Y,
		PILATUS,
		TOWN_BACK,
		TOWN_LEFT,
		TOWN_RIGHT,
		SPIRES,
		BRIDGE_X0,
		BRIDGE_X1,
		TOWER,
		PILES,
		POSTS,
		PANELS,
		BOXES,
		QUAY_BOXES,
		GLINTS,
		STARS,
		RAIN,
		litWindows,
		lucerneStyle
	} from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood?: SceneMood | 'ok' | 'warn' | 'critical';
		/** Problems on the board: the number on the protest sign. */
		problems: number;
		/** Bumped by the wall when a new problem appears: the pigeon startles once. */
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

	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const style = $derived(`${sceneStyle(theme, phase, mood)};${lucerneStyle(theme)}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));

	let windowSeed = $state(1);
	const ratio = $derived(phase === 'night' ? 0.18 : 0.3);
	const townLit = $derived(lit ? litWindows(windowSeed, TOWN_BACK.windows.length, ratio) : new Set<number>());
	const leftLit = $derived(lit ? litWindows(windowSeed + 3, TOWN_LEFT.windows.length, ratio) : new Set<number>());
	const rightLit = $derived(lit ? litWindows(windowSeed + 5, TOWN_RIGHT.windows.length, ratio) : new Set<number>());

	const trouble = $derived(problems > 0);
	const raining = $derived(mood === 'storm');
	const cloudCount = $derived(mood === 'storm' ? 6 : mood === 'clouded' ? 4 : 2);
	const CLOUDS = [
		{ y: 110, s: 1.1, dur: 260, delay: -40 },
		{ y: 260, s: 0.8, dur: 320, delay: -210 },
		{ y: 180, s: 0.95, dur: 290, delay: -150 },
		{ y: 380, s: 1.2, dur: 340, delay: -290 },
		{ y: 50, s: 0.7, dur: 300, delay: -250 },
		{ y: 330, s: 0.6, dur: 360, delay: -120 }
	];
	const FLOWERS = ['a', 'b', 'c'] as const;
	const TOWER_WINDOWS = [-30, 0, 30].map((dx) => TOWER.x + TOWER.w / 2 + dx - 5);

	// --- The pigeon in the beret -----------------------------------------------------

	const STROLL = [900, 1250];
	const SIGN_SPOT = 920;
	let gastonX = $state(1080);
	let gastonWalkMs = $state(0);
	let gastonWalking = $state(false);
	let gastonLook = $state(0);
	let gastonTip = $state(false);
	let startled = $state(false);
	let flyer = $state(false);
	let flyerEl = $state<HTMLDivElement | null>(null);
	let feathers = $state<{ id: number; x: number; drift: number; delay: number }[]>([]);

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

	function stopAll() {
		epoch++;
		for (const animation of running.splice(0)) animation.cancel();
		flyer = false;
		gastonTip = false;
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

	async function flyBy() {
		const mine = epoch;
		flyer = true;
		await tick();
		const y = between(200, 360);
		const animation = flyerEl?.animate(
			[
				{ transform: `translate(-260px, ${y + 40}px) rotate(4deg)` },
				{ transform: `translate(900px, ${y - 20}px) rotate(-2deg)`, offset: 0.5 },
				{ transform: `translate(2160px, ${y - 50}px) rotate(-4deg)` }
			],
			{ duration: 18000, easing: 'linear', fill: 'forwards' }
		);
		if (animation) {
			running.push(animation);
			await animation.finished.catch(() => undefined);
		}
		if (mine === epoch) flyer = false;
	}

	function scheduleMoments(first = false) {
		later(
			() => {
				if (!trouble && Math.random() < 0.5) {
					// Stop, face the room, lift the beret, and put it back.
					gastonLook = 0;
					gastonTip = true;
					later(() => (gastonTip = false), 2400);
				} else {
					void flyBy();
				}
				scheduleMoments();
			},
			first ? between(9000, 16000) : between(40000, 80000)
		);
	}

	function startAll() {
		if (!motion) {
			gastonX = trouble ? SIGN_SPOT : 1080;
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

	// Lit windows change now and then while the town is lit.
	$effect(() => {
		if (!lit || paused || !motion) return;
		const timer = setInterval(() => (windowSeed += 1), 120_000);
		return () => clearInterval(timer);
	});

	// A new problem: a start, and a few feathers drift down.
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
				<div class="glow beacon" style:left="{TOWER.x + TOWER.w / 2 - 110}px" style:top="{TOWER.top - 40}px"></div>
				<div class="glow river" style:left="{TOWER.x - 190}px" style:top="{WATER_Y + 30}px"></div>
			{/if}

			<!-- Mountains, old town, the bridge and its reflection. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<defs>
					<linearGradient id="lz-water" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--lz-water-1)" />
						<stop offset="1" stop-color="var(--lz-water-2)" />
					</linearGradient>
					<g id="lz-bridge">
						<path d={PILES} fill="var(--lz-wood-2)" />
						<rect x={BRIDGE_X0} y={DECK_Y} width={BRIDGE_X1 - BRIDGE_X0} height="10" fill="var(--lz-wood)" />
						<path d={POSTS} fill="var(--lz-wood)" />
						<path d={PANELS} fill="var(--lz-wood-2)" />
						<!-- the long, low roof and its ridge -->
						<path
							d="M{BRIDGE_X0 - 14} 748 L{BRIDGE_X0 + 20} 722 H{BRIDGE_X1 - 20} L{BRIDGE_X1 + 14} 748 Z"
							fill="var(--lz-bridge-roof)"
						/>
						<rect x={BRIDGE_X0 + 20} y="718" width={BRIDGE_X1 - BRIDGE_X0 - 40} height="6" fill="var(--lz-wood-2)" />
						<!-- rail with its flower boxes -->
						<rect x={BRIDGE_X0} y="800" width={BRIDGE_X1 - BRIDGE_X0} height="5" fill="var(--lz-wood-2)" />
						{#each BOXES as box (box.x)}
							<rect x={box.x} y="800" width="56" height="18" rx="2" fill="var(--lz-wood-2)" />
							<rect x={box.x + 2} y="788" width="52" height="14" rx="6" fill="var(--lz-leaf)" />
							{#each box.blooms as b, j (j)}
								<circle cx={box.x + b.dx} cy={792 + b.dy} r={b.r} fill="var(--lz-flower-{j % 2 ? 'b' : box.tone})" />
							{/each}
						{/each}
						<!-- the Wasserturm: octagonal, stone, a conical roof -->
						<rect x={TOWER.x} y={TOWER.top} width={TOWER.w} height={WATER_Y + 40 - TOWER.top} fill="var(--lz-stone)" />
						<rect x={TOWER.x + TOWER.w * 0.62} y={TOWER.top} width={TOWER.w * 0.38} height={WATER_Y + 40 - TOWER.top} fill="var(--lz-stone-2)" opacity="0.55" />
						<path
							d="M{TOWER.x + 10} {TOWER.top} h{TOWER.w - 20} M{TOWER.x + 10} {TOWER.top + 90} h{TOWER.w - 20} M{TOWER.x + 10} {TOWER.top + 180} h{TOWER.w - 20}"
							stroke="var(--lz-stone-2)"
							stroke-width="3"
						/>
						<rect x={TOWER.x - 8} y={TOWER.top - 8} width={TOWER.w + 16} height="12" fill="var(--lz-stone-2)" />
						<path
							d="M{TOWER.x - 6} {TOWER.top - 8} L{TOWER.x + TOWER.w / 2} {TOWER.roofTop} L{TOWER.x + TOWER.w + 6} {TOWER.top - 8} Z"
							fill="var(--lz-tower-roof)"
						/>
						<rect x={TOWER.x + TOWER.w / 2 - 2} y={TOWER.roofTop - 26} width="4" height="28" fill="var(--lz-wood-2)" />
					</g>
				</defs>

				<path d={PILATUS.alps} fill="var(--lz-alps)" />
				<path d={PILATUS.ridge} fill="var(--lz-rock)" />
				<path d={PILATUS.snow} fill="var(--lz-snow)" />
				<path d={SPIRES} fill="var(--lz-town-2)" />

				<path d={TOWN_BACK.bodies} fill="var(--lz-town-2)" />
				<path d={TOWN_BACK.roofs} fill="var(--lz-roof)" />
				<path d={TOWN_LEFT.bodies} fill="var(--lz-town)" />
				<path d={TOWN_LEFT.roofs} fill="var(--lz-roof)" />
				<path d={TOWN_RIGHT.bodies} fill="var(--lz-town)" />
				<path d={TOWN_RIGHT.roofs} fill="var(--lz-roof)" />
				{#each [{ row: TOWN_BACK, set: townLit }, { row: TOWN_LEFT, set: leftLit }, { row: TOWN_RIGHT, set: rightLit }] as { row, set } (row)}
					{#each row.windows as w, i (i)}
						<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="4" fill="var(--window)" />
						{#if set.has(i)}
							<rect class="lit" x={w.x} y={w.y} width={w.w} height={w.h} rx="4" fill="var(--window-lit)" />
						{/if}
					{/each}
				{/each}

				<!-- the Reuss, with the bridge upside down in it -->
				<rect x="0" y={WATER_Y} width={STAGE_W} height={STAGE_H - WATER_Y} fill="url(#lz-water)" />
				<use href="#lz-bridge" transform="translate(0 {WATER_Y * 2}) scale(1 -1)" opacity="0.22" />
				<rect x="0" y={WATER_Y} width={STAGE_W} height="3" fill="var(--lz-water-2)" opacity="0.5" />
				<g class="glints">
					<path d={GLINTS} stroke="var(--lz-glint)" stroke-width="3" stroke-linecap="round" />
					<path d={GLINTS} stroke="var(--lz-glint)" stroke-width="3" stroke-linecap="round" transform="translate(-1920 0)" />
				</g>

				<use href="#lz-bridge" />
				<!-- the tower's slit windows and the bridge lamps, lit after dark -->
				{#each TOWER_WINDOWS as x, i (i)}
					<rect x={x} y={TOWER.top + 40} width="10" height="30" rx="5" fill="var(--window)" />
					<rect x={x} y={TOWER.top + 130} width="10" height="30" rx="5" fill="var(--window)" />
					{#if lit}
						<rect class="lit" x={x} y={TOWER.top + 40} width="10" height="30" rx="5" fill="var(--window-lit)" />
						<rect class="lit" x={x} y={TOWER.top + 130} width="10" height="30" rx="5" fill="var(--window-lit)" />
					{/if}
				{/each}
				{#if lit}
					{#each [300, 540, 780, 1020, 1500] as x (x)}
						<circle class="lit" cx={x} cy="754" r="7" fill="var(--window-lit)" />
					{/each}
				{/if}
			</svg>

			<!-- The gesture: the Kapellbrücke goes dark (down), its lamps come on one by one (up), petals fall from the flower boxes (resolved). -->
			{#if active}
				{#key playKey}
					<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
						{#if active === 'down'}
							<g class="lz-dark" fill="#0a1020">
								<rect x={BRIDGE_X0 - 14} y="716" width={BRIDGE_X1 - BRIDGE_X0 + 28} height="150" />
								<rect x={TOWER.x - 8} y={TOWER.top - 8} width={TOWER.w + 16} height={WATER_Y + 48 - TOWER.top} />
								<path d="M{TOWER.x - 6} {TOWER.top - 8} L{TOWER.x + TOWER.w / 2} {TOWER.roofTop} L{TOWER.x + TOWER.w + 6} {TOWER.top - 8} Z" />
							</g>
						{:else if active === 'up'}
							{#each Array.from({ length: 16 }, (_, i) => 190 + i * 96) as x, i (x)}
								<g class="lz-lamp" style:animation-delay="{i * 0.12}s">
									<circle cx={x} cy="766" r="30" fill="#ffd98a" opacity="0.28" />
									<circle cx={x} cy="766" r="9" fill="#ffe7b0" />
								</g>
							{/each}
						{:else}
							{#each BOXES as box, i (box.x)}
								{#each [0, 1] as k (k)}
									<ellipse
										class="lz-petal"
										cx={box.x + 14 + k * 26}
										cy="792"
										rx="6"
										ry="3.5"
										fill="var(--lz-flower-{box.tone})"
										style:--dx="{k ? 38 : -34}px"
										style:--dy="{90 + ((i * 37 + k * 53) % 70)}px"
										style:animation-delay="{(i % 6) * 0.35 + k * 0.5}s"
									/>
								{/each}
							{/each}
						{/if}
					</svg>
				{/key}
			{/if}

			<!-- Two swans on the river. -->
			<div class="swans">
				<svg viewBox="0 0 200 80">
					{#each [0, 96] as dx, i (i)}
						<g transform="translate({dx} {i * 10}) scale({i ? 0.78 : 1})">
							<path d="M10 56 Q40 70 74 56 Q78 40 64 38 L16 40 Z" fill="var(--lz-swan)" />
							<path d="M64 40 Q80 38 76 20 Q74 8 84 8" stroke="var(--lz-swan)" stroke-width="7" stroke-linecap="round" fill="none" />
							<path d="M86 8 l10 3 -10 3 Z" fill="var(--lz-beak)" />
						</g>
					{/each}
				</svg>
			</div>

			{#if flyer}
				<div class="mover" bind:this={flyerEl}>
					<WallPigeon flap lookX={0.8} class="flyer" blinkOffset={2} />
				</div>
			{/if}

			<!-- The quay: stone ledge and its flower boxes. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<rect x="0" y={LEDGE_Y + 6} width={STAGE_W} height={STAGE_H - LEDGE_Y} fill="var(--lz-stone)" />
				<rect x="0" y={LEDGE_Y + 6} width={STAGE_W} height="12" fill="var(--lz-stone-2)" />
				<path d="M0 1030 H1920 M0 1002 H1920" stroke="var(--lz-stone-2)" stroke-width="3" opacity="0.6" />
				{#each QUAY_BOXES as x, i (x)}
					<rect x={x - 40} y={LEDGE_Y - 14} width="80" height="22" rx="3" fill="var(--lz-wood-2)" />
					<ellipse cx={x} cy={LEDGE_Y - 20} rx="38" ry="14" fill="var(--lz-leaf)" />
					{#each [-26, -14, -2, 10, 22] as dx, j (j)}
						<circle cx={x + dx} cy={LEDGE_Y - 28 + (j % 2) * 6} r="6" fill="var(--lz-flower-{FLOWERS[(i + j) % 3]})" />
					{/each}
				{/each}
			</svg>

			<!-- Gaston, in the beret: strolls, tips his hat, holds the sign. -->
			<div
				class="cast stroller"
				style:transform="translateX({gastonX - 75}px)"
				style:transition-duration="{gastonWalkMs}ms"
				style:top="{LEDGE_Y + 6 - 187}px"
			>
				<div class="waddle" class:waddle--on={gastonWalking}>
					<WallPigeon
						class="gaston"
						hat
						tip={gastonTip}
						sign={gastonSign}
						lookX={gastonLook}
						lookY={trouble ? -0.3 : 0}
						eyes={startled ? 'startled' : 'open'}
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
		width: 220px;
		height: 220px;
		border-radius: 50%;
		background: radial-gradient(circle, var(--glow) 0%, transparent 70%);
		animation: breathe 8s ease-in-out infinite;
	}
	.river {
		width: 520px;
		height: 160px;
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

	/* Water glints slide with the current and wrap on one tile width. */
	.glints {
		animation: flow 70s linear infinite;
	}
	@keyframes flow {
		to {
			transform: translateX(1920px);
		}
	}

	.swans {
		position: absolute;
		left: 0;
		top: 890px;
		width: 200px;
		animation: swim 140s linear infinite alternate;
	}
	.swans svg {
		display: block;
		width: 100%;
		animation: bob 5s ease-in-out infinite alternate;
	}
	@keyframes swim {
		from {
			transform: translateX(260px);
		}
		to {
			transform: translateX(1500px);
		}
	}
	@keyframes bob {
		to {
			transform: translateY(4px);
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

	/* The gesture. */
	.lz-dark {
		opacity: 0;
		animation: lz-dark 5.2s ease-in-out both;
	}
	.lz-lamp {
		transform-box: fill-box;
		transform-origin: 50% 50%;
		opacity: 0;
		animation: lz-lamp 4.2s ease-out both;
	}
	.lz-petal {
		opacity: 0;
		animation: lz-petal 4.4s ease-in-out both;
	}
	@keyframes lz-dark {
		15%,
		78% {
			opacity: 0.55;
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes lz-lamp {
		0% {
			opacity: 0;
			transform: scale(0.2);
		}
		12% {
			opacity: 1;
			transform: scale(1.5);
		}
		25%,
		80% {
			opacity: 1;
			transform: scale(1);
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes lz-petal {
		0% {
			opacity: 0;
			transform: translate(0, 0) rotate(0);
		}
		12% {
			opacity: 0.95;
		}
		60% {
			transform: translate(calc(var(--dx) * 0.2), calc(var(--dy) * 0.6)) rotate(140deg);
		}
		100% {
			opacity: 0;
			transform: translate(var(--dx), var(--dy)) rotate(300deg);
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
	.scene--still .swans {
		transform: translateX(800px);
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
