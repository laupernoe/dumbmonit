<script lang="ts">
	/**
	 * Chichen Itza — El Castillo, the stepped pyramid of Kukulkan with its four
	 * staircases and the temple on top, in the Yucatan jungle. Static, no
	 * animation. Follows the scene contract: fills its positioned parent, the
	 * headline zone (top left) is left clear, the palette lives here.
	 */
</script>

<div class="scene scene-chichenitza" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<!-- arbre de jungle : origine = pied du tronc -->
			<g id="ci-tree">
				<path d="M-3 0L-2-70H3L4 0z" fill="var(--ci-trunk)" />
				<circle cx="0" cy="-92" r="34" fill="var(--ci-leaf2)" /><circle cx="-26" cy="-72" r="26" fill="var(--ci-leaf)" /><circle cx="26" cy="-74" r="27" fill="var(--ci-leaf)" /><circle cx="2" cy="-108" r="22" fill="var(--ci-leaf3)" />
			</g>
			<!-- palmier : origine = pied -->
			<g id="ci-palm">
				<path d="M0 0C6-40 2-80 10-120" stroke="var(--ci-trunk)" stroke-width="6" stroke-linecap="round" />
				<g fill="var(--ci-leaf)"><path d="M10-120C-10-146-40-140-60-116C-34-128-14-126 10-120z" /><path d="M10-120C30-148 62-142 82-116C56-130 34-128 10-120z" /><path d="M10-120C-4-154 10-176 26-190C24-164 20-142 10-120z" /><path d="M10-120C-20-128-50-100-64-84C-38-100-20-110 10-120z" fill="var(--ci-leaf2)" /><path d="M10-120C40-128 70-100 84-82C58-100 38-110 10-120z" fill="var(--ci-leaf2)" /></g>
			</g>
		</defs>

		<!-- astre, étoiles, nuages -->
		<circle cx="1620" cy="190" r="48" fill="var(--sun)" />
		<circle cx="1604" cy="178" r="7" fill="var(--crater)" /><circle cx="1634" cy="204" r="5" fill="var(--crater)" />
		<g fill="var(--star)"><circle cx="1010" cy="118" r="2" /><circle cx="1180" cy="170" r="1.8" /><circle cx="1420" cy="110" r="2.2" /><circle cx="1520" cy="250" r="1.8" /><circle cx="1700" cy="120" r="2" /><circle cx="900" cy="230" r="1.8" /></g>
		<g fill="var(--cloud)" opacity=".8"><ellipse cx="1000" cy="170" rx="80" ry="11" /><ellipse cx="1050" cy="188" rx="50" ry="8" /><ellipse cx="1470" cy="150" rx="70" ry="10" /><ellipse cx="1740" cy="290" rx="60" ry="8" /></g>

		<!-- collines lointaines couvertes de forêt -->
		<path d="M560 482V400C640 380 700 396 780 384C860 370 940 396 1000 410V482z" fill="var(--ci-far)" />
		<path d="M1480 482V404C1540 384 1600 392 1660 376C1720 364 1760 380 1800 388V482z" fill="var(--ci-far)" />

		<!-- El Castillo -->
		<g transform="translate(1250 482) scale(1.18) translate(-1250 -482)">
			<!-- gradins -->
			<rect x="1060" y="463" width="380" height="19" fill="var(--ci-stone)"/><rect x="1060" y="463" width="380" height="3" fill="var(--ci-light)"/><rect x="1078" y="444" width="344" height="19" fill="var(--ci-stone)"/><rect x="1078" y="444" width="344" height="3" fill="var(--ci-light)"/><rect x="1096" y="425" width="308" height="19" fill="var(--ci-stone)"/><rect x="1096" y="425" width="308" height="3" fill="var(--ci-light)"/><rect x="1114" y="406" width="272" height="19" fill="var(--ci-stone)"/><rect x="1114" y="406" width="272" height="3" fill="var(--ci-light)"/><rect x="1132" y="387" width="236" height="19" fill="var(--ci-stone)"/><rect x="1132" y="387" width="236" height="3" fill="var(--ci-light)"/><rect x="1150" y="368" width="200" height="19" fill="var(--ci-stone)"/><rect x="1150" y="368" width="200" height="3" fill="var(--ci-light)"/><rect x="1168" y="349" width="164" height="19" fill="var(--ci-stone)"/><rect x="1168" y="349" width="164" height="3" fill="var(--ci-light)"/><rect x="1186" y="330" width="128" height="19" fill="var(--ci-stone)"/><rect x="1186" y="330" width="128" height="3" fill="var(--ci-light)"/><rect x="1204" y="311" width="92" height="19" fill="var(--ci-stone)"/><rect x="1204" y="311" width="92" height="3" fill="var(--ci-light)"/>
			<rect x="1060" y="478" width="380" height="4" fill="var(--ci-stone3)"/><rect x="1078" y="459" width="344" height="4" fill="var(--ci-stone3)"/><rect x="1096" y="440" width="308" height="4" fill="var(--ci-stone3)"/><rect x="1114" y="421" width="272" height="4" fill="var(--ci-stone3)"/><rect x="1132" y="402" width="236" height="4" fill="var(--ci-stone3)"/><rect x="1150" y="383" width="200" height="4" fill="var(--ci-stone3)"/><rect x="1168" y="364" width="164" height="4" fill="var(--ci-stone3)"/><rect x="1186" y="345" width="128" height="4" fill="var(--ci-stone3)"/><rect x="1204" y="326" width="92" height="4" fill="var(--ci-stone3)"/>
			<!-- face à l'ombre : côté droit plus sombre -->
			<path d="M1250 311H1296V330H1314V349H1332V368H1350V387H1368V406H1386V425H1404V444H1422V463H1440V482H1250z" fill="var(--ci-stone3)" opacity=".22" />
			<!-- escalier central : marches et rampes -->
			<path d="M1222 311H1278L1288 482H1212z" fill="var(--ci-stair)" />
			<path d="M1227 476H1273M1227 469.9H1273M1227 463.8H1273M1227 457.7H1273M1227 451.6H1273M1227 445.5H1273M1227 439.4H1273M1227 433.3H1273M1227 427.2H1273M1227 421.1H1273M1227 415H1273M1227 408.9H1273M1227 402.8H1273M1227 396.7H1273M1227 390.6H1273M1227 384.5H1273M1227 378.4H1273M1227 372.3H1273M1227 366.2H1273M1227 360.1H1273M1227 354H1273M1227 347.9H1273M1227 341.8H1273M1227 335.70000000000005H1273M1227 329.6H1273M1227 323.5H1273M1227 317.4H1273M1227 311.3H1273" stroke="var(--ci-stone3)" stroke-width="1.6" opacity=".55" transform="translate(0 0)" />
			<path d="M1219 311H1230L1224 482H1210zM1270 311H1281L1290 482H1276z" fill="var(--ci-stone2)" />
			<!-- têtes de serpent au pied -->
			<g fill="var(--ci-stone2)"><rect x="1198" y="466" width="22" height="16" rx="4" /><rect x="1280" y="466" width="22" height="16" rx="4" /></g>
			<g fill="var(--ci-stone3)"><circle cx="1209" cy="473" r="2.4" /><circle cx="1291" cy="473" r="2.4" /></g>
			<!-- temple -->
			<rect x="1214" y="271" width="72" height="40" fill="var(--ci-stone)" />
			<rect x="1208" y="263" width="84" height="9" fill="var(--ci-light)" /><rect x="1208" y="270" width="84" height="3" fill="var(--ci-stone3)" />
			<rect x="1218" y="254" width="64" height="9" fill="var(--ci-stone2)" /><rect x="1226" y="248" width="48" height="6" fill="var(--ci-stone)" />
			<path d="M1236 311V285h28v26z" fill="var(--ci-door)" />
			<rect x="1248" y="285" width="4" height="26" fill="var(--ci-stone2)" />
			<g fill="var(--ci-stone3)" opacity=".7"><rect x="1219" y="280" width="12" height="3" /><rect x="1269" y="280" width="12" height="3" /><rect x="1219" y="292" width="12" height="3" /><rect x="1269" y="292" width="12" height="3" /></g>
			<g fill="var(--ci-stone3)"><rect x="1214" y="248" width="3" height="6" /><rect x="1283" y="248" width="3" height="6" /></g>
		</g>

		<!-- jungle : arbres et palmiers de part et d'autre -->
		<use href="#ci-tree" x="640" y="482" /><use href="#ci-tree" x="760" y="482" transform="translate(0 0)" />
		<use href="#ci-palm" x="900" y="482" />
		<g transform="translate(990 482) scale(.8)"><use href="#ci-tree" /></g>
		<use href="#ci-tree" x="1540" y="482" />
		<g transform="translate(1636 482) scale(1.15)"><use href="#ci-tree" /></g>
		<use href="#ci-palm" x="1470" y="482" />
		<g transform="translate(1736 482) scale(.9)"><use href="#ci-palm" /></g>
		<g transform="translate(1100 482) scale(.7)"><use href="#ci-tree" /></g>
		<g transform="translate(1410 482) scale(.7)"><use href="#ci-tree" /></g>

		<!-- sol : herbe, pelouse devant le temple, pierres -->
		<rect x="540" y="482" width="1260" height="7" fill="var(--ci-edge)" />
		<path d="M1130 489H1370L1420 540H1080z" fill="var(--ci-path)" opacity=".7" />
		<g fill="var(--ci-stone2)"><ellipse cx="1010" cy="520" rx="22" ry="7" /><ellipse cx="1490" cy="528" rx="26" ry="8" /><ellipse cx="760" cy="540" rx="18" ry="6" /></g>
		<g stroke="var(--ci-blade)" stroke-width="2" stroke-linecap="round" opacity=".6"><path d="M620 520h100M820 550h120M1560 512h130M1660 560h100M1240 560h90" /></g>
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
	.scene-chichenitza {
		--sky1: #f2b98a;
		--sky2: #fce9c4;
		--sun: #fff0b8;
		--cloud: #fff;
		--crater: transparent;
		--star: transparent;
		--ground1: #8fae62;
		--ground2: #6f9150;
		--ci-edge: #7a9a55;
		--ci-path: #e4cd9a;
		--ci-blade: #fff;
		--ci-far: #a9b98a;
		--ci-stone: #e9d7ac;
		--ci-light: #f6e9c8;
		--ci-stone2: #d1b987;
		--ci-stone3: #b79b68;
		--ci-stair: #efdfb8;
		--ci-door: #4a3a2c;
		--ci-trunk: #7a5a3a;
		--ci-leaf: #4f8a49;
		--ci-leaf2: #3d7240;
		--ci-leaf3: #66a052;
	}
	:global(.dark) .scene-chichenitza {
		--sky1: #0d1229;
		--sky2: #34264a;
		--sun: #e8eef8;
		--cloud: #2a3658;
		--crater: #b9c4dc;
		--star: #e8eef8;
		--ground1: #1d3a3a;
		--ground2: #0f2226;
		--ci-edge: #1a3434;
		--ci-path: #4a4562;
		--ci-blade: #6fa37c;
		--ci-far: #1c2e46;
		--ci-stone: #7f7894;
		--ci-light: #9892ad;
		--ci-stone2: #625c78;
		--ci-stone3: #433e58;
		--ci-stair: #8a839d;
		--ci-door: #0a0d1a;
		--ci-trunk: #3a3340;
		--ci-leaf: #1f4d46;
		--ci-leaf2: #163b3a;
		--ci-leaf3: #2a5e50;
	}
</style>
