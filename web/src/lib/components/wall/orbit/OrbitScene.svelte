<script lang="ts">
	/**
	 * The wall's living backdrop: the view from an orbiting station. The Earth
	 * turns slowly below with its day/night terminator, cities glowing on the
	 * night side, a few satellites crossing the stars. A drop-in for the Paris
	 * scene (same props).
	 *
	 * Drawn on a 1920 x 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The planet's
	 * turn starts from the real time of day. The network's state is the space
	 * weather: calm space, drifting debris with advisories, a solar storm
	 * (flare, charged particles, aurora, a soft slow glow) when it is down.
	 * `pulse` replays the sentinel satellite's gesture and, on `resolved`, an
	 * aurora ribbon along the limb. Reduced motion freezes the picture;
	 * hidden tabs pause it.
	 */
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';
	import Sentinel from './Sentinel.svelte';
	import {
		STAGE_W,
		STAGE_H,
		EARTH,
		TILE,
		TURN_S,
		SURFACE,
		STARS,
		DEBRIS,
		PASSERS,
		PARTICLES,
		SENTINEL,
		orbitMood,
		orbitStyle
	} from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		/** Wall moods (calm, clouded, storm) or ok, warn, critical. */
		mood?: string;
		/** Problems on the board: the station beacon turns amber. */
		problems?: number;
		/** Bumped by the wall when a new problem appears: the beacon pings once. */
		startle?: number;
		paused?: boolean;
		focus?: number;
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
		class?: string;
	}

	let {
		theme,
		now,
		mood = 'ok',
		problems = 0,
		startle = 0,
		paused = false,
		focus = 0,
		pulse = null,
		class: className = ''
	}: Props = $props();

	const motion = !reducedMotion();

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	const level = $derived(orbitMood(mood));
	const style = $derived(orbitStyle(theme));
	const trouble = $derived(problems > 0);

	// The surface starts where the planet really is: one turn every TURN_S seconds.
	const turnDelay = untrack(() => -((now.getTime() / 1000) % TURN_S));

	const { cx, cy, r } = EARTH;
	const limbY = (x: number) => cy - Math.sqrt(r * r - (x - cx) ** 2);
	const limbArc = (x1: number, x2: number, grow = 0) =>
		`M${x1} ${limbY(x1) - grow}A${r + grow} ${r + grow} 0 0 1 ${x2} ${limbY(x2) - grow}`;
	const LIMB = limbArc(60, 1860, 4);
	const AURORA = limbArc(1000, 1800, 10);
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
			<svg width={STAGE_W} height={STAGE_H} viewBox="0 0 {STAGE_W} {STAGE_H}" fill="none">
				<defs>
					<clipPath id="orb-earth"><circle {cx} {cy} {r} /></clipPath>
					<radialGradient id="orb-ocean" cx="0.5" cy="0" r="0.45">
						<stop offset="0" stop-color="#2a79c4" /><stop offset="1" stop-color="#0a2a5e" />
					</radialGradient>
					<linearGradient id="orb-shade" gradientUnits="userSpaceOnUse" x1="760" y1="0" x2="1320" y2="0">
						<stop offset="0" stop-color="#ff9640" stop-opacity="0" />
						<stop offset=".14" stop-color="#ff9640" stop-opacity=".22" />
						<stop offset=".34" stop-color="#050818" stop-opacity=".6" />
						<stop offset="1" stop-color="#02040e" stop-opacity=".9" />
					</linearGradient>
					<linearGradient id="orb-fade" gradientUnits="userSpaceOnUse" x1="880" y1="0" x2="1180" y2="0">
						<stop offset="0" stop-color="#000" /><stop offset="1" stop-color="#fff" />
					</linearGradient>
					<mask id="orb-night" maskUnits="userSpaceOnUse" x="0" y="0" width={STAGE_W} height={STAGE_H}>
						<rect width={STAGE_W} height={STAGE_H} fill="url(#orb-fade)" />
					</mask>
					<radialGradient id="orb-sun" cx="0.5" cy="0.5" r="0.5">
						<stop offset="0" stop-color="#fff6d8" /><stop offset=".12" stop-color="#ffd98a" stop-opacity=".85" />
						<stop offset=".4" stop-color="#ff9a4a" stop-opacity=".25" /><stop offset="1" stop-color="#ff7a30" stop-opacity="0" />
					</radialGradient>
					<linearGradient id="orb-flare" x1="0" y1="1" x2="0" y2="0">
						<stop offset="0" stop-color="#ff7a30" stop-opacity=".9" /><stop offset="1" stop-color="#ffe0a0" stop-opacity="0" />
					</linearGradient>
					<pattern id="orb-wing" width="7" height="7" patternUnits="userSpaceOnUse">
						<rect width="7" height="7" fill="var(--panel)" /><path d="M7 0V7M0 7H7" stroke="#0a1230" stroke-width=".8" opacity=".6" />
					</pattern>
				</defs>

				<g fill="#fff">
					{#each STARS as s, i (i)}
						<circle class:tw={s.twinkle} cx={s.x} cy={s.y} r={s.r} opacity={s.twinkle ? 1 : 0.55} style:animation-delay="{s.delay}s" style:animation-duration="{s.dur}s" />
					{/each}
				</g>

				<!-- the sun, just beyond the day-side limb; it swells in a storm -->
				<g class="sun" class:sun--storm={level === 'critical'}>
					<circle cx="40" cy="690" r="520" fill="url(#orb-sun)" />
					<circle cx="40" cy="690" r="60" fill="#fff4cf" />
					{#if level === 'critical'}
						<path class="flare" d="M80 650C190 520 250 470 330 520C260 520 200 580 150 700z" fill="url(#orb-flare)" />
						<path class="flare f2" d="M70 740C200 700 320 740 400 820C310 790 200 800 100 800z" fill="url(#orb-flare)" />
					{/if}
				</g>

				<!-- satellites crossing the sky -->
				{#each PASSERS as p, i (i)}
					<g transform="translate(0 {p.y}) scale({p.s})">
						<g class="cross" style:animation-duration="{p.dur}s" style:animation-delay="{p.delay}s">
							<rect x="-5" y="-3" width="10" height="6" rx="1" fill="var(--hull)" />
							<rect x="-20" y="-2.5" width="13" height="5" fill="var(--panel)" /><rect x="7" y="-2.5" width="13" height="5" fill="var(--panel)" />
							<circle class="blink" cx="0" cy="-5" r="1.3" fill="#ff6a55" style:animation-delay="{i * 0.7}s" />
						</g>
					</g>
				{/each}

				<!-- advisories: debris drifts through -->
				{#if level !== 'ok'}
					{#each DEBRIS.slice(0, level === 'critical' ? 16 : 9) as d, i (i)}
						<g transform="translate(0 {d.y}) scale({d.s})">
							<g class="cross" style:animation-duration="{d.dur}s" style:animation-delay="{d.delay}s">
								<path class="rock" d={d.d} fill="#7c8296" stroke="#c4c9d8" stroke-width=".8" style:animation-duration="{d.spin}s" />
							</g>
						</g>
					{/each}
				{/if}

				<!-- the planet -->
				<g clip-path="url(#orb-earth)">
					<circle {cx} {cy} {r} fill="url(#orb-ocean)" />
					<g class="turn" style:animation-delay="{turnDelay}s">
						{#each [0, TILE] as dx (dx)}
							<g transform="translate({dx} 0)">
								{#each SURFACE.lands as l, i (i)}
									<path d={l.d} fill={i % 3 === 0 ? '#9a8a52' : '#3d7a48'} />
								{/each}
							</g>
						{/each}
					</g>
					<g class="turn turn--cloud" style:animation-delay="{turnDelay * 0.6}s" fill="#fff" opacity=".5">
						{#each [0, TILE] as dx (dx)}
							<g transform="translate({dx} 0)">
								{#each SURFACE.clouds as c, i (i)}<ellipse cx={c.cx} cy={c.cy} rx={c.rx} ry={c.ry} />{/each}
							</g>
						{/each}
					</g>
					<rect x="0" y="600" width={STAGE_W} height="500" fill="url(#orb-shade)" />
					<g mask="url(#orb-night)">
						<g class="turn" style:animation-delay="{turnDelay}s" fill="#ffd886">
							{#each [0, TILE] as dx (dx)}
								<g transform="translate({dx} 0)">
									{#each SURFACE.lights as p, i (i)}
										<circle class:glint={p.twinkle} cx={p.x} cy={p.y} r={p.r} />
									{/each}
								</g>
							{/each}
						</g>
					</g>
				</g>
				<path d={LIMB} stroke="#6fb4ff" stroke-width="5" opacity=".85" />
				<path d={limbArc(60, 1860, 14)} stroke="#4a8be0" stroke-width="16" opacity=".16" />
				<path d={limbArc(60, 1860, 30)} stroke="#4a8be0" stroke-width="30" opacity=".07" />

				{#if level === 'critical'}
					<path class="aurora" d={AURORA} stroke="#5dffb0" stroke-width="12" opacity=".5" />
					<path class="aurora a2" d={limbArc(1000, 1800, 26)} stroke="#a77bff" stroke-width="8" opacity=".35" />
				{/if}
				{#key pulse?.kind === 'resolved' ? pulse.at : 0}
					{#if pulse?.kind === 'resolved'}
						<path class="ribbon" d={limbArc(60, 1860, 8)} pathLength="1" stroke="#7dffc0" stroke-width="7" />
					{/if}
				{/key}

				<!-- the station -->
				<g transform="translate(1560 250)">
					<g class="bob">
						<path d="M-250 0H250" stroke="var(--hull)" stroke-width="5" />
						{#each [-210, -150, 150, 210] as x (x)}
							<g transform="translate({x} 0)"><rect x="-26" y="-92" width="52" height="84" fill="url(#orb-wing)" /><rect x="-26" y="8" width="52" height="84" fill="url(#orb-wing)" /><path d="M0-92V92" stroke="var(--hull)" stroke-width="2" /></g>
						{/each}
						<rect x="-66" y="-16" width="132" height="32" rx="8" fill="var(--hull)" />
						<rect x="-66" y="-16" width="132" height="9" rx="4" fill="#fff" opacity=".35" />
						<rect x="-96" y="-9" width="34" height="18" rx="4" fill="var(--hull)" opacity=".85" />
						<rect x="62" y="-9" width="34" height="18" rx="4" fill="var(--hull)" opacity=".85" />
						<circle cx="-24" cy="2" r="5" fill="#0a1230" /><circle cx="0" cy="2" r="5" fill="#0a1230" /><circle cx="24" cy="2" r="5" fill="#0a1230" />
						<path d="M0-16V-44M-14-44H14" stroke="var(--hull)" stroke-width="2.5" />
						<circle class="beacon" class:beacon--warn={trouble} cx="0" cy="-48" r="4" />
						{#key startle}
							{#if startle > 0}<circle class="ping" cx="0" cy="-48" r="4" />{/if}
						{/key}
					</g>
				</g>

				<!-- the sentinel: a device stand-in, home of the pulse gesture -->
				<g transform="translate({SENTINEL.x} {SENTINEL.y}) scale({SENTINEL.scale})">
					<Sentinel {pulse} />
				</g>

				{#if level === 'critical'}
					<g class="particles" stroke="#ffcf8a" stroke-width="1.6" stroke-linecap="round">
						{#each PARTICLES as p, i (i)}
							<g transform="translate(0 {p.y})"><path class="streak" d="M0 0l{p.len} {p.len * 0.32}" style:animation-duration="{p.dur}s" style:animation-delay="{p.delay}s" /></g>
						{/each}
					</g>
					<rect class="surge" width={STAGE_W} height={STAGE_H} fill="#ffb060" />
				{/if}
			</svg>
		</div>
	{/if}
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(180deg, var(--sky1), var(--sky2) 70%);
		--orb-hull: var(--hull);
		--orb-panel: var(--panel);
	}
	.stage {
		position: absolute;
		left: 0;
		top: 0;
		width: 1920px;
		height: 1080px;
		transform-origin: 0 0;
	}
	svg {
		display: block;
		--orb-hull: var(--hull);
		--orb-panel: var(--panel);
		--orb-ok: #5fd39a;
		--orb-bad: #ff6a55;
	}
	.tw {
		animation: twinkle ease-in-out infinite;
	}
	.glint {
		animation: twinkle 3.5s ease-in-out infinite;
	}
	.turn {
		animation: turn 520s linear infinite;
	}
	.turn--cloud {
		animation-duration: 340s;
	}
	.cross {
		animation: cross linear infinite;
	}
	.rock {
		transform-box: fill-box;
		transform-origin: center;
		animation: spin linear infinite;
	}
	.blink {
		animation: twinkle 1.6s ease-in-out infinite;
	}
	.bob {
		animation: bob 18s ease-in-out infinite;
	}
	.beacon {
		fill: #5fd39a;
		animation: twinkle 2.4s ease-in-out infinite;
	}
	.beacon--warn {
		fill: #ffb347;
	}
	.ping {
		fill: none;
		stroke: #ffb347;
		stroke-width: 1.6;
		transform-box: fill-box;
		transform-origin: center;
		animation: ping 2.6s ease-out both;
	}
	.ribbon {
		stroke-dasharray: 1;
		animation: draw 4.5s ease-in-out both;
	}
	.aurora {
		animation: twinkle 6s ease-in-out infinite;
	}
	.aurora.a2 {
		animation-delay: -3s;
	}
	.sun--storm circle:first-child {
		transform-box: fill-box;
		transform-origin: center;
		animation: swell 7s ease-in-out infinite;
	}
	.flare {
		transform-box: fill-box;
		transform-origin: left bottom;
		animation: flare 6s ease-in-out infinite;
	}
	.flare.f2 {
		animation-delay: -3s;
	}
	.streak {
		animation: streak linear infinite;
	}
	.surge {
		opacity: 0;
		pointer-events: none;
		animation: surge 11s ease-in-out infinite;
	}

	@keyframes twinkle {
		50% {
			opacity: 0.25;
		}
	}
	@keyframes turn {
		to {
			transform: translateX(-1800px);
		}
	}
	@keyframes cross {
		from {
			transform: translateX(-140px);
		}
		to {
			transform: translateX(2060px);
		}
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	@keyframes bob {
		50% {
			transform: translateY(6px) rotate(0.6deg);
		}
	}
	@keyframes ping {
		from {
			opacity: 0.9;
			transform: scale(1);
		}
		to {
			opacity: 0;
			transform: scale(7);
		}
	}
	@keyframes draw {
		from {
			stroke-dashoffset: 1;
			opacity: 0.95;
		}
		70% {
			stroke-dashoffset: 0;
			opacity: 0.95;
		}
		to {
			stroke-dashoffset: 0;
			opacity: 0;
		}
	}
	@keyframes swell {
		50% {
			transform: scale(1.25);
		}
	}
	@keyframes flare {
		0%,
		100% {
			transform: scale(0.6);
			opacity: 0.2;
		}
		50% {
			transform: scale(1.1);
			opacity: 0.95;
		}
	}
	@keyframes streak {
		from {
			transform: translateX(-200px);
			opacity: 0;
		}
		15%,
		80% {
			opacity: 0.8;
		}
		to {
			transform: translateX(2100px);
			opacity: 0;
		}
	}
	@keyframes surge {
		0%,
		60%,
		100% {
			opacity: 0;
		}
		66% {
			opacity: 0.13;
		}
		80% {
			opacity: 0.04;
		}
	}

	/* A hidden tab: everything holds its breath. */
	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	/* Reduced motion: a still picture. */
	.scene--still :global(*) {
		animation: none !important;
		transition: none !important;
	}
	.scene--still .cross {
		transform: translateX(900px);
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
		.cross {
			transform: translateX(900px);
		}
	}
</style>
