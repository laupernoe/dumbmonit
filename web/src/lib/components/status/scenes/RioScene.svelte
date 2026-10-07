<script lang="ts">
	/**
	 * Rio de Janeiro — Christ the Redeemer arms open on top of Corcovado, the
	 * Sugarloaf and Morro da Urca across Guanabara bay, the city at their feet,
	 * a few sailboats and a beach palm. Static, no animation. Follows the scene
	 * contract: fills its positioned parent, the headline zone (top left) is
	 * left clear, the palette lives here.
	 */
</script>

<div class="scene scene-rio" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<!-- voilier : origine = ligne de flottaison -->
			<g id="rio-boat">
				<path d="M-22 0H24L16 8H-14z" fill="var(--rio-hull)" />
				<path d="M0-2V-44L22-6z" fill="var(--rio-sail)" /><path d="M-3-2V-34L-20-6z" fill="var(--rio-sail2)" />
			</g>
			<!-- palmier de plage : origine = pied -->
			<g id="rio-palm">
				<path d="M0 0C6-30 2-60 12-92" stroke="var(--rio-trunk)" stroke-width="5" stroke-linecap="round" />
				<g fill="var(--rio-leaf)"><path d="M12-92C-6-114-34-108-52-88C-28-98-10-98 12-92z" /><path d="M12-92C30-116 58-110 74-88C52-100 32-98 12-92z" /><path d="M12-92C0-122 10-142 24-152C22-130 20-110 12-92z" /><path d="M12-92C-14-96-40-72-52-58C-30-72-14-82 12-92z" fill="var(--rio-leaf2)" /><path d="M12-92C38-96 62-72 74-56C52-72 34-82 12-92z" fill="var(--rio-leaf2)" /></g>
			</g>
		</defs>

		<!-- astre, étoiles, nuages -->
		<circle cx="1000" cy="170" r="46" fill="var(--sun)" />
		<circle cx="986" cy="158" r="7" fill="var(--crater)" /><circle cx="1014" cy="184" r="5" fill="var(--crater)" />
		<g fill="var(--star)"><circle cx="1180" cy="112" r="2" /><circle cx="1480" cy="130" r="2.2" /><circle cx="1620" cy="214" r="1.8" /><circle cx="860" cy="120" r="2" /><circle cx="1260" cy="190" r="1.8" /><circle cx="1700" cy="150" r="2" /></g>
		<g fill="var(--cloud)" opacity=".8"><ellipse cx="1470" cy="160" rx="80" ry="11" /><ellipse cx="1520" cy="178" rx="50" ry="8" /><ellipse cx="1180" cy="230" rx="70" ry="10" /><ellipse cx="1230" cy="246" rx="40" ry="7" /><ellipse cx="760" cy="250" rx="60" ry="8" /></g>

		<!-- îles et côte lointaine -->
		<path d="M560 482V462C620 452 660 458 700 466C740 452 780 456 800 482z" fill="var(--rio-far)" />
		<path d="M1560 482V446C1600 430 1650 434 1690 440C1730 436 1770 442 1800 452V482z" fill="var(--rio-far)" />

		<!-- Morro da Urca et Pain de Sucre -->
		<path d="M690 482C700 452 724 434 760 432C800 430 830 450 850 482z" fill="var(--rio-rock2)" />
		<path d="M706 474C716 452 736 440 760 440C786 440 808 452 822 474z" fill="var(--rio-forest2)" />
		<path d="M800 482C806 424 820 380 856 352C894 324 944 326 970 360C996 394 1006 444 1022 482z" fill="var(--rio-rock)" />
		<path d="M870 484C880 430 900 380 940 340C964 344 988 372 1000 408C1008 436 1012 460 1020 484z" fill="var(--rio-rock2)" opacity=".55" />
		<path d="M846 366C870 348 900 340 930 342C910 352 890 362 878 384C866 380 856 374 846 366z" fill="var(--rio-forest)" />
		<path d="M872 408C880 396 892 392 902 398M830 436C842 424 854 420 866 426" stroke="var(--rio-forest2)" stroke-width="4" stroke-linecap="round" />
		<!-- téléphérique -->
		<path d="M770 436L880 354" stroke="var(--rio-cable)" stroke-width="1.6" />
		<rect x="822" y="391" width="9" height="7" rx="1.5" fill="var(--rio-cable)" transform="rotate(-36 826 395)" />

		<!-- le Corcovado -->
		<g transform="translate(-130 0)">
		<path d="M1050 482C1090 452 1120 380 1176 308C1214 258 1272 228 1330 226C1384 226 1420 252 1452 306C1486 366 1520 440 1580 482z" fill="var(--rio-forest)" />
		<path d="M1330 226C1384 226 1420 252 1452 306C1486 366 1520 440 1580 482H1420C1390 410 1370 330 1330 226z" fill="var(--rio-forest2)" opacity=".7" />
		<path d="M1176 340C1220 330 1250 350 1270 392M1360 330C1394 348 1410 380 1420 420M1230 440C1260 420 1290 424 1310 446M1470 432C1490 440 1500 452 1512 468" stroke="var(--rio-forest2)" stroke-width="5" stroke-linecap="round" opacity=".6" />
		<g fill="var(--rio-forest3)"><circle cx="1210" cy="400" r="14" /><circle cx="1238" cy="420" r="16" /><circle cx="1300" cy="450" r="18" /><circle cx="1180" cy="440" r="14" /><circle cx="1440" cy="400" r="12" /><circle cx="1470" cy="440" r="16" /></g>

		<!-- le Christ Rédempteur -->
		<g transform="translate(1330 230) scale(2)">
			<path d="M-9-2L-7-14H7L9-2z" fill="var(--rio-stone2)" />
			<path d="M-5.2-14L-4-44H4L5.2-14z" fill="var(--rio-christ)" />
			<path d="M0-44V-14H5.2L4-44z" fill="var(--rio-christ2)" />
			<path d="M-24-43L-4-44.5V-39.5L-24-38.5zM24-43L4-44.5V-39.5L24-38.5z" fill="var(--rio-christ)" />
			<path d="M24-43L4-44.5V-39.5L24-38.5z" fill="var(--rio-christ2)" />
			<circle cx="0" cy="-49" r="3.4" fill="var(--rio-christ)" />
		</g>

		</g>

		<!-- la ville au pied des collines -->
		<g fill="var(--rio-town)"><rect x="1010" y="458" width="14" height="24" /><rect x="1026" y="452" width="12" height="30" /><rect x="1040" y="462" width="16" height="20" /><rect x="1058" y="448" width="12" height="34" /><rect x="1072" y="460" width="18" height="22" /><rect x="1092" y="454" width="12" height="28" /><rect x="1106" y="464" width="16" height="18" /><rect x="1584" y="462" width="16" height="20" /><rect x="1602" y="452" width="12" height="30" /><rect x="1616" y="460" width="18" height="22" /></g>
		<g fill="var(--rio-town2)"><rect x="1013" y="464" width="3" height="3" /><rect x="1029" y="460" width="3" height="3" /><rect x="1061" y="456" width="3" height="3" /><rect x="1095" y="462" width="3" height="3" /><rect x="1605" y="460" width="3" height="3" /></g>
		<g fill="var(--rio-roof)"><rect x="1074" y="456" width="14" height="4" /><rect x="1042" y="458" width="12" height="4" /><rect x="1618" y="456" width="14" height="4" /></g>

		<!-- baie : reflets et voiliers -->
		<rect x="540" y="482" width="1260" height="6" fill="var(--rio-shore)" />
		<g stroke="var(--rio-wave)" stroke-width="2" stroke-linecap="round" opacity=".6"><path d="M620 512h100M820 540h120M1000 510h130M1180 534h100M1400 512h110M700 570h90M1260 568h130M1520 548h90" /></g>
		<g opacity=".25" transform="translate(0 0)"><path d="M1280 494H1380M1260 502H1400M1290 512H1370" stroke="var(--rio-sail)" stroke-width="3" stroke-linecap="round" /></g>
		<use href="#rio-boat" x="900" y="520" /><g transform="translate(1110 538) scale(.8)"><use href="#rio-boat" /></g><g transform="translate(1590 515) scale(.9)"><use href="#rio-boat" /></g>

		<!-- plage et palmiers au premier plan -->
		<g transform="translate(-300 0)">
		<path d="M1480 600C1560 560 1660 548 1900 552V640H1480z" fill="var(--rio-sand)" />
		<path d="M1500 600C1570 566 1660 556 1900 560" stroke="var(--rio-foam)" stroke-width="3" stroke-linecap="round" opacity=".8" />
		<use href="#rio-palm" x="1620" y="566" /><g transform="translate(1720 570) scale(.8)"><use href="#rio-palm" /></g></g>
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
	.scene-rio {
		--sky1: #86c6dc;
		--sky2: #fdebcf;
		--sun: #fff3c4;
		--cloud: #fff;
		--crater: transparent;
		--star: transparent;
		--water1: #5fb3c4;
		--water2: #3c8fa8;
		--rio-wave: #fff;
		--rio-shore: #4f9fb2;
		--rio-far: #9cc3b4;
		--rio-rock: #a7b3a6;
		--rio-rock2: #8a9a8c;
		--rio-cable: #46525a;
		--rio-forest: #4d8a5b;
		--rio-forest2: #3a7249;
		--rio-forest3: #5fa06a;
		--rio-christ: #fffaf0;
		--rio-christ2: #d9d2c0;
		--rio-stone2: #cfc7b4;
		--rio-town: #f1d9bc;
		--rio-town2: #8a6f58;
		--rio-roof: #d9784f;
		--rio-hull: #b9503f;
		--rio-sail: #fff;
		--rio-sail2: #f4d27a;
		--rio-sand: #f1d9a4;
		--rio-foam: #fff;
		--rio-trunk: #86684a;
		--rio-leaf: #4e8f4f;
		--rio-leaf2: #3b7340;
	}
	:global(.dark) .scene-rio {
		--sky1: #0a1230;
		--sky2: #2a2a52;
		--sun: #e8eef8;
		--cloud: #2a3658;
		--crater: #b9c4dc;
		--star: #e8eef8;
		--water1: #16294a;
		--water2: #0a1428;
		--rio-wave: #6f8fc4;
		--rio-shore: #1b3157;
		--rio-far: #1f2f4e;
		--rio-rock: #4c5875;
		--rio-rock2: #38425d;
		--rio-cable: #aab4cc;
		--rio-forest: #1f4a4a;
		--rio-forest2: #153a3d;
		--rio-forest3: #2a5a52;
		--rio-christ: #e6ecf8;
		--rio-christ2: #9aa6c4;
		--rio-stone2: #7c88a8;
		--rio-town: #3b4062;
		--rio-town2: #f5c76a;
		--rio-roof: #4a3a53;
		--rio-hull: #6a3c44;
		--rio-sail: #8b92a8;
		--rio-sail2: #e8bd4f;
		--rio-sand: #4a4a62;
		--rio-foam: #7a9cc8;
		--rio-trunk: #3a3340;
		--rio-leaf: #1f4d46;
		--rio-leaf2: #163b3a;
	}
</style>
