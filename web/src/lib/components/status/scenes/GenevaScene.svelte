<script lang="ts">
	/**
	 * Geneva — the Jet d'eau on the lake, Mont-Blanc and the Salève behind, the
	 * Mont-Blanc bridge, a sailing boat, a swan, and the Bains des Pâquis jetty
	 * whose little lighthouse takes the page's status colour (`--scene-tone`,
	 * set by the page) with two DumbMonit pigeons on the deck. The jet is a thin
	 * column crowned with feathery spray, arcs of falling water and a mist cloud
	 * on the lake. Follows the scene contract: fills its positioned parent, the
	 * headline zone (top left) is left clear, the palette lives here.
	 *
	 * `mood` is the weather (clear, clouded, storm with rain and lightning).
	 * `pulse` plays one gesture when `at` changes: on `down` the jet switches
	 * off and collapses into the lake, on `up` it shoots back up past its usual
	 * height, on `resolved` it flares higher under a rainbow. Everything stops
	 * under `prefers-reduced-motion`.
	 */
	import SceneWeather from './SceneWeather.svelte';
	import { createPulse, type SceneMood, type ScenePulse } from './scenePulse.svelte.js';

	interface Props {
		mood?: SceneMood;
		pulse?: ScenePulse;
	}
	let { mood = 'ok', pulse = null }: Props = $props();
	const pl = createPulse(() => pulse, { down: 7000, up: 4500, resolved: 6500 });
</script>

<div
	class="scene scene-geneva"
	class:p-down={pl.active === 'down'}
	class:p-up={pl.active === 'up'}
	class:p-resolved={pl.active === 'resolved'}
	aria-hidden="true"
>
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="gs-lake" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--water1)" />
				<stop offset="1" stop-color="var(--water2)" />
			</linearGradient>
			<linearGradient id="gs-jet" x1="0" y1="1" x2="0" y2="0">
				<stop offset="0" stop-color="var(--jet)" stop-opacity=".95" />
				<stop offset="1" stop-color="var(--jet)" stop-opacity=".55" />
			</linearGradient>
		</defs>
		<g transform="translate(0 80)">
			<circle cx="1000" cy="110" r="44" fill="var(--sun)" opacity=".9" />
			<g fill="var(--cloud)" opacity=".75">
				<ellipse cx="820" cy="120" rx="70" ry="16" />
				<ellipse cx="860" cy="108" rx="46" ry="16" />
				<ellipse cx="1560" cy="70" rx="80" ry="14" />
			</g>

			<!-- Alps with Mont-Blanc, the Salève, the old town -->
			<path d="M560 400V300l70-34 60 26 80-60 70 50 90-40 90 60 70-30 100-80 60 70 80-50 70 40 90-90 50 70 80-30 80 50 40-20V400z" fill="var(--alps)" />
			<path d="M1250 242l-34 38 22-10 18 22 20-18 14 16 18-28-12-8-26-36z" fill="var(--snow)" />
			<path d="M560 400V338c80-4 140-18 230-48l56 12 100-8 62 16 150 30V400z" fill="var(--saleve)" />
			<rect x="560" y="376" width="1220" height="24" fill="var(--town)" />
			<g fill="var(--town)">
				<rect x="640" y="356" width="40" height="22" />
				<rect x="700" y="346" width="34" height="32" />
				<rect x="770" y="352" width="46" height="26" />
				<rect x="850" y="342" width="36" height="36" />
				<rect x="920" y="354" width="50" height="24" />
				<rect x="1000" y="348" width="38" height="30" />
				<rect x="1070" y="358" width="44" height="20" />
				<path d="M780 352l6-44 6 44zM808 352l5-34 5 34z" />
			</g>
			<g class="win" opacity=".9">
				<rect x="656" y="364" width="5" height="6" /><rect x="714" y="358" width="5" height="6" /><rect x="868" y="356" width="5" height="6" /><rect x="1012" y="362" width="5" height="6" /><rect x="934" y="366" width="5" height="6" />
			</g>
			<!-- Mont-Blanc bridge -->
			<rect x="600" y="394" width="520" height="6" fill="var(--bridge)" />
			<g fill="var(--bridge)"><rect x="640" y="400" width="7" height="12" /><rect x="740" y="400" width="7" height="12" /><rect x="840" y="400" width="7" height="12" /><rect x="940" y="400" width="7" height="12" /><rect x="1040" y="400" width="7" height="12" /></g>

			<!-- lake -->
			<rect x="560" y="400" width="1220" height="260" fill="url(#gs-lake)" />
			<g stroke="var(--wh)" stroke-width="2" stroke-linecap="round" opacity=".5">
				<path class="ripple" d="M640 430h60M780 446h90M960 424h70M1100 462h110M700 486h70M860 500h110M1500 440h80M1620 470h90" />
			</g>
			<path d="M1292 424h16l-4 120h-8z" fill="var(--jet)" opacity=".16" />

			<!-- the Jet d'eau: a thin rising column, a feathered crown, arcs of falling spray, a mist cloud -->
			<g class="jetwrap">
				<g class="jet">
					<path d="M1288 404L1296 118h8L1312 404z" fill="var(--jet)" opacity=".18" />
					<path d="M1295 404L1298 122h4L1305 404z" fill="url(#gs-jet)" />
					<path class="flow" d="M1300 404V122" stroke="var(--cloud)" stroke-width="2" stroke-dasharray="14 18" stroke-linecap="round" opacity=".7" />
					<g stroke="var(--jet)" stroke-linecap="round" fill="none">
						<path class="arc" d="M1300 124C1322 108 1352 170 1368 404" stroke-width="3" opacity=".6" />
						<path class="arc a2" d="M1300 124C1278 108 1248 170 1232 404" stroke-width="3" opacity=".6" />
						<path class="arc a3" d="M1300 122C1330 96 1384 190 1408 404" stroke-width="2" opacity=".45" />
						<path class="arc a4" d="M1300 122C1270 96 1216 190 1192 404" stroke-width="2" opacity=".45" />
						<path class="arc a5" d="M1300 126C1312 112 1330 150 1338 404" stroke-width="2" opacity=".5" />
						<path class="arc a6" d="M1300 126C1288 112 1270 150 1262 404" stroke-width="2" opacity=".5" />
					</g>
					<g class="plume" fill="var(--jet)">
						<ellipse cx="1300" cy="122" rx="30" ry="18" opacity=".4" />
						<ellipse cx="1278" cy="142" rx="20" ry="14" opacity=".3" />
						<ellipse cx="1324" cy="144" rx="22" ry="15" opacity=".3" />
						<ellipse cx="1300" cy="160" rx="38" ry="22" opacity=".14" />
					</g>
					<g fill="var(--jet)">
						<circle class="drop d1" cx="1300" cy="122" r="2.2" />
						<circle class="drop d2" cx="1300" cy="122" r="2" />
						<circle class="drop d3" cx="1300" cy="122" r="2.4" />
						<circle class="drop d4" cx="1300" cy="122" r="1.8" />
						<circle class="drop d5" cx="1300" cy="122" r="2.2" />
						<circle class="drop d6" cx="1300" cy="122" r="2" />
						<circle class="drop d7" cx="1300" cy="122" r="1.6" />
						<circle class="drop d8" cx="1300" cy="122" r="1.8" />
						<circle class="drop d9" cx="1300" cy="122" r="1.5" />
						<circle class="drop d10" cx="1300" cy="122" r="1.7" />
					</g>
				</g>
			</g>
			<g class="base" fill="var(--jet)">
				<ellipse class="mist" cx="1300" cy="404" rx="64" ry="11" opacity=".5" />
				<ellipse class="mist m2" cx="1270" cy="398" rx="40" ry="14" opacity=".3" />
				<ellipse class="mist m3" cx="1334" cy="396" rx="44" ry="16" opacity=".3" />
				<ellipse class="mist m4" cx="1300" cy="386" rx="70" ry="22" opacity=".14" />
			</g>
			<g class="splash" stroke="var(--jet)" stroke-width="1.6" fill="none">
				<ellipse class="ring" cx="1232" cy="406" rx="12" ry="3" />
				<ellipse class="ring r2" cx="1368" cy="406" rx="12" ry="3" />
				<ellipse class="ring r3" cx="1192" cy="408" rx="10" ry="3" />
				<ellipse class="ring r4" cx="1408" cy="408" rx="10" ry="3" />
			</g>
			<g class="rainbow" fill="none" stroke-width="5" opacity="0">
				<path d="M1130 404A170 170 0 0 1 1470 404" stroke="#e4574f" />
				<path d="M1140 404A160 160 0 0 1 1460 404" stroke="#f2a43a" />
				<path d="M1150 404A150 150 0 0 1 1450 404" stroke="#efd94e" />
				<path d="M1160 404A140 140 0 0 1 1440 404" stroke="#5fb36a" />
				<path d="M1170 404A130 130 0 0 1 1430 404" stroke="#4f8fd6" />
			</g>

			<!-- sailing boat and swan -->
			<g class="sail">
				<path d="M60 8L60 70L16 70Z" fill="var(--cloud)" />
				<path d="M64 22L64 70L98 70Z" fill="var(--cloud)" opacity=".8" />
				<path d="M8 74h104l-12 12H20z" fill="var(--hull)" />
			</g>
			<g transform="translate(760 500)"><g class="swan">
				<path d="M0 22c0-10 22-10 46-10 16 0 30 0 30 10 0 10-22 14-38 14S0 32 0 22z" fill="var(--wh)" />
				<path d="M62 16c-8-6-5-24 3-27 5-2 8 1 8 5" stroke="var(--wh)" stroke-width="6" stroke-linecap="round" />
				<path d="M74-8l8 2-8 3z" fill="var(--beak)" />
			</g></g>

			<!-- Bains des Pâquis jetty with the lighthouse -->
			<rect x="1380" y="540" width="400" height="14" fill="var(--deck)" />
			<g fill="var(--dark)" opacity=".85"><rect x="1400" y="554" width="8" height="30" /><rect x="1450" y="554" width="8" height="30" /><rect x="1500" y="554" width="8" height="30" /><rect x="1550" y="554" width="8" height="30" /><rect x="1600" y="554" width="8" height="30" /><rect x="1650" y="554" width="8" height="30" /><rect x="1700" y="554" width="8" height="30" /></g>
			<rect x="1388" y="486" width="14" height="54" fill="var(--wh)" />
			<path d="M1384 486l11-18 11 18z" fill="var(--hull)" />
			<circle cx="1395" cy="476" r="8" fill="var(--scene-tone, var(--c-signal))" />
			<circle cx="1395" cy="476" r="18" fill="var(--scene-tone, var(--c-signal))" opacity=".25" />
			<rect x="1640" y="494" width="100" height="46" fill="var(--town)" />
			<path d="M1632 494h116l-14-18h-88z" fill="var(--hull)" />
			<g class="win"><rect x="1652" y="506" width="14" height="18" /><rect x="1680" y="506" width="14" height="18" /><rect x="1708" y="506" width="14" height="18" /></g>
			<use href="#dm-mascot" transform="translate(1500 510) scale(.1)" /><use href="#dm-mascot" transform="translate(1544 510) scale(.1)" />
		</g>
		<SceneWeather {mood} />
	</svg>
</div>

<style>
	.scene-geneva {
		--sky1: #bfdcf0;
		--sky2: #eef6fb;
		--sun: #fff0b8;
		--water1: #7fb6cf;
		--water2: #4f8cab;
		--alps: #b6c8dc;
		--saleve: #94a9bf;
		--snow: #fff;
		--town: #e6dccb;
		--bridge: #8f8a85;
		--jet: #fff;
		--wh: #fff;
		--hull: #c0453a;
		--deck: #b69b78;
		--dark: #3a3f52;
		--beak: #e97b3a;
		--win: #a9b3c4;
		--cloud: #fff;
	}
	:global(.dark) .scene-geneva {
		--sky1: #0a1530;
		--sky2: #1a2a4c;
		--sun: #e8eef8;
		--water1: #14284a;
		--water2: #08122a;
		--alps: #2c3a5c;
		--saleve: #222e4c;
		--snow: #8d9bbb;
		--town: #39425d;
		--bridge: #4d5671;
		--jet: #ffe9b8;
		--wh: #8b92a8;
		--hull: #7a3a44;
		--deck: #4a4538;
		--dark: #0a0f1d;
		--beak: #c9783c;
		--win: #f5c76a;
		--cloud: #2a3658;
	}
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(
			180deg,
			var(--sky1),
			var(--sky2) 73.5%,
			var(--water1) 74.4%,
			var(--water1) 80%,
			var(--water2)
		);
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
	}
	.win {
		fill: var(--win);
	}

	.jetwrap,
	.jet,
	.base {
		transform-box: fill-box;
		transform-origin: 50% 100%;
	}
	.plume {
		transform-box: fill-box;
		transform-origin: 50% 0;
	}
	.ring {
		transform-box: fill-box;
		transform-origin: center;
		opacity: 0;
	}
	.rainbow {
		opacity: 0;
	}
	.drop {
		opacity: 0;
	}
	.arc {
		stroke-dasharray: 22 30;
	}
	.d1 { --dx: 40px; --dy: 130px; }
	.d2 { --dx: -44px; --dy: 150px; animation-delay: -0.6s; }
	.d3 { --dx: 70px; --dy: 180px; animation-delay: -1.2s; }
	.d4 { --dx: -64px; --dy: 170px; animation-delay: -1.8s; }
	.d5 { --dx: 22px; --dy: 200px; animation-delay: -2.3s; }
	.d6 { --dx: -26px; --dy: 190px; animation-delay: -2.8s; }
	.d7 { --dx: 96px; --dy: 220px; animation-delay: -0.3s; }
	.d8 { --dx: -92px; --dy: 230px; animation-delay: -1.5s; }
	.d9 { --dx: 10px; --dy: 120px; animation-delay: -2.1s; }
	.d10 { --dx: -12px; --dy: 140px; animation-delay: -0.9s; }
	.a2 { animation-delay: -0.5s; }
	.a3 { animation-delay: -1.1s; }
	.a4 { animation-delay: -0.2s; }
	.a5 { animation-delay: -0.8s; }
	.a6 { animation-delay: -1.4s; }
	.m2 { animation-delay: -1.5s; }
	.m3 { animation-delay: -3s; }
	.m4 { animation-delay: -4.5s; }
	.r2 { animation-delay: -0.7s; }
	.r3 { animation-delay: -1.3s; }
	.r4 { animation-delay: -1.9s; }
	.sail {
		transform: translate(980px, 408px);
	}

	@media (prefers-reduced-motion: no-preference) {
		.jet { animation: sway 14s ease-in-out infinite; }
		.flow { animation: climb 1.6s linear infinite; }
		.plume { animation: swell 3.2s ease-in-out infinite; }
		.drop { animation: spray 3.2s ease-in infinite; }
		.arc { animation: fall 1.8s linear infinite; }
		.mist { animation: breathe 6s ease-in-out infinite; }
		.ring { animation: ring 2.4s ease-out infinite; }
		.ripple { animation: shimmer 6s ease-in-out infinite; }
		.sail { animation: cruise 120s linear infinite; }
		.swan { animation: bob 5s ease-in-out infinite; }
		.p-down .jetwrap { animation: jet-off 7s cubic-bezier(0.4, 0, 0.2, 1) both; }
		.p-down .base { animation: base-off 7s ease-in-out both; }
		.p-down .ring { animation: none; }
		.p-up .jetwrap { animation: jet-on 3.2s cubic-bezier(0.2, 0.9, 0.3, 1) both; }
		.p-up .base { animation: base-on 3.2s ease-out both; }
		.p-resolved .jetwrap { animation: jet-flare 6s ease-in-out both; }
		.p-resolved .rainbow { animation: rainbow 6.5s ease-in-out both; }
	}
	@media (prefers-reduced-motion: reduce) {
		.drop {
			opacity: 0.7;
			transform: translate(calc(var(--dx) * 0.5), calc(var(--dy) * 0.4));
		}
	}
	@keyframes sway {
		50% { transform: rotate(0.7deg); }
	}
	@keyframes climb {
		to { stroke-dashoffset: -32; }
	}
	@keyframes swell {
		50% { transform: scale(1.12, 1.14); }
	}
	@keyframes fall {
		to { stroke-dashoffset: -104; }
	}
	@keyframes ring {
		0% { transform: scale(0.3); opacity: 0.8; }
		100% { transform: scale(2.4); opacity: 0; }
	}
	@keyframes spray {
		0% { transform: translate(0, 0); opacity: 0.9; }
		100% { transform: translate(var(--dx), var(--dy)); opacity: 0; }
	}
	@keyframes breathe {
		50% { opacity: 0.25; }
	}
	@keyframes shimmer {
		50% { opacity: 0.25; }
	}
	@keyframes cruise {
		from { transform: translate(580px, 408px); }
		to { transform: translate(1500px, 408px); }
	}
	@keyframes bob {
		50% { transform: translateY(4px) rotate(-1.5deg); }
	}
	@keyframes jet-off {
		0% { transform: scaleY(1); opacity: 1; }
		10% { transform: scaleY(0.78); }
		14% { transform: scaleY(0.9); }
		30% { transform: scaleY(0.04); opacity: 0.5; }
		78% { transform: scaleY(0.02); opacity: 0; }
		100% { transform: scaleY(1); opacity: 1; }
	}
	@keyframes base-off {
		0%, 12% { opacity: 1; }
		35%, 80% { opacity: 0.08; }
		100% { opacity: 1; }
	}
	@keyframes jet-on {
		0% { transform: scaleY(0.02); opacity: 0.6; }
		45% { transform: scaleY(1.3); opacity: 1; }
		65% { transform: scaleY(0.94); }
		100% { transform: scaleY(1); }
	}
	@keyframes base-on {
		0% { opacity: 0.1; transform: scale(0.4, 0.6); }
		40% { opacity: 1; transform: scale(1.4, 1.5); }
		100% { opacity: 1; transform: scale(1); }
	}
	@keyframes jet-flare {
		0%, 100% { transform: scaleY(1); }
		25%, 70% { transform: scaleY(1.22); }
	}
	@keyframes rainbow {
		0% { opacity: 0; }
		25%, 70% { opacity: 0.6; }
		100% { opacity: 0; }
	}
</style>
