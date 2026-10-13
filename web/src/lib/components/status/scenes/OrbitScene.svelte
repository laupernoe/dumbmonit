<script lang="ts">
	/**
	 * Space station: the Earth's limb below, day side on the left, city lights
	 * on the night side, a station truss and small satellites (stand-ins for
	 * devices). Space weather follows `mood`: calm, drifting debris (warn), a
	 * solar flare with an aurora (critical). `pulse` replays the sentinel
	 * satellite's gesture (see wall/orbit/Sentinel.svelte), plus a green ribbon
	 * along the limb when an incident is resolved.
	 */
	import Sentinel from '#lib/components/wall/orbit/Sentinel.svelte';
	import { orbitMood } from '#lib/components/wall/orbit/geometry.js';

	interface Props {
		mood?: string;
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}
	let { mood = 'ok', pulse = null }: Props = $props();

	const level = $derived(orbitMood(mood));

	const cx = 1170;
	const cy = 1260;
	const r = 800;
	const limbY = (x: number) => cy - Math.sqrt(r * r - (x - cx) ** 2);
	const arc = (x1: number, x2: number, grow = 0) =>
		`M${x1} ${limbY(x1) - grow}A${r + grow} ${r + grow} 0 0 1 ${x2} ${limbY(x2) - grow}`;

	function rng(seed: number) {
		let a = seed;
		return () => {
			a = (Math.imul(a, 1664525) + 1013904223) >>> 0;
			return a / 4294967296;
		};
	}
	const rand = rng(11);
	const STARS = Array.from({ length: 70 }, () => ({
		x: Math.round(580 + rand() * 1180),
		y: Math.round(90 + rand() * 360),
		r: +(0.7 + rand() * 1.4).toFixed(1)
	}));
	// Cities: a handful of hubs along the night side, each a small cluster.
	const LIGHTS: { x: number; y: number; r: number }[] = [];
	for (const hx of [1290, 1350, 1420, 1490, 1560, 1610, 1330, 1460]) {
		const n = 8 + Math.floor(rand() * 8);
		for (let i = 0; i < n; i++) {
			const x = Math.round(hx + (rand() - 0.5) * 60);
			const y = Math.round(limbY(x) + 16 + rand() * (70 + (hx % 7) * 12));
			if (y < 618) LIGHTS.push({ x, y, r: +(0.9 + rand() * 1.5).toFixed(1) });
		}
	}
	const DEBRIS = [
		{ y: 150, s: 1, dur: 70, delay: -10 },
		{ y: 230, s: 0.7, dur: 95, delay: -60 },
		{ y: 310, s: 1.3, dur: 80, delay: -30 },
		{ y: 380, s: 0.8, dur: 110, delay: -90 },
		{ y: 190, s: 0.6, dur: 120, delay: -45 },
		{ y: 420, s: 1.1, dur: 85, delay: -70 }
	];
	const FLARE_STREAKS = [140, 210, 280, 340, 400, 460];
</script>

<div class="scene scene-orbit" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none">
		<defs>
			<clipPath id="orbs-earth"><circle {cx} {cy} {r} /></clipPath>
			<radialGradient id="orbs-ocean" cx="0.5" cy="0" r="0.5">
				<stop offset="0" stop-color="#2a79c4" /><stop offset="1" stop-color="#0a2a5e" />
			</radialGradient>
			<linearGradient id="orbs-shade" gradientUnits="userSpaceOnUse" x1="1000" y1="0" x2="1320" y2="0">
				<stop offset="0" stop-color="#ff9640" stop-opacity="0" />
				<stop offset=".15" stop-color="#ff9640" stop-opacity=".22" />
				<stop offset=".4" stop-color="#050818" stop-opacity=".6" />
				<stop offset="1" stop-color="#02040e" stop-opacity=".9" />
			</linearGradient>
			<radialGradient id="orbs-sun" cx="0.5" cy="0.5" r="0.5">
				<stop offset="0" stop-color="#fff6d8" />
				<stop offset=".15" stop-color="#ffd98a" stop-opacity=".8" />
				<stop offset=".45" stop-color="#ff9a4a" stop-opacity=".22" />
				<stop offset="1" stop-color="#ff7a30" stop-opacity="0" />
			</radialGradient>
			<linearGradient id="orbs-flare" x1="0" y1="1" x2="0" y2="0">
				<stop offset="0" stop-color="#ff7a30" stop-opacity=".9" /><stop offset="1" stop-color="#ffe0a0" stop-opacity="0" />
			</linearGradient>
			<pattern id="orbs-wing" width="8" height="8" patternUnits="userSpaceOnUse">
				<rect width="8" height="8" fill="var(--orb-panel)" /><path d="M8 0V8M0 8H8" stroke="#0a1230" stroke-width=".9" opacity=".6" />
			</pattern>
		</defs>

		<g fill="#fff">
			{#each STARS as s, i (i)}
				<circle class:tw={i % 4 === 0} cx={s.x} cy={s.y} r={s.r} opacity=".6" style:animation-delay="{-(i % 7)}s" />
			{/each}
		</g>

		<!-- sun beyond the day-side limb -->
		<g class:storm={level === 'critical'}>
			<circle class="glow" cx="590" cy="560" r="330" fill="url(#orbs-sun)" />
			<circle cx="590" cy="560" r="34" fill="#fff4cf" />
			{#if level === 'critical'}
				<path class="flare" d="M610 530C690 440 740 400 800 436C740 440 690 486 660 570z" fill="url(#orbs-flare)" />
				<path class="flare f2" d="M610 590C700 570 790 590 850 650C780 630 700 640 630 640z" fill="url(#orbs-flare)" />
			{/if}
		</g>

		<!-- the planet -->
		<g clip-path="url(#orbs-earth)">
			<circle {cx} {cy} {r} fill="url(#orbs-ocean)" />
			<g fill="#3d7a48">
				<path d="M700 560Q740 520 800 530Q860 520 880 570Q850 620 760 620Q700 610 700 560z" />
				<path d="M930 520Q990 500 1040 530Q1080 560 1050 610Q990 620 950 590Q920 560 930 520z" fill="#9a8a52" />
				<path d="M1130 500Q1190 490 1230 520Q1250 570 1200 600Q1150 590 1120 550z" />
				<path d="M1330 500Q1400 480 1470 510Q1530 540 1500 600Q1420 620 1350 590Q1310 540 1330 500z" />
				<path d="M1560 540Q1620 530 1660 570Q1650 610 1590 612Q1550 590 1560 540z" fill="#9a8a52" />
			</g>
			<g class="clouds" fill="#fff" opacity=".5">
				<ellipse cx="800" cy="585" rx="70" ry="7" /><ellipse cx="1010" cy="540" rx="60" ry="6" /><ellipse cx="1180" cy="580" rx="80" ry="7" /><ellipse cx="1420" cy="545" rx="70" ry="6" />
			</g>
			<rect x="900" y="400" width="500" height="260" fill="url(#orbs-shade)" />
			<rect x="1400" y="400" width="420" height="260" fill="#02040e" opacity=".9" />
			<g fill="#ffd886" opacity=".95">
				{#each LIGHTS as p, i (i)}<circle class:glint={i % 9 === 0} cx={p.x} cy={p.y} r={p.r} />{/each}
			</g>
		</g>
		<path d={arc(660, 1700, 3)} stroke="#6fb4ff" stroke-width="4" opacity=".85" />
		<path d={arc(660, 1700, 11)} stroke="#4a8be0" stroke-width="14" opacity=".15" />
		{#if level === 'critical'}
			<path class="aurora" d={arc(1250, 1680, 9)} stroke="#5dffb0" stroke-width="9" opacity=".5" />
			<path class="aurora a2" d={arc(1250, 1680, 22)} stroke="#a77bff" stroke-width="6" opacity=".35" />
		{/if}
		{#key pulse?.kind === 'resolved' ? pulse.at : 0}
			{#if pulse?.kind === 'resolved'}
				<path class="ribbon" d={arc(680, 1690, 7)} pathLength="1" stroke="#7dffc0" stroke-width="5" />
			{/if}
		{/key}

		<!-- drifting debris (advisories) -->
		{#if level !== 'ok'}
			{#each DEBRIS.slice(0, level === 'critical' ? 6 : 4) as d, i (i)}
				<g transform="translate(0 {d.y}) scale({d.s})">
					<g class="cross" style:animation-duration="{d.dur}s" style:animation-delay="{d.delay}s">
						<path class="rock" d="M-8 -2L-2 -7L7 -4L8 3L1 7L-7 5z" fill="#7c8296" stroke="#c4c9d8" stroke-width=".8" />
					</g>
				</g>
			{/each}
		{/if}

		<!-- passing satellites -->
		<g transform="translate(0 130)">
			<g class="cross slow" style:animation-duration="150s" style:animation-delay="-50s">
				<rect x="-4" y="-2.5" width="8" height="5" rx="1" fill="var(--orb-hull)" />
				<rect x="-16" y="-2" width="11" height="4" fill="var(--orb-panel)" /><rect x="5" y="-2" width="11" height="4" fill="var(--orb-panel)" />
				<circle class="tw" cx="0" cy="-4" r="1.1" fill="#ff6a55" />
			</g>
		</g>
		<g transform="translate(0 460) scale(.7)">
			<g class="cross slow" style:animation-duration="210s" style:animation-delay="-170s">
				<rect x="-4" y="-2.5" width="8" height="5" rx="1" fill="var(--orb-hull)" />
				<rect x="-16" y="-2" width="11" height="4" fill="var(--orb-panel)" /><rect x="5" y="-2" width="11" height="4" fill="var(--orb-panel)" />
			</g>
		</g>

		<!-- the station -->
		<g transform="translate(860 215)">
			<g class="bob">
				<path d="M-150 0H150" stroke="var(--orb-hull)" stroke-width="4" />
				{#each [-125, -88, 88, 125] as x (x)}
					<g transform="translate({x} 0)"><rect x="-15" y="-62" width="30" height="56" fill="url(#orbs-wing)" /><rect x="-15" y="6" width="30" height="56" fill="url(#orbs-wing)" /></g>
				{/each}
				<rect x="-44" y="-11" width="88" height="22" rx="6" fill="var(--orb-hull)" />
				<rect x="-44" y="-11" width="88" height="6" rx="3" fill="#fff" opacity=".35" />
				<circle cx="-16" cy="1" r="3.5" fill="#0a1230" /><circle cx="0" cy="1" r="3.5" fill="#0a1230" /><circle cx="16" cy="1" r="3.5" fill="#0a1230" />
				<path d="M0-11V-30M-9-30H9" stroke="var(--orb-hull)" stroke-width="2" />
				<circle class="tw" cx="0" cy="-33" r="2.6" fill="#5fd39a" />
			</g>
		</g>

		<!-- sentinel -->
		<g transform="translate(1480 250) scale(1.1)"><Sentinel {pulse} /></g>

		{#if level === 'critical'}
			<g stroke="#ffcf8a" stroke-width="1.4" stroke-linecap="round">
				{#each FLARE_STREAKS as y, i (i)}
					<g transform="translate(0 {y})"><path class="streak" d="M0 0l60 20" style:animation-delay="{-i * 0.7}s" /></g>
				{/each}
			</g>
		{/if}
	</svg>
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(180deg, var(--sky1), var(--sky2));
	}
	.scene svg {
		display: block;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
		--orb-ok: #5fd39a;
		--orb-bad: #ff6a55;
	}
	.scene-orbit {
		--sky1: #0b1230;
		--sky2: #243a78;
		--orb-hull: #e3e8f5;
		--orb-panel: #4a7bd6;
	}
	:global(.dark) .scene-orbit {
		--sky1: #02040d;
		--sky2: #0b1330;
		--orb-hull: #c9d1e6;
		--orb-panel: #3a64b8;
	}
	.tw {
		animation: twinkle 4s ease-in-out infinite;
	}
	.glint {
		animation: twinkle 3.4s ease-in-out infinite;
	}
	.cross {
		animation: cross 80s linear infinite;
	}
	.cross.slow {
		animation-name: cross;
	}
	.rock {
		animation: spin 14s linear infinite;
		transform-box: fill-box;
		transform-origin: center;
	}
	.bob {
		animation: bob 16s ease-in-out infinite;
	}
	.clouds {
		animation: drift 60s ease-in-out infinite alternate;
	}
	.aurora {
		animation: twinkle 6s ease-in-out infinite;
	}
	.aurora.a2 {
		animation-delay: -3s;
	}
	.ribbon {
		stroke-dasharray: 1;
		animation: draw 4.5s ease-in-out both;
	}
	.storm .glow {
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
		animation: streak 3.6s linear infinite;
	}

	@keyframes twinkle {
		50% {
			opacity: 0.25;
		}
	}
	@keyframes cross {
		from {
			transform: translateX(520px);
		}
		to {
			transform: translateX(1840px);
		}
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	@keyframes bob {
		50% {
			transform: translateY(4px) rotate(0.6deg);
		}
	}
	@keyframes drift {
		to {
			transform: translateX(40px);
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
			transform: translateX(560px);
			opacity: 0;
		}
		15%,
		80% {
			opacity: 0.8;
		}
		to {
			transform: translateX(1800px);
			opacity: 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
		}
		.cross {
			transform: translateX(1100px);
		}
		.streak {
			opacity: 0;
		}
	}
</style>
