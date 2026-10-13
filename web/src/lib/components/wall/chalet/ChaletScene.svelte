<script lang="ts">
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The wall's living backdrop: the inside of a mountain chalet. A stone
	 * fireplace crackles, a cat sleeps on a rug, a mug of hot chocolate steams on
	 * a side table, and through the window the snow falls on firs and peaks.
	 * A drop-in for the Paris scene (same props, plus `pulse`).
	 *
	 * Drawn on a 1920 x 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The room follows
	 * the wall's theme; the sky in the window follows the real hour.
	 *
	 * The network's state is the weather outside: calm is a few flakes, clouded
	 * (`warn`) is heavier snow under a grey sky, storm (`critical`) is a blizzard
	 * with lightning, and the cat wakes up. The number of problems is chalked on
	 * a board. A new problem makes the cat jump once. `pulse` plays one gesture
	 * indoors: the fire gutters out (down), roars back with sparks (up), the cat
	 * stretches and the cocoa steam draws a heart (resolved).
	 *
	 * Everything moves with transform and opacity only; nothing but the storm
	 * flashes, softly. Hidden tabs pause, reduced motion freezes a still pose.
	 */
	import { reducedMotion } from '#lib/ui/index.js';
	import { phaseAt, type SceneMood, type SceneTheme } from '#lib/components/wall/paris/daylight.js';
	import {
		STAGE_W,
		STAGE_H,
		GLASS,
		FLOOR_Y,
		RUG,
		FLAKES,
		FIRS,
		PEAKS,
		STARS,
		BOOKS,
		BOOK_COLOURS,
		chaletStyle
	} from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood: SceneMood | 'ok' | 'warn' | 'critical';
		/** Problems on the board: the number chalked on the sign. */
		problems: number;
		/** Bumped by the wall when a new problem appears: the cat jumps once. */
		startle?: number;
		/** The tab is hidden: nothing moves. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
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

	// --- Stage fit ------------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Weather and light ----------------------------------------------------------

	const level = $derived(mood === 'critical' || mood === 'storm' ? 2 : mood === 'warn' || mood === 'clouded' ? 1 : 0);
	// Re-read once a minute: nothing in the sky moves faster than that.
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const phase = $derived(phaseAt(new Date(minuteKey * 60_000)));
	const style = $derived(chaletStyle(theme, phase, level));
	const flakes = $derived(FLAKES.slice(0, level === 0 ? 70 : level === 1 ? 120 : 200));
	const sunny = $derived(phase === 'day' || phase === 'golden');
	const night = $derived(phase === 'night');

	// --- Gestures --------------------------------------------------------------------

	let playing = $state<{ kind: 'down' | 'up' | 'resolved'; id: number } | null>(null);
	$effect(() => {
		const p = pulse;
		if (!p) return;
		playing = { kind: p.kind, id: p.at };
		const timer = setTimeout(() => (playing = null), 5200);
		return () => clearTimeout(timer);
	});

	let startled = $state(false);
	// svelte-ignore state_referenced_locally
	let seenStartle = startle;
	$effect(() => {
		if (startle === seenStartle) return;
		seenStartle = startle;
		startled = true;
		const timer = setTimeout(() => (startled = false), 1600);
		return () => clearTimeout(timer);
	});

	const sign = $derived(
		problems > 0 ? m.wall_scene_sign_problems({ count: problems }).toLocaleUpperCase(getLocale()) : null
	);
	const signSize = $derived(sign ? Math.min(40, Math.round(520 / Math.max(6, sign.length))) : 40);
</script>

<div
	class="scene {className}"
	class:scene--paused={paused}
	class:scene--still={!motion}
	class:p-down={playing?.kind === 'down'}
	class:p-up={playing?.kind === 'up'}
	class:p-resolved={playing?.kind === 'resolved'}
	class:lv0={level === 0}
	class:lv1={level === 1}
	class:lv2={level === 2}
	{style}
	bind:clientWidth={boxW}
	bind:clientHeight={boxH}
	aria-hidden="true"
>
	{#if scale > 0}
		<div class="stage" style:transform="translate({offsetX}px, {offsetY}px) scale({scale})">
			<svg viewBox="0 0 {STAGE_W} {STAGE_H}" width={STAGE_W} height={STAGE_H} fill="none" focusable="false">
				<defs>
					<pattern id="cw-logs" width="80" height="52" patternUnits="userSpaceOnUse">
						<rect width="80" height="52" fill="var(--wall)" />
						<rect y="40" width="80" height="12" fill="var(--wall2)" />
						<path d="M0 1.5H80" stroke="var(--wall3)" stroke-width="3" />
					</pattern>
					<pattern id="cw-stone" width="70" height="40" patternUnits="userSpaceOnUse">
						<rect width="70" height="40" fill="var(--stone)" />
						<path d="M0 1H70M0 21H70M16 1V21M52 21V41" stroke="var(--mortar)" stroke-width="2.4" opacity=".55" />
					</pattern>
					<linearGradient id="cw-sky" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--sky1)" />
						<stop offset="1" stop-color="var(--sky2)" />
					</linearGradient>
					<radialGradient id="cw-glow" cx=".5" cy=".5" r=".5">
						<stop offset="0" stop-color="#ffb35c" stop-opacity=".5" />
						<stop offset=".5" stop-color="#ff9a3c" stop-opacity=".16" />
						<stop offset="1" stop-color="#ff9a3c" stop-opacity="0" />
					</radialGradient>
					<radialGradient id="cw-moon" cx=".5" cy=".5" r=".5">
						<stop offset=".4" stop-color="#eef3fb" stop-opacity=".5" />
						<stop offset="1" stop-color="#eef3fb" stop-opacity="0" />
					</radialGradient>
					<clipPath id="cw-win"><rect x={GLASS.x} y={GLASS.y} width={GLASS.w} height={GLASS.h} /></clipPath>
				</defs>

				<!-- room -->
				<rect width={STAGE_W} height={FLOOR_Y} fill="url(#cw-logs)" />
				<rect width={STAGE_W} height="50" fill="var(--beam)" />
				<rect y={FLOOR_Y} width={STAGE_W} height={STAGE_H - FLOOR_Y} fill="var(--floor)" />
				<path d="M0 920H1920M0 982H1920M0 1044H1920" stroke="var(--floor-line)" stroke-width="3" />
				<rect y={FLOOR_Y - 18} width={STAGE_W} height="20" fill="var(--beam)" />
				<rect width={STAGE_W} height={STAGE_H} fill="#000" opacity="var(--shade)" />

				<!-- bookshelf -->
				<g transform="translate(96 470)">
					<rect x="-12" y="-8" width="220" height="400" fill="var(--beam)" />
					{#each BOOKS as row, r}
						<g transform="translate(0 {r * 126 + 60})">
							{#each row as b}
								<rect x={b.x} y={-b.h} width={b.w} height={b.h} fill={BOOK_COLOURS[b.c]} opacity=".9" />
							{/each}
							<rect x="-12" y="0" width="220" height="10" fill="var(--wall3)" opacity=".55" />
						</g>
					{/each}
				</g>

				<!-- the window onto the night -->
				<g clip-path="url(#cw-win)">
					<rect x={GLASS.x} y={GLASS.y} width={GLASS.w} height={GLASS.h} fill="url(#cw-sky)" />
					<g fill="#fff" opacity="var(--star-op)">
						{#each STARS as s}
							<circle class="star" cx={s.x} cy={s.y} r={s.r} style="animation-delay:{s.delay}s" />
						{/each}
					</g>
					{#if night}
						<circle cx="1230" cy="250" r="64" fill="url(#cw-moon)" />
						<circle cx="1230" cy="250" r="30" fill="#eef3fb" />
						<circle cx="1220" cy="244" r="6" fill="#cdd8ea" opacity=".6" /><circle cx="1240" cy="260" r="4" fill="#cdd8ea" opacity=".6" />
					{:else if sunny}
						<circle cx="1230" cy="250" r="34" fill="#fff6d6" opacity=".85" />
					{/if}
					{#each PEAKS as p}
						<path d={p.d} fill={p.far ? 'var(--far)' : 'var(--near)'} />
						<path d={p.snow} fill="var(--snow)" opacity=".92" />
					{/each}
					<path d="M774 540Q930 490 1060 530T1346 506V616H774Z" fill="var(--snow)" opacity=".85" />
					{#each FIRS as f}
						<rect x={f.trunk.x} y={f.trunk.y} width={f.trunk.w} height={f.trunk.h} fill="#2a1d16" />
						<path d={f.body} fill="var(--fir)" />
						<path d={f.snow} fill="var(--snow)" />
					{/each}
					<path d="M774 584Q900 560 1040 584T1346 574V616H774Z" fill="var(--snow)" />
					<g class="clouds">
						<g class="cloud c1"><ellipse cx="900" cy="230" rx="90" ry="22" /><ellipse cx="950" cy="214" rx="56" ry="22" /></g>
						<g class="cloud c2"><ellipse cx="1230" cy="290" rx="100" ry="24" /><ellipse cx="1180" cy="276" rx="56" ry="20" /></g>
						<g class="cloud c3"><ellipse cx="1060" cy="192" rx="130" ry="28" /><ellipse cx="1130" cy="178" rx="68" ry="24" /></g>
						<g class="cloud c4"><ellipse cx="840" cy="320" rx="80" ry="20" /><ellipse cx="880" cy="308" rx="46" ry="18" /></g>
					</g>
					{#each flakes as f}
						<circle
							class="fl"
							cx={f.x}
							cy={GLASS.y}
							r={f.r}
							fill="var(--snow)"
							style="--d:{f.dur}s;--dl:{f.delay}s;--k:{f.k}"
						/>
					{/each}
					<g class="bolt">
						<rect x={GLASS.x} y={GLASS.y} width={GLASS.w} height={GLASS.h} fill="#fff" />
						<path d="M1090 144L1050 300L1082 296L1030 470L1116 280L1084 284Z" fill="#fff6c8" />
					</g>
					<rect class="dimmer" x={GLASS.x} y={GLASS.y} width={GLASS.w} height={GLASS.h} fill="#0a1226" />
				</g>
				<path d="M728 120H826Q806 300 820 480Q770 492 728 480Z" fill="var(--curtain)" />
				<path d="M1294 120H1392V480Q1350 492 1300 480Q1316 300 1294 120Z" fill="var(--curtain)" />
				<path d="M756 130Q770 300 760 470M1362 130Q1350 300 1364 470" stroke="var(--curtain2)" stroke-width="4" />
				<path d="M728 380Q776 396 824 380" stroke="var(--curtain2)" stroke-width="6" stroke-linecap="round" />
				<path d="M1296 380Q1344 396 1392 380" stroke="var(--curtain2)" stroke-width="6" stroke-linecap="round" />
				<path d="M760 130H1360V144H774V616H1346V144H1360V630H760Z" fill="var(--beam)" fill-rule="evenodd" />
				<rect x="1054" y="144" width="12" height="472" fill="var(--beam)" />
				<rect x="774" y="374" width="572" height="12" fill="var(--beam)" />
				<rect x="738" y="628" width="644" height="26" fill="var(--beam)" />
				<path d="M770 628Q830 606 890 628M1200 628Q1250 612 1310 628" stroke="var(--snow)" stroke-width="8" stroke-linecap="round" opacity=".85" />

				<!-- chalkboard with the number of problems -->
				{#if sign}
					<g transform="translate(330 500) rotate(-2)">
						<line x1="40" y1="0" x2="130" y2="-50" stroke="var(--beam)" stroke-width="3" /><line x1="270" y1="0" x2="180" y2="-50" stroke="var(--beam)" stroke-width="3" />
						<rect x="0" y="0" width="310" height="120" rx="6" fill="#26332f" stroke="var(--beam)" stroke-width="10" />
						<text x="155" y="74" text-anchor="middle" font-family="'Segoe Print', 'Bradley Hand', cursive" font-size={signSize * 0.8} fill="#f1efe6" opacity=".9">{sign}</text>
					</g>
				{/if}

				<!-- fireplace -->
				<rect x="1470" y="50" width="360" height={FLOOR_Y - 50} fill="url(#cw-stone)" />
				<rect x="1420" y="440" width="460" height="32" fill="var(--beam)" />
				<rect x="1440" y={FLOOR_Y - 26} width="420" height="26" fill="var(--stone2)" />
				<path d="M1520 {FLOOR_Y - 26}V700Q1520 580 1650 580Q1780 580 1780 700V{FLOOR_Y - 26}Z" fill="var(--stone2)" />
				<path d="M1546 {FLOOR_Y - 26}V704Q1546 604 1650 604Q1754 604 1754 704V{FLOOR_Y - 26}Z" fill="#241711" />
				<path d="M1410 440Q1520 480 1650 462T1890 440" stroke="#2f6b43" stroke-width="12" stroke-linecap="round" />
				<g fill="#c8342f"><circle cx="1500" cy="470" r="6" /><circle cx="1650" cy="470" r="6" /><circle cx="1800" cy="466" r="6" /></g>
				<path d="M1540 472h34l-5 62q-3 14-12 10t-12-14Z" fill="#b5383a" />
				<path d="M1740 472h34l-5 62q-3 14-12 10t-12-14Z" fill="#3f7a4f" />
				<g transform="translate(1650 {FLOOR_Y - 26}) scale(1.8)">
					<g class="flames">
						<path class="flame f1" d="M-40 0C-60 -28 -40 -44 -36 -70C-18 -50 -10 -30 -18 0Z" fill="#ee6a1f" />
						<path class="flame f2" d="M-18 0C-34 -36 -6 -54 2 -92C26 -60 34 -28 20 0Z" fill="#ee6a1f" />
						<path class="flame f3" d="M16 0C10 -30 30 -40 40 -66C56 -44 58 -20 46 0Z" fill="#ee6a1f" />
						<path class="flame f2" d="M-10 0C-18 -24 0 -36 4 -58C18 -38 22 -18 14 0Z" fill="#ffc04a" />
						<path class="flame f1" d="M-2 0C-6 -16 4 -22 6 -34C14 -22 16 -10 10 0Z" fill="#fff0b0" />
					</g>
					<rect x="-54" y="-12" width="106" height="16" rx="8" fill="#5a3a26" transform="rotate(-6)" />
					<rect x="-50" y="-4" width="100" height="14" rx="7" fill="#6e4a30" transform="rotate(5)" />
					{#key playing?.id}
						{#if playing?.kind === 'up'}
							<g fill="#ffc04a">
								{#each [-44, -28, -12, 4, 20, 36, 52] as dx, i}
									<circle class="spark" cx={dx / 1.6} cy="-80" r="2.4" style="--sx:{dx}px;--sd:{i * 0.12}s" />
								{/each}
							</g>
						{/if}
					{/key}
				</g>
				<g class="glow"><ellipse cx="1650" cy="780" rx="620" ry="420" fill="url(#cw-glow)" /></g>

				<!-- rug and the sleeping cat -->
				<ellipse cx={RUG.cx} cy={RUG.cy} rx={RUG.rx} ry={RUG.ry} fill="var(--rug)" />
				<ellipse cx={RUG.cx} cy={RUG.cy} rx={RUG.rx - 36} ry={RUG.ry - 14} stroke="var(--rug2)" stroke-width="7" />
				<ellipse cx={RUG.cx} cy={RUG.cy} rx={RUG.rx - 100} ry={RUG.ry - 32} stroke="var(--rug2)" stroke-width="4" opacity=".7" />
				<g transform="translate({RUG.cx} {RUG.cy - 10}) scale(2.6)">
					<g class="catwrap" class:startled>
						{#key playing?.id}
							<g class="cat">
								<path d="M48 2Q74 2 66 20Q54 32 0 30" stroke="#8e5726" stroke-width="12" stroke-linecap="round" />
								<ellipse cx="2" cy="0" rx="54" ry="24" fill="#c98543" />
								<path d="M-20 -20q6 12 0 22M0 -23q6 14 0 26M20 -22q6 14 0 24" stroke="#8e5726" stroke-width="4" stroke-linecap="round" />
								<circle cx="-44" cy="-2" r="17" fill="#c98543" />
								<path d="M-58 -13l-2 -17 14 9ZM-35 -19l8 -15 4 17Z" fill="#c98543" />
								<path class="eyes-shut" d="M-54 -3q4 4 8 0M-41 -3q4 4 8 0" stroke="#5a3410" stroke-width="2" stroke-linecap="round" />
								<g class="eyes-open">
									<ellipse cx="-50" cy="-5" rx="3.5" ry="4.5" fill="#d9e36a" /><ellipse cx="-37" cy="-5" rx="3.5" ry="4.5" fill="#d9e36a" />
									<path d="M-50 -8v6M-37 -8v6" stroke="#1a1410" stroke-width="2" />
								</g>
								<path d="M-45 4l-3 3h6Z" fill="#e9a3a0" />
							</g>
						{/key}
					</g>
				</g>

				<!-- hot chocolate on a side table -->
				<g transform="translate(500 770) scale(2)">
					<rect x="-57" y="-8" width="104" height="9" rx="3" fill="var(--table)" />
					<path d="M-45 1V45M35 1V45M-45 22H35" stroke="var(--table)" stroke-width="5" />
					<path d="M21 -34q22 0 22 16t-22 16" stroke="var(--mug)" stroke-width="5" />
					<path d="M-23 -42h46l-4 36q-1 6-8 6h-22q-7 0-8-6Z" fill="var(--mug)" />
					<ellipse cx="0" cy="-41" rx="23" ry="5" fill="#5a3322" />
					<circle cx="-8" cy="-43" r="5" fill="#fff4e6" /><circle cx="7" cy="-42" r="4.5" fill="#fff4e6" />
					<g class="steam" stroke="#eef2f8" stroke-width="3" stroke-linecap="round">
						<path class="s1" d="M-10 -56q-8 -10 0 -20t0 -18" />
						<path class="s2" d="M1 -56q8 -10 0 -20t0 -18" />
						<path class="s3" d="M12 -56q-8 -10 0 -20t0 -18" />
					</g>
					{#key playing?.id}
						{#if playing?.kind === 'resolved'}
							<path class="heart" d="M1 -70c-18 -14 -28 -26 -16 -32 7 -3 13 1 16 7 3 -6 9 -10 16 -7 12 6 2 18 -16 32Z" fill="#eef2f8" />
						{/if}
					{/key}
				</g>
				<rect class="flash" width={STAGE_W} height={STAGE_H} fill="#cfe0ff" />
			</svg>
		</div>
	{/if}
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: var(--wall, #4a3224);
	}
	.stage {
		position: absolute;
		left: 0;
		top: 0;
		width: 1920px;
		height: 1080px;
		transform-origin: 0 0;
		will-change: transform;
	}
	/* weather outside the window */
	.cloud {
		fill: #dfe7f1;
		opacity: 0;
		transition: opacity 1.4s;
		animation: drift 50s ease-in-out infinite alternate;
	}
	.c2 {
		animation-duration: 66s;
		animation-direction: alternate-reverse;
	}
	.c3 {
		animation-duration: 58s;
	}
	.c4 {
		animation-duration: 72s;
	}
	.lv0 .c3 {
		opacity: 0.3;
	}
	.lv1 .cloud {
		opacity: 0.8;
	}
	.lv2 .cloud {
		opacity: 0.92;
		fill: #3b4660;
	}
	.dimmer {
		opacity: var(--dim, 0);
		transition: opacity 1.4s;
	}
	.lv1 {
		--w: -40px;
	}
	.lv0 {
		--w: 16px;
	}
	.lv2 {
		--w: -150px;
	}
	.fl {
		animation: fall var(--d) linear infinite;
		animation-delay: var(--dl);
	}
	@keyframes fall {
		from {
			transform: translate(0, -12px);
		}
		to {
			transform: translate(calc(var(--w) * var(--k)), 490px);
		}
	}
	@keyframes drift {
		to {
			transform: translateX(70px);
		}
	}
	.star {
		animation: twinkle 4s ease-in-out infinite;
	}
	@keyframes twinkle {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.35;
		}
	}
	.bolt,
	.flash {
		opacity: 0;
		pointer-events: none;
	}
	.lv2 .bolt {
		animation: bolt 9s infinite;
	}
	.lv2 .flash {
		animation: roomflash 9s infinite;
	}
	@keyframes bolt {
		0%,
		88%,
		100% {
			opacity: 0;
		}
		89% {
			opacity: 0.85;
		}
		90% {
			opacity: 0.1;
		}
		91.5% {
			opacity: 0.95;
		}
		95% {
			opacity: 0;
		}
	}
	@keyframes roomflash {
		0%,
		88%,
		100% {
			opacity: 0;
		}
		89% {
			opacity: 0.1;
		}
		90% {
			opacity: 0;
		}
		91.5% {
			opacity: 0.12;
		}
		95% {
			opacity: 0;
		}
	}
	/* fire */
	.flame,
	.flames,
	.glow,
	.cat,
	.catwrap,
	.steam path,
	.spark,
	.heart {
		transform-box: fill-box;
		transform-origin: 50% 100%;
	}
	.flames {
		transform-origin: 50% 100%;
	}
	.flame {
		animation: flick 1.1s ease-in-out infinite;
	}
	.f2 {
		animation-duration: 0.85s;
		animation-delay: -0.3s;
	}
	.f3 {
		animation-duration: 1.35s;
		animation-delay: -0.7s;
	}
	.lv2 .flame {
		animation-duration: 0.7s;
	}
	.glow {
		animation: glow 2.2s ease-in-out infinite;
		transform-origin: 50% 50%;
	}
	@keyframes flick {
		0%,
		100% {
			transform: scale(1, 1) skewX(0);
		}
		30% {
			transform: scale(0.93, 1.1) skewX(3deg);
		}
		65% {
			transform: scale(1.06, 0.92) skewX(-3deg);
		}
	}
	@keyframes glow {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.72;
		}
	}
	/* the cat breathes, wakes in a storm, jumps when a problem appears */
	.cat {
		animation: breathe 4.5s ease-in-out infinite;
	}
	@keyframes breathe {
		0%,
		100% {
			transform: scaleY(1);
		}
		50% {
			transform: scaleY(1.04);
		}
	}
	.eyes-open {
		opacity: 0;
	}
	.lv2 .eyes-open,
	.startled .eyes-open {
		opacity: 1;
	}
	.lv2 .eyes-shut,
	.startled .eyes-shut {
		opacity: 0;
	}
	.startled {
		animation: hop 1.2s ease-out;
	}
	@keyframes hop {
		0%,
		100% {
			transform: translateY(0);
		}
		25% {
			transform: translateY(-26px) scale(1.06);
		}
		50% {
			transform: translateY(0);
		}
		70% {
			transform: translateY(-8px);
		}
	}
	.steam path {
		opacity: 0;
		animation: steam 3.6s ease-out infinite;
	}
	.steam .s2 {
		animation-delay: 1.2s;
	}
	.steam .s3 {
		animation-delay: 2.4s;
	}
	@keyframes steam {
		0% {
			opacity: 0;
			transform: translateY(8px);
		}
		30% {
			opacity: 0.7;
		}
		100% {
			opacity: 0;
			transform: translateY(-12px);
		}
	}
	/* pulses */
	.p-down .flame {
		animation: none;
	}
	.p-down .flames {
		animation: gutter 5s ease-in-out both;
	}
	.p-down .glow {
		animation: dim 5s ease-in-out both;
	}
	@keyframes gutter {
		0% {
			transform: scale(1);
		}
		20%,
		72% {
			transform: scale(0.85, 0.07);
		}
		100% {
			transform: scale(1);
		}
	}
	@keyframes dim {
		0%,
		100% {
			opacity: 1;
		}
		20%,
		72% {
			opacity: 0.1;
		}
	}
	.p-up .flames {
		animation: flare 3s ease-out both;
	}
	@keyframes flare {
		0% {
			transform: scale(0.15);
		}
		25% {
			transform: scale(1.3, 1.55);
		}
		100% {
			transform: scale(1);
		}
	}
	.spark {
		opacity: 0;
		animation: spark 1.8s ease-out both;
		animation-delay: var(--sd);
	}
	@keyframes spark {
		0% {
			opacity: 1;
			transform: translate(0, 0);
		}
		100% {
			opacity: 0;
			transform: translate(var(--sx), -90px);
		}
	}
	.p-resolved .cat {
		animation: stretch 4s ease-in-out both;
	}
	@keyframes stretch {
		0%,
		100% {
			transform: scale(1, 1);
		}
		30%,
		65% {
			transform: scale(1.14, 0.9);
		}
	}
	.heart {
		opacity: 0;
		transform-origin: 50% 50%;
		animation: heart 4s ease-out both;
	}
	@keyframes heart {
		0% {
			opacity: 0;
			transform: translateY(8px) scale(0.5);
		}
		30% {
			opacity: 0.85;
			transform: translateY(-6px) scale(1);
		}
		100% {
			opacity: 0;
			transform: translateY(-34px) scale(1.1);
		}
	}
	/* hidden tab: nothing moves */
	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	/* reduced motion: a still picture in the right pose */
	.scene--still :global(*) {
		animation: none !important;
		transition: none !important;
	}
	.scene--still .fl {
		display: none;
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
