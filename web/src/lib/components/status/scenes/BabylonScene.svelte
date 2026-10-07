<script lang="ts">
	/**
	 * The Hanging Gardens of Babylon — a stepped ziggurat of brick terraces
	 * planted with palms and trailing vines, the blue Ishtar Gate in the
	 * distance, the Euphrates in front. Static, no animation. Follows the scene
	 * contract: fills its positioned parent, the headline zone (top left) is
	 * left clear, the palette lives here.
	 */
</script>

<div class="scene scene-babylon" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="bb-wat" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--water1)" /><stop offset="1" stop-color="var(--water2)" />
			</linearGradient>
			<g id="bb-palm">
				<path d="M0 0C3-16 1-30 4-46" stroke="var(--trunk)" stroke-width="3.4" stroke-linecap="round" />
				<g fill="var(--leaf2)"><path d="M4-46C-8-56-22-54-32-44C-18-48-8-48 4-46zM4-46C16-56 30-54 40-44C26-48 16-48 4-46zM4-46C-2-62-14-68-24-66C-12-62-6-56 4-46zM4-46C10-62 22-68 32-66C20-62 14-56 4-46zM4-46C4-60 4-68 4-74C10-66 10-56 4-46z" /></g>
			</g>
		</defs>

		<!-- sun / moon, stars, clouds -->
		<circle cx="1630" cy="190" r="56" fill="var(--sun-halo)" opacity=".5" />
		<circle cx="1630" cy="190" r="36" fill="var(--sun)" />
		<circle cx="1618" cy="180" r="5" fill="var(--crater)" /><circle cx="1642" cy="200" r="4" fill="var(--crater)" />
		<g fill="var(--star)"><circle cx="800" cy="120" r="2" /><circle cx="1010" cy="104" r="2.2" /><circle cx="1230" cy="116" r="1.8" /><circle cx="1400" cy="108" r="2" /><circle cx="1500" cy="150" r="2.2" /><circle cx="880" cy="200" r="1.8" /><circle cx="1120" cy="170" r="1.6" /><circle cx="1720" cy="130" r="2" /></g>
		<g fill="var(--cloud)" opacity=".75"><ellipse cx="900" cy="150" rx="100" ry="9" /><ellipse cx="960" cy="166" rx="58" ry="7" /><ellipse cx="1480" cy="130" rx="80" ry="8" /></g>

		<!-- far city wall and plain -->
		<path d="M560 482V450h60v-10h12v10h18v-10h12v10h18v-10h12v10h60v-18h40v18h90v-10h12v10h18v-10h12v10h18v-10h12v10h40v32z" fill="var(--far)" />
		<path d="M1640 482V446h26v-10h10v10h14v-10h10v10h26v-14h20v14h40v36z" fill="var(--far)" />

		<!-- Ishtar Gate, left -->
		<g>
			<rect x="640" y="372" width="64" height="110" fill="var(--lapis)" />
			<rect x="780" y="372" width="64" height="110" fill="var(--lapis)" />
			<rect x="704" y="404" width="76" height="78" fill="var(--lapis-d)" />
			<path d="M716 482V440a26 26 0 0 1 52 0V482z" fill="var(--arch)" />
			<g fill="var(--lapis)"><path d="M632 372h80v-12h-10v-10h-10v10h-12v-10h-10v10h-12v-10h-10v10h-6zM772 372h80v-12h-10v-10h-10v10h-12v-10h-10v10h-12v-10h-10v10h-6z" /></g>
			<g fill="var(--tile)"><rect x="632" y="368" width="80" height="6" /><rect x="772" y="368" width="80" height="6" /><rect x="632" y="420" width="80" height="5" /><rect x="772" y="420" width="80" height="5" /><rect x="704" y="404" width="76" height="5" /></g>
			<!-- aurochs / dragon reliefs -->
			<g fill="var(--tile)"><path d="M650 400h22l2-8 6 2-2 10v8h-4v-8h-14v8h-4v-8h-6z" /><path d="M790 400h22l2-8 6 2-2 10v8h-4v-8h-14v8h-4v-8h-6z" /><path d="M650 448h22l2-8 6 2-2 10v8h-4v-8h-14v8h-4v-8h-6z" /><path d="M790 448h22l2-8 6 2-2 10v8h-4v-8h-14v8h-4v-8h-6z" /></g>
			<g fill="var(--lamp)" opacity=".9"><rect x="736" y="420" width="12" height="16" rx="6" opacity="var(--lamp-on)" /></g>
		</g>

		<!-- the gardens -->
		<g>
		<rect x="930" y="438" width="630" height="44" fill="var(--brick)" />
		<rect x="930" y="438" width="630" height="5" fill="var(--brick-lit)" />
		<path d="M944 482V462a9 9 0 0 1 18 0V482zM988 482V462a9 9 0 0 1 18 0V482zM1034 482V462a9 9 0 0 1 18 0V482zM1078 482V462a9 9 0 0 1 18 0V482zM1124 482V462a9 9 0 0 1 18 0V482zM1168 482V462a9 9 0 0 1 18 0V482zM1214 482V462a9 9 0 0 1 18 0V482zM1258 482V462a9 9 0 0 1 18 0V482zM1304 482V462a9 9 0 0 1 18 0V482zM1348 482V462a9 9 0 0 1 18 0V482zM1394 482V462a9 9 0 0 1 18 0V482zM1438 482V462a9 9 0 0 1 18 0V482zM1484 482V462a9 9 0 0 1 18 0V482zM1528 482V462a9 9 0 0 1 18 0V482z" fill="var(--arch)" />
		<rect x="924" y="431" width="642" height="9" fill="var(--soil)" />
		<g fill="var(--leaf)"><ellipse cx="934" cy="428" rx="13" ry="9" /><ellipse cx="953" cy="428" rx="13" ry="9" /><ellipse cx="972" cy="428" rx="13" ry="9" /><ellipse cx="991" cy="428" rx="13" ry="9" /><ellipse cx="1010" cy="428" rx="13" ry="9" /><ellipse cx="1029" cy="428" rx="13" ry="9" /><ellipse cx="1048" cy="428" rx="13" ry="9" /><ellipse cx="1067" cy="428" rx="13" ry="9" /><ellipse cx="1086" cy="428" rx="13" ry="9" /><ellipse cx="1105" cy="428" rx="13" ry="9" /><ellipse cx="1124" cy="428" rx="13" ry="9" /><ellipse cx="1143" cy="428" rx="13" ry="9" /><ellipse cx="1162" cy="428" rx="13" ry="9" /><ellipse cx="1181" cy="428" rx="13" ry="9" /><ellipse cx="1200" cy="428" rx="13" ry="9" /><ellipse cx="1219" cy="428" rx="13" ry="9" /><ellipse cx="1238" cy="428" rx="13" ry="9" /><ellipse cx="1257" cy="428" rx="13" ry="9" /><ellipse cx="1276" cy="428" rx="13" ry="9" /><ellipse cx="1295" cy="428" rx="13" ry="9" /><ellipse cx="1314" cy="428" rx="13" ry="9" /><ellipse cx="1333" cy="428" rx="13" ry="9" /><ellipse cx="1352" cy="428" rx="13" ry="9" /><ellipse cx="1371" cy="428" rx="13" ry="9" /><ellipse cx="1390" cy="428" rx="13" ry="9" /><ellipse cx="1409" cy="428" rx="13" ry="9" /><ellipse cx="1428" cy="428" rx="13" ry="9" /><ellipse cx="1447" cy="428" rx="13" ry="9" /><ellipse cx="1466" cy="428" rx="13" ry="9" /><ellipse cx="1485" cy="428" rx="13" ry="9" /><ellipse cx="1504" cy="428" rx="13" ry="9" /><ellipse cx="1523" cy="428" rx="13" ry="9" /><ellipse cx="1542" cy="428" rx="13" ry="9" /><ellipse cx="1561" cy="428" rx="13" ry="9" /></g>
		<path d="M928 438c-3 4 3 9 0 14M944 438c-3 9 3 18 0 28M960 438c-3 14 3 28 0 42M976 438c-3 7 3 14 0 21M992 438c-3 11 3 23 0 35M1008 438c-3 4 3 9 0 14M1024 438c-3 9 3 18 0 28M1040 438c-3 14 3 28 0 42M1056 438c-3 7 3 14 0 21M1072 438c-3 11 3 23 0 35M1088 438c-3 4 3 9 0 14M1104 438c-3 9 3 18 0 28M1120 438c-3 14 3 28 0 42M1136 438c-3 7 3 14 0 21M1152 438c-3 11 3 23 0 35M1168 438c-3 4 3 9 0 14M1184 438c-3 9 3 18 0 28M1200 438c-3 14 3 28 0 42M1216 438c-3 7 3 14 0 21M1232 438c-3 11 3 23 0 35M1248 438c-3 4 3 9 0 14M1264 438c-3 9 3 18 0 28M1280 438c-3 14 3 28 0 42M1296 438c-3 7 3 14 0 21M1312 438c-3 11 3 23 0 35M1328 438c-3 4 3 9 0 14M1344 438c-3 9 3 18 0 28M1360 438c-3 14 3 28 0 42M1376 438c-3 7 3 14 0 21M1392 438c-3 11 3 23 0 35M1408 438c-3 4 3 9 0 14M1424 438c-3 9 3 18 0 28M1440 438c-3 14 3 28 0 42M1456 438c-3 7 3 14 0 21M1472 438c-3 11 3 23 0 35M1488 438c-3 4 3 9 0 14M1504 438c-3 9 3 18 0 28M1520 438c-3 14 3 28 0 42M1536 438c-3 7 3 14 0 21M1552 438c-3 11 3 23 0 35" stroke="var(--leaf2)" stroke-width="3.2" stroke-linecap="round" />
		<rect x="980" y="394" width="530" height="44" fill="var(--brick)" />
		<rect x="980" y="394" width="530" height="5" fill="var(--brick-lit)" />
		<path d="M993 438V418a9 9 0 0 1 18 0V438zM1037 438V418a9 9 0 0 1 18 0V438zM1081 438V418a9 9 0 0 1 18 0V438zM1126 438V418a9 9 0 0 1 18 0V438zM1170 438V418a9 9 0 0 1 18 0V438zM1214 438V418a9 9 0 0 1 18 0V438zM1258 438V418a9 9 0 0 1 18 0V438zM1302 438V418a9 9 0 0 1 18 0V438zM1346 438V418a9 9 0 0 1 18 0V438zM1391 438V418a9 9 0 0 1 18 0V438zM1435 438V418a9 9 0 0 1 18 0V438zM1479 438V418a9 9 0 0 1 18 0V438z" fill="var(--arch)" />
		<rect x="974" y="387" width="542" height="9" fill="var(--soil)" />
		<g fill="var(--leaf)"><ellipse cx="984" cy="384" rx="13" ry="9" /><ellipse cx="1003" cy="384" rx="13" ry="9" /><ellipse cx="1022" cy="384" rx="13" ry="9" /><ellipse cx="1041" cy="384" rx="13" ry="9" /><ellipse cx="1060" cy="384" rx="13" ry="9" /><ellipse cx="1079" cy="384" rx="13" ry="9" /><ellipse cx="1098" cy="384" rx="13" ry="9" /><ellipse cx="1117" cy="384" rx="13" ry="9" /><ellipse cx="1136" cy="384" rx="13" ry="9" /><ellipse cx="1155" cy="384" rx="13" ry="9" /><ellipse cx="1174" cy="384" rx="13" ry="9" /><ellipse cx="1193" cy="384" rx="13" ry="9" /><ellipse cx="1212" cy="384" rx="13" ry="9" /><ellipse cx="1231" cy="384" rx="13" ry="9" /><ellipse cx="1250" cy="384" rx="13" ry="9" /><ellipse cx="1269" cy="384" rx="13" ry="9" /><ellipse cx="1288" cy="384" rx="13" ry="9" /><ellipse cx="1307" cy="384" rx="13" ry="9" /><ellipse cx="1326" cy="384" rx="13" ry="9" /><ellipse cx="1345" cy="384" rx="13" ry="9" /><ellipse cx="1364" cy="384" rx="13" ry="9" /><ellipse cx="1383" cy="384" rx="13" ry="9" /><ellipse cx="1402" cy="384" rx="13" ry="9" /><ellipse cx="1421" cy="384" rx="13" ry="9" /><ellipse cx="1440" cy="384" rx="13" ry="9" /><ellipse cx="1459" cy="384" rx="13" ry="9" /><ellipse cx="1478" cy="384" rx="13" ry="9" /><ellipse cx="1497" cy="384" rx="13" ry="9" /><ellipse cx="1516" cy="384" rx="13" ry="9" /></g>
		<path d="M978 394c-3 11 3 23 0 35M994 394c-3 4 3 9 0 14M1010 394c-3 9 3 18 0 28M1026 394c-3 14 3 28 0 42M1042 394c-3 7 3 14 0 21M1058 394c-3 11 3 23 0 35M1074 394c-3 4 3 9 0 14M1090 394c-3 9 3 18 0 28M1106 394c-3 14 3 28 0 42M1122 394c-3 7 3 14 0 21M1138 394c-3 11 3 23 0 35M1154 394c-3 4 3 9 0 14M1170 394c-3 9 3 18 0 28M1186 394c-3 14 3 28 0 42M1202 394c-3 7 3 14 0 21M1218 394c-3 11 3 23 0 35M1234 394c-3 4 3 9 0 14M1250 394c-3 9 3 18 0 28M1266 394c-3 14 3 28 0 42M1282 394c-3 7 3 14 0 21M1298 394c-3 11 3 23 0 35M1314 394c-3 4 3 9 0 14M1330 394c-3 9 3 18 0 28M1346 394c-3 14 3 28 0 42M1362 394c-3 7 3 14 0 21M1378 394c-3 11 3 23 0 35M1394 394c-3 4 3 9 0 14M1410 394c-3 9 3 18 0 28M1426 394c-3 14 3 28 0 42M1442 394c-3 7 3 14 0 21M1458 394c-3 11 3 23 0 35M1474 394c-3 4 3 9 0 14M1490 394c-3 9 3 18 0 28M1506 394c-3 14 3 28 0 42" stroke="var(--leaf2)" stroke-width="3.2" stroke-linecap="round" />
		<rect x="1030" y="350" width="430" height="44" fill="var(--brick)" />
		<rect x="1030" y="350" width="430" height="5" fill="var(--brick-lit)" />
		<path d="M1045 394V374a9 9 0 0 1 18 0V394zM1093 394V374a9 9 0 0 1 18 0V394zM1140 394V374a9 9 0 0 1 18 0V394zM1188 394V374a9 9 0 0 1 18 0V394zM1236 394V374a9 9 0 0 1 18 0V394zM1284 394V374a9 9 0 0 1 18 0V394zM1332 394V374a9 9 0 0 1 18 0V394zM1379 394V374a9 9 0 0 1 18 0V394zM1427 394V374a9 9 0 0 1 18 0V394z" fill="var(--arch)" />
		<rect x="1024" y="343" width="442" height="9" fill="var(--soil)" />
		<g fill="var(--leaf)"><ellipse cx="1034" cy="340" rx="13" ry="9" /><ellipse cx="1053" cy="340" rx="13" ry="9" /><ellipse cx="1072" cy="340" rx="13" ry="9" /><ellipse cx="1091" cy="340" rx="13" ry="9" /><ellipse cx="1110" cy="340" rx="13" ry="9" /><ellipse cx="1129" cy="340" rx="13" ry="9" /><ellipse cx="1148" cy="340" rx="13" ry="9" /><ellipse cx="1167" cy="340" rx="13" ry="9" /><ellipse cx="1186" cy="340" rx="13" ry="9" /><ellipse cx="1205" cy="340" rx="13" ry="9" /><ellipse cx="1224" cy="340" rx="13" ry="9" /><ellipse cx="1243" cy="340" rx="13" ry="9" /><ellipse cx="1262" cy="340" rx="13" ry="9" /><ellipse cx="1281" cy="340" rx="13" ry="9" /><ellipse cx="1300" cy="340" rx="13" ry="9" /><ellipse cx="1319" cy="340" rx="13" ry="9" /><ellipse cx="1338" cy="340" rx="13" ry="9" /><ellipse cx="1357" cy="340" rx="13" ry="9" /><ellipse cx="1376" cy="340" rx="13" ry="9" /><ellipse cx="1395" cy="340" rx="13" ry="9" /><ellipse cx="1414" cy="340" rx="13" ry="9" /><ellipse cx="1433" cy="340" rx="13" ry="9" /><ellipse cx="1452" cy="340" rx="13" ry="9" /><ellipse cx="1471" cy="340" rx="13" ry="9" /></g>
		<path d="M1028 350c-3 7 3 14 0 21M1044 350c-3 11 3 23 0 35M1060 350c-3 4 3 9 0 14M1076 350c-3 9 3 18 0 28M1092 350c-3 14 3 28 0 42M1108 350c-3 7 3 14 0 21M1124 350c-3 11 3 23 0 35M1140 350c-3 4 3 9 0 14M1156 350c-3 9 3 18 0 28M1172 350c-3 14 3 28 0 42M1188 350c-3 7 3 14 0 21M1204 350c-3 11 3 23 0 35M1220 350c-3 4 3 9 0 14M1236 350c-3 9 3 18 0 28M1252 350c-3 14 3 28 0 42M1268 350c-3 7 3 14 0 21M1284 350c-3 11 3 23 0 35M1300 350c-3 4 3 9 0 14M1316 350c-3 9 3 18 0 28M1332 350c-3 14 3 28 0 42M1348 350c-3 7 3 14 0 21M1364 350c-3 11 3 23 0 35M1380 350c-3 4 3 9 0 14M1396 350c-3 9 3 18 0 28M1412 350c-3 14 3 28 0 42M1428 350c-3 7 3 14 0 21M1444 350c-3 11 3 23 0 35M1460 350c-3 4 3 9 0 14" stroke="var(--leaf2)" stroke-width="3.2" stroke-linecap="round" />
		<rect x="1080" y="306" width="330" height="44" fill="var(--brick)" />
		<rect x="1080" y="306" width="330" height="5" fill="var(--brick-lit)" />
		<path d="M1095 350V330a9 9 0 0 1 18 0V350zM1142 350V330a9 9 0 0 1 18 0V350zM1189 350V330a9 9 0 0 1 18 0V350zM1236 350V330a9 9 0 0 1 18 0V350zM1283 350V330a9 9 0 0 1 18 0V350zM1330 350V330a9 9 0 0 1 18 0V350zM1377 350V330a9 9 0 0 1 18 0V350z" fill="var(--arch)" />
		<rect x="1074" y="299" width="342" height="9" fill="var(--soil)" />
		<g fill="var(--leaf)"><ellipse cx="1084" cy="296" rx="13" ry="9" /><ellipse cx="1103" cy="296" rx="13" ry="9" /><ellipse cx="1122" cy="296" rx="13" ry="9" /><ellipse cx="1141" cy="296" rx="13" ry="9" /><ellipse cx="1160" cy="296" rx="13" ry="9" /><ellipse cx="1179" cy="296" rx="13" ry="9" /><ellipse cx="1198" cy="296" rx="13" ry="9" /><ellipse cx="1217" cy="296" rx="13" ry="9" /><ellipse cx="1236" cy="296" rx="13" ry="9" /><ellipse cx="1255" cy="296" rx="13" ry="9" /><ellipse cx="1274" cy="296" rx="13" ry="9" /><ellipse cx="1293" cy="296" rx="13" ry="9" /><ellipse cx="1312" cy="296" rx="13" ry="9" /><ellipse cx="1331" cy="296" rx="13" ry="9" /><ellipse cx="1350" cy="296" rx="13" ry="9" /><ellipse cx="1369" cy="296" rx="13" ry="9" /><ellipse cx="1388" cy="296" rx="13" ry="9" /><ellipse cx="1407" cy="296" rx="13" ry="9" /></g>
		<path d="M1078 306c-3 14 3 28 0 42M1094 306c-3 7 3 14 0 21M1110 306c-3 11 3 23 0 35M1126 306c-3 4 3 9 0 14M1142 306c-3 9 3 18 0 28M1158 306c-3 14 3 28 0 42M1174 306c-3 7 3 14 0 21M1190 306c-3 11 3 23 0 35M1206 306c-3 4 3 9 0 14M1222 306c-3 9 3 18 0 28M1238 306c-3 14 3 28 0 42M1254 306c-3 7 3 14 0 21M1270 306c-3 11 3 23 0 35M1286 306c-3 4 3 9 0 14M1302 306c-3 9 3 18 0 28M1318 306c-3 14 3 28 0 42M1334 306c-3 7 3 14 0 21M1350 306c-3 11 3 23 0 35M1366 306c-3 4 3 9 0 14M1382 306c-3 9 3 18 0 28M1398 306c-3 14 3 28 0 42M1414 306c-3 7 3 14 0 21" stroke="var(--leaf2)" stroke-width="3.2" stroke-linecap="round" />
		<rect x="1130" y="262" width="230" height="44" fill="var(--brick)" />
		<rect x="1130" y="262" width="230" height="5" fill="var(--brick-lit)" />
		<path d="M1144 306V286a9 9 0 0 1 18 0V306zM1190 306V286a9 9 0 0 1 18 0V306zM1236 306V286a9 9 0 0 1 18 0V306zM1282 306V286a9 9 0 0 1 18 0V306zM1328 306V286a9 9 0 0 1 18 0V306z" fill="var(--arch)" />
		<rect x="1124" y="255" width="242" height="9" fill="var(--soil)" />
		<g fill="var(--leaf)"><ellipse cx="1134" cy="252" rx="13" ry="9" /><ellipse cx="1153" cy="252" rx="13" ry="9" /><ellipse cx="1172" cy="252" rx="13" ry="9" /><ellipse cx="1191" cy="252" rx="13" ry="9" /><ellipse cx="1210" cy="252" rx="13" ry="9" /><ellipse cx="1229" cy="252" rx="13" ry="9" /><ellipse cx="1248" cy="252" rx="13" ry="9" /><ellipse cx="1267" cy="252" rx="13" ry="9" /><ellipse cx="1286" cy="252" rx="13" ry="9" /><ellipse cx="1305" cy="252" rx="13" ry="9" /><ellipse cx="1324" cy="252" rx="13" ry="9" /><ellipse cx="1343" cy="252" rx="13" ry="9" /><ellipse cx="1362" cy="252" rx="13" ry="9" /></g>
		<path d="M1128 262c-3 9 3 18 0 28M1144 262c-3 14 3 28 0 42M1160 262c-3 7 3 14 0 21M1176 262c-3 11 3 23 0 35M1192 262c-3 4 3 9 0 14M1208 262c-3 9 3 18 0 28M1224 262c-3 14 3 28 0 42M1240 262c-3 7 3 14 0 21M1256 262c-3 11 3 23 0 35M1272 262c-3 4 3 9 0 14M1288 262c-3 9 3 18 0 28M1304 262c-3 14 3 28 0 42M1320 262c-3 7 3 14 0 21M1336 262c-3 11 3 23 0 35M1352 262c-3 4 3 9 0 14" stroke="var(--leaf2)" stroke-width="3.2" stroke-linecap="round" />
		</g>
		<rect x="930" y="474" width="630" height="8" fill="var(--soil)" />
		<use href="#bb-palm" transform="translate(1190 258) scale(1.15)" />
		<use href="#bb-palm" transform="translate(1260 256) scale(1.45)" />
		<use href="#bb-palm" transform="translate(1330 258) scale(1.1)" />
		<use href="#bb-palm" transform="translate(1100 302) scale(1)" />
		<use href="#bb-palm" transform="translate(1420 302) scale(1.05)" />
		<use href="#bb-palm" transform="translate(1000 346) scale(0.95)" />
		<use href="#bb-palm" transform="translate(1520 346) scale(0.9)" />

		<!-- bank -->
		<path d="M560 482H1780V496C1500 490 1100 492 560 498z" fill="var(--bank)" />
		<!-- river -->
		<rect x="560" y="494" width="1220" height="130" fill="url(#bb-wat)" />
		<g opacity=".28" transform="translate(0 988) scale(1 -1)"><rect x="930" y="494" width="630" height="44" fill="var(--brick)" /><rect x="980" y="450" width="530" height="44" fill="var(--leaf)" /><rect x="640" y="494" width="204" height="40" fill="var(--lapis)" /></g>
		<g stroke="var(--glint)" stroke-width="2" stroke-linecap="round" opacity=".55"><path d="M640 530h70M800 548h90M960 524h60M1130 542h110M1300 530h80M1450 552h90M1620 534h70M720 580h110M1000 584h80M1260 586h120" /></g>

		<!-- reed boat -->
		<g transform="translate(1040 560)">
			<path d="M0 0C20 12 90 12 112-6C112-8 104-4 96-2C70 2 30 2 6-6z" fill="var(--hull)" />
			<path d="M56-4V-34M56-34l30 24" stroke="var(--hull)" stroke-width="3" stroke-linecap="round" />
			<path d="M56-34l-22 18" stroke="var(--hull)" stroke-width="3" stroke-linecap="round" />
		</g>
		<!-- foreground bank palms -->
		<g transform="translate(878 502)"><use href="#bb-palm" transform="translate(0 0) scale(1.7)" /></g>
		<g transform="translate(1600 502)"><use href="#bb-palm" transform="translate(0 0) scale(1.9)" /></g>
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
			var(--bank) 74.4%,
			var(--water1) 76%,
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
	.scene-babylon {
		--sky1: #aacfd0;
		--sky2: #f7e6c4;
		--sun: #fff3c8;
		--sun-halo: #ffe3a6;
		--cloud: #fff;
		--star: transparent;
		--crater: transparent;
		--far: #e2cfa6;
		--brick: #d0a56e;
		--brick-lit: #ecc88e;
		--arch: #8f6a45;
		--soil: #7a5a3a;
		--leaf: #6fa352;
		--leaf2: #4f8a47;
		--trunk: #7a5a3a;
		--lapis: #2f68ad;
		--lapis-d: #25558f;
		--tile: #f0cf74;
		--lamp: #f5c76a;
		--lamp-on: 0;
		--bank: #bfa371;
		--water1: #7fb7b5;
		--water2: #4f8e93;
		--glint: #fff;
		--hull: #5b4330;
	}
	:global(.dark) .scene-babylon {
		--sky1: #0b1230;
		--sky2: #2f2a52;
		--sun: #e8eef8;
		--sun-halo: #4d5280;
		--cloud: #2a3658;
		--star: #e8eef8;
		--crater: #b9c4dc;
		--far: #28274a;
		--brick: #6d5a66;
		--brick-lit: #8b7886;
		--arch: #f5c76a;
		--soil: #3b3043;
		--leaf: #2f6a56;
		--leaf2: #1f4a4a;
		--trunk: #2f2a40;
		--lapis: #2c4f92;
		--lapis-d: #213c73;
		--tile: #e8bd4f;
		--lamp: #f5c76a;
		--lamp-on: 1;
		--bank: #25213d;
		--water1: #1a2a50;
		--water2: #0a1428;
		--glint: #9fb4e0;
		--hull: #0e0b1a;
	}
</style>
