<script lang="ts">
	/**
	 * Geneva — the Jet d'eau on the lake, Mont-Blanc and the Salève behind, the
	 * Mont-Blanc bridge, a sailing boat, a swan, and the Bains des Pâquis jetty
	 * whose little lighthouse takes the page's status colour (`--scene-tone`,
	 * set by the page) with two DumbMonit pigeons on the deck. The jet climbs
	 * and its spray falls; a boat drifts. Everything stops under
	 * `prefers-reduced-motion`. Follows the scene contract: fills its positioned
	 * parent, the headline zone (top left) is left clear, the palette lives here.
	 */
</script>

<div class="scene scene-geneva" aria-hidden="true">
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

			<!-- the Jet d'eau -->
			<g class="jet">
				<path d="M1291 404L1297 118h6L1309 404z" fill="url(#gs-jet)" />
				<path class="flow" d="M1300 404V118" stroke="var(--cloud)" stroke-width="3" stroke-dasharray="20 16" stroke-linecap="round" opacity=".7" />
				<g class="plume" fill="var(--jet)">
					<ellipse cx="1300" cy="124" rx="34" ry="22" opacity=".5" />
					<ellipse cx="1282" cy="146" rx="22" ry="16" opacity=".4" />
					<ellipse cx="1320" cy="148" rx="24" ry="18" opacity=".4" />
				</g>
				<g fill="var(--jet)">
					<circle class="drop d1" cx="1300" cy="122" r="2.2" />
					<circle class="drop d2" cx="1300" cy="122" r="2" />
					<circle class="drop d3" cx="1300" cy="122" r="2.4" />
					<circle class="drop d4" cx="1300" cy="122" r="1.8" />
					<circle class="drop d5" cx="1300" cy="122" r="2.2" />
					<circle class="drop d6" cx="1300" cy="122" r="2" />
				</g>
			</g>
			<ellipse class="mist" cx="1300" cy="406" rx="52" ry="9" fill="var(--jet)" opacity=".5" />

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

	.jet {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: sway 14s ease-in-out infinite;
	}
	@keyframes sway {
		50% {
			transform: rotate(0.7deg);
		}
	}
	.flow {
		animation: climb 1.6s linear infinite;
	}
	@keyframes climb {
		to {
			stroke-dashoffset: -36;
		}
	}
	.plume {
		transform-box: fill-box;
		transform-origin: 50% 0;
		animation: swell 5s ease-in-out infinite;
	}
	@keyframes swell {
		50% {
			transform: scale(1.08, 1.12);
		}
	}
	.drop {
		opacity: 0;
		animation: spray 3.2s ease-in infinite;
	}
	.d1 { --dx: 40px; --dy: 130px; }
	.d2 { --dx: -44px; --dy: 150px; animation-delay: -0.6s; }
	.d3 { --dx: 70px; --dy: 180px; animation-delay: -1.2s; }
	.d4 { --dx: -64px; --dy: 170px; animation-delay: -1.8s; }
	.d5 { --dx: 22px; --dy: 200px; animation-delay: -2.3s; }
	.d6 { --dx: -26px; --dy: 190px; animation-delay: -2.8s; }
	@keyframes spray {
		0% {
			transform: translate(0, 0);
			opacity: 0.9;
		}
		100% {
			transform: translate(var(--dx), var(--dy));
			opacity: 0;
		}
	}
	.mist {
		animation: breathe 6s ease-in-out infinite;
	}
	@keyframes breathe {
		50% {
			opacity: 0.25;
		}
	}
	.ripple {
		animation: shimmer 6s ease-in-out infinite;
	}
	@keyframes shimmer {
		50% {
			opacity: 0.25;
		}
	}
	.sail {
		transform: translate(980px, 408px);
		animation: cruise 120s linear infinite;
	}
	@keyframes cruise {
		from {
			transform: translate(580px, 408px);
		}
		to {
			transform: translate(1500px, 408px);
		}
	}
	.swan {
		animation: bob 5s ease-in-out infinite;
	}
	@keyframes bob {
		50% {
			transform: translateY(4px) rotate(-1.5deg);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.jet,
		.flow,
		.plume,
		.drop,
		.mist,
		.ripple,
		.sail,
		.swan {
			animation: none;
		}
		.drop {
			opacity: 0.7;
			transform: translate(calc(var(--dx) * 0.5), calc(var(--dy) * 0.4));
		}
	}
</style>
