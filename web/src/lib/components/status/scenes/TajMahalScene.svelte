<script lang="ts">
	/**
	 * The Taj Mahal at dawn — white marble mausoleum with its great dome, four
	 * minarets and flanking red-sandstone buildings, mirrored in the long
	 * reflecting pool between cypress-lined gardens. Static, no animation.
	 * Follows the scene contract: fills its positioned parent, the headline
	 * zone (top left) is left clear, the palette lives here.
	 */
</script>

<div class="scene scene-tajmahal" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<clipPath id="tj-pool"><path d="M1150 398H1250L1400 620H1000Z" /></clipPath>
			<g id="tj-minaret">
				<rect x="-7" y="-150" width="14" height="150" fill="var(--marble)" />
				<rect x="0" y="-150" width="7" height="150" fill="var(--marble2)" />
				<rect x="-11" y="-112" width="22" height="5" fill="var(--marble)" /><rect x="-11" y="-72" width="22" height="5" fill="var(--marble)" />
				<rect x="-13" y="-152" width="26" height="6" fill="var(--marble)" />
				<rect x="-9" y="-170" width="18" height="18" fill="var(--marble)" />
				<path d="M-10-170a10 12 0 0 1 20 0z" fill="var(--marble)" />
				<rect x="-1" y="-188" width="2" height="12" fill="var(--gold)" />
				<path d="M-4-166v16a4 4 0 0 0 8 0v-16a4 4 0 0 0-8 0z" fill="var(--arch)" opacity=".5" />
			</g>
			<g id="tj-main">
				<!-- wings and body -->
				<rect x="1000" y="372" width="400" height="20" fill="var(--marble2)" />
				<rect x="1080" y="296" width="240" height="78" fill="var(--marble)" />
				<rect x="1200" y="296" width="120" height="78" fill="var(--marble2)" opacity=".35" />
				<!-- central pishtaq -->
				<rect x="1152" y="282" width="96" height="92" fill="var(--marble)" />
				<path d="M1170 374V320C1170 306 1190 296 1200 284C1210 296 1230 306 1230 320V374z" fill="var(--arch)" />
				<path d="M1178 374V326C1178 316 1192 308 1200 298C1208 308 1222 316 1222 326V374z" fill="var(--arch2)" />
				<!-- side arches -->
				<path d="M1100 374V332a14 14 0 0 1 28 0V374zM1272 374V332a14 14 0 0 1 28 0V374z" fill="var(--arch)" />
				<path d="M1100 340a14 12 0 0 1 28 0v-8a14 14 0 0 0-28 0z" fill="var(--arch2)" />
				<path d="M1138 374v-26a8 8 0 0 1 16 0v26zM1246 374v-26a8 8 0 0 1 16 0v26z" fill="var(--arch)" opacity=".7" />
				<!-- corner chhatris -->
				<g fill="var(--marble)"><rect x="1092" y="278" width="20" height="20" /><rect x="1288" y="278" width="20" height="20" /><path d="M1090 278a12 14 0 0 1 24 0zM1286 278a12 14 0 0 1 24 0z" /><rect x="1101" y="258" width="2" height="8" /><rect x="1297" y="258" width="2" height="8" /></g>
				<g fill="var(--arch)" opacity=".6"><rect x="1096" y="284" width="4" height="14" /><rect x="1104" y="284" width="4" height="14" /><rect x="1292" y="284" width="4" height="14" /><rect x="1300" y="284" width="4" height="14" /></g>
				<!-- dome -->
				<rect x="1164" y="262" width="72" height="24" fill="var(--marble)" />
				<path d="M1160 270C1136 238 1160 198 1200 168C1240 198 1264 238 1240 270Z" fill="var(--marble)" />
				<path d="M1200 168C1226 198 1240 238 1240 270H1210C1220 240 1216 200 1200 168Z" fill="var(--marble2)" />
				<rect x="1199" y="140" width="2" height="30" fill="var(--gold)" />
				<circle cx="1200" cy="156" r="3.5" fill="var(--gold)" /><circle cx="1200" cy="147" r="2.5" fill="var(--gold)" />
				<g fill="var(--marble)"><rect x="1164" y="256" width="72" height="6" /></g>
				<!-- minarets -->
				<use href="#tj-minaret" x="1022" y="386" /><use href="#tj-minaret" x="1378" y="386" />
				<use href="#tj-minaret" x="1054" y="378" transform="translate(0 0) scale(1)" opacity="0" />
				<!-- flanking sandstone buildings -->
				<g>
					<rect x="830" y="350" width="150" height="42" fill="var(--sand)" />
					<rect x="1420" y="350" width="150" height="42" fill="var(--sand)" />
					<rect x="830" y="346" width="150" height="6" fill="var(--sand2)" /><rect x="1420" y="346" width="150" height="6" fill="var(--sand2)" />
					<path d="M850 392v-18a8 8 0 0 1 16 0v18zM888 392v-18a8 8 0 0 1 16 0v18zM926 392v-18a8 8 0 0 1 16 0v18zM1442 392v-18a8 8 0 0 1 16 0v18zM1480 392v-18a8 8 0 0 1 16 0v18zM1518 392v-18a8 8 0 0 1 16 0v18z" fill="var(--arch)" opacity=".75" />
					<g fill="var(--marble)"><path d="M852 346a14 18 0 0 1 28 0zM896 346a14 18 0 0 1 28 0zM940 346a14 18 0 0 1 28 0zM1434 346a14 18 0 0 1 28 0zM1478 346a14 18 0 0 1 28 0zM1522 346a14 18 0 0 1 28 0z" /></g>
					<g fill="var(--gold)"><rect x="865" y="318" width="2" height="12" /><rect x="909" y="318" width="2" height="12" /><rect x="953" y="318" width="2" height="12" /><rect x="1447" y="318" width="2" height="12" /><rect x="1491" y="318" width="2" height="12" /><rect x="1535" y="318" width="2" height="12" /></g>
				</g>
			</g>
		</defs>
		<circle cx="1560" cy="170" r="40" fill="var(--sun)" opacity=".9" />
		<g fill="var(--cloud)" opacity=".7"><ellipse cx="860" cy="130" rx="80" ry="14" /><ellipse cx="940" cy="118" rx="44" ry="12" /><ellipse cx="1330" cy="110" rx="60" ry="10" /></g>
		<!-- distant trees -->
		<path d="M560 392C700 372 820 380 900 392zM1500 392C1600 376 1700 380 1780 392z" fill="var(--tree2)" />
		<use href="#tj-main" />
		<!-- ground -->
		<path d="M560 392H1780V620H560Z" fill="var(--lawn)" />
		<path d="M560 392H1780V398H560Z" fill="var(--marble2)" />
		<path d="M1150 398H1250L1400 620H1000Z" fill="var(--pool)" />
		<g clip-path="url(#tj-pool)"><g opacity=".5" transform="translate(0 784) scale(1 -1)"><use href="#tj-main" /></g>
			<rect x="1000" y="398" width="400" height="222" fill="var(--pool)" opacity=".4" />
			<g stroke="var(--cloud)" stroke-width="2.5" stroke-linecap="round" opacity=".5"><path d="M1130 440h50M1230 452h70M1100 490h90M1230 510h100M1060 560h110M1250 580h120M1160 600h60" /></g>
		</g>
		<!-- central path edges -->
		<path d="M1150 398L1000 620M1250 398L1400 620" stroke="var(--marble2)" stroke-width="7" />
		<path d="M1134 398L960 620M1266 398L1440 620" stroke="var(--path)" stroke-width="18" opacity=".6" />
		<!-- cypresses -->
		<g fill="var(--tree)">
			<path d="M1098 440c-9-22-9-48 0-70 9 22 9 48 0 70z" /><path d="M1302 440c9-22 9-48 0-70-9 22-9 48 0 70z" />
			<path d="M1062 490c-11-28-11-62 0-90 11 28 11 62 0 90z" /><path d="M1338 490c11-28 11-62 0-90-11 28-11 62 0 90z" />
			<path d="M1010 560c-14-36-14-80 0-116 14 36 14 80 0 116z" /><path d="M1390 560c14-36 14-80 0-116-14 36-14 80 0 116z" />
		</g>
		<g fill="var(--tree2)" opacity=".8"><ellipse cx="800" cy="440" rx="70" ry="16" /><ellipse cx="1600" cy="440" rx="70" ry="16" /><ellipse cx="760" cy="520" rx="90" ry="20" /><ellipse cx="1640" cy="520" rx="90" ry="20" /></g>
	</svg>
</div>

<style>
	.scene-tajmahal {
		--sky1: #f6c7b4;
		--sky2: #fdeed9;
		--sun: #fff0c4;
		--water1: #9dc2b2;
		--water2: #7aa595;
		--marble: #fbf5ea;
		--marble2: #e1d8cb;
		--arch: #8ab3b2;
		--arch2: #5f8f93;
		--gold: #e0a82e;
		--sand: #c9694b;
		--sand2: #a85238;
		--pool: #a8d0d6;
		--lawn: #86ad73;
		--path: #e8dcc2;
		--tree: #35694b;
		--tree2: #6f9a63;
		--cloud: #fff;
	}
	:global(.dark) .scene-tajmahal {
		--sky1: #0d1630;
		--sky2: #2a2a52;
		--sun: #e8eef8;
		--water1: #16294a;
		--water2: #0a1428;
		--marble: #a8b2cc;
		--marble2: #7b86a6;
		--arch: #1a2540;
		--arch2: #f5c76a;
		--gold: #e8bd4f;
		--sand: #6a3c4c;
		--sand2: #51303e;
		--pool: #1d3a62;
		--lawn: #12302f;
		--path: #3a4a6e;
		--tree: #0a1c20;
		--tree2: #17403c;
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
