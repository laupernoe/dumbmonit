<script lang="ts">
	/**
	 * Dubai at dusk — the Burj Khalifa's stepped, needle-thin tower amid
	 * skyscrapers, the Burj Al Arab sail on the gulf and a dune on the shore.
	 * Static, no animation. Follows the scene contract: fills its positioned
	 * parent, the headline zone (top left) is left clear, the palette lives here.
	 */
	// Burj Khalifa: setback tiers, [half width at the base, top y, half width at the top]
	const tiers = [
		[34, 400, 30],
		[28, 330, 24],
		[22, 270, 18],
		[17, 214, 13],
		[12, 168, 8],
		[7, 130, 4]
	];
	const bx = 1130;
	const base = 484;
	const tierPath = tiers
		.map(([w, y], i) => {
			const prev = i === 0 ? base : tiers[i - 1][1];
			return `M${bx - w} ${prev}L${bx - w} ${y}L${bx + w} ${y}L${bx + w} ${prev}Z`;
		})
		.join('');
</script>

<div class="scene scene-dubai" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="db-wat" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--water1)" />
				<stop offset="1" stop-color="var(--water2)" />
			</linearGradient>
			<linearGradient id="db-glass" x1="0" y1="0" x2="1" y2="0">
				<stop offset="0" stop-color="var(--glass1)" />
				<stop offset="1" stop-color="var(--glass2)" />
			</linearGradient>
		</defs>
		<circle cx="1000" cy="250" r="44" fill="var(--sun)" opacity=".9" />
		<g fill="var(--cloud)" opacity=".7">
			<ellipse cx="1330" cy="140" rx="80" ry="11" />
			<ellipse cx="1370" cy="130" rx="42" ry="10" />
			<ellipse cx="1620" cy="230" rx="70" ry="10" />
		</g>
		<!-- haze layer: far towers -->
		<g fill="var(--far)" opacity=".75">
			<rect x="760" y="430" width="22" height="54" />
			<rect x="786" y="404" width="26" height="80" />
			<rect x="816" y="440" width="20" height="44" />
			<rect x="1460" y="400" width="24" height="84" />
			<rect x="1488" y="384" width="20" height="100" />
			<rect x="1512" y="420" width="28" height="64" />
			<rect x="1546" y="396" width="22" height="88" />
			<rect x="1572" y="430" width="30" height="54" />
		</g>
		<!-- mid towers -->
		<g>
			<rect x="900" y="360" width="38" height="124" fill="url(#db-glass)" />
			<rect x="944" y="320" width="30" height="164" fill="var(--tower2)" />
			<rect x="980" y="388" width="34" height="96" fill="url(#db-glass)" />
			<!-- Emirates towers: two triangular tops -->
			<path d="M1240 484V280L1258 244L1276 280V484Z" fill="var(--tower)" />
			<path d="M1284 484V310L1300 278L1316 310V484Z" fill="var(--tower2)" />
			<path d="M1258 244V232" stroke="var(--tower)" stroke-width="3" />
			<path d="M1300 278V266" stroke="var(--tower2)" stroke-width="3" />
			<!-- twisted tower -->
			<path d="M1340 484V330L1346 322L1372 336L1368 380L1380 420L1376 484Z" fill="var(--tower)" />
			<path d="M1346 322L1358 318L1372 336Z" fill="var(--glass2)" />
			<rect x="1392" y="372" width="34" height="112" fill="url(#db-glass)" />
			<rect x="1430" y="414" width="26" height="70" fill="var(--tower2)" />
		</g>
		<!-- window grid on mid towers -->
		<g class="win" opacity=".6">
			<rect x="906" y="372" width="3" height="3" /><rect x="914" y="372" width="3" height="3" /><rect x="922" y="372" width="3" height="3" />
			<rect x="906" y="388" width="3" height="3" /><rect x="922" y="404" width="3" height="3" /><rect x="914" y="420" width="3" height="3" />
			<rect x="952" y="340" width="3" height="3" /><rect x="962" y="356" width="3" height="3" /><rect x="952" y="388" width="3" height="3" /><rect x="962" y="420" width="3" height="3" />
			<rect x="1248" y="320" width="3" height="3" /><rect x="1262" y="352" width="3" height="3" /><rect x="1250" y="400" width="3" height="3" /><rect x="1292" y="350" width="3" height="3" /><rect x="1304" y="390" width="3" height="3" />
			<rect x="1400" y="388" width="3" height="3" /><rect x="1414" y="420" width="3" height="3" />
		</g>
		<!-- Burj Khalifa -->
		<g>
			<path d={tierPath} fill="var(--burj)" />
			<path d="M{bx} 484V130" stroke="var(--burj2)" stroke-width="3" />
			<path d="M{bx - 34} 484V400M{bx + 34} 484V400M{bx - 28} 400V330M{bx + 28} 400V330M{bx - 22} 330V270M{bx + 22} 330V270" stroke="var(--burj2)" stroke-width="2" />
			<rect x={bx - 1.5} y="86" width="3" height="46" fill="var(--burj)" />
			<g fill="var(--burj2)" opacity=".5">
				<rect x={bx + 4} y="400" width="30" height="84" />
				<rect x={bx + 4} y="330" width="24" height="70" />
				<rect x={bx + 3} y="270" width="19" height="60" />
				<rect x={bx + 2} y="214" width="15" height="56" />
				<rect x={bx + 1} y="168" width="11" height="46" />
			</g>
		</g>
		<!-- Burj Al Arab on the gulf -->
		<g transform="translate(1478 0)">
			<path d="M0 484C-4 430-2 380 6 330C22 360 38 340 44 330C32 380 34 430 36 484Z" fill="var(--sail)" />
			<path d="M6 330C20 410 26 440 36 484" stroke="var(--sail2)" stroke-width="3" />
			<rect x="4" y="326" width="2" height="22" fill="var(--sail2)" />
			<ellipse cx="18" cy="486" rx="40" ry="4" fill="var(--sail2)" opacity=".7" />
		</g>
		<!-- shore + dune -->
		<path d="M560 482C640 470 720 474 780 482V490H560Z" fill="var(--sand2)" />
		<path d="M1640 490C1670 462 1730 452 1780 458V490Z" fill="var(--sand)" />
		<!-- water -->
		<rect x="540" y="484" width="1260" height="160" fill="url(#db-wat)" />
		<g stroke="var(--wh)" stroke-width="2" opacity=".5" stroke-linecap="round">
			<path d="M620 520h70M790 540h100M1180 536h90M1330 560h110M1500 530h80M1640 552h90M720 580h80M940 574h120" />
		</g>
		<g opacity=".18" transform="translate(0 968) scale(1 -1)">
			<rect x="1096" y="484" width="68" height="110" fill="var(--burj)" />
			<rect x="1240" y="484" width="80" height="70" fill="var(--tower)" />
			<rect x="900" y="484" width="110" height="50" fill="var(--tower2)" />
		</g>
		<!-- dhow -->
		<g transform="translate(1210 540)">
			<path d="M0 0h74c-4 8-12 12-22 12H14C6 12 2 6 0 0Z" fill="var(--dhow)" />
			<path d="M30 0V-36L58 -4Z" fill="var(--wh)" />
			<path d="M30 -36V0" stroke="var(--dhow)" stroke-width="2" />
		</g>
		<!-- status buoy -->
		<g>
			<circle cx="1010" cy="566" r="9" fill="var(--scene-tone, var(--c-signal))" />
			<circle cx="1010" cy="566" r="22" fill="var(--scene-tone, var(--c-signal))" opacity=".25" />
			<rect x="1006" y="574" width="8" height="10" fill="var(--burj2)" />
		</g>
	</svg>
</div>

<style>
	.scene-dubai {
		--sky1: #f2b58f;
		--sky2: #fde6c8;
		--sun: #ffd37a;
		--water1: #78b8c4;
		--water2: #3f8ea4;
		--burj: #aebdd0;
		--burj2: #7488a3;
		--tower: #8fa3bb;
		--tower2: #b6c4d6;
		--glass1: #8fb4cc;
		--glass2: #c4dae6;
		--far: #c7bcb8;
		--sail: #fffaf0;
		--sail2: #d8cdbd;
		--sand: #e2b779;
		--sand2: #d3a56c;
		--dhow: #9a5b34;
		--wh: #fff;
		--win: #fff;
		--cloud: #fff;
	}
	:global(.dark) .scene-dubai {
		--sky1: #0d1630;
		--sky2: #22284e;
		--sun: #e8eef8;
		--water1: #15294a;
		--water2: #0a1428;
		--burj: #6f7ea0;
		--burj2: #4a5679;
		--tower: #44517a;
		--tower2: #55628c;
		--glass1: #3b4a74;
		--glass2: #5b6b96;
		--far: #2a3558;
		--sail: #aab2c8;
		--sail2: #7b84a0;
		--sand: #5b4d57;
		--sand2: #46404f;
		--dhow: #5b3f3c;
		--wh: #8b92a8;
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
</style>
