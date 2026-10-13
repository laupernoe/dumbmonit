<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop: Victoria Harbour seen from a Kowloon
	 * promenade — the Central skyline under the Peak, the Star Ferry and a red
	 * junk crossing the water, neon signs hanging over the street, red taxis on
	 * the road, and the DumbMonit pigeons on the railing. A drop-in for the
	 * Paris scene (same props, plus `pulse`).
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky
	 * follows the real hour inside the brightness band of the wall's theme
	 * (see `paris/daylight.ts`); the neon blazes after dark and sleeps by day,
	 * the Symphony of Lights beams sweep from the towers at night.
	 *
	 * `mood` is the network as weather and takes the wall's calm / clouded /
	 * storm, or the status page's ok / warn / critical: cloud gathers, then rain
	 * with slow lightning flashes. The pigeon in the beret holds up the problem
	 * count; the other opens an umbrella in the rain. `pulse` replays a gesture
	 * whenever its `at` changes: `down` — the neon sputters out, `up` — it
	 * flickers back on, `resolved` — fireworks burst over the harbour.
	 *
	 * Motion is constant and gentle (ferry, junk, taxis, ripples, beams); it is
	 * all transform and opacity. Hidden tabs pause everything; reduced motion
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
		WATER_Y,
		LEDGE_Y,
		LANE_A,
		LANE_B,
		FAR_TOWERS,
		MID_TOWERS,
		LANDMARKS,
		REFLECTIONS,
		FIREWORKS,
		FIREWORK_RAYS,
		STARS,
		RAIN,
		hongKongStyle
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
		/** A short gesture, replayed whenever `at` changes. */
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
		class?: string;
	}

	let {
		theme,
		now,
		mood = 'calm',
		problems,
		startle = 0,
		paused = false,
		focus = 0,
		pulse = null,
		class: className = ''
	}: Props = $props();

	const motion = !reducedMotion();

	// --- Stage fit ----------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light and weather --------------------------------------------------------

	const weather = $derived<SceneMood>(
		mood === 'storm' || mood === 'critical' ? 'storm' : mood === 'clouded' || mood === 'warn' ? 'clouded' : 'calm'
	);
	const raining = $derived(weather === 'storm');
	const minute = $derived(new Date(Math.floor(now.getTime() / 60_000) * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const style = $derived(`${sceneStyle(theme, phase, weather)};${hongKongStyle(theme)}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));
	const cloudCount = $derived(raining ? 7 : weather === 'clouded' ? 5 : 2);
	const CLOUDS = [
		{ y: 120, s: 1.1, dur: 260, delay: -40 },
		{ y: 300, s: 0.8, dur: 320, delay: -210 },
		{ y: 220, s: 0.95, dur: 290, delay: -150 },
		{ y: 400, s: 1.25, dur: 340, delay: -290 },
		{ y: 60, s: 0.7, dur: 300, delay: -250 },
		{ y: 470, s: 0.6, dur: 360, delay: -120 },
		{ y: 540, s: 1.0, dur: 380, delay: -60 }
	];

	// --- The cast -----------------------------------------------------------------

	const trouble = $derived(problems > 0);
	/** Gaston, in the beret, strolls along the railing between these points. */
	const STROLL = [620, 1240];
	const SIGN_SPOT = 700;
	let gastonX = $state(900);
	let gastonWalkMs = $state(0);
	let gastonWalking = $state(false);
	let gastonLook = $state(0);
	let gastonPeck = $state(false);
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
			gastonLook = 0.5; // the harbour is worth a look
			if (Math.random() < 0.5) {
				later(() => (gastonPeck = true), 1200);
				later(() => (gastonPeck = false), 2600);
			}
			later(stroll, between(5000, 11000));
		}, gastonWalkMs);
	}

	function startAll() {
		if (!motion) {
			gastonX = trouble ? SIGN_SPOT : 900;
			gastonLook = trouble ? -0.8 : 0.4;
			return;
		}
		if (trouble) {
			gastonLook = -0.8;
			gastonWalkMs = Math.round((Math.abs(SIGN_SPOT - gastonX) / 60) * 1000);
			gastonWalking = Math.abs(SIGN_SPOT - gastonX) > 4;
			gastonX = SIGN_SPOT;
			later(() => (gastonWalking = false), gastonWalkMs);
		} else later(stroll, 2000);
	}

	$effect(() => {
		void trouble;
		const hidden = paused;
		untrack(() => {
			epoch++;
			gastonPeck = false;
			gastonWalking = false;
			if (!hidden) startAll();
		});
		return () => {
			epoch++;
		};
	});

	// A new problem: everyone startles once.
	let lastStartle = untrack(() => startle);
	$effect(() => {
		const value = startle;
		if (value === lastStartle) return;
		lastStartle = value;
		startled = true;
		setTimeout(() => (startled = false), 1800);
	});

	const gastonSign = $derived(
		trouble ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null
	);

	// --- The gesture ---------------------------------------------------------------

	let active = $state<'down' | 'up' | 'resolved' | null>(null);
	let playKey = $state(0);
	let seen: number | null = untrack(() => pulse?.at ?? null);
	let timer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const p = pulse;
		if (!p || p.at === seen) return;
		seen = p.at;
		active = null;
		clearTimeout(timer);
		void tick().then(() => {
			active = p.kind;
			playKey++;
			timer = setTimeout(() => (active = null), p.kind === 'resolved' ? 7000 : 5200);
		});
	});
	$effect(() => () => clearTimeout(timer));
</script>

<div
	class="scene {className}"
	class:scene--paused={paused}
	class:scene--still={!motion}
	class:lit
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
				<div class="cloud" style:--still-x="{(i * 530 + 380) % 1700}px" style:top="{cloud.y}px" style:animation-duration="{cloud.dur}s" style:animation-delay="{cloud.delay}s">
					<svg viewBox="0 0 300 110" style:width="{300 * cloud.s}px" style:height="{110 * cloud.s}px">
						<path d="M30 104 C2 104 0 66 30 62 C24 32 60 18 86 34 C98 6 152 0 170 30 C192 14 234 22 238 54 C270 52 292 78 276 104 Z" fill={raining ? 'var(--cloud-storm)' : 'var(--cloud)'} />
					</svg>
				</div>
			{/each}

			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}" fill="none">
				<defs>
					<pattern id="hkw-warm" width="16" height="20" patternUnits="userSpaceOnUse">
						<rect x="5" y="6" width="6" height="8" fill="var(--hk-win)" />
					</pattern>
					<pattern id="hkw-cool" width="22" height="14" patternUnits="userSpaceOnUse">
						<rect x="6" y="5" width="9" height="4" fill="var(--hk-win2)" />
					</pattern>
					<linearGradient id="hkw-sea" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" style="stop-color:color-mix(in srgb, var(--sky-bottom) 45%, var(--hk-sea))" />
						<stop offset="1" stop-color="var(--hk-sea-deep)" />
					</linearGradient>
					<linearGradient id="hkw-haze" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--sky-bottom)" stop-opacity="0" />
						<stop offset="1" stop-color="var(--sky-bottom)" stop-opacity=".55" />
					</linearGradient>
					<path id="hkw-spark" d={FIREWORK_RAYS} stroke="currentColor" stroke-width="4" stroke-linecap="round" />
					<g id="hkw-taxi">
						<path d="M-92 0v-30c0-8 6-12 14-14l28-6 22-24h64l22 24 28 6c8 2 14 6 14 14v30z" fill="var(--c, var(--hk-taxi))" />
						<path d="M-40 -50l18-20h52l18 20z" fill="var(--hk-win)" opacity=".85" />
						<rect x="-16" y="-84" width="32" height="12" rx="4" fill="var(--hk-white)" />
						<circle cx="-52" cy="0" r="16" fill="var(--hk-street)" stroke="var(--hk-lane)" stroke-width="3" /><circle cx="60" cy="0" r="16" fill="var(--hk-street)" stroke="var(--hk-lane)" stroke-width="3" />
						<rect x="-96" y="-30" width="10" height="8" fill="var(--hk-win)" class="head" /><rect x="88" y="-30" width="10" height="8" fill="var(--hk-red)" class="head" />
					</g>
				</defs>

				<!-- the Peak and the far towers -->
				<path d="M700 706C900 640 1080 480 1260 400C1340 368 1470 372 1560 420C1700 490 1820 600 1960 706z" fill="var(--hk-hill)" />
				<path d="M1330 392v-44" stroke="var(--hk-far)" stroke-width="4" /><circle class="pz neon-dot" cx="1330" cy="344" r="5" fill="var(--hk-red)" />
				<g fill="var(--hk-far)">
					{#each FAR_TOWERS as [x, w, t], i (i)}<rect {x} y={t} width={w} height={WATER_Y + 6 - t} />{/each}
				</g>

				<!-- Central landmarks: Bank of China, Central Plaza, IFC, a lower slab -->
				<g fill="var(--hk-tower)">
					<path d="M{LANDMARKS.boc.x - 36} {WATER_Y + 6}V{LANDMARKS.boc.top + 90}l36-92 36 92v{WATER_Y - LANDMARKS.boc.top - 84}z" />
					<path d="M{LANDMARKS.plaza.x - 32} {WATER_Y + 6}V{LANDMARKS.plaza.top + 24}h64v{WATER_Y - LANDMARKS.plaza.top - 18}z" />
					<path d="M{LANDMARKS.plaza.x - 10} {LANDMARKS.plaza.top + 24}l10-60 10 60z" />
					<path d="M{LANDMARKS.ifc.x - 34} {WATER_Y + 6}V{LANDMARKS.ifc.top + 22}l8-22h52l8 22v{WATER_Y - LANDMARKS.ifc.top - 16}z" />
					<rect x={LANDMARKS.ifc.x - 3} y={LANDMARKS.ifc.top - 50} width="6" height="50" />
					<rect x={LANDMARKS.hsbc.x - 60} y={LANDMARKS.hsbc.top} width="120" height={WATER_Y + 6 - LANDMARKS.hsbc.top} />
				</g>
				<g class="pz neon-glow">
					<rect class="plaza" x={LANDMARKS.plaza.x - 32} y={LANDMARKS.plaza.top + 40} width="64" height="14" />
					<rect class="plaza plaza--b" x={LANDMARKS.plaza.x - 32} y={LANDMARKS.plaza.top + 62} width="64" height="14" />
					<circle cx={LANDMARKS.plaza.x} cy={LANDMARKS.plaza.top - 36} r="5" fill="var(--hk-red)" />
					<circle cx={LANDMARKS.ifc.x} cy={LANDMARKS.ifc.top - 52} r="5" fill="var(--hk-red)" />
					<path d="M{LANDMARKS.boc.x - 36} {LANDMARKS.boc.top + 90}l36-92 36 92" stroke="var(--hk-cyan)" stroke-width="3" stroke-linejoin="round" />
					<rect x={LANDMARKS.hsbc.x - 60} y={LANDMARKS.hsbc.top + 30} width="120" height="5" fill="var(--hk-amber)" />
				</g>
				<g class="beams" stroke-width="4" stroke-linecap="round">
					<path class="beam beam--a" d="M{LANDMARKS.ifc.x} {LANDMARKS.ifc.top - 52}L{LANDMARKS.ifc.x + 360} {LANDMARKS.ifc.top - 330}" stroke="var(--hk-cyan)" />
					<path class="beam beam--b" d="M{LANDMARKS.plaza.x} {LANDMARKS.plaza.top - 36}L{LANDMARKS.plaza.x - 340} {LANDMARKS.plaza.top - 300}" stroke="var(--hk-pink)" />
					<path class="beam beam--c" d="M{LANDMARKS.ifc.x} {LANDMARKS.ifc.top - 52}L{LANDMARKS.ifc.x - 120} {LANDMARKS.ifc.top - 360}" stroke="var(--hk-green)" />
				</g>

				<!-- mid towers, windows, haze, quay -->
				<g fill="var(--hk-mid)">
					{#each MID_TOWERS as [x, w, t], i (i)}<rect {x} y={t + 60} width={w} height={WATER_Y + 6 - t - 60} />{/each}
				</g>
				<g class="wins">
					<g class="pz">
						{#each MID_TOWERS as [x, w, t], i (i)}<rect {x} y={t + 60} width={w} height={WATER_Y + 6 - t - 60} fill={i % 3 === 0 ? 'url(#hkw-cool)' : 'url(#hkw-warm)'} />{/each}
						<rect x={LANDMARKS.ifc.x - 34} y={LANDMARKS.ifc.top + 30} width="68" height={WATER_Y - LANDMARKS.ifc.top - 24} fill="url(#hkw-warm)" />
						<rect x={LANDMARKS.plaza.x - 32} y={LANDMARKS.plaza.top + 90} width="64" height={WATER_Y - LANDMARKS.plaza.top - 84} fill="url(#hkw-cool)" />
						<rect x={LANDMARKS.hsbc.x - 60} y={LANDMARKS.hsbc.top + 40} width="120" height={WATER_Y - LANDMARKS.hsbc.top - 34} fill="url(#hkw-warm)" />
					</g>
				</g>
				<rect x="0" y={WATER_Y - 200} width={STAGE_W} height="206" fill="url(#hkw-haze)" />
				<rect x="0" y={WATER_Y + 2} width={STAGE_W} height="8" fill="var(--hk-quay)" />

				<!-- the harbour -->
				<rect x="0" y={WATER_Y + 10} width={STAGE_W} height={LEDGE_Y - WATER_Y - 10} fill="url(#hkw-sea)" />
				<g class="wins" opacity=".55">
					<g class="pz">
						{#each REFLECTIONS as r, i (i)}
							<rect class="refl" x={r.x} y={WATER_Y + 14} width={r.w} height={r.h} rx="2" style="fill:var({r.c});animation-delay:-{r.d}ms" />
						{/each}
					</g>
				</g>
				<g stroke="var(--hk-ripple)" stroke-width="3" stroke-linecap="round" opacity=".4">
					<path d="M140 760h120M520 800h160M960 744h140M1380 790h160M1700 756h120M300 850h200M1100 856h180" />
				</g>

				<!-- a red-sailed junk, the other way -->
				<g class="junk"><g class="bob bob--slow">
					<path d="M-70 0H64L48 20H-52z" fill="var(--hk-hull)" />
					<path d="M-6 -4V-96L-64 -12z" fill="var(--hk-sail)" /><path d="M0 -4V-84L52 -12z" fill="var(--hk-sail)" opacity=".85" />
					<path d="M-6 -96V0M0 -84V0" stroke="var(--hk-hull)" stroke-width="3" />
					<circle class="pz" cx="58" cy="-6" r="4" fill="var(--hk-amber)" />
				</g></g>

				<!-- the Star Ferry -->
				<g class="ferry"><g class="bob">
					<path d="M-150 2H150L124 40H-124z" fill="var(--hk-hull)" />
					<path d="M-140 -10H144v12H-140z" fill="var(--hk-white)" />
					<rect x="-118" y="-52" width="226" height="42" fill="var(--hk-white)" />
					<rect x="-92" y="-96" width="164" height="44" fill="var(--hk-white)" /><path d="M-100 -96h180l-14-14h-152z" fill="var(--hk-hull)" />
					<path d="M-118 -52h226" stroke="var(--hk-hull)" stroke-width="3" opacity=".5" />
					<g class="pz" fill="var(--hk-win)">
						{#each [-104, -62, -20, 22, 64] as x (x)}<rect {x} y="-42" width="30" height="20" />{/each}
						{#each [-78, -42, -6, 30] as x (x)}<rect {x} y="-86" width="26" height="20" />{/each}
					</g>
					<path d="M-4 -110v-40" stroke="var(--hk-hull)" stroke-width="5" /><circle class="pz" cx="-4" cy="-154" r="6" fill="var(--hk-amber)" />
					<path d="M-150 14C-210 18-250 30-300 42M-142 30C-190 34-224 46-262 56" stroke="var(--hk-ripple)" stroke-width="3" stroke-linecap="round" opacity=".5" />
				</g></g>

				<!-- fireworks over the harbour -->
				{#if active === 'resolved'}
					{#key playKey}
						{#each FIREWORKS as f, i (i)}
							<g transform="translate({f.x} {f.y}) scale({f.s})" style="color:var({f.c})">
								<g class="burst" style="animation-delay:{f.d}ms"><use href="#hkw-spark" /></g>
							</g>
						{/each}
					{/key}
				{/if}

				<!-- our side: two buildings with neon, the railing, the road -->
				<g>
					<rect x="-20" y="600" width="360" height={LEDGE_Y - 600} fill="var(--hk-bank)" />
					<rect x="-30" y="590" width="380" height="14" fill="var(--hk-bank-2)" />
					<rect x="1580" y="540" width="360" height={LEDGE_Y - 540} fill="var(--hk-bank)" />
					<rect x="1570" y="530" width="380" height="14" fill="var(--hk-bank-2)" />
					<g class="wins"><g class="pz">
						<rect x="0" y="620" width="320" height="270" fill="url(#hkw-warm)" />
						<rect x="1600" y="560" width="320" height="330" fill="url(#hkw-warm)" />
					</g></g>
					<g class="neons">
						<g class="pz sign" style="color:var(--hk-red)">
							<rect x="288" y="636" width="64" height="230" rx="8" stroke="var(--hk-red)" stroke-width="7" />
							{#each ['大', '酒', '樓'] as ch, i (i)}<text x="320" y={694 + i * 66} text-anchor="middle" font-size="52" font-weight="700" fill="var(--hk-red)">{ch}</text>{/each}
						</g>
						<g class="pz sign" style="color:var(--hk-cyan)">
							<rect x="86" y="676" width="190" height="50" rx="25" stroke="var(--hk-cyan)" stroke-width="6" />
							<text x="181" y="713" text-anchor="middle" font-size="32" font-weight="700" fill="var(--hk-cyan)">茶 餐 廳</text>
						</g>
						<g class="pz sign" style="color:var(--hk-pink)">
							<rect x="60" y="770" width="150" height="9" rx="4.5" fill="var(--hk-pink)" /><rect x="90" y="792" width="100" height="9" rx="4.5" fill="var(--hk-amber)" />
							<path d="M214 760l24-24 24 24M238 736v-30" stroke="var(--hk-pink)" stroke-width="5" stroke-linecap="round" />
						</g>
						<g class="pz sign" style="color:var(--hk-green)">
							<rect x="1500" y="580" width="66" height="250" rx="8" stroke="var(--hk-green)" stroke-width="7" />
							{#each ['旅', '館', 'H'] as ch, i (i)}<text x="1533" y={640 + i * 70} text-anchor="middle" font-size="54" font-weight="700" fill="var(--hk-green)">{ch}</text>{/each}
						</g>
						<g class="pz sign" style="color:var(--hk-amber)">
							<rect x="1580" y="610" width="220" height="56" rx="10" stroke="var(--hk-amber)" stroke-width="6" />
							<text x="1690" y="650" text-anchor="middle" font-size="34" font-weight="700" fill="var(--hk-amber)">金 鋪 24H</text>
						</g>
						<g class="pz sign" style="color:var(--hk-red)">
							<circle cx="1760" cy="740" r="38" stroke="var(--hk-red)" stroke-width="6" />
							<path d="M1744 740h32M1760 724v32" stroke="var(--hk-red)" stroke-width="6" stroke-linecap="round" />
						</g>
					</g>

					<rect x="0" y={LEDGE_Y} width={STAGE_W} height="12" fill="var(--hk-rail)" />
					<rect x="0" y={LEDGE_Y + 12} width={STAGE_W} height="28" fill="var(--hk-bank-2)" />
					<g fill="var(--hk-rail)">{#each Array.from({ length: 17 }, (_, i) => i * 120 + 40) as x (x)}<rect {x} y={LEDGE_Y - 26} width="8" height="28" />{/each}</g>
					<rect x="0" y={LEDGE_Y - 28} width={STAGE_W} height="5" fill="var(--hk-rail)" />
					<rect x="0" y={LEDGE_Y + 40} width={STAGE_W} height={STAGE_H - LEDGE_Y - 40} fill="var(--hk-street)" />
					<path d="M0 1030H1920" stroke="var(--hk-lane)" stroke-width="3" stroke-dasharray="46 34" />
					<g class="street-glow" opacity=".16">
						<rect x="0" y={LEDGE_Y + 40} width={STAGE_W} height="24" fill="var(--hk-red)" />
					</g>
				</g>

				<!-- taxis: a red one going right, a green one coming back -->
				<g transform="translate(0 {LANE_A})"><g class="taxi taxi--a" style="--c:var(--hk-taxi)"><use href="#hkw-taxi" /></g></g>
				<g transform="translate(0 {LANE_B})"><g class="taxi taxi--b" style="--c:var(--hk-taxi-2)"><g transform="scale(-1 1)"><use href="#hkw-taxi" /></g></g></g>
			</svg>

			<!-- Mimi, on the railing (an umbrella when it rains). -->
			<div class="cast" style:left="1330px" style:top="{LEDGE_Y + 6 - 175}px">
				<div class="pecker" class:pecker--busy={!trouble}>
					<WallPigeon class="mimi" lookX={-0.6} lookY={0.2} eyes={startled ? 'startled' : 'open'} umbrella={raining} blinkOffset={3.1} />
				</div>
			</div>

			<!-- Gaston, in the beret: strolls, pecks, holds the sign. -->
			<div class="cast stroller" style:transform="translateX({gastonX - 75}px)" style:transition-duration="{gastonWalkMs}ms" style:top="{LEDGE_Y + 6 - 187}px">
				<div class="waddle" class:waddle--on={gastonWalking} class:peck--on={gastonPeck}>
					<WallPigeon class="gaston" hat sign={gastonSign} lookX={gastonLook} lookY={trouble ? -0.3 : 0} eyes={startled ? 'startled' : 'open'} blinkOffset={0.7} />
				</div>
			</div>

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
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
		contain: strict;
		background: var(--sky-bottom);
		transition: background 60s linear;
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
			opacity: 0.2;
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

	/* Neon by day is glass tubing: pale. After dark it blazes. */
	.wins,
	.neons {
		opacity: 0.12;
		transition: opacity 20s ease;
	}
	.lit .wins,
	.lit .neons {
		opacity: 1;
	}
	.sign,
	.neon-glow,
	.neon-dot {
		filter: drop-shadow(0 0 7px currentColor);
	}
	.neon-glow {
		color: var(--hk-cyan);
	}
	.beams {
		opacity: 0;
		transition: opacity 20s ease;
	}
	.lit .beams {
		opacity: 0.55;
	}
	.beam {
		transform-box: fill-box;
		transform-origin: 0 100%;
	}
	.plaza {
		fill: var(--hk-pink);
	}
	.plaza--b {
		fill: var(--hk-amber);
	}
	.street-glow {
		opacity: 0;
	}
	.lit .street-glow {
		opacity: 0.16;
	}
	.head {
		opacity: 0;
	}
	.lit .head {
		opacity: 1;
		filter: drop-shadow(0 0 8px var(--hk-win));
	}

	/* Ships and taxis cross the whole stage; the pose without motion is mid-way. */
	.ferry {
		transform: translate(520px, 768px);
		animation: sail 120s linear infinite;
		animation-delay: -34s;
	}
	.junk {
		transform: translate(1300px, 746px);
		animation: sail-back 170s linear infinite;
		animation-delay: -60s;
	}
	.taxi--a {
		animation: drive-right 38s linear infinite;
		animation-delay: -14s;
	}
	.taxi--b {
		animation: drive-left 52s linear infinite;
		animation-delay: -30s;
	}
	@keyframes sail {
		from {
			transform: translate(-340px, 768px);
		}
		to {
			transform: translate(2260px, 768px);
		}
	}
	@keyframes sail-back {
		from {
			transform: translate(2200px, 746px) scaleX(-1);
		}
		to {
			transform: translate(-260px, 746px) scaleX(-1);
		}
	}
	@keyframes drive-right {
		from {
			transform: translate(-260px, 0);
		}
		to {
			transform: translate(2180px, 0);
		}
	}
	@keyframes drive-left {
		from {
			transform: translate(2180px, 0);
		}
		to {
			transform: translate(-260px, 0);
		}
	}
	.taxi--a,
	.taxi--b {
		transform: translate(900px, 0);
	}
	.bob {
		animation: bob 5s ease-in-out infinite;
	}
	.bob--slow {
		animation-duration: 7s;
	}
	@keyframes bob {
		50% {
			transform: translateY(4px) rotate(0.6deg);
		}
	}
	.refl {
		animation: shimmer 3.4s ease-in-out infinite alternate;
		transform-box: fill-box;
		transform-origin: 50% 0;
	}
	@keyframes shimmer {
		to {
			opacity: 0.4;
			transform: scaleY(0.82);
		}
	}
	.beam--a {
		animation: sweep 11s ease-in-out infinite alternate;
	}
	.beam--b {
		animation: sweep 13s ease-in-out infinite alternate-reverse;
	}
	.beam--c {
		animation: sweep 9s ease-in-out infinite alternate;
	}
	@keyframes sweep {
		to {
			transform: rotate(14deg);
		}
	}
	.plaza {
		animation: plaza 14s steps(1) infinite;
	}
	.plaza--b {
		animation-delay: -7s;
	}
	@keyframes plaza {
		0% {
			fill: var(--hk-pink);
		}
		25% {
			fill: var(--hk-cyan);
		}
		50% {
			fill: var(--hk-amber);
		}
		75% {
			fill: var(--hk-green);
		}
	}

	/* The gesture: `.pz` is every neon tube, lit window and lamp. */
	.p-down .pz {
		animation: blackout 5.2s ease-out both;
	}
	.p-up .pz {
		animation: relight 2.6s ease-out both;
	}
	.p-down .beams,
	.p-up .beams {
		opacity: 0;
		transition: none;
	}
	@keyframes blackout {
		0% {
			opacity: 1;
		}
		6% {
			opacity: 0.1;
		}
		11% {
			opacity: 0.9;
		}
		16% {
			opacity: 0.05;
		}
		22%,
		82% {
			opacity: 0.02;
		}
		100% {
			opacity: 1;
		}
	}
	@keyframes relight {
		0% {
			opacity: 0.02;
		}
		12% {
			opacity: 1;
		}
		20% {
			opacity: 0.08;
		}
		34% {
			opacity: 1;
		}
		42% {
			opacity: 0.25;
		}
		56%,
		100% {
			opacity: 1;
		}
	}
	.burst {
		transform-box: fill-box;
		transform-origin: center;
		opacity: 0;
		animation: burst 3s ease-out both;
	}
	@keyframes burst {
		0% {
			opacity: 1;
			transform: scale(0.1);
		}
		60% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: scale(1.5) translateY(16px);
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
	@keyframes peck {
		30%,
		60% {
			transform: rotate(16deg) translateY(10px);
		}
	}
	.pecker--busy {
		animation: peck-loop 8s ease-in-out infinite;
	}
	@keyframes peck-loop {
		0%,
		62%,
		100% {
			transform: none;
		}
		68%,
		74% {
			transform: rotate(14deg) translateY(9px);
		}
		80% {
			transform: none;
		}
	}

	.rain {
		position: absolute;
		left: 0;
		top: -540px;
		width: 1920px;
		height: 1620px;
		animation: rain 7s linear infinite;
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

	/* A hidden tab: everything holds its breath. */
	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	/* Reduced motion: a still picture in the right pose, no flashes. */
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
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
		.flash,
		.bolt {
			display: none;
		}
	}
</style>
