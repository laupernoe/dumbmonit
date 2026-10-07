<script lang="ts">
	/**
	 * The statue of Zeus at Olympia — Phidias' seated colossus of ivory and
	 * gold in the cella of its Doric temple (cut-away view): olive wreath,
	 * Nike in the right hand, eagle-topped sceptre in the left, ebony throne.
	 * Static, no animation. Follows the scene contract: fills its positioned
	 * parent, the headline zone (top left) is left clear, the palette lives here.
	 */
</script>

<div class="scene scene-zeus" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<radialGradient id="zs-glow" cx=".5" cy=".5" r=".5">
				<stop offset="0" stop-color="var(--glow)" stop-opacity=".9" /><stop offset="1" stop-color="var(--glow)" stop-opacity="0" />
			</radialGradient>
			<linearGradient id="zs-grass" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--grass1)" /><stop offset="1" stop-color="var(--grass2)" />
			</linearGradient>
			<g id="zs-cypress"><path d="M0 0V-10C-14-30-12-70 0-120C12-70 14-30 0-10z" fill="var(--cypress)" /></g>
		</defs>

		<!-- sun / moon, stars, clouds -->
		<circle cx="840" cy="170" r="50" fill="var(--sun-halo)" opacity=".5" />
		<circle cx="840" cy="170" r="32" fill="var(--sun)" />
		<circle cx="828" cy="160" r="5" fill="var(--crater)" /><circle cx="852" cy="180" r="4" fill="var(--crater)" />
		<g fill="var(--star)"><circle cx="1000" cy="110" r="2" /><circle cx="1200" cy="100" r="2.2" /><circle cx="1480" cy="116" r="1.8" /><circle cx="1650" cy="110" r="2" /><circle cx="1730" cy="190" r="2.2" /><circle cx="740" cy="230" r="1.8" /><circle cx="1600" cy="180" r="1.6" /></g>
		<g fill="var(--cloud)" opacity=".75"><ellipse cx="1560" cy="130" rx="100" ry="9" /><ellipse cx="1620" cy="146" rx="58" ry="7" /><ellipse cx="760" cy="110" rx="70" ry="7" /></g>

		<!-- hills -->
		<path d="M560 482V420C660 400 760 360 860 366C960 372 1020 400 1120 410C1300 420 1500 396 1620 380C1700 372 1760 388 1780 396V482z" fill="var(--hill-far)" />
		<path d="M560 482V450C720 436 860 440 1000 454C1260 466 1500 440 1780 448V482z" fill="var(--hill)" />
		<use href="#zs-cypress" transform="translate(760 450) scale(.6)" /><use href="#zs-cypress" transform="translate(792 448) scale(.5)" />
		<g fill="var(--olive)"><ellipse cx="1700" cy="450" rx="30" ry="18" /><ellipse cx="1736" cy="456" rx="22" ry="14" /></g>

		<!-- temple: roof and pediment -->
		<path d="M926 224L1300 128L1674 224z" fill="var(--st)" />
		<path d="M962 218L1300 138L1638 218z" fill="var(--tympanum)" />
		<path d="M926 224L1300 128L1674 224" stroke="var(--st2)" stroke-width="5" stroke-linejoin="round" />
		<path d="M936 216L1300 122L1664 216" stroke="var(--roof)" stroke-width="6" />
		<g fill="var(--st)" opacity=".9"><circle cx="1300" cy="176" r="8" /><path d="M1291 218v-26h18v26zM1220 218l10-14h22l8 14zM1348 218l8-14h22l10 14z" /></g>

		<!-- interior of the cella -->
		<rect x="940" y="224" width="720" height="258" fill="var(--wall)" />
		<rect x="940" y="224" width="720" height="22" fill="var(--wall-shade)" />
		<ellipse cx="1300" cy="360" rx="250" ry="210" fill="url(#zs-glow)" />
		<!-- inner colonnade (two storeys) -->
		<g fill="var(--wall-col)"><rect x="1080" y="270" width="20" height="212" /><rect x="1140" y="270" width="20" height="212" /><rect x="1440" y="270" width="20" height="212" /><rect x="1500" y="270" width="20" height="212" /></g>
		<g fill="var(--wall-shade)"><rect x="1070" y="262" width="100" height="8" /><rect x="1430" y="262" width="100" height="8" /></g>
		<g fill="var(--wall-col)" opacity=".9"><rect x="1086" y="286" width="8" height="70" /><rect x="1146" y="286" width="8" height="70" /><rect x="1446" y="286" width="8" height="70" /><rect x="1506" y="286" width="8" height="70" /></g>
		<!-- floor -->
		<rect x="940" y="452" width="720" height="30" fill="var(--floor)" />
		<g stroke="var(--floor-line)" stroke-width="1.5" opacity=".6"><path d="M940 462H1660M940 472H1660M1020 452L1000 482M1140 452L1130 482M1260 452V482M1340 452V482M1460 452L1470 482M1580 452L1600 482" /></g>

		<g transform="translate(1300 440) scale(.93) translate(-1300 -440)">
		<!-- statue: throne -->
		<rect x="1160" y="458" width="280" height="24" fill="var(--ebony)" />
		<rect x="1176" y="436" width="248" height="24" fill="var(--ebony)" />
		<g fill="var(--gold)"><rect x="1176" y="436" width="248" height="4" /><rect x="1160" y="458" width="280" height="3" /></g>
		<rect x="1214" y="214" width="172" height="224" fill="var(--ebony)" />
		<path d="M1214 214h172v8h-172z" fill="var(--gold)" />
		<g fill="var(--gold)" opacity=".85"><rect x="1224" y="230" width="6" height="204" /><rect x="1370" y="230" width="6" height="204" /><circle cx="1227" cy="226" r="7" /><circle cx="1373" cy="226" r="7" /></g>
		<rect x="1186" y="334" width="40" height="104" fill="var(--ebony)" />
		<rect x="1374" y="334" width="40" height="104" fill="var(--ebony)" />
		<g fill="var(--gold)"><rect x="1186" y="334" width="40" height="5" /><rect x="1374" y="334" width="40" height="5" /><rect x="1196" y="350" width="20" height="4" /><rect x="1384" y="350" width="20" height="4" /><rect x="1196" y="364" width="20" height="4" /><rect x="1384" y="364" width="20" height="4" /></g>
		<!-- footstool -->
		<rect x="1236" y="424" width="128" height="14" fill="var(--gold)" />
		<rect x="1236" y="436" width="128" height="4" fill="var(--ebony)" opacity=".6" />

		<!-- statue: body -->
		<!-- lap and legs under the gold mantle -->
		<path d="M1236 352H1364L1384 392L1376 426H1224L1216 392z" fill="var(--gold)" />
		<path d="M1262 392L1300 420L1338 392M1250 372L1274 424M1350 372L1326 424M1300 356V400" stroke="var(--gold-d)" stroke-width="3" stroke-linecap="round" />
		<!-- feet -->
		<path d="M1244 426h38v10h-38zM1318 426h38v10h-38z" fill="var(--ivory)" />
		<path d="M1244 432h38M1318 432h38" stroke="var(--gold)" stroke-width="2" />
		<!-- bust: ivory torso, mantle over the left shoulder -->
		<path d="M1252 296C1262 282 1338 282 1348 296L1358 356H1242z" fill="var(--ivory)" />
		<path d="M1348 296C1362 304 1366 332 1366 356H1318C1334 340 1344 320 1348 296z" fill="var(--gold)" />
		<path d="M1300 300V350M1274 322C1286 330 1290 328 1300 322M1300 322C1310 328 1314 330 1326 322" stroke="var(--ivory-d)" stroke-width="2" stroke-linecap="round" />
		<!-- right arm (viewer's left) holding Nike -->
		<path d="M1254 298L1224 338L1190 330" stroke="var(--ivory)" stroke-width="20" stroke-linecap="round" stroke-linejoin="round" />
		<circle cx="1186" cy="332" r="9" fill="var(--ivory)" />
		<g fill="var(--gold)"><circle cx="1184" cy="298" r="5" /><path d="M1178 304h12l-3 28h-6zM1176 308l-12-14l4 22zM1192 308l12-14l-4 22z" /></g>
		<!-- left arm (viewer's right) holding the sceptre -->
		<path d="M1346 298L1372 340L1396 336" stroke="var(--ivory)" stroke-width="20" stroke-linecap="round" stroke-linejoin="round" />
		<circle cx="1398" cy="336" r="9" fill="var(--ivory)" />
		<rect x="1396" y="236" width="5" height="206" fill="var(--gold)" />
		<g fill="var(--gold)"><path d="M1398 242c-10-4-18-12-24-12c6 12 14 18 24 22c10-4 18-10 24-22c-6 0-14 8-24 12zM1394 224c0-8 8-12 8 0c2 4 8 4 8 8l-8 2l-8-2z" /></g>
		<!-- neck, head, beard, wreath -->
		<rect x="1288" y="272" width="24" height="18" fill="var(--ivory-d)" />
		<ellipse cx="1300" cy="252" rx="22" ry="25" fill="var(--ivory)" />
		<path d="M1278 252C1274 276 1286 292 1300 294C1314 292 1326 276 1322 252C1316 266 1310 270 1300 270C1290 270 1284 266 1278 252z" fill="var(--hair)" />
		<path d="M1276 244C1272 218 1288 220 1300 222C1312 220 1328 218 1324 244C1320 232 1312 228 1300 228C1288 228 1280 232 1276 244z" fill="var(--hair)" />
		<path d="M1276 236C1284 224 1316 224 1324 236" stroke="var(--olive-w)" stroke-width="5" stroke-linecap="round" />
		<g fill="var(--olive-w)"><ellipse cx="1280" cy="234" rx="6" ry="3" /><ellipse cx="1296" cy="224" rx="6" ry="3" /><ellipse cx="1312" cy="226" rx="6" ry="3" /><ellipse cx="1322" cy="236" rx="6" ry="3" /></g>
		<g fill="var(--hair)"><rect x="1288" y="248" width="9" height="3" rx="1.5" /><rect x="1304" y="248" width="9" height="3" rx="1.5" /></g>

		</g>

		<!-- front Doric columns -->
		<path d="M950 470L953 244H987L990 470z" fill="var(--st)" /><path d="M972.0 470L973.0 244H987L990 470z" fill="var(--st-shade)" opacity=".6" /><path d="M958.0 470V240M966.0 470V240M974.0 470V240M982.0 470V240" stroke="var(--flute)" stroke-width="1.2" opacity=".6" /><path d="M953 244C948 238 944 238 944 232H996C996 238 992 238 987 244z" fill="var(--st)" /><rect x="941" y="224" width="58" height="9" fill="var(--st2)" /><path d="M1008 470L1011 244H1045L1048 470z" fill="var(--st)" /><path d="M1030.0 470L1031.0 244H1045L1048 470z" fill="var(--st-shade)" opacity=".6" /><path d="M1016.0 470V240M1024.0 470V240M1032.0 470V240M1040.0 470V240" stroke="var(--flute)" stroke-width="1.2" opacity=".6" /><path d="M1011 244C1006 238 1002 238 1002 232H1054C1054 238 1050 238 1045 244z" fill="var(--st)" /><rect x="999" y="224" width="58" height="9" fill="var(--st2)" /><path d="M1552 470L1555 244H1589L1592 470z" fill="var(--st)" /><path d="M1574.0 470L1575.0 244H1589L1592 470z" fill="var(--st-shade)" opacity=".6" /><path d="M1560.0 470V240M1568.0 470V240M1576.0 470V240M1584.0 470V240" stroke="var(--flute)" stroke-width="1.2" opacity=".6" /><path d="M1555 244C1550 238 1546 238 1546 232H1598C1598 238 1594 238 1589 244z" fill="var(--st)" /><rect x="1543" y="224" width="58" height="9" fill="var(--st2)" /><path d="M1610 470L1613 244H1647L1650 470z" fill="var(--st)" /><path d="M1632.0 470L1633.0 244H1647L1650 470z" fill="var(--st-shade)" opacity=".6" /><path d="M1618.0 470V240M1626.0 470V240M1634.0 470V240M1642.0 470V240" stroke="var(--flute)" stroke-width="1.2" opacity=".6" /><path d="M1613 244C1608 238 1604 238 1604 232H1656C1656 238 1652 238 1647 244z" fill="var(--st)" /><rect x="1601" y="224" width="58" height="9" fill="var(--st2)" />
		<rect x="930" y="216" width="740" height="10" fill="var(--st2)" />
		<g fill="var(--st)"><path d="M930 216h740l-10-6h-720z" /></g>
		<g fill="var(--st-shade)" opacity=".7"><rect x="942" y="226" width="14" height="6" /><rect x="1642" y="226" width="14" height="6" /></g>
		<!-- steps -->
		<rect x="936" y="470" width="728" height="12" fill="var(--step1)" />
		<rect x="924" y="482" width="752" height="12" fill="var(--step2)" />
		<rect x="912" y="494" width="776" height="12" fill="var(--step3)" />

		<!-- ground -->
		<rect x="560" y="506" width="1220" height="116" fill="url(#zs-grass)" />
		<path d="M560 482H912V506H560zM1688 482H1780V506H1688z" fill="url(#zs-grass)" />
		<g stroke="var(--blade)" stroke-width="2" stroke-linecap="round" opacity=".6"><path d="M640 530h40M760 556h60M880 540h50M1700 546h60M1560 580h70M720 596h50" /></g>
		<use href="#zs-cypress" transform="translate(870 500) scale(1.2)" /><use href="#zs-cypress" transform="translate(1730 500) scale(1.3)" />
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
			var(--grass1) 74.4%,
			var(--grass1) 80%,
			var(--grass2)
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
	.scene-zeus {
		--sky1: #a9cfe4;
		--sky2: #f7ead2;
		--sun: #fff5d2;
		--sun-halo: #ffe7b0;
		--cloud: #fff;
		--star: transparent;
		--crater: transparent;
		--hill-far: #bccbb0;
		--hill: #a2bb8a;
		--cypress: #3d6b46;
		--olive: #7a9a6a;
		--st: #efdfbc;
		--st2: #d9c598;
		--st-shade: #a98f62;
		--flute: #a98f62;
		--tympanum: #7d9fbf;
		--roof: #c4724b;
		--wall: #8a6f56;
		--wall-shade: #6b5442;
		--wall-col: #7a6048;
		--glow: #ffd98a;
		--floor: #c9b48c;
		--floor-line: #8f7a58;
		--ebony: #3a2a24;
		--gold: #e0a82e;
		--gold-d: #a8761c;
		--ivory: #fbf3e2;
		--ivory-d: #e3d3b4;
		--hair: #d8b25c;
		--olive-w: #6f9a52;
		--step1: #efe3cb;
		--step2: #e1d1b2;
		--step3: #d3c19f;
		--grass1: #a9bf86;
		--grass2: #7fa06b;
		--blade: #5f8250;
	}
	:global(.dark) .scene-zeus {
		--sky1: #0b1230;
		--sky2: #29294d;
		--sun: #e8eef8;
		--sun-halo: #4d5280;
		--cloud: #2a3658;
		--star: #e8eef8;
		--crater: #b9c4dc;
		--hill-far: #1f2547;
		--hill: #18203a;
		--cypress: #0f2a2c;
		--olive: #1f3a3a;
		--st: #8b92a8;
		--st2: #6d7690;
		--st-shade: #3a4260;
		--flute: #4d5671;
		--tympanum: #2c4a80;
		--roof: #5c4458;
		--wall: #1e1a30;
		--wall-shade: #151225;
		--wall-col: #2a2540;
		--glow: #c98f36;
		--floor: #3a3350;
		--floor-line: #1e1a30;
		--ebony: #120d14;
		--gold: #f0c050;
		--gold-d: #a8761c;
		--ivory: #e8dfc8;
		--ivory-d: #b9ad92;
		--hair: #d8a844;
		--olive-w: #4f8a56;
		--step1: #7a8299;
		--step2: #667088;
		--step3: #535c75;
		--grass1: #1a2a3a;
		--grass2: #0e1626;
		--blade: #2a4a4a;
	}
</style>
