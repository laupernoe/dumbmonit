<script lang="ts">
	/**
	 * The wall's living backdrop: a London rooftop, Big Ben and Westminster,
	 * Tower Bridge, the London Eye, and the DumbMonit pigeons going about their
	 * day. A drop-in twin of `paris/ParisScene.svelte` (same props).
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky
	 * follows the real hour inside the brightness band of the wall's theme (see
	 * `paris/daylight.ts`): sun or moon on its arc, stars, lit windows, the
	 * clock face of Big Ben and the Eye's capsules glowing after dark.
	 *
	 * The network's state is woven in: clouds gather with advisories, a storm
	 * brings drizzle and a darker sky, Winston stops strolling and holds up a
	 * sign with the number of problems while looking at the list, Pip opens an
	 * umbrella, Percy wakes up. A new problem startles everyone once.
	 *
	 * Motion is gentle: drifting clouds, chimney smoke, the Eye turning (once
	 * round in ten minutes), a stroll, pecks, blinks, a snore. Every 40–80 s one
	 * small moment plays (a fly-by that drops a crumb, a red balloon, Big Ben
	 * chiming, Percy waking, a red double-decker crossing Tower Bridge). All of
	 * it is transform and opacity; hidden tabs pause everything; reduced motion
	 * freezes the scene in a still pose.
	 */
	import { tick, untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import WallPigeon from '#lib/components/wall/paris/WallPigeon.svelte';
	import {
		STAGE_W,
		STAGE_H,
		LEDGE_Y,
		FAR_BACK,
		FAR_FRONT,
		BEN_X,
		BEN_CLOCK,
		BIG_BEN,
		BEN_FINIAL,
		BEN_LINES,
		WESTMINSTER,
		WESTMINSTER_LINES,
		WESTMINSTER_WINDOWS,
		BRIDGE_DECK_Y,
		BRIDGE_TOWERS,
		BRIDGE_WINDOWS,
		BRIDGE_WALKWAY,
		BRIDGE_DECK,
		BRIDGE_CHAINS,
		EYE,
		EYE_SPOKES,
		EYE_CAPSULES,
		EYE_LEGS,
		TERRACE,
		ZINC_SEAMS,
		STARS,
		RAIN,
		litWindows
	} from './geometry.js';
	import {
		phaseAt,
		isLit,
		celestialAt,
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

	// --- Stage fit ----------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(
		Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale))
	);
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light --------------------------------------------------------------------

	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const BUS: Record<SceneTheme, string> = {
		light: '--bus:#c8322b;--bus-line:#8f211d;--bus-glass:#dfe6ee',
		dark: '--bus:#8a2a2a;--bus-line:#5c1c1c;--bus-glass:#2a3552',
		oled: '--bus:#3a1212;--bus-line:#240b0b;--bus-glass:#10131b'
	};
	const style = $derived(`${sceneStyle(theme, phase, mood)};${BUS[theme]}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));

	let windowSeed = $state(1);
	const litSet = $derived(lit ? litWindows(windowSeed, TERRACE.windows.length, phase === 'night' ? 0.18 : 0.3) : new Set<number>());

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

	/** Winston strolls along the parapet between these points. */
	const STROLL = [890, 1210];
	const SIGN_SPOT = 900;
	let winstonX = $state(1040);
	let winstonWalkMs = $state(0);
	let winstonWalking = $state(false);
	let winstonLook = $state(0);
	let winstonPeck = $state(false);

	/** Pip pecks at a biscuit by the chimney. */
	let pipLookY = $state(0);
	let pipLookX = $state(0.3);

	/** Percy naps on the lamp post. */
	let percyAwake = $state(false);
	let percyLook = $state(0);

	let startled = $state(false);
	let chiming = $state(false);

	let flyer = $state(false);
	let balloonOn = $state(false);
	let crumbOn = $state(false);
	let busOn = $state(false);
	let feathers = $state<{ id: number; x: number; drift: number; delay: number }[]>([]);
	let flyerEl = $state<HTMLDivElement | null>(null);
	let balloonEl = $state<HTMLDivElement | null>(null);
	let crumbEl = $state<HTMLDivElement | null>(null);
	let busEl = $state<HTMLDivElement | null>(null);

	// --- Scheduling ------------------------------------------------------------------

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
		balloonOn = false;
		crumbOn = false;
		busOn = false;
		winstonPeck = false;
		chiming = false;
		percyAwake = trouble;
		pipLookY = 0;
	}

	function stroll() {
		if (trouble) return;
		const target = Math.round(between(STROLL[0], STROLL[1]));
		const distance = Math.abs(target - winstonX);
		if (distance < 60) {
			later(stroll, 1500);
			return;
		}
		winstonLook = target > winstonX ? 0.7 : -0.7;
		winstonWalkMs = Math.round((distance / 34) * 1000);
		winstonWalking = true;
		winstonX = target;
		later(() => {
			winstonWalking = false;
			winstonLook = 0;
			if (Math.random() < 0.5) {
				later(() => (winstonPeck = true), 1200);
				later(() => (winstonPeck = false), 2600);
			}
			later(stroll, between(5000, 11000));
		}, winstonWalkMs);
	}

	function toSignSpot() {
		const distance = Math.abs(SIGN_SPOT - winstonX);
		winstonLook = -0.8;
		winstonWalkMs = Math.round((distance / 60) * 1000);
		winstonWalking = distance > 4;
		winstonX = SIGN_SPOT;
		later(() => (winstonWalking = false), winstonWalkMs);
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

	type Moment = 'flyby' | 'drop' | 'balloon' | 'bus' | 'wake' | 'chime';
	let deck: Moment[] = [];
	function nextMoment(): Moment | null {
		const allowed: Moment[] = ['flyby', 'drop', 'balloon', 'wake', 'chime', 'bus'];
		deck = deck.filter((m) => allowed.includes(m));
		if (deck.length === 0) deck = [...allowed].sort(() => Math.random() - 0.5);
		return deck.shift() ?? null;
	}

	async function play(moment: Moment) {
		const mine = epoch;
		if (moment === 'flyby' || moment === 'drop') {
			flyer = true;
			await tick();
			const y = between(230, 380);
			const duration = 19000;
			if (moment === 'drop') {
				// The crumb leaves the flyer right above Pip, who looks up, then eats it.
				later(() => {
					crumbOn = true;
					pipLookY = -1;
					void tick().then(() =>
						animate(crumbEl, [
							{ transform: `translate(1600px, ${y + 120}px) rotate(0deg)` },
							{ transform: `translate(1654px, ${LEDGE_Y - 14}px) rotate(220deg)` }
						], { duration: 2600, easing: 'cubic-bezier(0.45, 0, 0.85, 0.6)', fill: 'forwards' })
					);
				}, duration * 0.78);
				later(() => (pipLookY = 0.7), duration * 0.78 + 2600);
				later(() => {
					crumbOn = false;
					pipLookY = 0;
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
			if (mine === epoch) flyer = false;
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
		} else if (moment === 'bus') {
			// A red double-decker crosses Tower Bridge, left to right.
			busOn = true;
			await tick();
			await animate(
				busEl,
				[
					{ transform: `translate(150px, ${BRIDGE_DECK_Y - 78}px)`, opacity: 0 },
					{ transform: `translate(250px, ${BRIDGE_DECK_Y - 78}px)`, opacity: 1, offset: 0.1 },
					{ transform: `translate(700px, ${BRIDGE_DECK_Y - 78}px)`, opacity: 1, offset: 0.9 },
					{ transform: `translate(800px, ${BRIDGE_DECK_Y - 78}px)`, opacity: 0 }
				],
				{ duration: 20000, easing: 'linear', fill: 'forwards' }
			);
			if (mine === epoch) busOn = false;
		} else if (moment === 'wake') {
			percyAwake = true;
			percyLook = -0.8;
			later(() => (percyLook = 0.8), 2600);
			later(() => (percyLook = 0), 5000);
			later(() => {
				if (!trouble) percyAwake = false;
			}, 6800);
		} else if (moment === 'chime') {
			chiming = true;
			later(() => (chiming = false), 14000);
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
		percyAwake = trouble;
		percyLook = trouble ? -0.8 : 0;
		pipLookX = trouble ? -0.8 : 0.3;
		if (!motion) {
			winstonX = trouble ? SIGN_SPOT : 1040;
			winstonLook = trouble ? -0.8 : 0;
			return;
		}
		if (trouble) toSignSpot();
		else later(stroll, 2000);
		scheduleMoments(true);
	}

	$effect(() => {
		void trouble;
		const hidden = paused;
		untrack(() => {
			stopAll();
			if (!hidden) startAll();
		});
		return () => untrack(stopAll);
	});

	$effect(() => {
		if (!lit || paused || !motion) return;
		const timer = setInterval(() => (windowSeed += 1), 120_000);
		return () => clearInterval(timer);
	});

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
				x: winstonX + (i - 1.5) * 26,
				drift: Math.round(between(-40, 40)),
				delay: i * 260
			}));
			setTimeout(() => (feathers = []), 7000);
		});
	});

	const winstonSign = $derived(trouble ? `${problems} ${problems === 1 ? 'PROBLEM' : 'PROBLEMS'}` : null);
	const winstonEyes = $derived(startled ? 'startled' : 'open');
	const clockGlow = $derived(lit || chiming);
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

			{#if clockGlow}
				<div class="glow clock-glow" class:clock-glow--chime={chiming} style:left="{BEN_CLOCK.x - 60}px" style:top="{BEN_CLOCK.y - 60}px"></div>
			{/if}

			<!-- The city: far roofs, Westminster and Big Ben, Tower Bridge, the Eye, the terrace. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<path d={FAR_BACK} fill="var(--far-2)" />
				<path d={FAR_FRONT} fill="var(--far)" />

				<!-- London Eye: legs, then the wheel, which turns once in ten minutes. -->
				<path d={EYE_LEGS} stroke="var(--tower)" stroke-width="9" stroke-linecap="round" fill="none" />
				<g class="eye">
					<circle cx={EYE.x} cy={EYE.y} r={EYE.r} fill="none" stroke="var(--tower-line)" stroke-width="4" />
					<circle cx={EYE.x} cy={EYE.y} r={EYE.r - 14} fill="none" stroke="var(--tower-line)" stroke-width="2" />
					<path d={EYE_SPOKES} stroke="var(--tower-line)" stroke-width="1.2" fill="none" opacity="0.8" />
					{#each EYE_CAPSULES as c, i (i)}
						<circle cx={c.x} cy={c.y} r="6.5" fill={lit ? 'var(--window-lit)' : 'var(--tower)'} stroke="var(--tower-line)" stroke-width="1.5" />
					{/each}
				</g>
				<circle cx={EYE.x} cy={EYE.y} r="9" fill="var(--tower)" />

				<!-- Tower Bridge -->
				<path d={BRIDGE_CHAINS} stroke="var(--tower-line)" stroke-width="3" fill="none" />
				<path d={BRIDGE_DECK} fill="var(--tower)" />
				<path d={BRIDGE_WALKWAY} stroke="var(--tower-line)" stroke-width="3" fill="none" />
				<path d={BRIDGE_TOWERS} fill="var(--tower)" />
				<g fill={lit ? 'var(--window-lit)' : 'var(--window)'}>
					{#each BRIDGE_WINDOWS as w, i (i)}
						<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="5" />
					{/each}
				</g>

				<!-- Westminster and Big Ben -->
				<path d={WESTMINSTER} fill="var(--tower)" />
				<path d={WESTMINSTER_LINES} stroke="var(--tower-line)" stroke-width="2" fill="none" />
				<g fill={lit ? 'var(--window-lit)' : 'var(--window)'}>
					{#each WESTMINSTER_WINDOWS as w, i (i)}
						<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="4" />
					{/each}
				</g>
				<path d={BIG_BEN} fill="var(--tower)" />
				<path d={BEN_LINES} stroke="var(--tower-line)" stroke-width="2" fill="none" />
				<path d={BEN_FINIAL} stroke="var(--dome)" stroke-width="3" stroke-linecap="round" />
				<circle cx={BEN_CLOCK.x} cy={BEN_CLOCK.y} r={BEN_CLOCK.r + 5} fill="var(--dome)" />
				<circle cx={BEN_CLOCK.x} cy={BEN_CLOCK.y} r={BEN_CLOCK.r} fill={clockGlow ? 'var(--window-lit)' : 'var(--window)'} />
				<g stroke="var(--lamp)" stroke-width="3" stroke-linecap="round">
					<path d="M{BEN_X} {BEN_CLOCK.y} L{BEN_X} {BEN_CLOCK.y - 20} M{BEN_X} {BEN_CLOCK.y} L{BEN_X + 13} {BEN_CLOCK.y + 8}" />
				</g>

				<!-- The terrace -->
				<path d={TERRACE.walls} fill="var(--brick)" />
				<path d={TERRACE.stacks} fill="var(--brick)" />
				<path d={TERRACE.pots} fill="var(--pot)" />
				<path d={TERRACE.roofs} fill="var(--roof)" />
				<path d={TERRACE.sills} fill="var(--near-top)" />
				<g fill="var(--window)">
					{#each TERRACE.windows as w, i (i)}
						<rect x={w.x} y={w.y} width={w.w} height={w.h} rx="2" />
					{/each}
				</g>
				<g fill="var(--window-lit)">
					{#each TERRACE.windows as w, i (i)}
						{#if litSet.has(i)}
							<rect class="lit" x={w.x} y={w.y} width={w.w} height={w.h} rx="2" />
						{/if}
					{/each}
				</g>
			</svg>

			{#if busOn}
				<div class="mover bus-mover" bind:this={busEl}>
					<svg viewBox="0 0 200 100" class="bus">
						<rect x="4" y="6" width="192" height="80" rx="8" fill="var(--bus)" />
						<rect x="4" y="46" width="192" height="4" fill="var(--bus-line)" />
						<g fill={lit ? 'var(--window-lit)' : 'var(--bus-glass)'}>
							{#each [14, 48, 82, 116, 150] as wx (wx)}
								<rect x={wx} y="14" width="28" height="26" rx="3" />
								<rect x={wx} y="56" width="28" height="22" rx="3" />
							{/each}
						</g>
						<rect x="4" y="82" width="192" height="6" fill="var(--bus-line)" />
						<circle cx="42" cy="88" r="11" fill="var(--lamp)" />
						<circle cx="158" cy="88" r="11" fill="var(--lamp)" />
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
				<div class="glow lamp-glow" style:left="1690px" style:top="668px"></div>
			{/if}

			<!-- Our rooftop: chimney stack and pots, flower box, the parapet, the lamp post. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<rect x="1440" y="872" width="84" height="84" fill="var(--brick)" />
				<rect x="1434" y="866" width="96" height="10" fill="var(--near-top)" />
				<rect x="1444" y="836" width="14" height="32" rx="2" fill="var(--pot)" />
				<rect x="1464" y="824" width="16" height="44" rx="2" fill="var(--pot)" />
				<rect x="1486" y="840" width="14" height="28" rx="2" fill="var(--pot)" />
				<rect x="1506" y="832" width="14" height="36" rx="2" fill="var(--pot)" />

				<rect x="1262" y="930" width="120" height="24" rx="4" fill="var(--pot)" />
				<g fill="var(--pg-chest, #4c9d6f)">
					<ellipse cx="1280" cy="924" rx="18" ry="11" />
					<ellipse cx="1310" cy="918" rx="20" ry="13" />
					<ellipse cx="1342" cy="921" rx="19" ry="12" />
					<ellipse cx="1368" cy="925" rx="15" ry="10" />
				</g>
				<g fill="var(--balloon)">
					<circle cx="1288" cy="908" r="7" />
					<circle cx="1318" cy="902" r="8" />
					<circle cx="1346" cy="907" r="7" />
					<circle cx="1366" cy="913" r="6" />
				</g>

				<rect x="0" y={LEDGE_Y - 10} width={STAGE_W} height="16" fill="var(--near-top)" />
				<rect x="0" y={LEDGE_Y + 6} width={STAGE_W} height={STAGE_H - LEDGE_Y} fill="var(--near)" />
				<path d={ZINC_SEAMS} stroke="var(--seam)" stroke-width="3" />

				<!-- Pip's biscuit -->
				<circle cx="1712" cy="948" r="16" fill="var(--pastry, #d9a35b)" stroke="var(--pg-line, #1e2640)" stroke-width="4" />
				<g fill="var(--pastry-crust, #b07a3a)">
					<circle cx="1706" cy="944" r="2.4" />
					<circle cx="1716" cy="950" r="2.4" />
					<circle cx="1718" cy="942" r="2.4" />
				</g>
				<g fill="var(--pastry, #d9a35b)">
					<circle cx="1646" cy="958" r="3.5" />
					<circle cx="1760" cy="957" r="3" />
				</g>

				<!-- lamp post, lantern lit after dark -->
				<path d="M1794 1080 L1796 800 L1804 800 L1806 1080 Z" fill="var(--lamp)" />
				<path d="M1786 806 L1814 806 L1810 796 L1790 796 Z" fill="var(--lamp)" />
				<path d="M1782 796 L1818 796 L1824 758 L1776 758 Z" fill={lit ? 'var(--window-lit)' : 'var(--window)'} stroke="var(--lamp)" stroke-width="5" stroke-linejoin="round" />
				<path d="M1770 760 L1830 760 L1800 744 Z" fill="var(--lamp)" />
			</svg>

			<div class="smoke" style:left="1472px" style:top="790px">
				<span></span><span></span><span></span>
			</div>

			{#if crumbOn}
				<div class="mover" bind:this={crumbEl}>
					<svg viewBox="0 0 14 14" class="crumb"><circle cx="7" cy="7" r="6" fill="var(--pastry, #d9a35b)" /></svg>
				</div>
			{/if}

			<!-- Pip, by the biscuit (an umbrella when it rains). -->
			<div class="cast" style:left="1540px" style:top="{LEDGE_Y + 6 - 175}px">
				<div class="pecker" class:pecker--busy={!trouble && pipLookY === 0}>
					<WallPigeon
						class="pip"
						lookX={pipLookX}
						lookY={pipLookY}
						eyes={startled ? 'startled' : 'open'}
						umbrella={raining}
						blinkOffset={3.1}
					/>
				</div>
			</div>

			<!-- Percy, on the lamp: asleep, unless something is wrong. -->
			<div class="cast" style:left="1745px" style:top="{744 + 4 - 137}px">
				<div class="napper" class:napper--asleep={!percyAwake && !startled}>
					<WallPigeon class="percy" eyes={percyAwake || startled ? (startled ? 'startled' : 'open') : 'closed'} lookX={percyLook} lookY={percyAwake ? 0.2 : 0} blinkOffset={5.4} />
				</div>
				{#if !percyAwake && !startled}
					<svg class="zzz" viewBox="0 0 80 120">
						<text x="20" y="100">z</text>
						<text x="20" y="100">z</text>
						<text x="20" y="100">Z</text>
					</svg>
				{/if}
			</div>

			<!-- Winston: strolls, pecks, holds the sign. -->
			<div
				class="cast stroller"
				style:transform="translateX({winstonX - 75}px)"
				style:transition-duration="{winstonWalkMs}ms"
				style:top="{LEDGE_Y + 6 - 187}px"
			>
				<div class="waddle" class:waddle--on={winstonWalking} class:peck--on={winstonPeck}>
					<WallPigeon
						class="winston"
						sign={winstonSign}
						lookX={winstonLook}
						lookY={trouble ? -0.3 : 0}
						eyes={winstonEyes}
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
	.clock-glow {
		animation: breathe 6s ease-in-out infinite;
	}
	.clock-glow--chime {
		width: 180px;
		height: 180px;
		margin: -30px 0 0 -30px;
		animation-duration: 2s;
	}
	.eye {
		transform-box: fill-box;
		transform-origin: center;
		animation: turn 600s linear infinite;
	}
	@keyframes turn {
		to {
			transform: rotate(360deg);
		}
	}
	.bus-mover {
		opacity: 0;
	}
	.bus {
		display: block;
		width: 156px;
	}
	.lamp-glow {
		animation: breathe 9s ease-in-out infinite;
	}
	@keyframes breathe {
		50% {
			opacity: 0.55;
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
	.cast :global(.winston) {
		display: block;
		width: 150px;
	}
	.cast :global(.pip) {
		display: block;
		width: 140px;
	}
	.cast :global(.percy) {
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
