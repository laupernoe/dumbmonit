<script lang="ts">
	/**
	 * The wall's living backdrop: a Mars base at the foot of a red dune field.
	 * Glass habitat domes joined by tubes, a greenhouse, a comms mast, a solar
	 * field and a rover that roams the foreground, under a pale small sun,
	 * Earth as a blue dot and the two moons Phobos and Deimos. A drop-in for the
	 * Paris scene (same props) plus `pulse`.
	 *
	 * Drawn on a 1920 x 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` keeps a stage x in view on narrow screens). The sky follows the
	 * real hour inside the brightness band of the wall's theme: butterscotch by
	 * day, BLUE dusk and dawn (as on Mars), lit domes after dark.
	 *
	 * Weather is the network's state: calm = clear, clouded (warn) = dust haze,
	 * storm (critical) = a dust storm with racing sand, flying motes and the
	 * faint flashes of static discharge. Accepts `calm|clouded|storm` or
	 * `ok|warn|critical`.
	 *
	 * `pulse` replays a short gesture whenever `at` changes: down = the domes
	 * black out and the mast beacon blinks red; up = the lamps flicker back on
	 * and radio rings climb towards Earth; resolved = an orange service rover
	 * drives across the foreground flashing green.
	 *
	 * Hidden tabs pause everything; reduced motion freezes a still picture.
	 */
	import { reducedMotion } from '#lib/ui/index.js';
	import {
		phaseAt,
		celestialAt,
		type SceneTheme
	} from '#lib/components/wall/paris/daylight.js';
	import {
		STAGE_W,
		STAGE_H,
		STARS,
		MOTES,
		STREAKS,
		marsStyle,
		weatherOf,
		type MarsMood
	} from './palette.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood?: MarsMood;
		problems?: number;
		startle?: number;
		paused?: boolean;
		focus?: number;
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
		class?: string;
	}

	let {
		theme,
		now,
		mood = 'calm',
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

	const minute = $derived(new Date(Math.floor(now.getTime() / 60_000) * 60_000));
	const phase = $derived(phaseAt(minute));
	const body = $derived(celestialAt(minute));
	const weather = $derived(weatherOf(mood));
	const style = $derived(marsStyle(theme, phase, mood));
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));
	const sunOn = $derived(body.body === 'sun');
	const beacon = $derived(problems > 0 ? '#ff5a4a' : '#5fe08a');
	const kind = $derived(motion && pulse ? pulse.kind : 'none');

	/** Domes: centre x, radius. Base line is y = 800. */
	const BASE = 800;
	const DOMES = [
		{ cx: 880, r: 90 },
		{ cx: 1180, r: 150 },
		{ cx: 1470, r: 110 },
		{ cx: 1710, r: 72, green: true }
	];
	const dome = (cx: number, r: number) => `M${cx - r} ${BASE}A${r} ${r * 0.82} 0 0 1 ${cx + r} ${BASE}z`;
	const WHEELS = [-32, 0, 32];
</script>

{#snippet rover()}
	<rect x="-40" y="-40" width="80" height="22" rx="5" fill="var(--rover)" />
	<rect x="-40" y="-31" width="80" height="4" fill="var(--rover-acc)" />
	<rect x="-30" y="-49" width="50" height="9" rx="2" fill="var(--panel)" />
	<path d="M-14-49v9M2-49v9" stroke="var(--panel-line)" stroke-width="1.2" />
	<path d="M32-40V-66" stroke="var(--hull-2)" stroke-width="3" />
	<rect x="25" y="-74" width="15" height="9" rx="2" fill="var(--rover)" />
	<circle cx="37" cy="-70" r="2" fill="var(--wheel)" />
	<circle cx="42" cy="-28" r="3" fill="var(--lamp)" opacity="var(--lamp-on)" />
	{#each WHEELS as wx (wx)}
		<g transform="translate({wx} -11)">
			<g class="wheel">
				<circle r="11" fill="var(--wheel)" />
				<circle r="4" fill="var(--hull-2)" />
				<path d="M-9 0H9M0-9V9" stroke="var(--hull-2)" stroke-width="1.6" />
			</g>
		</g>
	{/each}
{/snippet}

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
			<div class="sky"></div>

			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}" fill="none" focusable="false">
				<g class="stars" class:stars--on={starsOn}>
					{#each STARS as s (s.x + ':' + s.y)}
						<circle class="star" cx={s.x} cy={s.y} r={s.r} fill="var(--star)" style:animation-delay="{s.d}s" />
					{/each}
				</g>
				{#if sunOn}
					<circle cx={body.x} cy={body.y} r="64" fill="var(--sun-halo)" />
					<circle cx={body.x} cy={body.y} r="24" fill="var(--sun)" />
				{/if}
				<!-- Earth, a blue dot with its own small moon -->
				<circle cx="1590" cy="240" r="22" fill="var(--earth-halo)" />
				<circle cx="1590" cy="240" r="8" fill="var(--earth)" />
				<circle cx="1604" cy="231" r="2" fill="var(--moon)" />
				<!-- Phobos crosses fast, Deimos lingers -->
				<g class="phobos"><ellipse cx="0" cy="0" rx="22" ry="17" fill="var(--moon)" /><circle cx="-6" cy="-3" r="5" fill="var(--far-2)" opacity=".5" /><circle cx="8" cy="5" r="3" fill="var(--far-2)" opacity=".5" /></g>
				<g class="deimos"><circle r="9" fill="var(--moon)" /><circle cx="-2" cy="-2" r="2.5" fill="var(--far-2)" opacity=".5" /></g>
			</svg>

			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}" fill="none" focusable="false">
				<!-- far rims and mesas -->
				<path d="M0 740V672C120 660 200 626 330 628C420 630 470 664 560 664C700 664 760 610 900 612C1040 614 1100 668 1240 664C1400 660 1460 630 1600 632C1720 634 1800 664 1920 656V740z" fill="var(--far)" />
				<path d="M0 740V700C160 690 300 676 460 690C640 704 760 684 960 690C1160 696 1300 680 1500 690C1680 698 1800 686 1920 692V740z" fill="var(--far-2)" />

				<!-- dust devil -->
				<g class="devil">
					<path d="M0 0C-14-40 14-80-4-120C-20-160 20-200 6-250" stroke="var(--dust)" stroke-width="16" stroke-linecap="round" opacity=".28" />
					<path d="M0 0C-8-30 8-60-2-90C-12-120 12-150 4-190" stroke="var(--dust)" stroke-width="30" stroke-linecap="round" opacity=".14" />
				</g>

				<!-- mid dune and the base -->
				<path d="M0 800V752C220 728 420 746 620 760C860 776 1100 760 1400 770C1640 778 1800 764 1920 756V1080H0z" fill="var(--dune-1)" />

				<!-- solar field -->
				<g fill="var(--panel)" stroke="var(--panel-line)" stroke-width="1.5">
					<path d="M460 792l70-26h60l-70 26z" /><path d="M560 792l70-26h60l-70 26z" /><path d="M660 792l70-26h60l-70 26z" />
				</g>

				<!-- comms mast with dish and beacon -->
				<g>
					<path d="M1360 800V610" stroke="var(--hull-2)" stroke-width="6" />
					<path d="M1346 800l14-60 14 60" stroke="var(--hull-2)" stroke-width="3" />
					<path d="M1340 632c4 18 32 18 36 0z" fill="var(--hull)" transform="rotate(-40 1360 632)" />
					<circle cx="1360" cy="602" r="6" fill={beacon} class="beacon" class:beacon--fast={kind === 'down'} />
				</g>

				<!-- tubes between the domes -->
				<g fill="var(--hull)" stroke="var(--hull-2)" stroke-width="2">
					<rect x="950" y="772" width="90" height="26" rx="12" />
					<rect x="1310" y="772" width="70" height="26" rx="12" />
					<rect x="1560" y="776" width="100" height="22" rx="11" />
				</g>

				<!-- domes -->
				{#each DOMES as d (d.cx)}
					<g>
						<rect x={d.cx - d.r * 0.55} y={BASE - d.r * 0.32} width={d.r * 1.1} height={d.r * 0.32} rx="4" fill="var(--hull-2)" />
						{#if d.green}
							<g fill="var(--plant)"><circle cx={d.cx - 26} cy={BASE - 24} r="16" /><circle cx={d.cx + 4} cy={BASE - 34} r="20" /><circle cx={d.cx + 34} cy={BASE - 22} r="14" /></g>
						{/if}
						{#key pulse?.at}
							<g class="lamps p-{kind}" style:animation-delay="{DOMES.indexOf(d) * 0.35}s">
								{#each [-0.32, -0.08, 0.16] as o (o)}
									<rect x={d.cx + o * d.r} y={BASE - d.r * 0.24} width={Math.max(8, d.r * 0.12)} height={d.r * 0.12} rx="2" fill="var(--lamp)" />
								{/each}
							</g>
						{/key}
						<path d={dome(d.cx, d.r)} fill="var(--glass)" stroke="var(--glass-edge)" stroke-width="3" />
						<path d="M{d.cx - d.r * 0.5} {BASE}A{d.r * 0.5} {d.r * 0.82} 0 0 1 {d.cx + d.r * 0.5} {BASE}M{d.cx - d.r * 0.82} {BASE - d.r * 0.3}Q{d.cx} {BASE - d.r * 0.52} {d.cx + d.r * 0.82} {BASE - d.r * 0.3}" stroke="var(--rib)" stroke-width="2" />
						{#key pulse?.at}
							<path class="blackout p-{kind}" d={dome(d.cx, d.r)} fill="#05060c" />
						{/key}
					</g>
				{/each}

				<!-- radio rings towards Earth -->
				{#key pulse?.at}
					{#if kind === 'up'}
						<g class="rings" stroke="var(--earth)" stroke-width="3">
							<circle cx="1372" cy="626" r="14" style:animation-delay="0s" />
							<circle cx="1372" cy="626" r="14" style:animation-delay=".7s" />
							<circle cx="1372" cy="626" r="14" style:animation-delay="1.4s" />
						</g>
					{/if}
				{/key}

				<!-- near dunes, rocks -->
				<path d="M0 860C260 826 520 846 800 858C1100 872 1400 846 1680 850C1800 852 1880 846 1920 840V1080H0z" fill="var(--dune-2)" />
				<path d="M0 960C300 930 620 950 940 964C1260 978 1580 946 1920 944V1080H0z" fill="var(--dune-3)" />
				<g fill="var(--rock)"><ellipse cx="220" cy="990" rx="40" ry="16" /><ellipse cx="270" cy="996" rx="22" ry="10" /><ellipse cx="1560" cy="1010" rx="52" ry="18" /><ellipse cx="1000" cy="900" rx="26" ry="9" /></g>
				<g stroke="var(--dune-2)" stroke-width="3" stroke-linecap="round" opacity=".6"><path d="M120 1030h120M700 1006h150M1300 1046h200M1760 1004h110" /></g>

				<!-- the rover roams -->
				<g transform="translate(0 990) scale(1.7)"><g class="rover">{@render rover()}</g></g>
				<!-- the service rover, on a resolved pulse -->
				{#key pulse?.at}
					{#if kind === 'resolved'}
						<g transform="translate(0 925) scale(1.4)">
							<g class="service">
								{@render rover()}
								<rect x="-40" y="-40" width="80" height="22" rx="5" fill="var(--rover-acc)" opacity=".85" />
								<circle class="green" cx="-30" cy="-54" r="5" fill="#5fe08a" />
							</g>
						</g>
					{/if}
				{/key}
			</svg>

			<!-- weather -->
			<div class="haze haze--{weather}"></div>
			{#if weather !== 'clear'}
				<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}" fill="none" focusable="false">
					<g fill="var(--dust)" opacity=".35">
						<ellipse class="band" cx="400" cy="640" rx="520" ry="26" style:animation-duration="70s" />
						<ellipse class="band" cx="1400" cy="760" rx="620" ry="34" style:animation-duration="95s" style:animation-delay="-40s" />
					</g>
					{#if weather === 'storm'}
						{#each STREAKS as s, i (i)}
							<rect class="streak" x="-400" y={s.y} width={s.w} height="5" rx="2.5" fill="var(--dust-streak)" style:animation-duration="{s.dur}s" style:animation-delay="{s.delay}s" />
						{/each}
						{#each MOTES as p, i (i)}
							<circle class="mote" cx="-20" cy={p.y} r={p.r} fill="var(--dust-streak)" style:animation-duration="{p.dur}s" style:animation-delay="{p.delay}s" />
						{/each}
					{/if}
				</svg>
			{/if}
			{#if weather === 'storm'}
				<div class="flash flash--a"></div>
				<div class="flash flash--b"></div>
			{/if}
		</div>
	{/if}
</div>

<style>
	@property --sky-top {
		syntax: '<color>';
		inherits: true;
		initial-value: #eccaa6;
	}
	@property --sky-bottom {
		syntax: '<color>';
		inherits: true;
		initial-value: #f8e6cf;
	}
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
		contain: strict;
		background: var(--sky-bottom);
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
	.star {
		animation: twinkle 5s ease-in-out infinite;
	}
	@keyframes twinkle {
		50% {
			opacity: 0.35;
		}
	}

	/* moons: at rest they sit at their still pose */
	.phobos {
		transform: translate(1180px, 300px);
		animation: phobos 150s linear infinite;
		animation-delay: -60s;
	}
	.deimos {
		transform: translate(560px, 190px);
		animation: deimos 520s linear infinite;
		animation-delay: -200s;
	}
	@keyframes phobos {
		from {
			transform: translate(-80px, 360px);
		}
		50% {
			transform: translate(1000px, 250px);
		}
		to {
			transform: translate(2000px, 360px);
		}
	}
	@keyframes deimos {
		from {
			transform: translate(2000px, 170px);
		}
		to {
			transform: translate(-80px, 230px);
		}
	}

	.devil {
		transform: translate(420px, 760px);
		transform-box: view-box;
		animation: devil 80s ease-in-out infinite alternate;
	}
	@keyframes devil {
		to {
			transform: translate(980px, 740px);
		}
	}

	.lamps {
		opacity: var(--lamp-on);
	}
	.blackout {
		opacity: 0;
	}
	.beacon {
		animation: blink 3s ease-in-out infinite;
	}
	.beacon--fast {
		animation: blink 0.5s steps(2, jump-none) 8;
	}
	@keyframes blink {
		50% {
			opacity: 0.25;
		}
	}

	/* pulses */
	.lamps.p-down {
		animation: lamps-out 5s ease both;
	}
	.blackout.p-down {
		animation: black-out 5s ease both;
	}
	.lamps.p-up {
		animation: lamps-on 2.4s steps(1, end) both;
	}
	@keyframes lamps-out {
		0%,
		8% {
			opacity: var(--lamp-on);
		}
		12% {
			opacity: 0;
		}
		20% {
			opacity: var(--lamp-on);
		}
		26%,
		80% {
			opacity: 0;
		}
		100% {
			opacity: var(--lamp-on);
		}
	}
	@keyframes black-out {
		0%,
		24% {
			opacity: 0;
		}
		34%,
		80% {
			opacity: 0.5;
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes lamps-on {
		0%,
		10% {
			opacity: 0;
		}
		18% {
			opacity: 1;
		}
		26% {
			opacity: 0.1;
		}
		40% {
			opacity: 1;
		}
		50% {
			opacity: 0.2;
		}
		100% {
			opacity: var(--lamp-on);
		}
	}
	.rings circle {
		transform-box: fill-box;
		transform-origin: center;
		opacity: 0;
		animation: ring 2.2s ease-out 3 both;
	}
	@keyframes ring {
		0% {
			opacity: 0.9;
			transform: translate(0, 0) scale(0.4);
		}
		100% {
			opacity: 0;
			transform: translate(120px, -240px) scale(5);
		}
	}
	.service {
		animation: service 8s linear both;
	}
	@keyframes service {
		from {
			transform: translateX(-200px);
		}
		to {
			transform: translateX(1500px);
		}
	}
	.green {
		animation: blink 0.5s steps(2, jump-none) infinite;
	}

	.rover {
		animation: roam 170s linear infinite;
		animation-delay: -40s;
		transform: translateX(300px);
	}
	@keyframes roam {
		from {
			transform: translateX(-120px);
		}
		to {
			transform: translateX(1200px);
		}
	}
	.wheel {
		animation: spin 3.5s linear infinite;
	}
	.service .wheel {
		animation-duration: 0.9s;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	/* weather */
	.haze {
		position: absolute;
		inset: 0;
		background: var(--dust);
		opacity: 0;
		transition: opacity 3s ease;
	}
	.haze--haze {
		opacity: 0.3;
	}
	/* A real dust storm: the sky goes a rusty brown, not just a warmer tint. */
	.haze--storm {
		background: linear-gradient(to bottom, #5a2e1a, #96532f 70%, #b06a40);
		opacity: 0.68;
	}
	.band {
		animation: drift 80s linear infinite alternate;
	}
	@keyframes drift {
		from {
			transform: translateX(-160px);
		}
		to {
			transform: translateX(160px);
		}
	}
	.streak {
		animation: sweep linear infinite;
	}
	@keyframes sweep {
		to {
			transform: translateX(2700px);
		}
	}
	.mote {
		animation: blow linear infinite;
	}
	@keyframes blow {
		from {
			transform: translate(0, 0);
		}
		to {
			transform: translate(2000px, 120px);
		}
	}
	.flash {
		position: absolute;
		inset: 0;
		background: var(--flash);
		opacity: 0;
	}
	.flash--a {
		animation: flash 11s linear infinite;
	}
	.flash--b {
		animation: flash 17s linear infinite;
		animation-delay: -6s;
		clip-path: inset(0 40% 30% 0);
	}
	@keyframes flash {
		0%,
		90%,
		93%,
		100% {
			opacity: 0;
		}
		91% {
			opacity: 0.55;
		}
		92% {
			opacity: 0.08;
		}
		92.6% {
			opacity: 0.4;
		}
	}

	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	.scene--still :global(*) {
		animation: none !important;
		transition: none !important;
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
