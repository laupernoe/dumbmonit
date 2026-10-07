<script lang="ts">
	/**
	 * The Colossus of Rhodes: a giant bronze sun god straddling the harbour entrance on two stone moles, torch raised, sailing ships passing beneath. Static, no animation. Follows the scene contract: fills its
	 * positioned parent, the headline zone (top left) is left clear, the
	 * palette lives here with a light and a dark set.
	 */
</script>

<div class="scene scene-rhodes" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
<defs><linearGradient id="rh-wat" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--water1)"/><stop offset="1" stop-color="var(--water2)"/></linearGradient><radialGradient id="rh-flame"><stop offset="0" stop-color="var(--flame)" stop-opacity=".9"/><stop offset="1" stop-color="var(--flame)" stop-opacity="0"/></radialGradient></defs>
<g fill="var(--star)"><circle cx="1010" cy="118" r="2"/><circle cx="1250" cy="104" r="2.2"/><circle cx="1560" cy="120" r="2"/><circle cx="1650" cy="230" r="2.2"/><circle cx="1130" cy="190" r="1.8"/><circle cx="1440" cy="160" r="1.8"/><circle cx="780" cy="200" r="2"/></g>
<circle cx="1204" cy="196" r="82" fill="var(--sun)" opacity=".55"/><circle cx="1204" cy="196" r="56" fill="var(--sun)"/>
<g fill="var(--cloud)" opacity=".75"><ellipse cx="820" cy="140" rx="80" ry="12"/><ellipse cx="870" cy="154" rx="50" ry="9"/><ellipse cx="1640" cy="250" rx="80" ry="11"/><ellipse cx="1580" cy="130" rx="50" ry="8"/></g>
<path d="M560 482C640 430 760 410 860 452C940 426 1030 436 1100 482Z" fill="var(--hill)" opacity=".8"/>
<path d="M1320 482C1400 440 1500 420 1600 454C1670 436 1730 446 1780 470V482Z" fill="var(--hill)" opacity=".8"/>
<g fill="var(--town)" opacity=".9"><rect x="1600" y="448" width="22" height="18"/><rect x="1626" y="440" width="26" height="26"/><rect x="1656" y="452" width="22" height="14"/><rect x="1682" y="444" width="20" height="22"/></g>
<rect x="560" y="480" width="1220" height="180" fill="url(#rh-wat)"/>
<g stroke="var(--wave)" stroke-width="2.4" stroke-linecap="round" opacity=".55"><path d="M640 500h70M780 520h90M930 508h60M1040 540h110M1200 512h80M1340 534h90M1500 506h70M1620 528h100M700 570h80M900 586h100M1120 576h70M1320 590h100M1520 574h80"/></g>
<g transform="translate(1202 486) scale(0.9)"><path d="M-30 0H34L26 12H-22Z" fill="var(--hull)"/><rect x="-1" y="-54" width="2.4" height="54" fill="var(--hull)"/><path d="M3-52L3-6L32-6Z" fill="var(--sail)"/><path d="M-3-44L-3-6L-24-6Z" fill="var(--sail)" opacity=".85"/></g><g transform="translate(1440 500) scale(0.7)"><path d="M-30 0H34L26 12H-22Z" fill="var(--hull)"/><rect x="-1" y="-54" width="2.4" height="54" fill="var(--hull)"/><path d="M3-52L3-6L32-6Z" fill="var(--sail2)"/><path d="M-3-44L-3-6L-24-6Z" fill="var(--sail2)" opacity=".85"/></g><g transform="translate(900 498) scale(0.65)"><path d="M-30 0H34L26 12H-22Z" fill="var(--hull)"/><rect x="-1" y="-54" width="2.4" height="54" fill="var(--hull)"/><path d="M3-52L3-6L32-6Z" fill="var(--sail2)"/><path d="M-3-44L-3-6L-24-6Z" fill="var(--sail2)" opacity=".85"/></g>
<!-- moles -->
<g>
<path d="M980 482V446H1146V482Z" fill="var(--stone)"/><rect x="980" y="446" width="166" height="6" fill="var(--stone2)"/>
<path d="M1262 482V446H1428V482Z" fill="var(--stone)"/><rect x="1262" y="446" width="166" height="6" fill="var(--stone2)"/>
<rect x="990" y="394" width="30" height="52" fill="var(--stone)"/><path d="M986 394h38v-8h-6v-6h-6v6h-6v-6h-6v6h-6v-6h-6v6h-2z" fill="var(--stone2)"/><rect x="1002" y="410" width="6" height="12" fill="var(--dark)"/>
<rect x="1388" y="400" width="30" height="46" fill="var(--stone)"/><path d="M1384 400h38v-8h-6v-6h-6v6h-6v-6h-6v6h-6v-6h-6v6h-2z" fill="var(--stone2)"/><rect x="1400" y="414" width="6" height="12" fill="var(--dark)"/>
<g fill="var(--dark)" opacity=".35"><rect x="1010" y="458" width="12" height="24" rx="6"/><rect x="1060" y="458" width="12" height="24" rx="6"/><rect x="1300" y="458" width="12" height="24" rx="6"/><rect x="1350" y="458" width="12" height="24" rx="6"/></g>
</g>
<!-- Colossus -->
<g>
<rect x="1062" y="424" width="64" height="24" fill="var(--stone2)"/><rect x="1282" y="424" width="64" height="24" fill="var(--stone2)"/>
<path d="M1070 426L1118 426L1100 396L1196 318L1176 292L1090 372L1066 400Z" fill="var(--bronze2)" opacity="0"/>
<path d="M1068 424L1110 424L1204 330L1184 304Z" fill="var(--bronze2)"/>
<path d="M1340 424L1298 424L1210 330L1224 304Z" fill="var(--bronze)"/>
<path d="M1160 330L1166 262L1194 236H1218L1244 262L1250 330Q1204 346 1160 330Z" fill="var(--bronze)"/>
<path d="M1160 330L1166 262L1180 250L1190 330Z" fill="var(--bronze2)" opacity=".6"/>
<path d="M1166 262L1140 316L1160 322L1172 292Z" fill="var(--bronze2)"/>
<path d="M1156 318L1178 340L1204 312L1232 340L1252 318L1250 332Q1204 350 1158 332Z" fill="var(--cloth)"/>
<path d="M1226 252L1250 214L1262 154L1274 156L1264 220L1242 266Z" fill="var(--bronze)"/>
<path d="M1262 156L1268 130" stroke="var(--bronze2)" stroke-width="7" stroke-linecap="round"/>
<path d="M1256 134h24l-4 14h-16Z" fill="var(--bronze2)"/>
<circle cx="1268" cy="116" r="30" fill="url(#rh-flame)"/>
<path d="M1268 134C1254 124 1260 108 1268 94C1270 106 1282 110 1280 124C1280 130 1274 134 1268 134Z" fill="var(--flame)"/>
<path d="M1268 130C1262 124 1266 116 1268 110C1272 116 1274 122 1268 130Z" fill="var(--flame2)"/>
<rect x="1196" y="218" width="16" height="22" fill="var(--bronze2)"/>
<circle cx="1204" cy="196" r="19" fill="var(--bronze)"/><path d="M1204 178a19 19 0 0 0-19 19c8-4 14-12 19-19Z" fill="var(--bronze2)" opacity=".5"/>
<g fill="var(--flame)"><path d="M1194.1 175.7L1176.5 166L1191.2 179.8Z"/><path d="M1197.8 173.2L1184 158.5L1193.7 176.1Z"/><path d="M1202.1 171.9L1193.5 153.7L1197.3 173.5Z"/><path d="M1206.5 172L1204 152L1201.5 172Z"/><path d="M1210.7 173.5L1214.5 153.7L1205.9 171.9Z"/><path d="M1214.3 176.1L1224 158.5L1210.2 173.2Z"/><path d="M1216.8 179.8L1231.5 166L1213.9 175.7Z"/></g>
</g>
	</svg>
</div>

<style>
	.scene { position: absolute; inset: 0; overflow: hidden; background: linear-gradient(180deg, var(--sky1), var(--sky2) 73.5%, var(--water1) 74.4%, var(--water1) 80%, var(--water2)); }
	svg { position: absolute; inset: 0; width: 100%; height: 100%; -webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%); mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%); }
	.scene-rhodes {
		--sky1: #f7cfa0; --sky2: #fdecd0; --sun: #fff3c8; --star: transparent; --cloud: #fff; --water1: #5fb0c4; --water2: #2f7e9a; --wave: #fff; --hill: #d7b88e; --town: #f3e6cc; --stone: #ecdcba; --stone2: #d1bd94; --dark: #4a3b2c; --bronze: #c88a46; --bronze2: #98602b; --cloth: #a9703a; --flame: #f0a232; --flame2: #fff0a8; --hull: #5c3d28; --sail: #fbf3e1; --sail2: #e9b88a;
	}
	:global(.dark) .scene-rhodes {
		--sky1: #0d1229; --sky2: #2d2248; --sun: #e8eef8; --star: #e8eef8; --cloud: #2a3658; --water1: #17305a; --water2: #0a1530; --wave: #9fb6e0; --hill: #2a2b4c; --town: #6a6486; --stone: #6d6e8c; --stone2: #51526e; --dark: #0e0d22; --bronze: #9a6a3c; --bronze2: #6b4529; --cloth: #7a5030; --flame: #ffb347; --flame2: #fff0a8; --hull: #14122a; --sail: #8a90a8; --sail2: #8a6a60;
	}
</style>
