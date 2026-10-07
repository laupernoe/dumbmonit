<script lang="ts">
	/**
	 * The wall's living backdrop: a New York rooftop under the Manhattan
	 * skyline, and the DumbMonit pigeons going about their day. A drop-in
	 * sibling of the Paris scene, with the same props.
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). Across the
	 * river: the Empire State Building and the Chrysler Building, the Brooklyn
	 * Bridge, the Statue of Liberty out in the harbour. The sky follows the
	 * real hour — dawn, day, golden hour, dusk, night, with the sun or the moon
	 * on its arc, lit windows, a string of lights on the bridge and the crown
	 * of the Empire State washed in colour for the first five minutes of every
	 * hour after dark — inside the brightness band of the wall's theme (see
	 * `../paris/daylight.ts`).
	 *
	 * The network's state is woven in: clouds gather with advisories, a storm
	 * brings a soft drizzle and a slightly darker sky, the pigeon on the
	 * parapet stops strolling and holds up a sign with the number of problems
	 * while looking at the list, the pizza pigeon opens an umbrella beside the
	 * water tower, the napper on the floodlight wakes up. A new problem startles
	 * everyone once.
	 *
	 * Motion is constant and gentle: drifting clouds, steam from the stack, a
	 * stroll, pecks, blinks, a snore. Every 40–80 s one small moment plays (a
	 * fly-by, a red balloon, the napper waking for a look around, the Empire
	 * State changing colours at night, and, rarely, a yellow cab crossing the
	 * bridge). All of it is transform and opacity on small layers; nothing
	 * flashes. Hidden tabs pause everything; reduced motion freezes the scene
	 * in a still pose.
	 */
	import { tick, untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import WallPigeon from '#lib/components/wall/paris/WallPigeon.svelte';
	import {
		phaseAt,
		isLit,
		celestialAt,
		isSparkleTime,
		sceneStyle,
		type SceneMood,
		type SceneTheme
	} from '#lib/components/wall/paris/daylight.js';
	import {
		STAGE_W,
		STAGE_H,
		LEDGE_Y,
		SKY_BACK,
		SKY_FRONT,
		ESB,
		ESB_LINES,
		ESB_CROWN,
		ESB_TOP,
		CHRYSLER,
		CHRYSLER_LINES,
		CHRYSLER_TOP,
		BRIDGE_TOWERS,
		BRIDGE_DECK,
		BRIDGE_CABLES,
		BRIDGE_STAYS,
		DECK_Y,
		STATUE,
		WALKUPS,
		TAR_SEAMS,
		STARS,
		RAIN,
		litWindows
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

	/** What this city adds to the shared palette: the cab, the pizza, the stack's stripes. */
	const EXTRA: Record<SceneTheme, string> = {
		light: '--cab:#f6c62b;--cheese:#f2b84b;--crust:#d9a35b;--pepperoni:#c0563f;--stripe:#e8793a',
		dark: '--cab:#d9a521;--cheese:#c98f35;--crust:#a5763a;--pepperoni:#8f3f31;--stripe:#a8532b',
		oled: '--cab:#5c4a14;--cheese:#5e4524;--crust:#47331a;--pepperoni:#3c1b14;--stripe:#3a1f12'
	};

	// Re-read once a minute: nothing in the sky moves faster than that.
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const style = $derived(`${sceneStyle(theme, phase, mood)};${EXTRA[theme]}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));

	// Windows light up in a different pattern every couple of minutes.
	let windowSeed = $state(1);
	const litSet = $derived(lit ? litWindows(windowSeed, WALKUPS.windows.length, phase === 'night' ? 0.18 : 0.3) : new Set<number>());

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

	/** Gaston strolls along the parapet between these points. */
	const STROLL = [860, 1170];
	const SIGN_SPOT = 880;
	let gastonX = $state(1020);
	let gastonWalkMs = $state(0);
	let gastonWalking = $state(false);
	let gastonLook = $state(0);
	let gastonPeck = $state(false);

	/** Mimi pecks at her slice by the water tower. */
	const mimiLookX = $derived(trouble ? -0.8 : 0.35);

	/** Jules naps on the floodlight. */
	let julesAwake = $state(false);
	let julesLook = $state(0);

	let startled = $state(false);
	let lightsMoment = $state(false);
	let lightsColor = $state('#fff0c0');
	const washing = $derived(lit && (lightsMoment || isSparkleTime(minute)));

	/** One-off props for the moments. */
	let flyer = $state<null | { y: number }>(null);
	let balloonOn = $state(false);
	let cabOn = $state(false);
	let feathers = $state<{ id: number; x: number; drift: number; delay: number }[]>([]);
	let flyerEl = $state<HTMLDivElement | null>(null);
	let balloonEl = $state<HTMLDivElement | null>(null);
	let cabEl = $state<HTMLDivElement | null>(null);

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
		balloonOn = false;
		cabOn = false;
		gastonPeck = false;
		lightsMoment = false;
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

	type Moment = 'flyby' | 'cab' | 'balloon' | 'wake' | 'lights';
	const LIGHTS = ['#ff5a4d', '#5b8cff', '#ffffff', '#3fcf8e', '#b777ff', '#ffb347'];
	let deck: Moment[] = [];
	function nextMoment(): Moment | null {
		const allowed: Moment[] = ['flyby', 'balloon', 'wake', 'cab'];
		if (lit) allowed.push('lights');
		deck = deck.filter((m) => allowed.includes(m));
		if (deck.length === 0) deck = [...allowed].sort(() => Math.random() - 0.5);
		return deck.shift() ?? null;
	}

	async function play(moment: Moment) {
		const mine = epoch;
		if (moment === 'flyby') {
			const y = between(230, 380);
			flyer = { y };
			await tick();
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
			if (mine === epoch) flyer = null;
		} else if (moment === 'cab') {
			// A yellow cab crosses the Brooklyn Bridge, a long way off.
			cabOn = true;
			await tick();
			await animate(
				cabEl,
				[
					{ transform: `translate(-60px, ${DECK_Y - 17}px)` },
					{ transform: `translate(770px, ${DECK_Y - 17}px)` }
				],
				{ duration: 34000, easing: 'linear', fill: 'forwards' }
			);
			if (mine === epoch) cabOn = false;
		} else if (moment === 'balloon') {
			balloonOn = true;
			await tick();
			await animate(
				balloonEl,
				[
					{ transform: 'translate(1560px, 980px)' },
					{ transform: 'translate(1680px, 520px)', offset: 0.45 },
					{ transform: 'translate(1880px, -260px)' }
				],
				{ duration: 52000, easing: 'cubic-bezier(0.3, 0, 0.6, 1)', fill: 'forwards' }
			);
			if (mine === epoch) balloonOn = false;
		} else if (moment === 'wake') {
			julesAwake = true;
			julesLook = -0.8;
			later(() => (julesLook = 0.8), 2600);
			later(() => (julesLook = 0), 5000);
			later(() => {
				if (!trouble) julesAwake = false;
			}, 6800);
		} else if (moment === 'lights') {
			// The Empire State changes its colours: three slow breaths, then back to gold.
			lightsColor = LIGHTS[Math.floor(Math.random() * LIGHTS.length)];
			lightsMoment = true;
			later(() => (lightsMoment = false), 24000);
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
		if (!motion) {
			gastonX = trouble ? SIGN_SPOT : 1020;
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

	const gastonSign = $derived(trouble ? `${problems} ${problems === 1 ? 'PROBLEM' : 'PROBLEMS'}` : null);
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
				<div class="glow beacon" style:left="{ESB_TOP.x - 60}px" style:top="{ESB_TOP.y - 60}px"></div>
			{/if}

			<!-- The city: far blocks, the two great towers, the bridge, the harbour, the walk-ups. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<path d={SKY_BACK.d} fill="var(--far-2)" />

				<path d={CHRYSLER} fill="var(--tower)" />
				<path d={CHRYSLER_LINES} fill="none" stroke="var(--tower-line)" stroke-width="1.6" stroke-linecap="round" />
				<path d={ESB} fill="var(--tower)" />
				<path d={ESB_LINES} fill="none" stroke="var(--tower-line)" stroke-width="1.4" />
				{#if washing}
					<path class="wash" d={ESB_CROWN} fill={lightsMoment ? lightsColor : '#fff0c0'} />
				{/if}
				{#if lit}
					<circle class="blink" cx={ESB_TOP.x} cy={ESB_TOP.y} r="3.4" fill="#ff5a4d" />
					<circle class="blink blink--late" cx={CHRYSLER_TOP.x} cy={CHRYSLER_TOP.y} r="2.6" fill="#ff5a4d" />
				{/if}

				<path d={SKY_FRONT.d} fill="var(--far)" />
				{#if lit}
					<path class="lit" d={SKY_FRONT.windows} fill="var(--window-lit)" opacity="0.75" />
				{/if}

				<!-- the Statue of Liberty, out in the harbour -->
				<g transform="translate(1340 872) scale(1.1)">
					<path d={STATUE.island} fill="var(--far-2)" />
					<path d={STATUE.pedestal} fill="var(--far-2)" />
					<path d={STATUE.body} fill="var(--far-2)" />
					<circle cx={STATUE.head.cx} cy={STATUE.head.cy} r={STATUE.head.r} fill="var(--far-2)" />
					<path d={STATUE.crown} stroke="var(--far-2)" stroke-width="2.2" stroke-linecap="round" />
					{#if lit}
						<circle class="lit" cx={STATUE.torch.x} cy={STATUE.torch.y} r="2.6" fill="var(--window-lit)" />
					{/if}
				</g>

				<!-- the Brooklyn Bridge -->
				<path d={BRIDGE_TOWERS} fill="var(--tower)" fill-rule="evenodd" />
				<path d={BRIDGE_DECK} fill="var(--tower)" />
				<path d={BRIDGE_STAYS} fill="none" stroke="var(--tower-line)" stroke-width="1" opacity="0.75" />
				<path d={BRIDGE_CABLES} fill="none" stroke="var(--tower-line)" stroke-width="2.4" stroke-linecap="round" />

				<path d={WALKUPS.roof} fill="var(--roof)" />
				<path d={WALKUPS.brick} fill="var(--brick)" />
				<path d={WALKUPS.stone} fill="var(--facade)" />
				<path d={WALKUPS.trim} fill="var(--near-top)" opacity="0.5" />
				<path d={WALKUPS.escape} fill="none" stroke="var(--lamp)" stroke-width="2.2" stroke-linecap="round" opacity="0.8" />
				<g fill="var(--window)">
					{#each WALKUPS.windows as w, i (i)}
						<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="2" />
					{/each}
				</g>
				<g fill="var(--window-lit)">
					{#each WALKUPS.windows as w, i (i)}
						{#if litSet.has(i)}
							<rect class="lit" x={w.x} y={w.y} width={w.w} height={w.h} rx="2" />
						{/if}
					{/each}
				</g>
			</svg>

			{#if cabOn}
				<div class="mover" bind:this={cabEl}>
					<svg viewBox="0 0 68 28" class="cab">
						<path d="M2 21 L2 15 Q2 12 5 12 L16 12 L22 5 L46 5 L52 12 L63 12 Q66 12 66 15 L66 21 Z" fill="var(--cab)" />
						<path d="M24 7 L44 7 L49 12 L19 12 Z" fill="var(--window)" />
						<rect x="29" y="1" width="10" height="4" rx="1" fill="var(--cab)" />
						<path d="M2 17 H66" stroke="var(--lamp)" stroke-width="1.6" stroke-dasharray="3 3" />
						<circle cx="17" cy="22" r="5" fill="var(--lamp)" />
						<circle cx="51" cy="22" r="5" fill="var(--lamp)" />
						{#if lit}
							<ellipse cx="68" cy="16" rx="5" ry="3" fill="var(--window-lit)" opacity="0.85" />
						{/if}
					</svg>
				</div>
			{/if}

			{#if balloonOn}
				<div class="mover" bind:this={balloonEl}>
					<svg viewBox="0 0 80 190" class="balloon">
						<path d="M40 92 C36 120 46 140 38 188" fill="none" stroke="var(--pg-line, #1e2640)" stroke-width="2.5" opacity="0.6" />
						<ellipse cx="40" cy="46" rx="34" ry="42" fill="var(--balloon)" />
						<ellipse cx="28" cy="32" rx="8" ry="13" fill="#fff" opacity="0.35" />
						<path d="M34 86 L46 86 L40 94 Z" fill="var(--balloon)" />
					</svg>
				</div>
			{/if}

			{#if flyer}
				<div class="mover" bind:this={flyerEl}>
					<WallPigeon flap lookX={0.8} class="flyer" blinkOffset={2} />
				</div>
			{/if}

			{#if lit}
				<div class="glow lamp-glow" style:left="1760px" style:top="702px"></div>
			{/if}

			<!-- Our rooftop: bulkhead, steam stack, water tower, floodlight, the parapet. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<!-- roof-access bulkhead -->
				<rect x="150" y="868" width="130" height="84" fill="var(--facade)" />
				<rect x="144" y="860" width="142" height="10" fill="var(--near-top)" />
				<rect x="196" y="896" width="38" height="56" rx="2" fill="var(--lamp)" />
				<rect x="250" y="888" width="16" height="16" rx="2" fill="var(--window)" />

				<!-- steam stack, striped like the real ones -->
				<rect x="1232" y="786" width="30" height="166" fill="var(--facade)" />
				<g fill="var(--stripe)">
					<rect x="1232" y="786" width="30" height="14" />
					<rect x="1232" y="814" width="30" height="14" />
				</g>
				<rect x="1228" y="780" width="38" height="8" rx="2" fill="var(--near-top)" />
				<rect x="1226" y="944" width="42" height="10" rx="2" fill="var(--near-top)" />

				<!-- the water tower: legs, bracing, platform, barrel, hoops, cone -->
				<g stroke="var(--lamp)" stroke-width="3" stroke-linecap="round" fill="none">
					<path d="M1460 802 V952 M1492 802 V952 M1528 802 V952 M1560 802 V952" stroke-width="5" />
					<path d="M1460 840 L1560 880 M1560 840 L1460 880 M1460 900 L1560 940 M1560 900 L1460 940" opacity="0.75" />
				</g>
				<rect x="1440" y="790" width="140" height="12" rx="2" fill="var(--lamp)" />
				<path d="M1452 704 L1568 704 L1566 790 L1454 790 Z" fill="var(--pot)" />
				<path d="M1466 704 V790 M1480 704 V790 M1494 704 V790 M1508 704 V790 M1522 704 V790 M1536 704 V790 M1550 704 V790" stroke="var(--brick)" stroke-width="2" opacity="0.6" />
				<g fill="var(--lamp)">
					<rect x="1451" y="716" width="118" height="4" />
					<rect x="1452" y="746" width="116" height="4" />
					<rect x="1453" y="774" width="114" height="4" />
				</g>
				<path d="M1442 706 L1510 662 L1578 706 Z" fill="var(--roof)" />
				<path d="M1509 662 L1510 646 L1511 662 Z" stroke="var(--lamp)" stroke-width="2" />
				{#if lit}
					<circle class="blink" cx="1510" cy="644" r="3" fill="#ff5a4d" />
				{/if}

				<!-- the parapet and the roof below it -->
				<rect x="0" y={LEDGE_Y - 10} width={STAGE_W} height="16" fill="var(--near-top)" />
				<rect x="0" y={LEDGE_Y + 6} width={STAGE_W} height={STAGE_H - LEDGE_Y} fill="var(--near)" />
				<path d={TAR_SEAMS} stroke="var(--seam)" stroke-width="3" />

				<!-- Mimi's slice -->
				<path d="M1738 962 L1810 950 L1810 962 Z" fill="var(--cheese)" stroke="var(--pg-line, #1e2640)" stroke-width="3" stroke-linejoin="round" />
				<rect x="1806" y="946" width="12" height="16" rx="5" fill="var(--crust)" stroke="var(--pg-line, #1e2640)" stroke-width="3" />
				<g fill="var(--pepperoni)">
					<circle cx="1772" cy="957" r="3.4" />
					<circle cx="1792" cy="955" r="3.2" />
				</g>
				<g fill="var(--cheese)">
					<circle cx="1722" cy="959" r="3" />
					<circle cx="1836" cy="958" r="2.6" />
				</g>

				<!-- the floodlight pole, its lamp lit after dark -->
				<path d="M1836 1080 L1837 800 L1843 800 L1844 1080 Z" fill="var(--lamp)" />
				<path d="M1818 802 L1862 802 L1862 794 L1818 794 Z" fill="var(--lamp)" />
				<path d="M1846 798 L1872 786 L1876 806 L1850 814 Z" fill={lit ? 'var(--window-lit)' : 'var(--window)'} stroke="var(--lamp)" stroke-width="4" stroke-linejoin="round" />
			</svg>

			<!-- Steam from the stack, slow and pale. -->
			<div class="smoke" style:left="1225px" style:top="728px">
				<span></span><span></span><span></span>
			</div>

			<!-- Mimi, by her slice (an umbrella when it rains). -->
			<div class="cast" style:left="1620px" style:top="{LEDGE_Y + 6 - 175}px">
				<div class="pecker" class:pecker--busy={!trouble}>
					<WallPigeon
						class="mimi"
						lookX={mimiLookX}
						lookY={trouble ? 0 : 0.2}
						eyes={startled ? 'startled' : 'open'}
						umbrella={raining}
						blinkOffset={3.1}
					/>
				</div>
			</div>

			<!-- Jules, on the floodlight: asleep, unless something is wrong. -->
			<div class="cast" style:left="1785px" style:top="{794 + 4 - 137}px">
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
	.wash {
		opacity: 0;
		animation: wash 8s ease-in-out infinite;
	}
	@keyframes wash {
		50% {
			opacity: 0.8;
		}
	}
	.blink {
		animation: blink 3.2s ease-in-out infinite;
	}
	.blink--late {
		animation-delay: -1.6s;
	}
	@keyframes blink {
		0%,
		60%,
		100% {
			opacity: 0.15;
		}
		30% {
			opacity: 1;
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
		will-change: transform;
	}
	.mover :global(.flyer) {
		display: block;
		width: 120px;
	}
	.balloon {
		display: block;
		width: 60px;
		animation: sway 7s ease-in-out infinite;
		transform-origin: 50% 100%;
	}
	@keyframes sway {
		0%,
		100% {
			transform: rotate(-4deg);
		}
		50% {
			transform: rotate(4deg);
		}
	}
	.cab {
		display: block;
		width: 40px;
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
			transform: translate(70px, -210px) scale(2.6);
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
