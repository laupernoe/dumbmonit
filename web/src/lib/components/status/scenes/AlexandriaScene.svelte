<script lang="ts">
	/**
	 * The Pharos of Alexandria: a three-stage lighthouse (square, octagonal, round) with a fire in its lantern, on its rock in the harbour, with feluccas and palms. Static, no animation. Follows the scene contract: fills its
	 * positioned parent, the headline zone (top left) is left clear, the
	 * palette lives here with a light and a dark set.
	 */
</script>

<div class="scene scene-alexandria" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
<defs><linearGradient id="al-wat" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--water1)"/><stop offset="1" stop-color="var(--water2)"/></linearGradient><radialGradient id="al-fire"><stop offset="0" stop-color="var(--flame)" stop-opacity=".95"/><stop offset="1" stop-color="var(--flame)" stop-opacity="0"/></radialGradient></defs>
<g fill="var(--star)"><circle cx="1010" cy="118" r="2"/><circle cx="1250" cy="104" r="2.2"/><circle cx="1560" cy="120" r="2"/><circle cx="1650" cy="230" r="2.2"/><circle cx="1130" cy="190" r="1.8"/><circle cx="1440" cy="160" r="1.8"/><circle cx="780" cy="200" r="2"/></g>
<circle cx="1470" cy="200" r="48" fill="var(--sun)"/>
<g fill="var(--cloud)" opacity=".75"><ellipse cx="840" cy="150" rx="80" ry="12"/><ellipse cx="890" cy="164" rx="50" ry="9"/><ellipse cx="1620" cy="300" rx="80" ry="11"/></g>
<!-- skyline of the city -->
<g fill="var(--town)" opacity=".9"><rect x="1360" y="450" width="40" height="30"/><rect x="1404" y="438" width="30" height="42"/><path d="M1438 480V446a14 14 0 0 1 28 0V480Z"/><rect x="1470" y="452" width="50" height="28"/><rect x="1524" y="440" width="26" height="40"/><rect x="1554" y="456" width="60" height="24"/><rect x="1618" y="446" width="30" height="34"/><rect x="1652" y="458" width="60" height="22"/></g>
<rect x="560" y="480" width="1220" height="180" fill="url(#al-wat)"/>
<g stroke="var(--wave)" stroke-width="2.4" stroke-linecap="round" opacity=".5"><path d="M600 500h70M760 520h90M930 508h60M1050 540h110M1230 512h80M1390 534h90M1540 506h70M1650 528h100M700 574h80M920 590h100M1160 578h70M1360 592h100M1560 576h80"/></g>
<g transform="translate(900 498) scale(0.8)"><path d="M-34 0H38L28 12H-24Z" fill="var(--hull)"/><rect x="-1" y="-48" width="2.4" height="48" fill="var(--hull)"/><path d="M2-46L38-8H2Z" fill="var(--sail)"/><path d="M-2-30L-2-6H-22Z" fill="var(--sail)" opacity=".85"/></g><g transform="translate(1480 516) scale(0.9)"><path d="M-34 0H38L28 12H-24Z" fill="var(--hull)"/><rect x="-1" y="-48" width="2.4" height="48" fill="var(--hull)"/><path d="M2-46L38-8H2Z" fill="var(--sail2)"/><path d="M-2-30L-2-6H-22Z" fill="var(--sail2)" opacity=".85"/></g><g transform="translate(700 540) scale(0.7)"><path d="M-34 0H38L28 12H-24Z" fill="var(--hull)"/><rect x="-1" y="-48" width="2.4" height="48" fill="var(--hull)"/><path d="M2-46L38-8H2Z" fill="var(--sail2)"/><path d="M-2-30L-2-6H-22Z" fill="var(--sail2)" opacity=".85"/></g>
<!-- breakwater and rock -->
<path d="M560 484H1780V500H560Z" fill="var(--rock)" opacity="0"/>
<path d="M1000 482C1010 466 1060 452 1100 450H1300C1350 452 1390 466 1404 482Z" fill="var(--rock)"/>
<path d="M1000 482C1030 474 1380 474 1404 482" stroke="var(--foam)" stroke-width="3" opacity=".7"/>
<rect x="1080" y="446" width="244" height="8" fill="var(--stone2)"/>
<path d="M1324 450H1580V460H1324Z" fill="var(--stone2)"/><rect x="1324" y="456" width="262" height="26" fill="var(--stone)"/>
<g fill="var(--dark)" opacity=".4"><rect x="1340" y="464" width="12" height="18" rx="6"/><rect x="1380" y="464" width="12" height="18" rx="6"/><rect x="1420" y="464" width="12" height="18" rx="6"/><rect x="1460" y="464" width="12" height="18" rx="6"/><rect x="1500" y="464" width="12" height="18" rx="6"/><rect x="1540" y="464" width="12" height="18" rx="6"/></g>
<path d="M1120 450C1124 425 1117 410 1122 400" stroke="var(--trunk)" stroke-width="4" stroke-linecap="round"/><g fill="var(--tree)"><path d="M1122 400q-26-4-40 14q20-12 40-6zM1122 400q26-4 40 14q-20-12-40-6zM1122 400q-14-22-34-20q22 2 34 20zM1122 400q14-22 34-20q-22 2-34 20z"/></g><path d="M1280 450C1284 428 1277 414.8 1282 406" stroke="var(--trunk)" stroke-width="4" stroke-linecap="round"/><g fill="var(--tree)"><path d="M1282 406q-26-4-40 14q20-12 40-6zM1282 406q26-4 40 14q-20-12-40-6zM1282 406q-14-22-34-20q22 2 34 20zM1282 406q14-22 34-20q-22 2-34 20z"/></g>
<!-- Pharos -->
<g>
<rect x="1140" y="390" width="124" height="60" fill="var(--stone)"/>
<path d="M1148 450L1162 270H1242L1256 450Z" fill="var(--stone)"/>
<path d="M1210 270H1242L1256 450H1218Z" fill="var(--stone2)" opacity=".55"/>
<path d="M1150 306H1254M1150 342H1254M1150 378H1254M1150 414H1254" stroke="var(--stone2)" stroke-width="2.4"/>
<g fill="var(--dark)"><path d="M1190 450v-26a12 12 0 0 1 24 0v26zM1186 394v-14a5 5 0 0 1 10 0v14zM1208 394v-14a5 5 0 0 1 10 0v14zM1172 330v-12a4 4 0 0 1 8 0v12zM1224 330v-12a4 4 0 0 1 8 0v12zM1198 306v-12a4 4 0 0 1 8 0v12z" opacity=".75"/></g>
<rect x="1154" y="262" width="96" height="10" fill="var(--stone2)"/>
<path d="M1170 262L1176 176H1228L1234 262Z" fill="var(--stone)"/>
<path d="M1212 262L1228 176H1228L1234 262Z" fill="var(--stone2)" opacity=".55"/>
<path d="M1172 236h60M1174 210h56" stroke="var(--stone2)" stroke-width="2.4"/>
<g fill="var(--dark)" opacity=".7"><rect x="1198" y="226" width="6" height="14" rx="3"/><rect x="1198" y="196" width="6" height="14" rx="3"/></g>
<rect x="1168" y="168" width="68" height="10" fill="var(--stone2)"/>
<rect x="1180" y="146" width="44" height="24" fill="var(--stone)"/>
<path d="M1180 146v24M1190 146v24M1202 146v24M1214 146v24M1224 146v24" stroke="var(--dark)" stroke-width="3" opacity="0"/>
<g fill="var(--dark)" opacity=".85"><rect x="1183" y="150" width="6" height="18"/><rect x="1197" y="150" width="6" height="18"/><rect x="1211" y="150" width="6" height="18"/></g>
<path d="M1176 146H1228L1202 122Z" fill="var(--stone2)"/>
<path d="M1202 122v-14" stroke="var(--stone2)" stroke-width="3"/>
<circle cx="1202" cy="157" r="46" fill="url(#al-fire)"/>
<path d="M1202 168C1188 160 1192 150 1202 138C1206 148 1216 152 1214 162C1214 166 1208 168 1202 168Z" fill="var(--flame)"/>
<path d="M1202 166C1196 160 1200 154 1202 148C1206 154 1208 160 1202 166Z" fill="var(--flame2)"/>
<path d="M1196 108l6-12 6 12z" fill="var(--flame)"/>
</g>
	</svg>
</div>

<style>
	.scene { position: absolute; inset: 0; overflow: hidden; background: linear-gradient(180deg, var(--sky1), var(--sky2) 73.5%, var(--water1) 74.4%, var(--water1) 80%, var(--water2)); }
	svg { position: absolute; inset: 0; width: 100%; height: 100%; -webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%); mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%); }
	.scene-alexandria {
		--sky1: #f7d4a4; --sky2: #fdeed3; --sun: #fff3c8; --star: transparent; --cloud: #fff; --water1: #64b6c6; --water2: #2f86a0; --wave: #fff; --foam: #fff; --town: #eadbba; --rock: #c9ad82; --stone: #f2e7cb; --stone2: #d8c69f; --dark: #5a4a38; --trunk: #86684a; --tree: #6a8f56; --flame: #f2a53a; --flame2: #fff0a8; --hull: #5c3d28; --sail: #fbf3e1; --sail2: #e6a97a;
	}
	:global(.dark) .scene-alexandria {
		--sky1: #0b1026; --sky2: #26203f; --sun: #e8eef8; --star: #e8eef8; --cloud: #2a3658; --water1: #17305a; --water2: #0a1530; --wave: #9fb6e0; --foam: #9fb6e0; --town: #4a4670; --rock: #3a3858; --stone: #7d7f9c; --stone2: #5a5c7a; --dark: #0e0d22; --trunk: #3f3646; --tree: #1f4a4f; --flame: #ffb347; --flame2: #fff0a8; --hull: #14122a; --sail: #8a90a8; --sail2: #8a6a60;
	}
</style>
