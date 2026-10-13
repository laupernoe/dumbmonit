<script lang="ts">
	/**
	 * Raclette night — a chalet table under a red-and-white checked cloth, a
	 * raclette grill with three little poêlons of cheese, potatoes, pickles and
	 * two glasses of fendant, a smoke alarm on the ceiling and a snowy Swiss
	 * village in the window. Cartoon, on purpose.
	 *
	 * Fills its positioned parent, the headline zone (top left) is left clear,
	 * the palette lives here with a light and a dark set.
	 *
	 * `mood` is the weather: ok = the cheese melts gently and the sky is clear,
	 * warn = clouds, the cheese starts to smoke, critical = the cheese is black,
	 * a smoke cloud fills the room, the alarm blinks red and a blizzard rages.
	 * `pulse` replays a short gesture whenever `at` changes: down = a poêlon
	 * falls off the grill with a cheese splat, up = a hand puts a fresh slice
	 * in, resolved = two glasses clink, "Santé !". Reduced motion: still picture.
	 */
	import { reducedMotion } from '#lib/ui/index.js';

	interface Props {
		mood?: 'ok' | 'warn' | 'critical';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}

	let { mood = 'ok', pulse = null }: Props = $props();

	const motion = !reducedMotion();
	const kind = $derived(motion && pulse ? pulse.kind : null);

	// A deterministic snowfall inside the window (x 1412..1668, 24 flakes).
	const flakes = Array.from({ length: 24 }, (_, i) => ({
		x: 1414 + ((i * 53) % 252),
		y0: (i * 37) % 170,
		r: 1.2 + (i % 3) * 0.5,
		d: 4 + (i % 5) * 0.9,
		delay: -((i * 0.83) % 6),
		extra: i >= 12
	}));
</script>

{#snippet pan(x: number)}
	<g transform="translate({x} 448)">
		<g class="pan">
			<rect x="30" y="3" width="26" height="5" rx="2.5" fill="var(--wood)" />
			<path d="M-34 0H34L28 22H-28Z" fill="var(--pan)" />
			<path d="M-34 0H34L33 4H-33Z" fill="var(--metal)" />
			<ellipse cx="0" cy="1" rx="31" ry="5.5" fill="var(--cheese)" />
			<path d="M-31 1C-31 8 -28 12 -26 8V3ZM22 3C22 11 26 14 28 8L31 1Z" fill="var(--cheese)" />
			<ellipse cx="-9" cy="-0.5" rx="9" ry="2" fill="var(--cheese-hi)" />
			<g class="bubbles" fill="var(--cheese-hi)">
				<circle cx="10" cy="0" r="2.4" />
				<circle cx="-18" cy="2" r="1.8" />
				<circle cx="20" cy="2" r="1.4" />
			</g>
			<g class="char" fill="#0d0907">
				<ellipse cx="-8" cy="1" rx="10" ry="2.6" />
				<ellipse cx="15" cy="2" rx="8" ry="2.2" />
			</g>
		</g>
	</g>
{/snippet}

{#snippet glass(x: number)}
	<g transform="translate({x} 432)">
		<path d="M-14 0H14C14 17 7 28 0 28C-7 28 -14 17 -14 0Z" fill="var(--glass)" opacity=".7" />
		<path d="M-12.5 7H12.5C12 18 7 26 0 26C-7 26 -12.5 18 -12.5 7Z" fill="var(--wine)" />
		<rect x="-1.5" y="28" width="3" height="16" fill="var(--glass)" />
		<ellipse cx="0" cy="45" rx="11" ry="3" fill="var(--glass)" />
		<path d="M-9 3V12" stroke="#fff" stroke-width="2" stroke-linecap="round" opacity=".7" />
	</g>
{/snippet}

<div
	class="scene scene-raclette"
	class:m-ok={mood === 'ok'}
	class:m-warn={mood === 'warn'}
	class:m-critical={mood === 'critical'}
	class:still={!motion}
	aria-hidden="true"
>
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="rc-wall" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--wall1)" />
				<stop offset="1" stop-color="var(--wall2)" />
			</linearGradient>
			<linearGradient id="rc-sky" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--skyA)" />
				<stop offset="1" stop-color="var(--skyB)" />
			</linearGradient>
			<pattern id="rc-plank" width="64" height="10" patternUnits="userSpaceOnUse">
				<path d="M0.5 0V10" stroke="var(--plank)" stroke-width="1.4" />
			</pattern>
			<pattern id="rc-check" width="44" height="44" patternUnits="userSpaceOnUse">
				<rect width="44" height="44" fill="var(--cloth-w)" />
				<rect width="22" height="44" fill="var(--cloth-r)" opacity=".55" />
				<rect width="44" height="22" fill="var(--cloth-r)" opacity=".55" />
			</pattern>
			<clipPath id="rc-win"><rect x="1412" y="122" width="256" height="176" /></clipPath>
			<radialGradient id="rc-glow" cx=".5" cy=".5" r=".5">
				<stop offset="0" stop-color="var(--ember)" stop-opacity=".55" />
				<stop offset="1" stop-color="var(--ember)" stop-opacity="0" />
			</radialGradient>
		</defs>

		<!-- chalet wall -->
		<rect x="560" y="80" width="1220" height="340" fill="url(#rc-wall)" />
		<rect x="560" y="80" width="1220" height="340" fill="url(#rc-plank)" />
		<rect x="560" y="80" width="1220" height="14" fill="var(--wood)" opacity=".6" />
		<!-- garland of little Swiss flags -->
		<path d="M580 96C760 128 940 128 1120 96" stroke="var(--ink)" stroke-width="1.4" opacity=".6" />
		{#each [0, 1, 2, 3, 4, 5, 6, 7] as i (i)}
			<path d="M{612 + i * 62} {109 + Math.sin((i / 8) * Math.PI) * 17}l16 0l-8 15z" fill="var(--cloth-r)" />
			<path d="M{618 + i * 62} {114 + Math.sin((i / 8) * Math.PI) * 17}h4v-3h2v3h4v2h-4v3h-2v-3h-4z" fill="#fff" />
		{/each}

		<!-- window: sky, weather, mountains, village -->
		<g clip-path="url(#rc-win)">
			<rect x="1412" y="122" width="256" height="176" fill="url(#rc-sky)" />
			<circle class="sun" cx="1630" cy="160" r="17" fill="var(--sun)" />
			<g class="clouds" fill="var(--cloud)">
				<ellipse cx="1470" cy="152" rx="34" ry="11" />
				<ellipse cx="1500" cy="144" rx="22" ry="12" />
				<ellipse cx="1610" cy="178" rx="40" ry="12" />
				<ellipse cx="1580" cy="168" rx="24" ry="12" />
				<ellipse class="heavy" cx="1540" cy="136" rx="70" ry="16" />
			</g>
			<path d="M1412 240L1470 172L1500 206L1560 140L1630 232L1668 202V298H1412Z" fill="var(--mount)" />
			<path d="M1560 140L1542 164L1553 160L1562 170L1573 157L1581 163ZM1470 172L1459 186L1467 183L1475 189L1481 181Z" fill="#fff" />
			<rect x="1412" y="262" width="256" height="36" fill="var(--snow)" />
			<rect x="1430" y="256" width="28" height="18" fill="var(--chalet)" />
			<path d="M1424 257L1444 242L1464 257Z" fill="#fff" />
			<rect x="1436" y="262" width="5" height="6" fill="var(--lit)" />
			<rect x="1594" y="254" width="30" height="20" fill="var(--chalet)" />
			<path d="M1588 255L1609 238L1630 255Z" fill="#fff" />
			<rect x="1600" y="261" width="5" height="6" fill="var(--lit)" />
			<rect x="1613" y="261" width="5" height="6" fill="var(--lit)" />
			<rect x="1512" y="238" width="15" height="28" fill="var(--chalet)" />
			<path d="M1510 239L1519.5 214L1529 239Z" fill="var(--cloth-r)" />
			<path d="M1566 270L1576 244L1586 270ZM1649 272L1658 250L1667 272ZM1480 272L1489 250L1498 272Z" fill="var(--fir)" />
			{#each flakes as f, i (i)}
				<circle class="flake" class:extra={f.extra} cx={f.x} cy={116 + f.y0} r={f.r} fill="#fff" style="--d:{f.d}s;--y0:{f.y0}px;animation-delay:{f.delay}s" />
			{/each}
			<path class="bolt" d="M1590 122L1570 190L1586 188L1566 262L1606 176L1588 178L1608 122Z" fill="#fffbd0" />
			<rect class="flash" x="1412" y="122" width="256" height="176" fill="#fff" />
		</g>
		<!-- window frame, sill, heart shutters -->
		<rect x="1406" y="116" width="268" height="188" stroke="var(--frame)" stroke-width="12" />
		<path d="M1540 122V298M1412 210H1668" stroke="var(--frame)" stroke-width="6" />
		<rect x="1396" y="304" width="288" height="12" rx="3" fill="var(--frame)" />
		<rect x="1364" y="112" width="34" height="196" rx="3" fill="var(--shutter)" />
		<rect x="1682" y="112" width="34" height="196" rx="3" fill="var(--shutter)" />
		<path d="M1381 192c-8-9-3-15 0-11c3-4 8 2 0 11zM1699 192c-8-9-3-15 0-11c3-4 8 2 0 11z" fill="var(--wall1)" />

		<!-- smoke alarm -->
		<circle cx="1190" cy="116" r="23" fill="#f3f3ef" stroke="#c9c9c2" stroke-width="2" />
		<circle cx="1190" cy="116" r="12" stroke="#c9c9c2" stroke-width="1.5" />
		<path d="M1180 116h20M1190 106v20" stroke="#c9c9c2" stroke-width="1" />
		<circle class="halo" cx="1190" cy="116" r="30" fill="#ff2a2a" />
		<circle class="led" cx="1190" cy="116" r="4.5" fill="var(--led)" />
		<g class="beeps" stroke="#e11" stroke-width="2.4" stroke-linecap="round">
			<path d="M1226 100c8 8 8 24 0 32M1236 92c13 13 13 37 0 50" />
			<path d="M1154 100c-8 8-8 24 0 32M1144 92c-13 13-13 37 0 50" />
		</g>

		<!-- table and cloth -->
		<rect x="560" y="410" width="1220" height="210" fill="url(#rc-check)" />
		<rect x="560" y="410" width="1220" height="6" fill="var(--wood)" opacity=".7" />
		<rect x="560" y="500" width="1220" height="120" fill="var(--shade)" opacity=".18" />
		<path d="M560 500H1780" stroke="var(--cloth-r)" stroke-width="3" opacity=".35" />

		<!-- potatoes -->
		<ellipse cx="760" cy="482" rx="82" ry="15" fill="var(--plate)" stroke="var(--plate-edge)" stroke-width="2" />
		<g fill="var(--potato)" stroke="var(--potato-line)" stroke-width="1.6">
			<ellipse cx="722" cy="466" rx="25" ry="15" />
			<ellipse cx="770" cy="462" rx="27" ry="16" />
			<ellipse cx="800" cy="474" rx="21" ry="12" />
			<ellipse cx="746" cy="476" rx="19" ry="10" />
		</g>
		<path d="M705 464h12M758 459h14M792 472h9" stroke="var(--potato-line)" stroke-width="1.6" stroke-linecap="round" />
		<g class="steam" stroke="var(--steam)" stroke-width="3" stroke-linecap="round">
			<path class="s1" d="M730 445c-8-10 8-14 0-26" />
			<path class="s2" d="M772 440c-8-10 8-14 0-26" />
			<path class="s3" d="M800 450c-7-9 7-13 0-23" />
		</g>
		<!-- pickle jar -->
		<rect x="886" y="428" width="56" height="50" rx="9" fill="var(--glass)" opacity=".75" />
		<rect x="890" y="418" width="48" height="12" rx="3" fill="var(--metal)" />
		<g fill="var(--pickle)">
			<rect x="893" y="440" width="12" height="30" rx="6" transform="rotate(-8 899 455)" />
			<rect x="908" y="438" width="12" height="32" rx="6" />
			<rect x="922" y="441" width="12" height="29" rx="6" transform="rotate(9 928 455)" />
		</g>
		<g fill="#fffbe8"><circle cx="905" cy="472" r="4" /><circle cx="919" cy="473" r="4" /></g>

		<!-- the raclette grill -->
		<ellipse cx="1180" cy="480" rx="200" ry="26" fill="url(#rc-glow)" opacity=".5" />
		<rect x="1030" y="468" width="22" height="12" rx="3" fill="var(--metal2)" />
		<rect x="1308" y="468" width="22" height="12" rx="3" fill="var(--metal2)" />
		<rect x="1010" y="418" width="340" height="56" rx="10" fill="var(--metal2)" />
		<rect x="1026" y="430" width="308" height="42" rx="6" fill="#1b1512" />
		<rect class="coil" x="1034" y="432" width="292" height="5" rx="2.5" fill="var(--ember)" />
		<rect x="1000" y="402" width="360" height="18" rx="9" fill="var(--metal)" />
		<ellipse cx="1060" cy="399" rx="22" ry="6" fill="#b4543a" />
		<ellipse cx="1112" cy="399" rx="22" ry="6" fill="#b4543a" />
		<path d="M1240 398c6-13 24-13 30 0z" fill="#d9d2c4" />
		<rect x="1252" y="397" width="6" height="6" fill="#d9d2c4" />
		<circle cx="1308" cy="399" r="7" fill="#d23" />

		{@render pan(1080)}
		{@render pan(1180)}

		{#key pulse?.at}
			<g class="fall" class:go={kind === 'down'}>{@render pan(1280)}</g>
			{#if kind === 'down'}
				<path class="splat" d="M1318 508c-14-6-22 2-30-3c-8 8-4 14 5 15c-8 6 2 12 12 8c2 8 14 8 17 0c10 2 14-8 6-12c10-4 6-12-10-8z" fill="var(--cheese-yellow)" stroke="#c9961f" stroke-width="2" />
				<g class="ouch" stroke="var(--ink)" stroke-width="3" stroke-linecap="round">
					<path d="M1300 470l-8-12M1322 466l4-14M1344 474l12-8" />
				</g>
			{/if}
			{#if kind === 'up'}
				<g transform="translate(1180 440)">
					<ellipse class="plop" cx="0" cy="8" rx="26" ry="5" stroke="var(--cheese-hi)" stroke-width="2" />
					<g class="hand">
						<rect x="12" y="-8" width="130" height="20" rx="6" fill="var(--sleeve)" />
						<rect x="12" y="-8" width="14" height="20" fill="var(--cloth-w)" opacity=".75" />
						<ellipse cx="6" cy="2" rx="14" ry="10" fill="var(--skin)" />
						<ellipse cx="-6" cy="-3" rx="8" ry="4" fill="var(--skin)" />
						<g class="slice">
							<path d="M-20 -14H18L14 -6H-16Z" fill="var(--cheese-yellow)" stroke="#c9961f" stroke-width="1.5" />
							<circle cx="-6" cy="-10" r="1.6" fill="#d9a62b" />
							<circle cx="7" cy="-10.5" r="1.2" fill="#d9a62b" />
						</g>
					</g>
				</g>
			{/if}
		{/key}

		<!-- smoke -->
		<g class="smoke-thin" stroke="var(--smoke)" stroke-width="5" stroke-linecap="round">
			<path class="s1" d="M1080 428c-14-18 14-30 0-52" />
			<path class="s2" d="M1180 428c-14-18 14-30 0-52" />
			<path class="s3" d="M1280 428c-14-18 14-30 0-52" />
		</g>
		<g class="smoke-big" fill="var(--smoke)">
			<circle class="p1" cx="1180" cy="400" r="26" />
			<circle class="p2" cx="1210" cy="350" r="34" />
			<circle class="p3" cx="1160" cy="296" r="40" />
			<circle class="p4" cx="1210" cy="236" r="46" />
			<circle class="p5" cx="1170" cy="176" r="48" />
			<circle class="p6" cx="1230" cy="140" r="38" />
		</g>
		<rect class="tint" x="560" y="80" width="1220" height="540" fill="#e11" />

		<!-- right: the bottle and two glasses of fendant -->
		<path d="M1590 478V430C1590 416 1598 412 1598 396V372H1612V396C1612 412 1620 416 1620 430V478Z" fill="var(--bottle)" />
		<rect x="1594" y="436" width="22" height="24" fill="var(--cloth-r)" />
		<path d="M1601 441h4v-4h2v4h4v2h-4v4h-2v-4h-4z" fill="#fff" />
		<rect x="1597" y="364" width="18" height="10" rx="2" fill="#d6b04a" />
		{#key pulse?.at}
			<g class="gL" class:clink={kind === 'resolved'}>{@render glass(1470)}</g>
			<g class="gR" class:clink={kind === 'resolved'}>{@render glass(1524)}</g>
			{#if kind === 'resolved'}
				<g class="sparks" fill="#ffd84a" stroke="var(--ink)" stroke-width="1">
					<path d="M1497 418l3-9l3 9l9 3l-9 3l-3 9l-3-9l-9-3z" />
					<path d="M1478 400l2-5l2 5l5 2l-5 2l-2 5l-2-5l-5-2z" />
					<path d="M1520 396l2-5l2 5l5 2l-5 2l-2 5l-2-5l-5-2z" />
				</g>
				<text class="sante" x="1500" y="380" text-anchor="middle" font-size="26" font-weight="800" font-family="system-ui, sans-serif" fill="var(--ink)" stroke="var(--wall1)" stroke-width="5" paint-order="stroke">Santé !</text>
			{/if}
		{/key}
	</svg>
</div>

<style>
	.scene { position: absolute; inset: 0; overflow: hidden; background: var(--wall1); }
	svg { position: absolute; inset: 0; width: 100%; height: 100%; -webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%); mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%); }
	.scene-raclette {
		--wall1: #f4e1bf; --wall2: #e8cf9f; --plank: #d6b887; --wood: #a8703a; --frame: #8b5a2b; --shutter: #c4372c; --ink: #3a2a1e; --shade: #5a2a22;
		--cloth-r: #d8322b; --cloth-w: #fff8f0; --metal: #8d929c; --metal2: #4a4e59; --pan: #3d4048; --ember: #ff8a2a;
		--cheese: #f7cf4f; --cheese-hi: #fff0a8; --cheese-yellow: #f7d154; --potato: #e6bc6a; --potato-line: #b98a3c; --plate: #fff; --plate-edge: #d9cdb8;
		--pickle: #7da23a; --glass: #d4ecf5; --wine: #f2e69a; --skin: #f1c7a0; --sleeve: #b33a2f; --bottle: #4e7a3a; --steam: rgba(255,255,255,.75);
		--sun: #ffe27a; --cloud: #fff; --mount: #a9bad2; --snow: #f6fbff; --chalet: #8a5a34; --fir: #2f5d46; --lit: #ffdc7a; --smoke: rgba(120,118,125,.8); --led: #3cc060;
		--skyA: #7ec0f0; --skyB: #d6ecfb;
	}
	:global(.dark) .scene-raclette {
		--wall1: #3a2a24; --wall2: #2a1d1a; --plank: #4a362c; --wood: #6b4524; --frame: #5a3a1c; --shutter: #8f2a22; --ink: #f6e9d3; --shade: #000;
		--cloth-r: #a8281f; --cloth-w: #cdbfae; --metal: #6c717c; --metal2: #2d3037; --pan: #25272d; --cheese: #e8b93a; --cheese-hi: #ffe58a; --cheese-yellow: #e8bc3e;
		--potato: #c79a4e; --potato-line: #8a6226; --plate: #dcd2c2; --plate-edge: #a89c88; --pickle: #5f8228; --glass: #8fb2c2; --wine: #d8c86e; --skin: #d9a981;
		--sleeve: #8a2c24; --bottle: #365a28; --steam: rgba(255,255,255,.4); --sun: #e8eef8; --cloud: #4a5878; --mount: #4a5878; --snow: #9fb2cc; --chalet: #4e3320;
		--fir: #1d3d2e; --smoke: rgba(150,150,160,.5); --skyA: #0c1430; --skyB: #26345c;
	}

	/* mood: the cheese, the sky, the alarm */
	.m-warn { --cheese: #d6962a; --cheese-hi: #f0c870; --skyA: #8a9bb0; --skyB: #c9d2dc; --sun: transparent; }
	.m-critical { --cheese: #1c1410; --cheese-hi: #4a3a30; --skyA: #3c4658; --skyB: #6b7684; --sun: transparent; --led: #ff2a2a; }
	:global(.dark) .m-warn { --skyA: #1a2238; --skyB: #3a4560; }
	:global(.dark) .m-critical { --skyA: #0a0e1a; --skyB: #222a40; }

	.clouds, .smoke-thin, .smoke-big, .bolt, .flash, .tint, .halo, .beeps, .char { opacity: 0; }
	.heavy { display: none; }
	.m-warn .clouds, .m-critical .clouds { opacity: .9; }
	.m-critical .heavy { display: block; }
	.m-warn .smoke-thin { opacity: 1; }
	.m-critical .smoke-big { opacity: .88; }
	.m-critical .char { opacity: .85; }
	.m-critical .tint { opacity: .07; }
	.m-critical .halo { opacity: .35; }
	.m-critical .beeps { opacity: .9; }
	.m-critical .coil { fill: #ffb347; }
	.m-critical .bubbles { display: none; }
	.flake.extra { display: none; }
	.m-critical .flake.extra { display: inline; }

	/* ambient motion */
	.flake { animation: snow var(--d) linear infinite; }
	.m-critical .flake { animation-name: blizzard; animation-duration: calc(var(--d) * .35); }
	@keyframes snow { from { transform: translate(0, calc(var(--y0) * -1)); } to { transform: translate(-10px, calc(190px - var(--y0))); } }
	@keyframes blizzard { from { transform: translate(0, calc(var(--y0) * -1)); } to { transform: translate(-90px, calc(190px - var(--y0))); } }
	.m-critical .bolt { animation: bolt 5.5s steps(1) infinite; }
	.m-critical .flash { animation: flash 5.5s steps(1) infinite; }
	@keyframes bolt { 0%, 100% { opacity: 0; } 60% { opacity: 1; } 63% { opacity: 0; } 66% { opacity: 1; } 69% { opacity: 0; } }
	@keyframes flash { 0%, 100% { opacity: 0; } 60% { opacity: .65; } 63% { opacity: 0; } 66% { opacity: .5; } 69% { opacity: 0; } }
	.m-critical .led { animation: blink .8s steps(1) infinite; }
	.m-critical .halo { animation: halo .8s ease-in-out infinite; }
	.m-critical .beeps { animation: blink .8s steps(1) infinite; }
	.m-critical .tint { animation: tint .8s ease-in-out infinite; }
	@keyframes blink { 50% { opacity: .12; } }
	@keyframes halo { 50% { opacity: .05; } }
	@keyframes tint { 50% { opacity: .02; } }
	.coil { animation: glow 3s ease-in-out infinite; }
	@keyframes glow { 50% { opacity: .75; } }
	.bubbles circle { transform-box: fill-box; transform-origin: center; animation: bubble 2.4s ease-in-out infinite; }
	.bubbles circle:nth-child(2) { animation-delay: -.8s; }
	.bubbles circle:nth-child(3) { animation-delay: -1.6s; }
	@keyframes bubble { 0%, 100% { transform: scale(.4); } 50% { transform: scale(1.1); } }
	.steam path, .smoke-thin path { animation: rise 3.4s ease-in-out infinite; }
	.steam .s2, .smoke-thin .s2 { animation-delay: -1.1s; }
	.steam .s3, .smoke-thin .s3 { animation-delay: -2.2s; }
	@keyframes rise { 0% { transform: translateY(6px); opacity: 0; } 40% { opacity: .9; } 100% { transform: translateY(-10px); opacity: 0; } }
	.smoke-big circle { transform-box: fill-box; transform-origin: center; animation: puff 4s ease-in-out infinite; }
	.smoke-big .p2 { animation-delay: -.7s; } .smoke-big .p3 { animation-delay: -1.4s; } .smoke-big .p4 { animation-delay: -2.1s; } .smoke-big .p5 { animation-delay: -2.8s; } .smoke-big .p6 { animation-delay: -3.4s; }
	@keyframes puff { 50% { transform: scale(1.12) translate(6px, -6px); } }

	/* gestures */
	.fall { transform-box: fill-box; transform-origin: center; }
	.fall.go { animation: pan-fall 3.4s linear both; }
	@keyframes pan-fall {
		0%, 14% { transform: none; opacity: 1; animation-timing-function: cubic-bezier(.5, 0, .9, .6); }
		42% { transform: translate(52px, 54px) rotate(115deg); animation-timing-function: ease-out; }
		48% { transform: translate(56px, 50px) rotate(100deg); }
		80% { transform: translate(56px, 50px) rotate(100deg); opacity: 1; }
		92% { transform: translate(56px, 50px) rotate(100deg); opacity: 0; }
		93% { transform: none; opacity: 0; }
		100% { transform: none; opacity: 1; }
	}
	.splat { transform-box: fill-box; transform-origin: center bottom; animation: splat 3.4s ease-out both; }
	@keyframes splat { 0%, 40% { transform: scale(0); opacity: 0; } 48% { transform: scale(1.15); opacity: 1; } 55%, 80% { transform: scale(1); opacity: 1; } 95%, 100% { transform: scale(1); opacity: 0; } }
	.ouch { animation: ouch 3.4s ease-out both; }
	@keyframes ouch { 0%, 40% { opacity: 0; } 46%, 60% { opacity: 1; } 70%, 100% { opacity: 0; } }
	.hand { animation: hand 3.2s ease-in-out both; }
	@keyframes hand { 0% { transform: translate(120px, -6px); opacity: 0; } 12% { opacity: 1; } 35% { transform: translate(0, -8px); } 52% { transform: translate(0, 2px); } 70% { transform: translate(0, -8px); opacity: 1; } 100% { transform: translate(130px, -6px); opacity: 0; } }
	.slice { animation: slice 3.2s linear both; }
	@keyframes slice { 0%, 44% { transform: none; opacity: 1; } 54% { transform: translateY(7px); opacity: 1; } 56%, 100% { transform: translateY(7px); opacity: 0; } }
	.plop { transform-box: fill-box; transform-origin: center; animation: plop 3.2s ease-out both; }
	@keyframes plop { 0%, 52% { transform: scale(.2); opacity: 0; } 56% { opacity: 1; } 78%, 100% { transform: scale(1.3); opacity: 0; } }
	.gL, .gR { transform-box: fill-box; transform-origin: center bottom; }
	.gL.clink { animation: clink-l 3s ease-in-out both; }
	.gR.clink { animation: clink-r 3s ease-in-out both; }
	@keyframes clink-l { 0%, 12% { transform: none; } 40%, 48% { transform: translate(10px, -12px) rotate(14deg); } 44% { transform: translate(12px, -12px) rotate(18deg); } 75%, 100% { transform: none; } }
	@keyframes clink-r { 0%, 12% { transform: none; } 40%, 48% { transform: translate(-10px, -12px) rotate(-14deg); } 44% { transform: translate(-12px, -12px) rotate(-18deg); } 75%, 100% { transform: none; } }
	.sparks, .sante { animation: sparks 3s ease-out both; transform-box: fill-box; transform-origin: center; }
	@keyframes sparks { 0%, 38% { opacity: 0; transform: scale(.3); } 46% { opacity: 1; transform: scale(1.2); } 75% { opacity: 1; transform: scale(1); } 100% { opacity: 0; } }

	/* reduced motion: a still picture */
	.still :global(*) { animation: none !important; }
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) { animation: none !important; }
	}
</style>
