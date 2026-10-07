<script lang="ts">
	/**
	 * Al-Khazneh, the Treasury of Petra — the rose-red rock facade with its
	 * columns, broken pediment and tholos, framed by the stratified walls of
	 * the Siq. Static, no animation. Follows the scene contract: fills its
	 * positioned parent, the headline zone (top left) is left clear, the
	 * palette lives here.
	 */
</script>

<div class="scene scene-petra" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<pattern id="pt-strata" width="40" height="34" patternUnits="userSpaceOnUse">
				<rect width="40" height="34" fill="var(--rock)" />
				<path d="M0 8Q10 4 20 8T40 8V14Q30 18 20 14T0 14Z" fill="var(--rock2)" opacity=".55" />
				<path d="M0 24Q10 20 20 24T40 24V28Q30 32 20 28T0 28Z" fill="var(--rock3)" opacity=".5" />
			</pattern>
			<linearGradient id="pt-shade" x1="0" y1="0" x2="1" y2="0">
				<stop offset="0" stop-color="var(--shade)" stop-opacity=".55" />
				<stop offset="1" stop-color="var(--shade)" stop-opacity="0" />
			</linearGradient>
		</defs>
		<circle cx="1480" cy="140" r="36" fill="var(--sun)" opacity=".9" />
		<!-- back cliff holding the facade -->
		<path d="M1010 500V200L1030 186L1046 150L1086 140L1112 112L1170 118L1210 100L1262 120L1304 108L1346 136L1372 150L1380 190L1392 206V500Z" fill="url(#pt-strata)" />
		<path d="M1010 500V200L1030 186L1046 150L1086 140L1112 112L1170 118L1210 100L1262 120L1304 108L1346 136L1372 150L1380 190L1392 206V500Z" fill="var(--glow)" opacity=".25" />
		<!-- facade: upper tier -->
		<g>
			<rect x="1112" y="214" width="176" height="108" fill="var(--face)" />
			<path d="M1112 214L1150 200H1190V222H1112zM1288 214L1250 200H1210V222H1288z" fill="var(--face2)" />
			<path d="M1112 214L1200 196L1288 214" stroke="var(--face2)" stroke-width="5" />
			<!-- tholos -->
			<rect x="1170" y="236" width="60" height="86" fill="var(--face2)" />
			<path d="M1166 240h68v-8a34 14 0 0 0-68 0z" fill="var(--face)" />
			<path d="M1176 240V190a24 24 0 0 1 48 0v50z" fill="var(--face)" />
			<path d="M1180 188a20 20 0 0 1 40 0v52h-40z" fill="var(--face)" />
			<path d="M1190 236v-30a10 10 0 0 1 20 0v30z" fill="var(--dark)" opacity=".7" />
			<circle cx="1200" cy="168" r="6" fill="var(--face2)" /><rect x="1198" y="152" width="4" height="12" fill="var(--face2)" />
			<!-- side pavilions with half pediments -->
			<rect x="1124" y="236" width="38" height="86" fill="var(--face2)" />
			<rect x="1238" y="236" width="38" height="86" fill="var(--face2)" />
			<path d="M1118 236h50l-6-14h-38zM1232 236h50l-6-14h-38z" fill="var(--face)" />
			<g fill="var(--dark)" opacity=".5"><rect x="1136" y="262" width="14" height="44" rx="2" /><rect x="1250" y="262" width="14" height="44" rx="2" /></g>
		</g>
		<!-- lower tier: portico -->
		<g>
			<rect x="1088" y="322" width="224" height="158" fill="var(--face)" />
			<rect x="1088" y="322" width="224" height="10" fill="var(--face2)" />
			<path d="M1088 322L1200 296L1312 322z" fill="var(--face2)" opacity=".001" />
			<path d="M1092 322L1160 306V322zM1308 322L1240 306V322z" fill="var(--face2)" />
			<rect x="1098" y="332" width="204" height="12" fill="var(--face2)" />
			<!-- doorway -->
			<path d="M1180 480V402a20 20 0 0 1 40 0V480z" fill="var(--dark)" />
			<path d="M1172 480V398a28 28 0 0 1 56 0V480h-8V402a20 20 0 0 0-40 0V480z" fill="var(--face2)" />
			<!-- columns -->
			<g fill="var(--face2)">
				<rect x="1108" y="346" width="14" height="134" /><rect x="1138" y="346" width="14" height="134" />
				<rect x="1248" y="346" width="14" height="134" /><rect x="1278" y="346" width="14" height="134" />
			</g>
			<g fill="var(--face)" opacity=".8">
				<rect x="1108" y="346" width="5" height="134" /><rect x="1138" y="346" width="5" height="134" /><rect x="1248" y="346" width="5" height="134" /><rect x="1278" y="346" width="5" height="134" />
			</g>
			<g fill="var(--face2)"><rect x="1103" y="340" width="24" height="8" /><rect x="1133" y="340" width="24" height="8" /><rect x="1243" y="340" width="24" height="8" /><rect x="1273" y="340" width="24" height="8" /></g>
			<g fill="var(--dark)" opacity=".4"><rect x="1126" y="362" width="9" height="80" rx="2" /><rect x="1265" y="362" width="9" height="80" rx="2" /></g>
		</g>
		<!-- steps and ground -->
		<path d="M1060 480H1340V492H1060z" fill="var(--face2)" />
		<path d="M560 520C760 500 900 496 1060 492H1340C1500 496 1640 500 1780 520V620H560Z" fill="var(--sand)" />
		<path d="M1080 540C1160 520 1240 520 1320 540" stroke="var(--sand2)" stroke-width="5" stroke-linecap="round" opacity=".6" />
		<!-- left wall of the Siq -->
		<g>
			<path d="M560 520V400C640 400 700 380 760 330C820 280 880 250 960 200C1010 170 1030 190 1040 230C1050 290 1020 380 1040 520Z" fill="url(#pt-strata)" />
			<path d="M560 520V400C640 400 700 380 760 330C820 280 880 250 960 200C1010 170 1030 190 1040 230C1050 290 1020 380 1040 520Z" fill="var(--shade)" opacity=".28" />
			<path d="M960 200C1010 170 1030 190 1040 230C1050 290 1020 380 1040 520H980C990 400 1000 300 960 200Z" fill="url(#pt-shade)" />
		</g>
		<!-- right wall -->
		<g>
			<path d="M1780 520V110C1700 100 1640 140 1580 190C1520 240 1450 260 1410 330C1380 390 1400 440 1370 520Z" fill="url(#pt-strata)" />
			<path d="M1780 520V110C1700 100 1640 140 1580 190C1520 240 1450 260 1410 330C1380 390 1400 440 1370 520Z" fill="var(--shade)" opacity=".18" />
			<path d="M1420 320C1380 390 1400 440 1370 520H1420C1440 440 1440 380 1470 300Z" fill="var(--shade)" opacity=".35" />
		</g>
		<!-- boulders -->
		<g fill="var(--rock3)"><ellipse cx="960" cy="530" rx="46" ry="14" /><ellipse cx="1440" cy="540" rx="58" ry="16" /><ellipse cx="1500" cy="548" rx="30" ry="10" /></g>
	</svg>
</div>

<style>
	.scene-petra {
		--sky1: #f3cfa4;
		--sky2: #fbe6c4;
		--sun: #fff1c8;
		--water1: #dcae78;
		--water2: #c0905c;
		--rock: #d98c6a;
		--rock2: #c0715a;
		--rock3: #e9a98a;
		--glow: #fbd0a0;
		--shade: #7a3b3a;
		--face: #e8a888;
		--face2: #c97b62;
		--dark: #5a2e2c;
		--sand: #e4b684;
		--sand2: #bf8a5c;
	}
	:global(.dark) .scene-petra {
		--sky1: #0d1229;
		--sky2: #2a2142;
		--sun: #e8eef8;
		--water1: #3a2a3c;
		--water2: #1a1226;
		--rock: #8a5a66;
		--rock2: #6c4558;
		--rock3: #a0707a;
		--glow: #f5c76a;
		--shade: #1a1226;
		--face: #b87c80;
		--face2: #8a5668;
		--dark: #f5c76a;
		--sand: #4a3544;
		--sand2: #2e2030;
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
