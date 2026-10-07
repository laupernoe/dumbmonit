<script lang="ts">
	/**
	 * Machu Picchu — the Inca citadel in the cloud forest: stone terraces
	 * and thatched houses on the ridge, Huayna Picchu rising behind, Andean
	 * peaks fading into the mist and a llama grazing. Static, no animation.
	 * Follows the scene contract: fills its positioned parent, the headline
	 * zone (top left) is left clear, the palette lives here.
	 */
</script>

<div class="scene scene-machupicchu" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<clipPath id="mp-hill"><path d="M740 620C800 500 940 380 1100 366C1260 360 1400 430 1520 620z" /></clipPath>
		</defs>

		<!-- astre, étoiles, nuages -->
		<circle cx="1560" cy="170" r="46" fill="var(--sun)" />
		<circle cx="1546" cy="158" r="7" fill="var(--crater)" /><circle cx="1574" cy="184" r="5" fill="var(--crater)" />
		<g fill="var(--star)"><circle cx="1010" cy="118" r="2" /><circle cx="1160" cy="150" r="1.8" /><circle cx="1400" cy="110" r="2.2" /><circle cx="1700" cy="120" r="2" /><circle cx="1330" cy="190" r="1.8" /><circle cx="860" cy="210" r="1.8" /></g>
		<g fill="var(--cloud)" opacity=".8"><ellipse cx="1000" cy="170" rx="80" ry="11" /><ellipse cx="1050" cy="188" rx="50" ry="8" /><ellipse cx="1420" cy="140" rx="70" ry="10" /><ellipse cx="1700" cy="260" rx="60" ry="8" /></g>

		<!-- cordillère lointaine dans la brume -->
		<path d="M560 482V300L640 250L700 290L780 230L860 300L940 270L1010 330L1060 482z" fill="var(--mp-far)" />
		<path d="M1380 482L1450 300L1520 260L1580 300L1650 240L1720 290L1780 270V482z" fill="var(--mp-far)" />
		<path d="M640 250L670 270L700 290L650 296zM780 230L810 262L760 266zM1650 240L1680 264L1630 268zM1520 260L1545 280L1500 282z" fill="var(--mp-snow)" />
		<path d="M560 482V360C620 340 680 352 740 336C800 320 850 346 900 360C950 372 1000 380 1060 400V482z" fill="var(--mp-mid)" />
		<path d="M1400 482V380C1460 350 1520 360 1580 340C1640 320 1700 346 1780 360V482z" fill="var(--mp-mid)" />
		<g fill="var(--mp-mist)" opacity=".55"><ellipse cx="900" cy="372" rx="170" ry="16" /><ellipse cx="1560" cy="378" rx="190" ry="16" /><ellipse cx="760" cy="330" rx="110" ry="9" /></g>

		<!-- Huayna Picchu -->
		<path d="M1060 520C1096 400 1146 316 1196 220C1214 188 1230 170 1248 170C1266 172 1278 196 1292 232C1322 304 1366 404 1410 520z" fill="var(--mp-peak)" />
		<path d="M1248 170C1266 172 1278 196 1292 232C1322 304 1366 404 1410 520H1290C1270 400 1260 280 1248 170z" fill="var(--mp-peak2)" opacity=".75" />
		<path d="M1196 280C1208 300 1214 330 1206 380M1154 360C1170 380 1176 420 1170 470M1316 320C1308 350 1316 390 1336 430M1246 220C1240 260 1232 310 1234 360" stroke="var(--mp-peak2)" stroke-width="5" stroke-linecap="round" opacity=".7" />
		<path d="M1230 196C1240 182 1256 182 1266 198C1254 204 1240 204 1230 196z" fill="var(--mp-grass)" />
		<g fill="var(--mp-mist)" opacity=".6"><ellipse cx="1180" cy="400" rx="150" ry="14" /><ellipse cx="1330" cy="440" rx="120" ry="12" /><ellipse cx="1240" cy="300" rx="70" ry="7" /></g>

		<!-- citadelle : colline, terrasses, maisons -->
		<g clip-path="url(#mp-hill)">
			<rect x="740" y="360" width="800" height="270" fill="var(--mp-grass)" />
			<path d="M740 406C900 360 1260 356 1520 428V452H740z" fill="var(--mp-grass)" /><path d="M740 406C900 360 1260 356 1520 428" stroke="var(--mp-stone)" stroke-width="6" /><path d="M740 428C900 382 1260 378 1520 450V474H740z" fill="var(--mp-grass2)" /><path d="M740 428C900 382 1260 378 1520 450" stroke="var(--mp-stone)" stroke-width="6" /><path d="M740 450C900 404 1260 400 1520 472V496H740z" fill="var(--mp-grass)" /><path d="M740 450C900 404 1260 400 1520 472" stroke="var(--mp-stone)" stroke-width="6" /><path d="M740 472C900 426 1260 422 1520 494V518H740z" fill="var(--mp-grass2)" /><path d="M740 472C900 426 1260 422 1520 494" stroke="var(--mp-stone)" stroke-width="6" /><path d="M740 494C900 448 1260 444 1520 516V540H740z" fill="var(--mp-grass)" /><path d="M740 494C900 448 1260 444 1520 516" stroke="var(--mp-stone)" stroke-width="6" /><path d="M740 516C900 470 1260 466 1520 538V562H740z" fill="var(--mp-grass2)" /><path d="M740 516C900 470 1260 466 1520 538" stroke="var(--mp-stone)" stroke-width="6" /><path d="M740 538C900 492 1260 488 1520 560V584H740z" fill="var(--mp-grass)" /><path d="M740 538C900 492 1260 488 1520 560" stroke="var(--mp-stone)" stroke-width="6" /><path d="M740 560C900 514 1260 510 1520 582V606H740z" fill="var(--mp-grass2)" /><path d="M740 560C900 514 1260 510 1520 582" stroke="var(--mp-stone)" stroke-width="6" />
			<!-- escalier central -->
			<path d="M1100 372L1128 372L1160 600H1040z" fill="var(--mp-stone2)" opacity=".55" />
			<path d="M1102 392H1124M1096 412H1128M1090 432H1132M1084 452H1136M1078 472H1140M1072 492H1144M1064 512H1148M1056 532H1152" stroke="var(--mp-stone)" stroke-width="3" />
		</g>
		<g>
			<g transform="translate(990 402)"><rect x="0" y="-20" width="38" height="20" fill="var(--mp-stone)" /><rect x="0" y="-4" width="38" height="4" fill="var(--mp-stone2)" /><path d="M-6-20L19.0-42L44-20z" fill="var(--mp-thatch)" /><path d="M14.0 0V-12H24.0V0z" fill="var(--mp-door)" /></g><g transform="translate(1040 396)"><rect x="0" y="-20" width="38" height="20" fill="var(--mp-stone)" /><rect x="0" y="-4" width="38" height="4" fill="var(--mp-stone2)" /><path d="M-6-20L19.0-42L44-20z" fill="var(--mp-thatch)" /><path d="M14.0 0V-12H24.0V0z" fill="var(--mp-door)" /></g><g transform="translate(1092 392)"><rect x="0" y="-20" width="38" height="20" fill="var(--mp-stone)" /><rect x="0" y="-4" width="38" height="4" fill="var(--mp-stone2)" /><path d="M-6-20L19.0-42L44-20z" fill="var(--mp-thatch)" /><path d="M14.0 0V-12H24.0V0z" fill="var(--mp-door)" /></g><g transform="translate(1150 390)"><rect x="0" y="-20" width="38" height="20" fill="var(--mp-stone)" /><rect x="0" y="-4" width="38" height="4" fill="var(--mp-stone2)" /><path d="M-6-20L19.0-42L44-20z" fill="var(--mp-thatch)" /><path d="M14.0 0V-12H24.0V0z" fill="var(--mp-door)" /></g><g transform="translate(1208 392)"><rect x="0" y="-20" width="38" height="20" fill="var(--mp-stone)" /><rect x="0" y="-4" width="38" height="4" fill="var(--mp-stone2)" /><path d="M-6-20L19.0-42L44-20z" fill="var(--mp-thatch)" /><path d="M14.0 0V-12H24.0V0z" fill="var(--mp-door)" /></g>
		</g>
		<!-- lama -->
		<g transform="translate(1320 478)" fill="var(--mp-llama)">
			<ellipse cx="0" cy="-12" rx="18" ry="9" /><path d="M10-14L16-34L22-34L20-12z" /><ellipse cx="22" cy="-36" rx="7" ry="4.5" /><path d="M20-38L18-46L23-39zM25-39L26-47L28-39z" />
			<path d="M-10-6V6M-3-4V6M6-4V6M13-6V6" stroke="var(--mp-llama)" stroke-width="3" stroke-linecap="round" /><circle cx="-18" cy="-14" r="4" />
		</g>

		<!-- crêtes de forêt nuageuse, avant-plan latéral -->
		<path d="M560 540V430C620 420 700 424 780 450C830 466 860 490 880 540z" fill="var(--mp-forest)" />
		<path d="M1420 540C1450 480 1500 440 1580 430C1660 420 1730 424 1800 440V540z" fill="var(--mp-forest)" />
		<g fill="var(--mp-forest2)"><circle cx="620" cy="440" r="18" /><circle cx="670" cy="436" r="22" /><circle cx="730" cy="448" r="18" /><circle cx="1560" cy="446" r="20" /><circle cx="1620" cy="438" r="24" /><circle cx="1690" cy="440" r="20" /></g>

	</svg>
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(
			180deg,
			var(--sky1),
			var(--sky2) 73.5%,
			var(--ground1) 74.4%,
			var(--ground1) 80%,
			var(--ground2)
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
	.scene-machupicchu {
		--sky1: #9ccbe6;
		--sky2: #f8ead0;
		--sun: #fff3c4;
		--cloud: #fff;
		--crater: transparent;
		--star: transparent;
		--ground1: #6f9f5c;
		--ground2: #567f48;
		--mp-ground: #4d7a42;
		--mp-far: #aebfd0;
		--mp-snow: #fff;
		--mp-mid: #86a8a0;
		--mp-mist: #fff;
		--mp-peak: #4f8a5e;
		--mp-peak2: #3a6d4b;
		--mp-forest: #3f7a52;
		--mp-forest2: #2f6844;
		--mp-grass: #86b86b;
		--mp-grass2: #76a85d;
		--mp-stone: #d8d0bd;
		--mp-stone2: #b5ad99;
		--mp-thatch: #c9a35f;
		--mp-door: #4a3a2c;
		--mp-llama: #efe3cc;
	}
	:global(.dark) .scene-machupicchu {
		--sky1: #0a1230;
		--sky2: #2c2a4e;
		--sun: #e8eef8;
		--cloud: #2a3658;
		--crater: #b9c4dc;
		--star: #e8eef8;
		--ground1: #17332f;
		--ground2: #0d1f20;
		--mp-ground: #0d1f20;
		--mp-far: #2e3a5c;
		--mp-snow: #8b95b8;
		--mp-mid: #25365a;
		--mp-mist: #5b6a98;
		--mp-peak: #1f4a4a;
		--mp-peak2: #14393d;
		--mp-forest: #163f40;
		--mp-forest2: #0f3235;
		--mp-grass: #2c5e4e;
		--mp-grass2: #25523f;
		--mp-stone: #7f7894;
		--mp-stone2: #625c78;
		--mp-thatch: #7a6540;
		--mp-door: #0a0d1a;
		--mp-llama: #b9b2c8;
	}
</style>
