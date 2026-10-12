<script lang="ts">
	/**
	 * A zen garden — raked sand rippling round two stones, a koi pond with lily pads, a
	 * shishi-odoshi that tips and clacks, a maple and a stand of bamboo under far hills.
	 * Fills its positioned parent, the headline zone (top left) is left clear, the palette
	 * lives here. A slow ambient motion (koi, drip, falling leaves), still under reduced motion.
	 *
	 * `mood` is the weather (ok: clear, warn: clouds, critical: rain and lightning).
	 * `pulse` replays a gesture whenever `at` changes: `down` the koi dive and hide, `up`
	 * one leaps and rings spread, `resolved` the shishi-odoshi tips and clacks.
	 */
	interface Props {
		mood?: 'ok' | 'warn' | 'critical';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}
	let { mood = 'ok', pulse = null }: Props = $props();

	const stamp = $derived(pulse ? pulse.at : 0);
	const kind = $derived(pulse ? pulse.kind : 'none');

	const rings = (n: number, rx0: number, step: number, ry0: number, ryStep: number) =>
		Array.from({ length: n }, (_, i) => ({ rx: rx0 + i * step, ry: ry0 + i * ryStep }));
	const ringsA = rings(6, 38, 20, 11, 5.2);
	const ringsB = rings(4, 30, 18, 9, 4.6);
	const waves = Array.from({ length: 6 }, (_, i) => {
		const y = 432 + i * 9;
		return `M560 ${y}c60 -5 90 5 150 0s90 -5 150 0 90 5 150 0 90 -5 150 0`;
	});

	/** A koi's swim: keyframes on an ellipse, heading along the tangent. */
	function swim(name: string, cx: number, cy: number, rx: number, ry: number, dir: number, ph: number) {
		const N = 28;
		let prev = 0;
		let css = `@keyframes ${name}{`;
		let first = '';
		for (let k = 0; k <= N; k++) {
			const t = ph + (dir * 2 * Math.PI * k) / N;
			const x = cx + rx * Math.cos(t);
			const y = cy + ry * Math.sin(t);
			let a = (Math.atan2(ry * Math.cos(t) * dir, -rx * Math.sin(t) * dir) * 180) / Math.PI;
			if (k > 0) {
				while (a - prev > 180) a -= 360;
				while (a - prev < -180) a += 360;
			}
			prev = a;
			if (k === 0) first = `translate(${x.toFixed(1)} ${y.toFixed(1)}) rotate(${a.toFixed(0)})`;
			css += `${((k / N) * 100).toFixed(2)}%{transform:translate(${x.toFixed(1)}px,${y.toFixed(1)}px) rotate(${a.toFixed(1)}deg)}`;
		}
		return { css: css + '}', first };
	}
	const KOI = [
		{ id: 'zk1', ...swim('zk1', 1380, 524, 140, 28, 1, 0), dur: 30, body: 'var(--koi1)', patch: 'var(--koi-w)' },
		{ id: 'zk2', ...swim('zk2', 1370, 526, 100, 20, -1, 2), dur: 24, body: 'var(--koi-w)', patch: 'var(--koi2)' },
		{ id: 'zk3', ...swim('zk3', 1400, 520, 70, 14, 1, 4), dur: 20, body: 'var(--koi2)', patch: 'var(--koi1)' }
	];
	const koiCss = KOI.map((k) => `${k.css}.${k.id}{animation:${k.id} ${k.dur}s linear infinite}`).join('');

	const RAIN = (() => {
		let d = '';
		let s = 7;
		const rnd = () => ((s = (s * 16807) % 2147483647) / 2147483647);
		for (let i = 0; i < 110; i++) {
			const x = 560 + Math.round(rnd() * 1260);
			const y = 80 + Math.round(rnd() * 250);
			d += `M${x} ${y}l-6 22`;
		}
		return d;
	})();
</script>

{@html `<style>${koiCss}</style>`}

<div class="scene scene-zen m-{mood}" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="zen-pond" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--pond1)" /><stop offset="1" stop-color="var(--pond2)" />
			</linearGradient>
			<linearGradient id="zen-sand" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--sand2)" /><stop offset="1" stop-color="var(--sand1)" />
			</linearGradient>
			<clipPath id="zen-sandclip"><rect x="560" y="418" width="1220" height="202" /></clipPath>
			<g id="zen-koi">
				<path d="M17 0C9-8-6-7-14-1L-24-8-21 0-24 8-14 1C-6 7 9 8 17 0z" fill="var(--k)" />
				<path d="M6-6C1-8-5-6-8-2-4 0 2-2 6-6zM-2 5c4 2 8 1 11-2" fill="var(--kp)" opacity=".9" />
				<path d="M2-6l-4-5 8 3zM2 6l-4 5 8-3z" fill="var(--k)" opacity=".7" />
			</g>
		</defs>

		<!-- sky bodies and weather -->
		<g class="sunmoon"><circle cx="1500" cy="196" r="46" fill="var(--sun)" /><circle cx="1500" cy="196" r="70" fill="var(--sun)" opacity=".18" /></g>
		<g class="cloud cloud-a" fill="var(--cloud)">
			<ellipse cx="880" cy="170" rx="96" ry="15" /><ellipse cx="940" cy="156" rx="56" ry="15" />
			<ellipse cx="1330" cy="130" rx="86" ry="13" /><ellipse cx="1390" cy="118" rx="48" ry="12" />
		</g>
		<g class="cloud cloud-b" fill="var(--cloud)">
			<ellipse cx="1130" cy="214" rx="120" ry="19" /><ellipse cx="1190" cy="196" rx="70" ry="19" />
			<ellipse cx="1620" cy="150" rx="100" ry="16" /><ellipse cx="1560" cy="140" rx="56" ry="15" />
			<ellipse cx="760" cy="236" rx="90" ry="14" />
		</g>

		<!-- far hills in mist -->
		<path d="M560 410C640 340 720 320 800 350C880 290 980 270 1070 330C1150 300 1250 330 1330 370C1420 330 1520 300 1610 350C1690 340 1740 360 1780 380V420H560z" fill="var(--hill2)" />
		<path d="M560 420C700 380 800 370 900 392C1010 360 1130 372 1240 398C1380 370 1500 380 1600 396C1680 388 1740 398 1780 404V424H560z" fill="var(--hill1)" />
		<rect x="560" y="404" width="1220" height="18" fill="var(--hedge)" />
		<g fill="var(--hedge2)"><ellipse cx="680" cy="404" rx="46" ry="14" /><ellipse cx="780" cy="406" rx="36" ry="11" /><ellipse cx="1060" cy="404" rx="52" ry="13" /><ellipse cx="1230" cy="406" rx="40" ry="10" /></g>

		<!-- sand and rake lines -->
		<rect x="560" y="418" width="1220" height="202" fill="url(#zen-sand)" />
		<g clip-path="url(#zen-sandclip)" stroke="var(--rake)" stroke-width="1.6" stroke-linecap="round" fill="none">
			{#each waves as w, i (i)}<path d={w} opacity={0.75 - i * 0.05} />{/each}
			<g transform="translate(830 528)">
				{#each ringsA as r, i (i)}<ellipse rx={r.rx} ry={r.ry} />{/each}
			</g>
			<g transform="translate(1030 566)">
				{#each ringsB as r, i (i)}<ellipse rx={r.rx} ry={r.ry} />{/each}
			</g>
			<path d="M560 590c120 6 240 -4 360 4s220 4 340 -2M560 604c140 4 260 -2 380 4s200 2 300 -2" opacity=".6" />
		</g>
		<!-- stones -->
		<g>
			<ellipse cx="830" cy="530" rx="44" ry="15" fill="var(--shadow)" />
			<path d="M792 530c-6-30 18-52 42-50 24 2 40 22 36 50z" fill="var(--stone)" />
			<path d="M800 512c10-14 26-20 40-16-14 2-26 10-34 24z" fill="var(--stone-hi)" />
			<ellipse cx="900" cy="534" rx="20" ry="7" fill="var(--shadow)" /><path d="M884 534c-2-14 8-22 18-22 10 1 16 10 14 22z" fill="var(--stone2)" />
			<ellipse cx="1030" cy="568" rx="30" ry="10" fill="var(--shadow)" />
			<path d="M1004 568c-4-20 12-34 28-33 16 1 26 14 24 33z" fill="var(--stone)" />
			<path d="M1010 556c6-8 16-12 24-10-8 2-16 6-20 14z" fill="var(--stone-hi)" />
			<path d="M806 500c4-6 8-8 14-8M1020 548c3-4 6-5 10-5" stroke="var(--moss)" stroke-width="5" stroke-linecap="round" />
		</g>

		<!-- pond -->
		<ellipse cx="1380" cy="532" rx="222" ry="62" fill="var(--shadow)" />
		<ellipse cx="1380" cy="524" rx="212" ry="54" fill="url(#zen-pond)" />
		<g class="shimmer" stroke="var(--glint)" stroke-width="1.6" stroke-linecap="round"><path d="M1250 500h40M1450 494h50M1330 548h60M1500 538h40" /></g>
		<g class="koi">
			{#key stamp}
				<g class="veil v-{kind}">
					{#each KOI as k (k.id)}
						<g class={k.id} transform={k.first}>
							<g style="--k:{k.body};--kp:{k.patch}"><use href="#zen-koi" /></g>
						</g>
					{/each}
				</g>
			{/key}
		</g>
		<g fill="var(--pad)">
			<ellipse cx="1280" cy="540" rx="22" ry="6" /><ellipse cx="1312" cy="552" rx="16" ry="4.5" /><ellipse cx="1500" cy="512" rx="20" ry="5.5" />
		</g>
		<g fill="var(--lotus)"><path d="M1280 536l-6-8 6 3 6-3z" /><path d="M1500 508l-5-7 5 3 5-3z" /></g>
		{#key stamp}
			<g class="effects e-{kind}">
				<g class="leap"><g class="leap-r"><use href="#zen-koi" style="--k:var(--koi1);--kp:var(--koi-w)" /></g></g>
				<ellipse class="ring r1" cx="1380" cy="524" rx="120" ry="30" stroke="var(--glint)" stroke-width="3" />
				<ellipse class="ring r2" cx="1380" cy="524" rx="120" ry="30" stroke="var(--glint)" stroke-width="2.2" />
				<ellipse class="ring r3" cx="1380" cy="524" rx="120" ry="30" stroke="var(--glint)" stroke-width="1.6" />
			</g>
		{/key}
		<g fill="var(--stone)">
			<ellipse cx="1176" cy="528" rx="14" ry="8" /><ellipse cx="1190" cy="556" rx="12" ry="7" fill="var(--stone2)" /><ellipse cx="1230" cy="574" rx="16" ry="7" />
			<ellipse cx="1310" cy="582" rx="13" ry="6" fill="var(--stone2)" /><ellipse cx="1400" cy="584" rx="18" ry="7" /><ellipse cx="1492" cy="574" rx="14" ry="7" fill="var(--stone2)" />
			<ellipse cx="1566" cy="552" rx="14" ry="8" /><ellipse cx="1584" cy="520" rx="12" ry="7" fill="var(--stone2)" /><ellipse cx="1548" cy="486" rx="14" ry="7" />
			<ellipse cx="1440" cy="474" rx="16" ry="6" /><ellipse cx="1300" cy="476" rx="14" ry="6" fill="var(--stone2)" /><ellipse cx="1210" cy="496" rx="12" ry="7" />
		</g>

		<!-- shishi-odoshi -->
		<g transform="translate(1226 478)">
			<path d="M-6 0v62M6 0v62M-12 62h24" stroke="var(--wood)" stroke-width="6" stroke-linecap="round" />
			<path d="M-14 -24C-30 -40-60 -50-96 -50" stroke="var(--bamboo2)" stroke-width="8" stroke-linecap="round" />
			<path d="M-14 -24C-30 -40-60 -50-96 -50" stroke="var(--bamboo)" stroke-width="3" stroke-linecap="round" />
			<path class="stream" d="M-60 -42v34" stroke="var(--glint)" stroke-width="2.4" stroke-linecap="round" stroke-dasharray="4 5" />
			<ellipse cx="46" cy="30" rx="22" ry="8" fill="var(--stone)" /><path d="M28 30c-2-10 8-16 18-16s18 6 16 16z" fill="var(--stone2)" />
			<g class="tube">
				{#key stamp}
					<g class="tilt t-{kind}">
						<rect x="-62" y="-8" width="104" height="16" rx="6" fill="var(--bamboo2)" />
						<rect x="-62" y="-8" width="104" height="6" rx="3" fill="var(--bamboo)" />
						<path d="M-32-8v16M10-8v16" stroke="var(--bamboo2)" stroke-width="2.4" />
						<path class="fill" d="M-60-2h26" stroke="var(--glint)" stroke-width="4" stroke-linecap="round" />
					</g>
				{/key}
				<circle r="4.4" fill="var(--wood)" />
			</g>
			{#key stamp}
				<g class="clack c-{kind}" transform="translate(46 12)">
					<ellipse class="cr c1" rx="12" ry="5" stroke="var(--glint)" stroke-width="2" />
					<ellipse class="cr c2" rx="12" ry="5" stroke="var(--glint)" stroke-width="1.6" />
				</g>
			{/key}
			<path class="spill" d="M-62 8c-3 22-4 44-8 70" stroke="var(--glint)" stroke-width="3.4" stroke-linecap="round" />
		</g>

		<!-- maple -->
		<path d="M1688 412c-4-40 4-80-6-120M1684 330c24-16 36-34 44-58M1688 300c-26-14-40-30-46-52" stroke="var(--trunk)" stroke-width="12" stroke-linecap="round" />
		<g>
			<ellipse cx="1676" cy="236" rx="96" ry="52" fill="var(--maple2)" />
			<ellipse cx="1620" cy="262" rx="64" ry="34" fill="var(--maple1)" />
			<ellipse cx="1736" cy="268" rx="58" ry="32" fill="var(--maple3)" />
			<ellipse cx="1668" cy="206" rx="64" ry="30" fill="var(--maple1)" />
			<ellipse cx="1700" cy="244" rx="48" ry="22" fill="var(--maple3)" opacity=".85" />
		</g>
		<g class="leaf l1" fill="var(--maple1)"><path d="M0-6l3 5 6-1-3 5 4 5-6-1-4 4-4-4-6 1 4-5-3-5 6 1z" /></g>
		<g class="leaf l2" fill="var(--maple3)"><path d="M0-6l3 5 6-1-3 5 4 5-6-1-4 4-4-4-6 1 4-5-3-5 6 1z" /></g>
		<g class="leaf l3" fill="var(--maple2)"><path d="M0-6l3 5 6-1-3 5 4 5-6-1-4 4-4-4-6 1 4-5-3-5 6 1z" /></g>
		<g fill="var(--maple1)" opacity=".8"><ellipse cx="1612" cy="580" rx="6" ry="2.5" /><ellipse cx="1650" cy="590" rx="5" ry="2" /><ellipse cx="1590" cy="596" rx="5" ry="2" /></g>

		<!-- bamboo -->
		<g stroke-linecap="round">
			<path d="M1744 412V80M1772 412V80" stroke="var(--bamboo2)" stroke-width="9" />
			<path d="M1744 412V80M1772 412V80" stroke="var(--bamboo)" stroke-width="4" />
			<path d="M1722 412C1724 330 1720 250 1730 150" stroke="var(--bamboo2)" stroke-width="10" />
			<path d="M1722 412C1724 330 1720 250 1730 150" stroke="var(--bamboo)" stroke-width="4" />
			<path d="M1738 360h12M1766 330h12M1738 270h12M1766 220h12M1716 330h12M1718 240h12" stroke="var(--dark)" stroke-width="2" opacity=".5" />
		</g>
		<g class="sway" fill="var(--leaf)">
			<path d="M1744 130c-30-4-50 8-66 22 24-4 44-4 66-22zM1744 130c24-10 44-8 62-2-20 8-40 8-62 2zM1772 160c-28 0-48 14-60 28 22-6 42-10 60-28zM1772 160c22-8 40-6 56 2-18 6-36 6-56-2zM1730 190c-22-2-40 8-52 20 20-4 36-6 52-20z" />
		</g>

		<!-- low shrubs -->
		<g fill="var(--hedge2)"><ellipse cx="1130" cy="418" rx="30" ry="12" /><ellipse cx="1560" cy="420" rx="36" ry="12" /></g>
		<g fill="var(--maple1)" opacity=".85"><ellipse cx="1110" cy="414" rx="14" ry="7" /><ellipse cx="1580" cy="416" rx="16" ry="7" /></g>

		<!-- storm -->
		<g class="storm">
			<rect x="560" y="80" width="1220" height="540" fill="var(--storm)" />
			<path class="bolt" d="M1000 90l-20 64h16l-24 80 46-90h-20l22-54z" fill="var(--bolt)" />
			<g class="rain">
				<path d={RAIN} stroke="var(--rain)" stroke-width="1.8" stroke-linecap="round" />
				<path d={RAIN} transform="translate(0 -250)" stroke="var(--rain)" stroke-width="1.8" stroke-linecap="round" />
			</g>
			<rect class="flash" x="560" y="80" width="1220" height="540" fill="var(--bolt)" />
		</g>
	</svg>
</div>

<style>
	.scene-zen {
		--sky1: #bcd9e4;
		--sky2: #f3ead4;
		--sun: #fff1c8;
		--cloud: #ffffff;
		--hill1: #a6bfb4;
		--hill2: #c5d5d3;
		--hedge: #6f8f63;
		--hedge2: #547a51;
		--sand1: #e7dcc0;
		--sand2: #efe6cf;
		--rake: #c4b690;
		--shadow: rgb(70 60 40 / 0.22);
		--stone: #8a8d93;
		--stone2: #a2a5a8;
		--stone-hi: #b9bcc0;
		--moss: #6c8f4e;
		--pond1: #7db0b4;
		--pond2: #4b8387;
		--glint: #e8f6f4;
		--pad: #5f9a56;
		--lotus: #f2a6bd;
		--koi1: #e8602b;
		--koi2: #cf2f2a;
		--koi-w: #fbf4e6;
		--wood: #6e4e33;
		--bamboo: #86ad5e;
		--bamboo2: #5b8545;
		--leaf: #6fa04f;
		--dark: #2c2a28;
		--trunk: #4d3a30;
		--maple1: #d8452b;
		--maple2: #b8321f;
		--maple3: #ec8236;
		--storm: rgb(40 52 80 / 0.38);
		--bolt: #f4f7ff;
		--rain: rgb(220 232 245 / 0.7);
	}
	:global(.dark) .scene-zen {
		--sky1: #0a1226;
		--sky2: #29294d;
		--sun: #e8eef8;
		--cloud: #2a3558;
		--hill1: #1d2a3e;
		--hill2: #232f4a;
		--hedge: #1f3a2e;
		--hedge2: #172e25;
		--sand1: #4a473d;
		--sand2: #5a564a;
		--rake: #36332b;
		--shadow: rgb(0 0 0 / 0.35);
		--stone: #444a5a;
		--stone2: #565c6c;
		--stone-hi: #6a7084;
		--moss: #34523a;
		--pond1: #1b3a52;
		--pond2: #0c1c30;
		--glint: #9fc4d4;
		--pad: #2c5a40;
		--lotus: #a56a86;
		--koi1: #c75a2c;
		--koi2: #a42f2e;
		--koi-w: #d8d2c4;
		--wood: #3d2e24;
		--bamboo: #3f6a46;
		--bamboo2: #264a34;
		--leaf: #2f5a3c;
		--dark: #05070d;
		--trunk: #20181a;
		--maple1: #8a2f2b;
		--maple2: #6a2428;
		--maple3: #a8582f;
		--storm: rgb(4 8 20 / 0.45);
		--rain: rgb(150 176 206 / 0.5);
	}
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(180deg, var(--sky1), var(--sky2) 62%, var(--hill2) 63%);
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
	}

	/* Weather */
	.cloud {
		opacity: 0.4;
		transition: opacity 1.6s;
	}
	.cloud-b {
		opacity: 0;
	}
	.sunmoon {
		transition: opacity 1.6s;
	}
	.m-warn {
		--cloud: #e9ecf0;
	}
	.m-warn .cloud-a {
		opacity: 0.95;
	}
	.m-warn .cloud-b {
		opacity: 0.9;
	}
	.m-warn .sunmoon {
		opacity: 0.5;
	}
	.m-critical {
		--cloud: #7e8798;
	}
	.m-critical .cloud-a,
	.m-critical .cloud-b {
		opacity: 1;
	}
	.m-critical .sunmoon {
		opacity: 0;
	}
	:global(.dark) .m-warn {
		--cloud: #36436a;
	}
	:global(.dark) .m-critical {
		--cloud: #161d33;
	}
	.storm {
		opacity: 0;
		transition: opacity 1.6s;
	}
	.m-critical .storm {
		opacity: 1;
	}
	.bolt,
	.flash {
		opacity: 0;
	}
	.m-critical .bolt {
		animation: bolt 9s infinite;
	}
	.m-critical .flash {
		animation: flash 9s infinite;
	}
	.m-critical .rain {
		animation: rain 0.7s linear infinite;
	}
	@keyframes rain {
		to {
			transform: translate(-30px, 250px);
		}
	}
	@keyframes flash {
		0%,
		38%,
		44%,
		100% {
			opacity: 0;
		}
		39% {
			opacity: 0.5;
		}
		41% {
			opacity: 0.1;
		}
		42% {
			opacity: 0.38;
		}
	}
	@keyframes bolt {
		0%,
		38.5%,
		43%,
		100% {
			opacity: 0;
		}
		39%,
		42.5% {
			opacity: 1;
		}
		41% {
			opacity: 0.2;
		}
	}

	/* Ambient life */
	.stream {
		animation: drip 0.5s linear infinite;
	}
	@keyframes drip {
		to {
			stroke-dashoffset: -9;
		}
	}
	.shimmer {
		animation: shimmer 6s ease-in-out infinite;
	}
	@keyframes shimmer {
		50% {
			opacity: 0.3;
		}
	}
	.sway {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: sway 7s ease-in-out infinite;
	}
	@keyframes sway {
		50% {
			transform: rotate(1.6deg);
		}
	}
	.leaf {
		animation: fall 14s linear infinite;
		opacity: 0;
	}
	.l1 {
		animation-delay: -2s;
	}
	.l2 {
		animation-delay: -7s;
		animation-duration: 17s;
	}
	.l3 {
		animation-delay: -11s;
		animation-duration: 12s;
	}
	@keyframes fall {
		0% {
			opacity: 0;
			transform: translate(1650px, 270px) rotate(0);
		}
		8% {
			opacity: 1;
		}
		50% {
			transform: translate(1590px, 420px) rotate(200deg);
		}
		92% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(1530px, 560px) rotate(420deg);
		}
	}
	/* The tube fills and tips every so often, a little hit on the stone. */
	.tube {
		transform: rotate(16deg);
		animation: tipping 18s ease-in-out infinite;
	}
	@keyframes tipping {
		0%,
		70% {
			transform: rotate(16deg);
		}
		74%,
		78% {
			transform: rotate(-26deg);
		}
		83% {
			transform: rotate(18deg);
		}
		85% {
			transform: rotate(13deg);
		}
		88%,
		100% {
			transform: rotate(16deg);
		}
	}
	.spill {
		opacity: 0;
		animation: spill 18s linear infinite;
	}
	@keyframes spill {
		0%,
		73%,
		80%,
		100% {
			opacity: 0;
		}
		75%,
		78% {
			opacity: 0.9;
		}
	}
	.fill {
		opacity: 0;
		animation: fillup 18s linear infinite;
	}
	@keyframes fillup {
		0% {
			opacity: 0;
		}
		60%,
		72% {
			opacity: 0.85;
		}
		75%,
		100% {
			opacity: 0;
		}
	}
	.cr,
	.ring {
		opacity: 0;
		transform-box: fill-box;
		transform-origin: center;
	}
	.leap {
		opacity: 0;
	}

	/* Gestures. down: the koi dive and hide, then drift back. */
	.v-down {
		animation: hide 6s ease-in-out both;
	}
	@keyframes hide {
		0% {
			opacity: 1;
		}
		14%,
		78% {
			opacity: 0;
		}
		100% {
			opacity: 1;
		}
	}
	.e-down .ring {
		animation: ring-in 1.6s ease-in both;
	}
	.e-down .r2 {
		animation-delay: 0.35s;
	}
	@keyframes ring-in {
		0% {
			opacity: 0.8;
			transform: scale(1);
		}
		100% {
			opacity: 0;
			transform: scale(0.15);
		}
	}
	/* up: one koi leaps out of the pond and rings spread. */
	.e-up .leap {
		animation: leap 2.2s ease-out both;
	}
	.e-up .leap-r {
		animation: leap-turn 2.2s linear both;
	}
	@keyframes leap {
		0% {
			opacity: 0;
			transform: translate(1330px, 524px);
		}
		10% {
			opacity: 1;
		}
		50% {
			transform: translate(1380px, 432px);
		}
		88% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(1430px, 524px);
		}
	}
	@keyframes leap-turn {
		0% {
			transform: rotate(-60deg);
		}
		50% {
			transform: rotate(0deg);
		}
		100% {
			transform: rotate(60deg);
		}
	}
	.e-up .ring {
		animation: ring-out 2.4s ease-out 1.8s both;
	}
	.e-up .r2 {
		animation-delay: 2.1s;
	}
	.e-up .r3 {
		animation-delay: 2.4s;
	}
	@keyframes ring-out {
		0% {
			opacity: 0.85;
			transform: scale(0.1);
		}
		100% {
			opacity: 0;
			transform: scale(1);
		}
	}
	/* resolved: the shishi-odoshi tips and clacks, rings spread from the stone. */
	.t-resolved {
		animation: tip-now 3s ease-in-out both;
	}
	@keyframes tip-now {
		0%,
		12% {
			transform: rotate(0deg);
		}
		30%,
		50% {
			transform: rotate(-44deg);
		}
		62% {
			transform: rotate(8deg);
		}
		68% {
			transform: rotate(-3deg);
		}
		100% {
			transform: rotate(0deg);
		}
	}
	.c-resolved .cr {
		animation: clack 1.6s ease-out 1.85s both;
	}
	.c-resolved .c2 {
		animation-delay: 2.15s;
	}
	@keyframes clack {
		0% {
			opacity: 0.95;
			transform: scale(0.3);
		}
		100% {
			opacity: 0;
			transform: scale(3);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
