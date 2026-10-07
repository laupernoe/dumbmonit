<script lang="ts">
	/**
	 * Sydney Harbour in the morning — the Opera House's white sails in front,
	 * the Harbour Bridge arch behind, a few towers of the city and the bay.
	 * Static, no animation. Follows the scene contract: fills its positioned
	 * parent, the headline zone (top left) is left clear, the palette lives here.
	 */
	const cx = 1370;
	const half = 190;
	const deck = 446;
	const top = 292;
	const upper = (x: number) => top + ((x - cx) / half) ** 2 * (deck - top);
	const lower = (x: number) => top + 16 + ((x - cx) / half) ** 2 * (deck - top - 16);
	const xs = Array.from({ length: 29 }, (_, i) => cx - half + 20 + i * ((2 * half - 40) / 28));
	const pts = (f: (x: number) => number) =>
		Array.from({ length: 41 }, (_, i) => {
			const x = cx - half + (i * 2 * half) / 40;
			return `${x.toFixed(1)},${f(x).toFixed(1)}`;
		}).join(' ');
	const upperPts = pts(upper);
	const lowerPts = pts(lower);
	const diag = xs.slice(0, -1).map((x, i) => `${x.toFixed(1)},${lower(x).toFixed(1)} ${xs[i + 1].toFixed(1)},${upper(xs[i + 1]).toFixed(1)}`);
</script>

<div class="scene scene-sydney" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="sy-wat" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--water1)" />
				<stop offset="1" stop-color="var(--water2)" />
			</linearGradient>
			<path id="sy-sail" d="M0 0C4-46 34-96 92-128C96-92 94-40 90 0Z" />
		</defs>
		<circle cx="1000" cy="190" r="40" fill="var(--sun)" opacity=".9" />
		<g fill="var(--cloud)" opacity=".7">
			<ellipse cx="1180" cy="150" rx="80" ry="12" />
			<ellipse cx="1220" cy="140" rx="46" ry="12" />
			<ellipse cx="1600" cy="210" rx="70" ry="10" />
		</g>
		<!-- far city -->
		<g fill="var(--far)" opacity=".8">
			<rect x="1180" y="400" width="26" height="82" />
			<rect x="1210" y="372" width="30" height="110" />
			<rect x="1244" y="410" width="26" height="72" />
			<rect x="1276" y="384" width="24" height="98" />
			<rect x="1304" y="420" width="28" height="62" />
			<rect x="1236" y="316" width="3" height="56" />
			<path d="M1228 372h20l-3-18h-14zM1232 354h12l-2-10h-8z" />
		</g>
		<!-- bridge -->
		<g>
			<polyline points={upperPts} stroke="var(--steel)" stroke-width="7" stroke-linejoin="round" />
			<polyline points={lowerPts} stroke="var(--steel)" stroke-width="5" stroke-linejoin="round" />
			<g stroke="var(--steel)" stroke-width="2">
				{#each xs as x}
					<line x1={x} y1={lower(x)} x2={x} y2={deck} />
				{/each}
			</g>
			<g stroke="var(--steel)" stroke-width="1.6" opacity=".8">
				{#each diag as d}
					<polyline points={d} />
				{/each}
			</g>
			<rect x={cx - half - 30} y={deck - 4} width={2 * half + 60} height="10" fill="var(--steel)" />
			<rect x={cx - half - 30} y={deck + 6} width={2 * half + 60} height="3" fill="var(--steel2)" />
			<!-- pylons -->
			<g fill="var(--stone)">
				<path d="M{cx - half - 18} {deck + 30}V{deck - 70}h26V{deck + 30}z" />
				<path d="M{cx + half - 8} {deck + 30}V{deck - 70}h26V{deck + 30}z" />
			</g>
			<g fill="var(--stone2)">
				<rect x={cx - half - 22} y={deck - 76} width="34" height="8" />
				<rect x={cx + half - 12} y={deck - 76} width="34" height="8" />
			</g>
			<!-- approach piers -->
			<g fill="var(--stone2)">
				<rect x={cx - half - 30} y={deck + 8} width="10" height="40" />
				<rect x={cx + half + 20} y={deck + 8} width="10" height="40" />
			</g>
		</g>
		<!-- far shore -->
		<path d="M1560 484C1600 460 1680 456 1780 466V490H1560Z" fill="var(--hill)" />
		<path d="M560 484C700 470 840 476 980 478V490H560Z" fill="var(--hill)" opacity=".7" />
		<!-- water -->
		<rect x="540" y="484" width="1260" height="160" fill="url(#sy-wat)" />
		<g stroke="var(--wh)" stroke-width="2" opacity=".5" stroke-linecap="round">
			<path d="M620 520h70M780 540h100M1180 532h90M1330 556h110M1500 530h80M1640 552h90M720 580h80M940 574h120" />
		</g>
		<g opacity=".18" transform="translate(0 968) scale(1 -1)">
			<rect x="1000" y="484" width="300" height="40" fill="var(--sail)" />
			<rect x="1180" y="484" width="150" height="30" fill="var(--far)" />
		</g>
		<!-- Opera House -->
		<g>
			<rect x="860" y="468" width="360" height="22" fill="var(--stone)" />
			<rect x="860" y="486" width="360" height="6" fill="var(--stone2)" />
			<!-- far group of sails (smaller, behind) -->
			<g fill="var(--sail2)">
				<use href="#sy-sail" transform="translate(1010 468) scale(.8 .85)" />
				<use href="#sy-sail" transform="translate(1058 468) scale(.8 .7)" />
			</g>
			<!-- main hall -->
			<g fill="var(--sail)">
				<use href="#sy-sail" transform="translate(880 468) scale(.85 .8)" />
				<use href="#sy-sail" transform="translate(920 468) scale(.95 1.0)" />
				<use href="#sy-sail" transform="translate(962 468) scale(1.05 1.25)" />
				<use href="#sy-sail" transform="translate(1010 468) scale(1.1 1.5)" />
			</g>
			<g fill="var(--sail)">
				<use href="#sy-sail" transform="translate(1100 468) scale(.7 .75)" />
				<use href="#sy-sail" transform="translate(1136 468) scale(.8 1)" />
				<use href="#sy-sail" transform="translate(1176 468) scale(.8 .6)" />
			</g>
			<!-- rib shading -->
			<g stroke="var(--sail2)" stroke-width="2" opacity=".8">
				<path d="M920 468C935 420 980 380 1010 352M962 468C980 410 1040 340 1100 300M1010 468C1040 400 1090 340 1110 284" />
			</g>
			<g fill="var(--glass)" opacity=".85">
				<path d="M884 468h36v-12z" />
				<path d="M1100 468h38l-38-30z" />
			</g>
		</g>
		<!-- ferry -->
		<g transform="translate(1260 520)">
			<path d="M0 0h70l-8 12H8z" fill="var(--ferry)" />
			<rect x="12" y="-12" width="44" height="12" fill="var(--wh)" />
			<rect x="22" y="-22" width="22" height="10" fill="var(--wh)" />
			<rect x="14" y="-9" width="40" height="3" fill="var(--glass)" />
		</g>
		<!-- status buoy -->
		<g>
			<circle cx="1010" cy="566" r="9" fill="var(--scene-tone, var(--c-signal))" />
			<circle cx="1010" cy="566" r="22" fill="var(--scene-tone, var(--c-signal))" opacity=".25" />
			<rect x="1006" y="574" width="8" height="10" fill="var(--steel)" />
		</g>
	</svg>
</div>

<style>
	.scene-sydney {
		--sky1: #f3c9a6;
		--sky2: #fdeed6;
		--sun: #ffdd9a;
		--water1: #7fb7c9;
		--water2: #4f8fab;
		--stone: #d8c19c;
		--stone2: #b99f78;
		--sail: #fbf6ec;
		--sail2: #dcd2c2;
		--glass: #4d7a96;
		--steel: #5a6678;
		--steel2: #3d4656;
		--far: #a9b4c6;
		--hill: #7da07a;
		--ferry: #d8553b;
		--wh: #fff;
		--cloud: #fff;
	}
	:global(.dark) .scene-sydney {
		--sky1: #0d1630;
		--sky2: #1b2a4c;
		--sun: #e8eef8;
		--water1: #15294a;
		--water2: #0a1428;
		--stone: #5d6682;
		--stone2: #454e69;
		--sail: #aab2c8;
		--sail2: #7b84a0;
		--glass: #f5c76a;
		--steel: #4c5775;
		--steel2: #2f3855;
		--far: #2c3a5e;
		--hill: #1c3a3c;
		--ferry: #a1473c;
		--wh: #8b92a8;
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
</style>
