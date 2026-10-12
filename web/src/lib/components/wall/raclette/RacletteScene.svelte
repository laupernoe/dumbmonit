<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import {
		phaseAt,
		isLit,
		celestialAt,
		type SceneTheme
	} from '#lib/components/wall/paris/daylight.js';
	import {
		STAGE_W,
		STAGE_H,
		WIN,
		TABLE_Y,
		FRONT_Y,
		PANS,
		PAN_Y,
		GLASSES,
		GLASS_Y,
		FLAKES,
		WIN_STARS,
		FLAGS,
		ROOM_DIM,
		levelOf,
		racletteStyle,
		type RacletteMood
	} from './geometry.js';

	/**
	 * The wall's living backdrop: a raclette night in a Swiss chalet. A table
	 * under a red-and-white checked cloth, a raclette grill with four little
	 * poêlons of cheese, potatoes, pickles, two glasses of fendant, a smoke
	 * alarm and a pendant lamp, and a window on a snowy village. A drop-in for
	 * the Paris scene (same props), plus `pulse`.
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The window
	 * follows the real hour — dawn, day, golden hour, dusk, night, with the sun
	 * or the moon on its arc, stars, lit chalet windows — and the room darkens
	 * around the glow of the grill and the lamp.
	 *
	 * The network's state is the cheese: calm (or `ok`) it melts gently under a
	 * clear sky; clouded (`warn`) it starts to smoke under clouds; storm
	 * (`critical`) it is black, a cloud of smoke fills the room, the smoke alarm
	 * blinks red and a blizzard rages with flashes of lightning. The chalkboard
	 * counts the problems; a new one makes the alarm chirp once.
	 *
	 * `pulse` replays a gesture whenever `at` changes: down = a poêlon falls off
	 * the grill with a cheese splat, up = a hand puts in a fresh slice, resolved
	 * = glasses of fendant clink, "Santé !".
	 *
	 * Motion is gentle (snow, steam, bubbles, drifting clouds, the glow of the
	 * coil), transform and opacity only; nothing flashes but the alarm and the
	 * storm. Hidden tabs pause everything; reduced motion freezes the scene.
	 */
	interface Props {
		theme?: SceneTheme;
		now?: Date;
		mood?: RacletteMood;
		/** Problems on the board: the number on the chalkboard. */
		problems?: number;
		/** Bumped by the wall when a new problem appears: the alarm chirps once. */
		startle?: number;
		/** The tab is hidden: nothing moves, nothing is scheduled. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
		class?: string;
	}

	let {
		theme = 'light',
		now = new Date(),
		mood = 'calm',
		problems = 0,
		startle = 0,
		paused = false,
		focus = 0,
		pulse = null,
		class: className = ''
	}: Props = $props();

	const motion = !reducedMotion();

	// --- Stage fit ------------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light and mood ---------------------------------------------------------------

	// Re-read once a minute: nothing in the sky moves faster than that.
	const minute = $derived(new Date(Math.floor(now.getTime() / 60_000) * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const level = $derived(levelOf(mood));
	const style = $derived(racletteStyle(theme, phase, level));
	const starsOn = $derived(level === 'ok' && (phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn')));
	// The sun or the moon, on its arc across the window.
	const sky = $derived({
		x: ((body.x - 820) / 540) * WIN.w,
		y: 50 + ((body.y - 140) / 330) * 200,
		moon: body.body === 'moon'
	});
	const dim = $derived(theme === 'oled' ? 0.35 : ROOM_DIM[phase]);

	const trouble = $derived(problems > 0);
	const sign = $derived(trouble ? m.wall_scene_sign_problems({ count: problems }) : '');
	const kind = $derived(motion && !paused && pulse ? pulse.kind : null);

	// A new problem: the alarm chirps once.
	let chirp = $state(false);
	let lastStartle = untrack(() => startle);
	$effect(() => {
		const value = startle;
		if (value === lastStartle) return;
		lastStartle = value;
		if (!motion || paused) return;
		chirp = true;
		const timer = setTimeout(() => (chirp = false), 2600);
		return () => clearTimeout(timer);
	});
</script>

{#snippet pan(x: number)}
	<g transform="translate({x} {PAN_Y})">
		<g>
			<rect x="52" y="4" width="46" height="9" rx="4.5" fill="var(--wood)" />
			<path d="M-60 0H60L50 38H-50Z" fill="var(--pan)" />
			<path d="M-60 0H60L58 7H-58Z" fill="var(--metal)" />
			<ellipse cx="0" cy="2" rx="55" ry="9" fill="var(--cheese)" />
			<path d="M-55 2C-55 15 -50 22 -46 14V6ZM38 5C38 20 46 25 50 14L55 2Z" fill="var(--cheese)" />
			<ellipse cx="-16" cy="-1" rx="16" ry="3.5" fill="var(--cheese-hi)" />
			<g class="bubbles" fill="var(--cheese-hi)">
				<circle cx="18" cy="0" r="4.5" />
				<circle cx="-32" cy="4" r="3.2" />
				<circle cx="34" cy="3" r="2.6" />
			</g>
			<g class="char" fill="#0d0907">
				<ellipse cx="-14" cy="2" rx="18" ry="4.5" />
				<ellipse cx="27" cy="4" rx="14" ry="4" />
			</g>
		</g>
	</g>
{/snippet}

{#snippet glass(x: number)}
	<g transform="translate({x} {GLASS_Y}) scale(1.7)">
		<path d="M-14 0H14C14 17 7 28 0 28C-7 28 -14 17 -14 0Z" fill="var(--glass)" opacity=".7" />
		<path d="M-12.5 7H12.5C12 18 7 26 0 26C-7 26 -12.5 18 -12.5 7Z" fill="var(--wine)" />
		<rect x="-1.5" y="28" width="3" height="16" fill="var(--glass)" />
		<ellipse cx="0" cy="45" rx="11" ry="3" fill="var(--glass)" />
		<path d="M-9 3V12" stroke="#fff" stroke-width="2" stroke-linecap="round" opacity=".7" />
	</g>
{/snippet}

<div
	class="scene {className}"
	class:scene--paused={paused}
	class:scene--still={!motion}
	class:l-ok={level === 'ok'}
	class:l-warn={level === 'warn'}
	class:l-critical={level === 'critical'}
	class:chirp
	class:lit
	{style}
	bind:clientWidth={boxW}
	bind:clientHeight={boxH}
	aria-hidden="true"
>
	{#if scale > 0}
		<div class="stage" style:transform="translate({offsetX}px, {offsetY}px) scale({scale})">
			<svg viewBox="0 0 {STAGE_W} {STAGE_H}" width={STAGE_W} height={STAGE_H} fill="none" focusable="false">
				<defs>
					<linearGradient id="rw-wall" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--wall1)" />
						<stop offset="1" stop-color="var(--wall2)" />
					</linearGradient>
					<linearGradient id="rw-sky" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--sky-top)" />
						<stop offset="1" stop-color="var(--sky-bottom)" />
					</linearGradient>
					<pattern id="rw-plank" width="96" height="10" patternUnits="userSpaceOnUse">
						<path d="M0.5 0V10" stroke="var(--plank)" stroke-width="2" />
					</pattern>
					<pattern id="rw-check" width="72" height="72" patternUnits="userSpaceOnUse">
						<rect width="72" height="72" fill="var(--cloth-w)" />
						<rect width="36" height="72" fill="var(--cloth-r)" opacity=".55" />
						<rect width="72" height="36" fill="var(--cloth-r)" opacity=".55" />
					</pattern>
					<radialGradient id="rw-glow" cx=".5" cy=".5" r=".5">
						<stop offset="0" stop-color="var(--ember)" stop-opacity=".6" />
						<stop offset="1" stop-color="var(--ember)" stop-opacity="0" />
					</radialGradient>
					<radialGradient id="rw-lamp" cx=".5" cy=".3" r=".6">
						<stop offset="0" stop-color="var(--lamp-glow)" />
						<stop offset="1" stop-color="var(--lamp-glow)" stop-opacity="0" />
					</radialGradient>
					<clipPath id="rw-win"><rect x={WIN.x} y={WIN.y} width={WIN.w} height={WIN.h} /></clipPath>
				</defs>

				<!-- the chalet wall -->
				<rect width={STAGE_W} height={TABLE_Y + 20} fill="url(#rw-wall)" />
				<rect width={STAGE_W} height={TABLE_Y + 20} fill="url(#rw-plank)" />
				<rect width={STAGE_W} height="22" fill="var(--wood)" opacity=".6" />

				<!-- the window: sky, weather, mountains, village -->
				<g clip-path="url(#rw-win)">
					<rect x={WIN.x} y={WIN.y} width={WIN.w} height={WIN.h} fill="url(#rw-sky)" />
					<g transform="translate({WIN.x} {WIN.y})">
						{#if starsOn}
							{#each WIN_STARS as s, i (i)}
								<circle class="star" cx={s.x - WIN.x} cy={s.y - WIN.y} r={s.r} fill="#e8eef8" style="animation-delay:{-(i % 7)}s" />
							{/each}
						{/if}
						<g class="body" opacity={level === 'ok' ? 1 : level === 'warn' ? 0.25 : 0}>
							{#if sky.moon}
								<circle cx={sky.x} cy={sky.y} r="46" fill="#fbf7ec" opacity=".18" />
								<circle cx={sky.x} cy={sky.y} r="26" fill="#fbf7ec" />
								<circle cx={sky.x + 8} cy={sky.y - 6} r="5" fill="#e3ddc9" />
							{:else}
								<circle cx={sky.x} cy={sky.y} r="60" fill="#ffe9a0" opacity=".3" />
								<circle cx={sky.x} cy={sky.y} r="32" fill="#ffe27a" />
							{/if}
						</g>
						<g class="clouds" fill="var(--cloud)">
							<g class="cloud-a"><ellipse cx="120" cy="90" rx="70" ry="20" /><ellipse cx="160" cy="76" rx="40" ry="22" /></g>
							<g class="cloud-b"><ellipse cx="380" cy="130" rx="86" ry="22" /><ellipse cx="340" cy="116" rx="44" ry="22" /></g>
							<g class="heavy"><ellipse cx="250" cy="40" rx="190" ry="40" /><ellipse cx="60" cy="70" rx="90" ry="30" /><ellipse cx="450" cy="70" rx="90" ry="30" /></g>
						</g>
						<path d="M0 290L90 200L150 245L270 90L400 255L450 210L520 260V400H0Z" fill="var(--mount)" />
						<path d="M0 320L60 270L130 300L210 250L300 300L380 262L460 300L520 280V400H0Z" fill="var(--mount2)" />
						<path d="M270 90L236 152L256 144L270 164L290 140L306 154ZM90 200L72 224L86 218L96 230L108 214Z" fill="#fff" />
						<path d="M0 345Q130 315 260 340T520 330V400H0Z" fill="var(--snow)" />
						<!-- chalets, church, firs -->
						<g class="village">
							<rect x="40" y="304" width="70" height="42" fill="var(--chalet)" />
							<path d="M28 306L75 272L122 306Z" fill="#fff" />
							<rect class="wnd" x="54" y="318" width="12" height="14" />
							<rect class="wnd" x="82" y="318" width="12" height="14" />
							<rect x="372" y="310" width="76" height="44" fill="var(--chalet)" />
							<path d="M360 312L410 276L460 312Z" fill="#fff" />
							<rect class="wnd" x="386" y="324" width="12" height="14" />
							<rect class="wnd" x="418" y="324" width="12" height="14" />
							<rect x="214" y="270" width="34" height="68" fill="var(--chalet)" />
							<path d="M210 272L231 206L252 272Z" fill="var(--cloth-r)" />
							<circle class="wnd-round" cx="231" cy="294" r="8" />
							<path d="M300 346L320 292L340 346ZM150 350L168 306L186 350ZM476 352L492 316L508 352Z" fill="var(--fir)" />
							<path d="M310 312L320 292L330 312ZM160 322L168 306L176 322Z" fill="#fff" opacity=".8" />
						</g>
						{#each FLAKES as f, i (i)}
							<circle class="flake" class:extra={f.extra} cx={f.x - WIN.x} cy={f.y0} r={f.r} fill="#fff" style="--d:{f.d}s;--y0:{f.y0}px;animation-delay:{-(i * 0.7) % 6}s" />
						{/each}
						<path class="bolt" d="M330 0L292 150L326 144L270 340L370 130L332 138L376 0Z" fill="#fffbd0" />
						<rect class="storm-shade" width={WIN.w} height={WIN.h} fill="#141a28" />
						<rect class="flash" width={WIN.w} height={WIN.h} fill="#fff" />
					</g>
				</g>
				<rect x={WIN.x - 8} y={WIN.y - 8} width={WIN.w + 16} height={WIN.h + 16} stroke="var(--frame)" stroke-width="16" />
				<path d="M{WIN.x + WIN.w / 2} {WIN.y}V{WIN.y + WIN.h}M{WIN.x} {WIN.y + WIN.h / 2}H{WIN.x + WIN.w}" stroke="var(--frame)" stroke-width="10" />
				<rect x={WIN.x - 24} y={WIN.y + WIN.h + 6} width={WIN.w + 48} height="22" rx="5" fill="var(--frame)" />
				<rect x={WIN.x - 90} y={WIN.y - 14} width="62" height={WIN.h + 30} rx="5" fill="var(--shutter)" />
				<rect x={WIN.x + WIN.w + 28} y={WIN.y - 14} width="62" height={WIN.h + 30} rx="5" fill="var(--shutter)" />
				<path d="M{WIN.x - 59} {WIN.y + 190}c-20-22-8-36 0-26c8-10 20 4 0 26zM{WIN.x + WIN.w + 59} {WIN.y + 190}c-20-22-8-36 0-26c8-10 20 4 0 26z" fill="var(--wall1)" />

				<!-- garland of little Swiss flags -->
				<path d="M0 24C120 70 260 70 380 24C500 70 640 70 760 24C880 70 1020 70 1140 24" stroke="var(--ink)" stroke-width="2" opacity=".5" />
				{#each FLAGS.slice(0, 10) as f, i (i)}
					<g transform="translate({f.x} {f.y + 6})">
						<path d="M0 0h30l-15 28z" fill="var(--cloth-r)" />
						<path d="M11 6h8v6h6v6h-6v6h-8v-6h-6v-6h6z" fill="#fff" transform="translate(-1 -3) scale(.9)" />
					</g>
				{/each}

				<!-- chalkboard -->
				<g>
					<rect x="840" y="196" width="280" height="130" rx="8" fill="var(--board)" stroke="var(--wood)" stroke-width="10" />
					{#if trouble}
						<text x="980" y="276" text-anchor="middle" font-size={sign.length > 14 ? 30 : 40} font-weight="700" font-family="'Comic Sans MS', 'Marker Felt', system-ui, sans-serif" fill="var(--chalk)">{sign}</text>
						<path d="M890 296q90 10 180 0" stroke="var(--chalk)" stroke-width="3" stroke-linecap="round" opacity=".7" />
					{:else}
						<path d="M900 296L960 226L1040 296Z" fill="#f7cf4f" opacity=".9" />
						<circle cx="948" cy="278" r="7" fill="var(--board)" /><circle cx="990" cy="284" r="5" fill="var(--board)" /><circle cx="1010" cy="270" r="4" fill="var(--board)" />
						<path d="M1060 236l4 10l10 4l-10 4l-4 10l-4-10l-10-4l10-4zM880 230l3 7l7 3l-7 3l-3 7l-3-7l-7-3l7-3z" fill="var(--chalk)" />
					{/if}
				</g>

				<!-- the table -->
				<rect y={TABLE_Y} width={STAGE_W} height={STAGE_H - TABLE_Y} fill="url(#rw-check)" />
				<rect y={TABLE_Y} width={STAGE_W} height="10" fill="var(--wood)" opacity=".7" />
				<rect y={FRONT_Y} width={STAGE_W} height={STAGE_H - FRONT_Y} fill="var(--shade)" opacity=".2" />
				<path d="M0 {FRONT_Y}H{STAGE_W}" stroke="var(--cloth-r)" stroke-width="4" opacity=".35" />

				<!-- the room darkens with the hour; the lamp and the grill keep the light -->
				<rect width={STAGE_W} height={STAGE_H} fill="#0a0612" opacity={dim} />
				<ellipse class="lamp-glow" cx="600" cy="660" rx="420" ry="360" fill="url(#rw-lamp)" opacity={lit ? 1 : 0.35} />
				<ellipse class="grill-glow" cx="965" cy="900" rx="420" ry="90" fill="url(#rw-glow)" />

				<!-- the pendant lamp -->
				<path d="M600 0V150" stroke="var(--ink)" stroke-width="3" opacity=".7" />
				<path d="M550 200L575 150H625L650 200Z" fill="var(--cloth-r)" />
				<ellipse cx="600" cy="200" rx="50" ry="8" fill="#ffd98a" opacity={lit ? 1 : 0.6} />

				<!-- smoke alarm -->
				<circle class="halo" cx="1000" cy="92" r="54" fill="#ff2a2a" />
				<circle cx="1000" cy="92" r="34" fill="#f3f3ef" stroke="#c9c9c2" stroke-width="3" />
				<circle cx="1000" cy="92" r="18" stroke="#c9c9c2" stroke-width="2" />
				<path d="M984 92h32M1000 76v32" stroke="#c9c9c2" stroke-width="1.5" />
				<circle class="led" cx="1000" cy="92" r="6.5" fill="var(--led)" />
				<g class="beeps" stroke="#e11" stroke-width="4" stroke-linecap="round">
					<path d="M1052 70c12 12 12 36 0 48M1068 58c19 19 19 55 0 74" />
					<path d="M948 70c-12 12-12 36 0 48M932 58c-19 19-19 55 0 74" />
				</g>

				<!-- left: potatoes and pickles -->
				<ellipse cx="380" cy="962" rx="150" ry="28" fill="var(--plate)" stroke="var(--plate-edge)" stroke-width="3" />
				<g fill="var(--potato)" stroke="var(--potato-line)" stroke-width="2.5">
					<ellipse cx="320" cy="934" rx="46" ry="28" />
					<ellipse cx="404" cy="926" rx="50" ry="30" />
					<ellipse cx="458" cy="950" rx="38" ry="22" />
					<ellipse cx="368" cy="954" rx="34" ry="18" />
				</g>
				<path d="M290 930h22M392 920h26M446 948h16" stroke="var(--potato-line)" stroke-width="2.5" stroke-linecap="round" />
				<g class="steam" stroke="var(--steam)" stroke-width="5" stroke-linecap="round">
					<path class="s1" d="M330 890c-12-18 12-26 0-48" />
					<path class="s2" d="M408 880c-12-18 12-26 0-48" />
					<path class="s3" d="M456 900c-10-16 10-24 0-42" />
				</g>
				<rect x="540" y="870" width="96" height="92" rx="16" fill="var(--glass)" opacity=".75" />
				<rect x="546" y="850" width="84" height="22" rx="5" fill="var(--metal)" />
				<g fill="var(--pickle)">
					<rect x="554" y="888" width="22" height="58" rx="11" transform="rotate(-8 565 917)" />
					<rect x="582" y="884" width="22" height="62" rx="11" />
					<rect x="608" y="889" width="22" height="56" rx="11" transform="rotate(9 619 917)" />
				</g>
				<g fill="#fffbe8"><circle cx="572" cy="950" r="7" /><circle cx="598" cy="952" r="7" /></g>

				<!-- the raclette grill -->
				<rect x="680" y="900" width="40" height="22" rx="5" fill="var(--metal2)" />
				<rect x="1230" y="900" width="40" height="22" rx="5" fill="var(--metal2)" />
				<rect x="640" y="808" width="650" height="98" rx="16" fill="var(--metal2)" />
				<rect x="668" y="826" width="594" height="74" rx="10" fill="#1b1512" />
				<rect class="coil" x="682" y="830" width="566" height="8" rx="4" fill="var(--ember)" />
				<rect x="620" y="774" width="690" height="34" rx="17" fill="var(--metal)" />
				<ellipse cx="716" cy="768" rx="40" ry="11" fill="#b4543a" />
				<ellipse cx="808" cy="768" rx="40" ry="11" fill="#b4543a" />
				<path d="M1010 766c10-26 44-26 54 0z" fill="#d9d2c4" />
				<rect x="1030" y="764" width="12" height="10" fill="#d9d2c4" />
				<circle cx="1160" cy="766" r="12" fill="#d23" />
				<circle cx="1196" cy="768" r="10" fill="#6aa83a" />

				{#each PANS.slice(0, 3) as x (x)}
					{@render pan(x)}
				{/each}
				{#key pulse?.at}
					<g class="fall" class:go={kind === 'down'}>{@render pan(PANS[3])}</g>
					{#if kind === 'down'}
						<path class="splat" d="M1215 968c-26-12-42 4-58-6c-16 16-8 26 10 28c-16 12 4 24 24 16c4 16 28 16 34 0c20 4 28-16 12-24c20-8 12-24-22-14z" fill="var(--cheese-yellow)" stroke="#c9961f" stroke-width="3" />
						<g class="ouch" stroke="var(--ink)" stroke-width="5" stroke-linecap="round">
							<path d="M1150 900l-16-22M1200 890l8-26M1260 900l22-18" />
						</g>
					{/if}
					{#if kind === 'up'}
						<g transform="translate({PANS[1]} {PAN_Y - 6})">
							<ellipse class="plop" cx="0" cy="16" rx="50" ry="9" stroke="var(--cheese-hi)" stroke-width="3" />
							<g class="hand">
								<rect x="24" y="-16" width="320" height="38" rx="12" fill="var(--sleeve)" />
								<rect x="24" y="-16" width="26" height="38" fill="var(--cloth-w)" opacity=".75" />
								<ellipse cx="12" cy="4" rx="28" ry="19" fill="var(--skin)" />
								<ellipse cx="-12" cy="-6" rx="16" ry="8" fill="var(--skin)" />
								<g class="slice">
									<path d="M-40 -28H36L28 -12H-32Z" fill="var(--cheese-yellow)" stroke="#c9961f" stroke-width="3" />
									<circle cx="-12" cy="-20" r="3.2" fill="#d9a62b" />
									<circle cx="14" cy="-21" r="2.4" fill="#d9a62b" />
								</g>
							</g>
						</g>
					{/if}
				{/key}

				<!-- smoke: thin wisps when it starts to smoke, a cloud when it burns -->
				<g class="smoke-thin" stroke="var(--smoke)" stroke-width="9" stroke-linecap="round">
					{#each [0, 1, 2, 3] as i (i)}
						<path class={`s${i}`} d="M{PANS[i]} 840c-24-34 24-56 0-98" />
					{/each}
				</g>
				<g class="smoke-big" fill="var(--smoke)">
					<circle class="p1" cx="980" cy="770" r="50" />
					<circle class="p2" cx="1030" cy="680" r="64" />
					<circle class="p3" cx="960" cy="580" r="76" />
					<circle class="p4" cx="1040" cy="470" r="86" />
					<circle class="p5" cx="970" cy="360" r="92" />
					<circle class="p6" cx="1040" cy="250" r="84" />
					<circle class="p7" cx="990" cy="150" r="76" />
				</g>
				<rect class="tint" width={STAGE_W} height={STAGE_H} fill="#e11" />

				<!-- right: the bottle and two glasses of fendant -->
				<path d="M1690 960V880C1690 856 1706 850 1706 820V776H1738V820C1738 850 1754 856 1754 880V960Z" fill="var(--bottle)" />
				<rect x="1698" y="888" width="48" height="46" fill="var(--cloth-r)" />
				<path d="M1716 900h12v-10h4v10h12v4h-12v12h-4v-12h-12z" fill="#fff" transform="translate(-4 0)" />
				<rect x="1703" y="762" width="40" height="18" rx="4" fill="#d6b04a" />
				{#key pulse?.at}
					<g class="gL" class:clink={kind === 'resolved'}>{@render glass(GLASSES[0])}</g>
					<g class="gR" class:clink={kind === 'resolved'}>{@render glass(GLASSES[1])}</g>
					{#if kind === 'resolved'}
						<g class="sparks" fill="#ffd84a" stroke="var(--ink)" stroke-width="1.5">
							<path d="M1496 856l5-16l5 16l16 5l-16 5l-5 16l-5-16l-16-5z" />
							<path d="M1460 826l4-10l4 10l10 4l-10 4l-4 10l-4-10l-10-4z" />
							<path d="M1536 816l4-10l4 10l10 4l-10 4l-4 10l-4-10l-10-4z" />
						</g>
						<text class="sante" x="1496" y="780" text-anchor="middle" font-size="46" font-weight="800" font-family="system-ui, sans-serif" fill="var(--ink)" stroke="var(--wall1)" stroke-width="9" paint-order="stroke">Santé !</text>
					{/if}
				{/key}
			</svg>
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
		background: var(--wall1);
	}
	.stage { position: absolute; left: 0; top: 0; width: 1920px; height: 1080px; transform-origin: 0 0; }
	svg { display: block; }

	.clouds, .heavy, .smoke-thin, .smoke-big, .bolt, .flash, .storm-shade, .tint, .halo, .beeps, .char { opacity: 0; }
	.clouds { opacity: 0; }
	.wnd, .wnd-round { fill: #6b5a48; opacity: .7; }
	.lit .wnd, .lit .wnd-round { fill: #ffdc7a; opacity: 1; }
	.l-warn .clouds, .l-critical .clouds { opacity: .9; }
	.l-warn .heavy { display: none; }
	.l-critical .heavy { opacity: 1; }
	.l-warn .smoke-thin { opacity: 1; }
	.l-critical .smoke-big { opacity: .88; }
	.l-critical .char { opacity: .85; }
	.l-critical .storm-shade { opacity: .45; }
	.l-critical .tint { opacity: .07; }
	.l-critical .halo { opacity: .35; }
	.l-critical .beeps { opacity: .9; }
	.l-critical .coil { fill: #ffb347; }
	.l-critical .bubbles { display: none; }
	.heavy { opacity: 0; }
	.flake.extra { display: none; }
	.l-critical .flake.extra { display: inline; }

	/* ambient motion */
	.flake { animation: snow var(--d) linear infinite; }
	.l-critical .flake { animation-name: blizzard; animation-duration: calc(var(--d) * .3); }
	@keyframes snow { from { transform: translate(0, calc(var(--y0) * -1)); } to { transform: translate(-16px, calc(400px - var(--y0))); } }
	@keyframes blizzard { from { transform: translate(0, calc(var(--y0) * -1)); } to { transform: translate(-200px, calc(400px - var(--y0))); } }
	.cloud-a { animation: drift 70s ease-in-out infinite alternate; }
	.cloud-b { animation: drift 90s ease-in-out infinite alternate-reverse; }
	@keyframes drift { from { transform: translateX(-50px); } to { transform: translateX(80px); } }
	.star { animation: twinkle 5s ease-in-out infinite; }
	@keyframes twinkle { 50% { opacity: .35; } }
	.l-critical .bolt { animation: bolt 6s steps(1) infinite; }
	.l-critical .flash { animation: flash 6s steps(1) infinite; }
	@keyframes bolt { 0%, 100% { opacity: 0; } 60% { opacity: 1; } 62% { opacity: 0; } 65% { opacity: 1; } 68% { opacity: 0; } }
	@keyframes flash { 0%, 100% { opacity: 0; } 60% { opacity: .6; } 62% { opacity: 0; } 65% { opacity: .45; } 68% { opacity: 0; } }
	.l-critical .led, .l-critical .beeps { animation: blink .9s steps(1) infinite; }
	.l-critical .halo { animation: halo .9s ease-in-out infinite; }
	.l-critical .tint { animation: tint .9s ease-in-out infinite; }
	@keyframes blink { 50% { opacity: .12; } }
	@keyframes halo { 50% { opacity: .05; } }
	@keyframes tint { 50% { opacity: .02; } }
	.chirp:not(.l-critical) .led { animation: blink .5s steps(1) 4; fill: #ff2a2a; }
	.chirp:not(.l-critical) .beeps { opacity: .9; animation: blink .5s steps(1) 4; }
	.coil { animation: glow 3.4s ease-in-out infinite; }
	@keyframes glow { 50% { opacity: .75; } }
	.grill-glow { animation: glow 3.4s ease-in-out infinite; }
	.bubbles circle { transform-box: fill-box; transform-origin: center; animation: bubble 2.6s ease-in-out infinite; }
	.bubbles circle:nth-child(2) { animation-delay: -.9s; }
	.bubbles circle:nth-child(3) { animation-delay: -1.7s; }
	@keyframes bubble { 0%, 100% { transform: scale(.4); } 50% { transform: scale(1.1); } }
	.steam path, .smoke-thin path { animation: rise 3.6s ease-in-out infinite; }
	.steam .s2, .smoke-thin .s2 { animation-delay: -1.2s; }
	.steam .s3, .smoke-thin .s3 { animation-delay: -2.4s; }
	.smoke-thin .s1 { animation-delay: -.6s; }
	@keyframes rise { 0% { transform: translateY(10px); opacity: 0; } 40% { opacity: .9; } 100% { transform: translateY(-18px); opacity: 0; } }
	.smoke-big circle { transform-box: fill-box; transform-origin: center; animation: puff 4.4s ease-in-out infinite; }
	.smoke-big .p2 { animation-delay: -.7s; } .smoke-big .p3 { animation-delay: -1.4s; } .smoke-big .p4 { animation-delay: -2.1s; }
	.smoke-big .p5 { animation-delay: -2.8s; } .smoke-big .p6 { animation-delay: -3.4s; } .smoke-big .p7 { animation-delay: -4s; }
	@keyframes puff { 50% { transform: scale(1.12) translate(10px, -10px); } }

	/* gestures */
	.fall { transform-box: fill-box; transform-origin: center; }
	.fall.go { animation: pan-fall 3.6s linear both; }
	@keyframes pan-fall {
		0%, 14% { transform: none; opacity: 1; animation-timing-function: cubic-bezier(.5, 0, .9, .6); }
		42% { transform: translate(92px, 104px) rotate(115deg); animation-timing-function: ease-out; }
		48% { transform: translate(98px, 98px) rotate(100deg); }
		80% { transform: translate(98px, 98px) rotate(100deg); opacity: 1; }
		92% { transform: translate(98px, 98px) rotate(100deg); opacity: 0; }
		93% { transform: none; opacity: 0; }
		100% { transform: none; opacity: 1; }
	}
	.splat { transform-box: fill-box; transform-origin: center bottom; animation: splat 3.6s ease-out both; }
	@keyframes splat { 0%, 40% { transform: scale(0); opacity: 0; } 48% { transform: scale(1.15); opacity: 1; } 55%, 80% { transform: scale(1); opacity: 1; } 95%, 100% { transform: scale(1); opacity: 0; } }
	.ouch { animation: ouch 3.6s ease-out both; }
	@keyframes ouch { 0%, 40% { opacity: 0; } 46%, 60% { opacity: 1; } 70%, 100% { opacity: 0; } }
	.hand { animation: hand 3.4s ease-in-out both; }
	@keyframes hand { 0% { transform: translate(240px, -10px); opacity: 0; } 12% { opacity: 1; } 35% { transform: translate(0, -16px); } 52% { transform: translate(0, 4px); } 70% { transform: translate(0, -16px); opacity: 1; } 100% { transform: translate(260px, -10px); opacity: 0; } }
	.slice { animation: slice 3.4s linear both; }
	@keyframes slice { 0%, 44% { transform: none; opacity: 1; } 54% { transform: translateY(14px); opacity: 1; } 56%, 100% { transform: translateY(14px); opacity: 0; } }
	.plop { transform-box: fill-box; transform-origin: center; animation: plop 3.4s ease-out both; }
	@keyframes plop { 0%, 52% { transform: scale(.2); opacity: 0; } 56% { opacity: 1; } 78%, 100% { transform: scale(1.3); opacity: 0; } }
	.gL, .gR { transform-box: fill-box; transform-origin: center bottom; }
	.gL.clink { animation: clink-l 3.2s ease-in-out both; }
	.gR.clink { animation: clink-r 3.2s ease-in-out both; }
	@keyframes clink-l { 0%, 12% { transform: none; } 40%, 48% { transform: translate(20px, -22px) rotate(14deg); } 44% { transform: translate(24px, -22px) rotate(18deg); } 75%, 100% { transform: none; } }
	@keyframes clink-r { 0%, 12% { transform: none; } 40%, 48% { transform: translate(-20px, -22px) rotate(-14deg); } 44% { transform: translate(-24px, -22px) rotate(-18deg); } 75%, 100% { transform: none; } }
	.sparks, .sante { animation: sparks 3.2s ease-out both; transform-box: fill-box; transform-origin: center; }
	@keyframes sparks { 0%, 38% { opacity: 0; transform: scale(.3); } 46% { opacity: 1; transform: scale(1.2); } 75% { opacity: 1; transform: scale(1); } 100% { opacity: 0; } }

	/* A hidden tab: everything holds its breath. */
	.scene--paused :global(*) { animation-play-state: paused !important; }
	/* Reduced motion: a still picture in the right pose. */
	.scene--still :global(*) { animation: none !important; transition: none !important; }
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) { animation: none !important; transition: none !important; }
	}
</style>
