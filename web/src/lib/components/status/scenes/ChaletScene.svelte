<script lang="ts">
	/**
	 * A mountain chalet at night — log walls, a stone fireplace with a crackling fire,
	 * a window onto snowy firs and peaks, a cat asleep on a rug, a mug of hot chocolate.
	 * The weather happens outside the window (`mood`): clear and calm, heavy cloud,
	 * a blizzard with lightning. `pulse` plays one gesture indoors: the fire gutters out
	 * (down), roars back with sparks (up), the cat stretches and the cocoa steam draws
	 * a heart (resolved). Follows the scene contract: fills its positioned parent, the
	 * headline zone (top left) is left clear, the palette lives here.
	 */
	interface Props {
		mood?: 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}
	let { mood = 'ok', pulse = null }: Props = $props();

	const level = $derived(
		mood === 'critical' || mood === 'storm' ? 2 : mood === 'warn' || mood === 'clouded' ? 1 : 0
	);

	let playing = $state<{ kind: 'down' | 'up' | 'resolved'; id: number } | null>(null);
	$effect(() => {
		const p = pulse;
		if (!p) return;
		playing = { kind: p.kind, id: p.at };
		const timer = setTimeout(() => (playing = null), 5200);
		return () => clearTimeout(timer);
	});

	// A tiny deterministic generator keeps the snow identical between renders.
	let seed = 7;
	const rnd = () => (seed = (seed * 16807) % 2147483647) / 2147483647;
	const flakes = Array.from({ length: 36 }, () => ({
		x: Math.round(966 + rnd() * 328),
		r: +(1.1 + rnd() * 1.5).toFixed(1),
		d: +(5 + rnd() * 5).toFixed(1),
		dl: +(-rnd() * 10).toFixed(1),
		k: +(0.6 + rnd() * 0.8).toFixed(2)
	}));
	const shown = $derived(flakes.slice(0, level === 0 ? 14 : level === 1 ? 24 : 36));

	/** A fir: three stacked tiers with a snow cap on each. */
	function fir(x: number, base: number, h: number, w: number) {
		let body = '';
		let snow = '';
		for (let t = 0; t < 3; t++) {
			const top = base - h + t * h * 0.27;
			const bot = top + h * 0.42;
			const hw = w * (0.4 + 0.3 * t);
			body += `M${x} ${top}L${x - hw} ${bot}L${x + hw} ${bot}Z`;
			const cy = top + (bot - top) * 0.45;
			snow += `M${x} ${top}L${x - hw * 0.46} ${cy}L${x - hw * 0.2} ${cy - 2}L${x} ${cy + 3}L${x + hw * 0.25} ${cy - 2}L${x + hw * 0.46} ${cy}Z`;
		}
		return { body, snow, trunk: [x - w * 0.07, base - 2, w * 0.14, 10] };
	}
	const firs = [
		fir(985, 392, 70, 24),
		fir(1018, 396, 100, 32),
		fir(1066, 392, 62, 22),
		fir(1215, 394, 86, 28),
		fir(1262, 394, 112, 36),
		fir(1288, 398, 66, 22)
	];
	/** Peaks as triangles with a snow cap of the same slope. */
	const peaks = [
		{ x: 1030, y: 214, b: 340, f: 'm2' },
		{ x: 1120, y: 176, b: 340, f: 'm1' },
		{ x: 1215, y: 232, b: 340, f: 'm2' }
	].map((p) => {
		const s = 0.62;
		const cap = 46;
		return {
			f: p.f,
			d: `M${p.x - (p.b - p.y) * s} ${p.b}L${p.x} ${p.y}L${p.x + (p.b - p.y) * s} ${p.b}Z`,
			snow: `M${p.x} ${p.y}L${p.x - cap * s} ${p.y + cap}L${p.x - cap * 0.25} ${p.y + cap - 8}L${p.x} ${p.y + cap + 6}L${p.x + cap * 0.3} ${p.y + cap - 10}L${p.x + cap * s} ${p.y + cap}Z`
		};
	});
</script>

<div class="scene scene-chalet lv{level}" aria-hidden="true">
	<svg
		viewBox="560 80 1220 540"
		preserveAspectRatio="xMidYMid slice"
		fill="none"
		focusable="false"
		class:p-down={playing?.kind === 'down'}
		class:p-up={playing?.kind === 'up'}
		class:p-resolved={playing?.kind === 'resolved'}
	>
		<defs>
			<pattern id="ch-logs" width="60" height="38" patternUnits="userSpaceOnUse">
				<rect width="60" height="38" fill="var(--wall)" />
				<rect y="29" width="60" height="9" fill="var(--wall2)" />
				<path d="M0 1H60" stroke="var(--wall3)" stroke-width="2" />
			</pattern>
			<pattern id="ch-stone" width="46" height="26" patternUnits="userSpaceOnUse">
				<rect width="46" height="26" fill="var(--stone)" />
				<path
					d="M0 .5H46M0 13.5H46M10 .5V13.5M33 13.5V26"
					stroke="var(--mortar)"
					stroke-width="1.6"
					opacity=".55"
				/>
			</pattern>
			<linearGradient id="ch-sky" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--sky1)" />
				<stop offset="1" stop-color="var(--sky2)" />
			</linearGradient>
			<radialGradient id="ch-glow" cx=".5" cy=".5" r=".5">
				<stop offset="0" stop-color="var(--glow)" stop-opacity=".55" />
				<stop offset="1" stop-color="var(--glow)" stop-opacity="0" />
			</radialGradient>
			<clipPath id="ch-win"><rect x="962" y="152" width="336" height="246" /></clipPath>
		</defs>

		<!-- room -->
		<rect x="560" y="80" width="1220" height="445" fill="url(#ch-logs)" />
		<rect x="560" y="80" width="1220" height="22" fill="var(--beam)" />
		<rect x="560" y="520" width="1220" height="100" fill="var(--floor)" />
		<path d="M560 548H1780M560 580H1780M560 610H1780" stroke="var(--floorline)" stroke-width="2" />
		<rect x="560" y="510" width="1220" height="11" fill="var(--beam)" />
		<rect x="1100" y="80" width="14" height="440" fill="var(--beam)" opacity=".5" />

		<!-- window onto the night -->
		<g clip-path="url(#ch-win)">
			<rect x="962" y="152" width="336" height="246" fill="url(#ch-sky)" />
			<g fill="#fff" opacity="var(--star-op)">
				<circle cx="990" cy="176" r="1.3" /><circle cx="1040" cy="196" r="1" /><circle
					cx="1180"
					cy="170"
					r="1.4"
				/>
				<circle cx="1262" cy="190" r="1" /><circle cx="1090" cy="164" r="1" /><circle
					cx="1236"
					cy="214"
					r="1.2"
				/>
			</g>
			<circle cx="1250" cy="188" r="15" fill="var(--moon)" opacity="var(--star-op)" />
			{#each peaks as p}
				<path d={p.d} fill="var(--{p.f})" />
				<path d={p.snow} fill="var(--snow)" opacity=".9" />
			{/each}
			<path d="M962 350Q1060 322 1130 346T1298 334V398H962Z" fill="var(--snow)" opacity=".85" />
			{#each firs as f}
				<rect
					x={f.trunk[0]}
					y={f.trunk[1]}
					width={f.trunk[2]}
					height={f.trunk[3]}
					fill="var(--trunk)"
				/>
				<path d={f.body} fill="var(--fir)" />
				<path d={f.snow} fill="var(--snow)" />
			{/each}
			<path d="M962 380Q1040 368 1110 380T1298 376V398H962Z" fill="var(--snow)" />
			<g class="clouds">
				<g class="cloud c1"><ellipse cx="1040" cy="190" rx="46" ry="12" /><ellipse cx="1070" cy="182" rx="30" ry="12" /></g>
				<g class="cloud c2"><ellipse cx="1220" cy="214" rx="54" ry="13" /><ellipse cx="1190" cy="208" rx="30" ry="11" /></g>
				<g class="cloud c3"><ellipse cx="1120" cy="166" rx="70" ry="15" /><ellipse cx="1160" cy="160" rx="36" ry="13" /></g>
				<g class="cloud c4"><ellipse cx="1010" cy="168" rx="80" ry="18" /><ellipse cx="1130" cy="196" rx="90" ry="16" /><ellipse cx="1250" cy="170" rx="80" ry="18" /></g>
			</g>
			{#each shown as f}
				<circle
					class="fl"
					cx={f.x}
					cy="152"
					r={f.r}
					fill="var(--snow)"
					style="--d:{f.d}s;--dl:{f.dl}s;--k:{f.k}"
				/>
			{/each}
			<g class="bolt">
				<rect x="962" y="152" width="336" height="246" fill="#fff" />
				<path d="M1150 152L1128 238L1146 236L1118 330L1164 224L1146 226Z" fill="#fff6c8" />
			</g>
			<rect class="chill" x="962" y="152" width="336" height="246" fill="#0a1226" />
		</g>
		<path d="M946 136H1000Q988 200 996 306Q968 312 946 304Z" fill="var(--curtain)" />
		<path d="M1264 136H1314V304Q1290 312 1262 306Q1272 200 1264 136Z" fill="var(--curtain)" />
		<path d="M966 146Q976 220 970 300M1288 146Q1282 220 1292 300" stroke="var(--curtain2)" stroke-width="3" />
		<rect x="950" y="140" width="360" height="12" fill="var(--beam)" />
		<path d="M950 152H962V398H950ZM1298 152H1310V398H1298Z" fill="var(--beam)" />
		<rect x="1126" y="152" width="8" height="246" fill="var(--beam)" />
		<rect x="962" y="270" width="336" height="8" fill="var(--beam)" />
		<rect x="938" y="396" width="384" height="16" fill="var(--beam)" />
		<path d="M950 396Q980 384 1010 396" stroke="var(--snow)" stroke-width="5" stroke-linecap="round" opacity=".85" />

		<!-- fireplace -->
		<rect x="1410" y="102" width="260" height="418" fill="url(#ch-stone)" />
		<rect x="1380" y="336" width="320" height="22" fill="var(--beam)" />
		<rect x="1390" y="516" width="300" height="12" fill="var(--stone2)" />
		<path d="M1440 520V434Q1440 386 1540 386Q1640 386 1640 434V520Z" fill="var(--stone2)" />
		<path d="M1454 520V436Q1454 400 1540 400Q1626 400 1626 436V520Z" fill="var(--firebox)" />
		<g class="glow"><ellipse cx="1540" cy="470" rx="95" ry="55" fill="url(#ch-glow)" /></g>
		<g class="flames">
			<path class="flame f1" d="M1500 506C1480 478 1500 462 1504 436C1522 456 1530 476 1522 506Z" fill="var(--flame1)" />
			<path class="flame f2" d="M1522 506C1506 470 1534 452 1542 414C1566 446 1574 478 1560 506Z" fill="var(--flame1)" />
			<path class="flame f3" d="M1556 506C1550 476 1570 466 1580 440C1596 462 1598 486 1586 506Z" fill="var(--flame1)" />
			<path class="flame f2" d="M1530 506C1522 482 1540 470 1544 448C1558 468 1562 488 1554 506Z" fill="var(--flame2)" />
			<path class="flame f1" d="M1538 506C1534 490 1544 484 1546 472C1554 484 1556 496 1550 506Z" fill="var(--flame3)" />
		</g>
		<rect x="1488" y="496" width="106" height="16" rx="8" fill="var(--log)" transform="rotate(-6 1540 504)" />
		<rect x="1494" y="504" width="100" height="14" rx="7" fill="var(--log2)" transform="rotate(5 1540 510)" />
		{#key playing?.id}
			{#if playing?.kind === 'up'}
				<g fill="var(--flame2)">
					{#each [-34, -18, -4, 10, 24, 38, 50] as dx, i}
						<circle class="spark" cx={1540 + dx / 2} cy="440" r="2.2" style="--sx:{dx}px;--sd:{i * 0.12}s" />
					{/each}
				</g>
			{/if}
		{/key}
		<path d="M1388 334Q1450 360 1540 346T1692 334" stroke="#2f6b43" stroke-width="7" stroke-linecap="round" />
		<g fill="#c8342f"><circle cx="1430" cy="347" r="3.5" /><circle cx="1540" cy="348" r="3.5" /><circle cx="1650" cy="345" r="3.5" /></g>
		<path d="M1452 358h20l-3 36q-2 8-9 6t-8-8Z" fill="#b5383a" />
		<path d="M1604 358h20l-3 36q-2 8-9 6t-8-8Z" fill="#3f7a4f" />

		<!-- rug and the sleeping cat -->
		<ellipse cx="1170" cy="568" rx="206" ry="36" fill="var(--rug)" />
		<ellipse cx="1170" cy="568" rx="176" ry="27" stroke="var(--rug2)" stroke-width="4" />
		<ellipse cx="1170" cy="568" rx="140" ry="19" stroke="var(--rug2)" stroke-width="2" opacity=".7" />
		{#key playing?.id}
			<g class="cat">
				<path d="M1218 556Q1244 556 1236 574Q1224 586 1170 584" stroke="var(--cat2)" stroke-width="12" stroke-linecap="round" />
				<ellipse cx="1172" cy="554" rx="54" ry="24" fill="var(--cat)" />
				<path d="M1150 534q6 12 0 22M1170 531q6 14 0 26M1190 532q6 14 0 24" stroke="var(--cat2)" stroke-width="4" stroke-linecap="round" />
				<circle cx="1128" cy="552" r="17" fill="var(--cat)" />
				<path d="M1114 541l-2-17 14 9ZM1137 535l8-15 4 17Z" fill="var(--cat)" />
				<path class="eyes-shut" d="M1118 551q4 4 8 0M1131 551q4 4 8 0" stroke="var(--cat2)" stroke-width="2" stroke-linecap="round" />
				<g class="eyes-open">
					<ellipse cx="1122" cy="549" rx="3.5" ry="4.5" fill="#d9e36a" /><ellipse cx="1135" cy="549" rx="3.5" ry="4.5" fill="#d9e36a" />
					<path d="M1122 546v6M1135 546v6" stroke="#1a1410" stroke-width="2" />
				</g>
				<path d="M1127 558l-3 3h6Z" fill="#e9a3a0" />
			</g>
		{/key}

		<!-- hot chocolate on a side table -->
		<rect x="748" y="472" width="104" height="9" rx="3" fill="var(--table)" />
		<path d="M760 481V522M840 481V522M760 500H840" stroke="var(--table)" stroke-width="5" />
		<path d="M826 446q22 0 22 16t-22 16" stroke="var(--mug)" stroke-width="5" />
		<path d="M782 438h46l-4 36q-1 6-8 6h-22q-7 0-8-6Z" fill="var(--mug)" />
		<ellipse cx="805" cy="439" rx="23" ry="5" fill="var(--cocoa)" />
		<circle cx="797" cy="437" r="5" fill="#fff4e6" /><circle cx="812" cy="438" r="4.5" fill="#fff4e6" />
		<g class="steam" stroke="var(--cloud)" stroke-width="3" stroke-linecap="round">
			<path class="s1" d="M795 424q-8-10 0-20t0-18" />
			<path class="s2" d="M806 424q8-10 0-20t0-18" />
			<path class="s3" d="M817 424q-8-10 0-20t0-18" />
		</g>
		{#key playing?.id}
			{#if playing?.kind === 'resolved'}
				<path class="heart" d="M806 410c-18-14-28-26-16-32 7-3 13 1 16 7 3-6 9-10 16-7 12 6 2 18-16 32Z" fill="var(--cloud)" />
			{/if}
		{/key}
	</svg>
</div>

<style>
	.scene-chalet {
		--wall: #c9a27a;
		--wall2: #b78d63;
		--wall3: #dfbb92;
		--beam: #7b5233;
		--floor: #9a6b43;
		--floorline: #7d5334;
		--stone: #9c968c;
		--stone2: #7e786f;
		--mortar: #5e5a53;
		--firebox: #2b1d17;
		--glow: #ffb35c;
		--flame1: #ee6a1f;
		--flame2: #ffc04a;
		--flame3: #fff0b0;
		--log: #5a3a26;
		--log2: #6e4a30;
		--rug: #a8433a;
		--rug2: #e2c08e;
		--cat: #d08a45;
		--cat2: #8e5726;
		--curtain: #b5383a;
		--curtain2: #8e2a2e;
		--sky1: #3f5f8f;
		--sky2: #aec3dc;
		--m1: #8da3be;
		--m2: #a3b6cc;
		--snow: #f5f9ff;
		--fir: #2f5a46;
		--trunk: #4a3426;
		--cloud: #eef2f8;
		--moon: #fff;
		--star-op: 0;
		--mug: #e8e0d2;
		--cocoa: #5a3322;
		--table: #6c4528;
		--cloud-fill: #dfe7f1;
	}
	:global(.dark) .scene-chalet {
		--wall: #4a3224;
		--wall2: #3d281c;
		--wall3: #5c4130;
		--beam: #2c1b12;
		--floor: #3a2517;
		--floorline: #281910;
		--stone: #4f4c4a;
		--stone2: #3a3836;
		--mortar: #1c1a19;
		--glow: #ff9a3c;
		--rug: #6e2b2b;
		--rug2: #a88660;
		--cat: #a8682f;
		--cat2: #5f3a18;
		--curtain: #7a2528;
		--curtain2: #5a1a1d;
		--sky1: #070d22;
		--sky2: #1a2a4e;
		--m1: #1f2f52;
		--m2: #2c4068;
		--snow: #dce8f8;
		--fir: #0f2a22;
		--trunk: #2a1d16;
		--cloud: #aab7d0;
		--moon: #eef3fb;
		--star-op: 0.9;
		--mug: #cfc6b6;
		--table: #3f2816;
		--cloud-fill: #2b3a5c;
	}
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: var(--wall);
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
	}
	/* weather outside the window */
	.cloud {
		fill: var(--cloud-fill);
		opacity: 0;
		transition: opacity 1.2s;
		animation: drift 40s ease-in-out infinite alternate;
	}
	.c2 {
		animation-duration: 55s;
		animation-direction: alternate-reverse;
	}
	.c3 {
		animation-duration: 48s;
	}
	.lv0 .c3 {
		opacity: 0.3;
	}
	.lv1 .c1,
	.lv1 .c2,
	.lv1 .c3,
	.lv2 .cloud {
		opacity: 0.9;
	}
	.lv1 .c4 {
		opacity: 0.8;
	}
	.lv1 .cloud {
		fill: #9aa6bd;
	}
	:global(.dark) .lv1 .cloud {
		fill: #232c48;
	}
	.lv2 .cloud {
		fill: #2f3a54;
		opacity: 1;
	}
	:global(.dark) .lv2 .cloud {
		fill: #0e1426;
	}
	.chill {
		opacity: 0;
		transition: opacity 1.2s;
	}
	.lv1 .chill {
		opacity: 0.26;
	}
	.lv2 .chill {
		opacity: 0.5;
	}
	.lv0 {
		--w: 10px;
	}
	.lv1 {
		--w: -30px;
	}
	.lv2 {
		--w: -90px;
	}
	.fl {
		animation: fall var(--d) linear infinite;
		animation-delay: var(--dl);
	}
	@keyframes fall {
		from {
			transform: translate(0, -10px);
		}
		to {
			transform: translate(calc(var(--w) * var(--k)), 260px);
		}
	}
	@keyframes drift {
		to {
			transform: translateX(34px);
		}
	}
	.bolt {
		opacity: 0;
	}
	.lv2 .bolt {
		animation: bolt 8s infinite;
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
	/* fire */
	.flame,
	.glow,
	.cat,
	.steam path,
	.flames,
	.spark,
	.heart {
		transform-box: fill-box;
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
	/* the cat breathes, and wakes in a storm */
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
	.lv2 .eyes-open {
		opacity: 1;
	}
	.lv2 .eyes-shut {
		opacity: 0;
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
			opacity: 0.12;
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
			transform: translate(var(--sx), -70px);
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
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
		.fl {
			display: none;
		}
	}
</style>
