<script lang="ts">
	/**
	 * Lucerne in the morning light — the wooden Kapellbrücke with its flower boxes
	 * and the stone Wasserturm over the Reuss, Pilatus behind the old town, two
	 * swans, and a quay lamp that takes the page's status colour (`--scene-tone`,
	 * set by the page) with two DumbMonit pigeons beside it.
	 * Follows the scene contract: fills its positioned parent, the headline zone
	 * (top left) is left clear, the palette lives here.
	 *
	 * `mood` is the weather (clear, clouded, storm with rain and lightning).
	 * `pulse` plays one gesture when `at` changes: on `down` a fog bank rolls
	 * over the bridge and its lanterns go out, on `up` the lanterns light one
	 * by one along the bridge, on `resolved` flower petals rain from the
	 * flower boxes. Reduced motion freezes everything.
	 */
	import SwissExtras from '#lib/components/swiss/SwissExtras.svelte';
	import SceneWeather from './SceneWeatherS.svelte';
	import { createPulse, type SceneMood, type ScenePulse } from './scenePulseS.svelte.js';

	interface Props {
		mood?: SceneMood;
		pulse?: ScenePulse;
	}
	let { mood = 'ok', pulse = null }: Props = $props();
	const pl = createPulse(() => pulse, { down: 7000, up: 4500, resolved: 6500 });
	const LAMPS = Array.from({ length: 16 }, (_, i) => 640 + i * 64).filter((x) => x < 1170 || x > 1290);
	const PETALS = Array.from({ length: 26 }, (_, i) => ({
		x: 640 + ((i * 157) % 1000),
		c: ['var(--fl-a)', 'var(--fl-b)', 'var(--fl-c)'][i % 3],
		d: (i * 173) % 2400,
		dx: ((i * 29) % 70) - 35
	}));
</script>

<div
	class="scene scene-lucerne"
	class:p-down={pl.active === 'down'}
	class:p-up={pl.active === 'up'}
	class:p-resolved={pl.active === 'resolved'}
	aria-hidden="true"
>
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="lc-wat" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--water1)"/><stop offset="1" stop-color="var(--water2)"/></linearGradient>
			<g id="lc-bridge">
				<g fill="var(--wood2)"><rect x="613" y="436" width="4" height="40" /><rect x="643" y="436" width="4" height="40" /><rect x="673" y="436" width="4" height="40" /><rect x="703" y="436" width="4" height="40" /><rect x="733" y="436" width="4" height="40" /><rect x="763" y="436" width="4" height="40" /><rect x="793" y="436" width="4" height="40" /><rect x="823" y="436" width="4" height="40" /><rect x="853" y="436" width="4" height="40" /><rect x="883" y="436" width="4" height="40" /><rect x="913" y="436" width="4" height="40" /><rect x="943" y="436" width="4" height="40" /><rect x="973" y="436" width="4" height="40" /><rect x="1003" y="436" width="4" height="40" /><rect x="1033" y="436" width="4" height="40" /><rect x="1063" y="436" width="4" height="40" /><rect x="1093" y="436" width="4" height="40" /><rect x="1123" y="436" width="4" height="40" /><rect x="1153" y="436" width="4" height="40" /><rect x="1183" y="436" width="4" height="40" /><rect x="1213" y="436" width="4" height="40" /><rect x="1243" y="436" width="4" height="40" /><rect x="1273" y="436" width="4" height="40" /><rect x="1303" y="436" width="4" height="40" /><rect x="1333" y="436" width="4" height="40" /><rect x="1363" y="436" width="4" height="40" /><rect x="1393" y="436" width="4" height="40" /><rect x="1423" y="436" width="4" height="40" /><rect x="1453" y="436" width="4" height="40" /><rect x="1483" y="436" width="4" height="40" /><rect x="1513" y="436" width="4" height="40" /><rect x="1543" y="436" width="4" height="40" /><rect x="1573" y="436" width="4" height="40" /><rect x="1603" y="436" width="4" height="40" /><rect x="1633" y="436" width="4" height="40" /><rect x="1663" y="436" width="4" height="40" /><rect x="1693" y="436" width="4" height="40" /></g>
				<rect x="600" y="432" width="1100" height="8" fill="var(--wood)"/>
				<g fill="var(--wood)"><rect x="608" y="350" width="4" height="82" /><rect x="638" y="350" width="4" height="82" /><rect x="668" y="350" width="4" height="82" /><rect x="698" y="350" width="4" height="82" /><rect x="728" y="350" width="4" height="82" /><rect x="758" y="350" width="4" height="82" /><rect x="788" y="350" width="4" height="82" /><rect x="818" y="350" width="4" height="82" /><rect x="848" y="350" width="4" height="82" /><rect x="878" y="350" width="4" height="82" /><rect x="908" y="350" width="4" height="82" /><rect x="938" y="350" width="4" height="82" /><rect x="968" y="350" width="4" height="82" /><rect x="998" y="350" width="4" height="82" /><rect x="1028" y="350" width="4" height="82" /><rect x="1058" y="350" width="4" height="82" /><rect x="1088" y="350" width="4" height="82" /><rect x="1118" y="350" width="4" height="82" /><rect x="1148" y="350" width="4" height="82" /><rect x="1178" y="350" width="4" height="82" /><rect x="1208" y="350" width="4" height="82" /><rect x="1238" y="350" width="4" height="82" /><rect x="1268" y="350" width="4" height="82" /><rect x="1298" y="350" width="4" height="82" /><rect x="1328" y="350" width="4" height="82" /><rect x="1358" y="350" width="4" height="82" /><rect x="1388" y="350" width="4" height="82" /><rect x="1418" y="350" width="4" height="82" /><rect x="1448" y="350" width="4" height="82" /><rect x="1478" y="350" width="4" height="82" /><rect x="1508" y="350" width="4" height="82" /><rect x="1538" y="350" width="4" height="82" /><rect x="1568" y="350" width="4" height="82" /><rect x="1598" y="350" width="4" height="82" /><rect x="1628" y="350" width="4" height="82" /><rect x="1658" y="350" width="4" height="82" /><rect x="1688" y="350" width="4" height="82" /></g>
				<g fill="var(--wood2)"><path d="M614 354H636L625 372Z"/><path d="M644 354H666L655 372Z"/><path d="M674 354H696L685 372Z"/><path d="M704 354H726L715 372Z"/><path d="M734 354H756L745 372Z"/><path d="M764 354H786L775 372Z"/><path d="M794 354H816L805 372Z"/><path d="M824 354H846L835 372Z"/><path d="M854 354H876L865 372Z"/><path d="M884 354H906L895 372Z"/><path d="M914 354H936L925 372Z"/><path d="M944 354H966L955 372Z"/><path d="M974 354H996L985 372Z"/><path d="M1004 354H1026L1015 372Z"/><path d="M1034 354H1056L1045 372Z"/><path d="M1064 354H1086L1075 372Z"/><path d="M1094 354H1116L1105 372Z"/><path d="M1124 354H1146L1135 372Z"/><path d="M1154 354H1176L1165 372Z"/><path d="M1184 354H1206L1195 372Z"/><path d="M1214 354H1236L1225 372Z"/><path d="M1244 354H1266L1255 372Z"/><path d="M1274 354H1296L1285 372Z"/><path d="M1304 354H1326L1315 372Z"/><path d="M1334 354H1356L1345 372Z"/><path d="M1364 354H1386L1375 372Z"/><path d="M1394 354H1416L1405 372Z"/><path d="M1424 354H1446L1435 372Z"/><path d="M1454 354H1476L1465 372Z"/><path d="M1484 354H1506L1495 372Z"/><path d="M1514 354H1536L1525 372Z"/><path d="M1544 354H1566L1555 372Z"/><path d="M1574 354H1596L1585 372Z"/><path d="M1604 354H1626L1615 372Z"/><path d="M1634 354H1656L1645 372Z"/><path d="M1664 354H1686L1675 372Z"/></g>
				<path d="M590 354L612 336H1688L1710 354Z" fill="var(--broof)"/>
				<rect x="612" y="333" width="1076" height="4" fill="var(--wood2)"/>
				<rect x="600" y="412" width="1100" height="4" fill="var(--wood2)"/>
				<rect x="604" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="605" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="609" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="614" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="619" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="624" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="634" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="635" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="639" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="644" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="649" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="654" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="664" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="665" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="669" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="674" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="679" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="684" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="694" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="695" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="699" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="704" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="709" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="714" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="724" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="725" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="729" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="734" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="739" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="744" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="754" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="755" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="759" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="764" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="769" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="774" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="784" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="785" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="789" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="794" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="799" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="804" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="814" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="815" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="819" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="824" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="829" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="834" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="844" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="845" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="849" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="854" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="859" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="864" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="874" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="875" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="879" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="884" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="889" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="894" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="904" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="905" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="909" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="914" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="919" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="924" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="934" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="935" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="939" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="944" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="949" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="954" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="964" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="965" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="969" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="974" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="979" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="984" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="994" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="995" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="999" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="1004" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1009" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="1014" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1024" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1025" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1029" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1034" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1039" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1044" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1054" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1055" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1059" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="1064" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1069" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="1074" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1084" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1085" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1089" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="1094" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1099" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="1104" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1114" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1115" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1119" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1124" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1129" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1134" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1144" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1145" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1149" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="1154" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1159" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="1164" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1294" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1295" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1299" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1304" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1309" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1314" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1324" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1325" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1329" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1334" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1339" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1344" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1354" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1355" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1359" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="1364" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1369" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="1374" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1384" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1385" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1389" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="1394" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1399" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="1404" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1414" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1415" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1419" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1424" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1429" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1434" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1444" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1445" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1449" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="1454" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1459" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="1464" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1474" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1475" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1479" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="1484" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1489" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="1494" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1504" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1505" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1509" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1514" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1519" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1524" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1534" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1535" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1539" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="1544" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1549" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="1554" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1564" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1565" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1569" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1574" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1579" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1584" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1594" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1595" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1599" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="1604" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1609" cy="415" r="2.6" fill="var(--fl-b)"/><circle cx="1614" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1624" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1625" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1629" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1634" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1639" cy="415" r="2.6" fill="var(--fl-c)"/><circle cx="1644" cy="417" r="2.6" fill="var(--fl-b)"/><rect x="1654" y="420" width="26" height="9" rx="1" fill="var(--wood2)"/><rect x="1655" y="414" width="24" height="8" rx="4" fill="var(--leaf)"/><circle cx="1659" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="1664" cy="417" r="2.6" fill="var(--fl-b)"/><circle cx="1669" cy="415" r="2.6" fill="var(--fl-a)"/><circle cx="1674" cy="417" r="2.6" fill="var(--fl-b)"/>
				<rect x="1190" y="300" width="80" height="150" fill="var(--stone)"/>
				<rect x="1244" y="300" width="26" height="150" fill="var(--stone2)" opacity=".55"/>
				<path d="M1190 340H1270M1190 390H1270" stroke="var(--stone2)" stroke-width="2"/>
				<rect x="1184" y="294" width="92" height="8" fill="var(--stone2)"/>
				<path d="M1186 294L1230 196L1274 294Z" fill="var(--troof)"/>
				<rect x="1228" y="176" width="3" height="22" fill="var(--wood2)"/>
				<g fill="var(--dark)" opacity=".8"><rect x="1208" y="318" width="6" height="18" rx="3"/><rect x="1227" y="318" width="6" height="18" rx="3"/><rect x="1246" y="318" width="6" height="18" rx="3"/><rect x="1208" y="360" width="6" height="18" rx="3"/><rect x="1246" y="360" width="6" height="18" rx="3"/></g>
				<g class="win" opacity=".9"><rect x="1227" y="360" width="6" height="18" rx="3"/></g>
			</g>
		</defs>
		<g transform="translate(0 0)">
			<circle cx="1500" cy="170" r="40" fill="var(--sun)" opacity=".9"/>
			<g fill="var(--cloud)" opacity=".75"><ellipse cx="850" cy="130" rx="64" ry="14"/><ellipse cx="890" cy="118" rx="40" ry="14"/><ellipse cx="1280" cy="150" rx="70" ry="12"/></g>
			<path d="M560 330L640 290L720 310L800 270L880 300L960 330L1040 360V440H560Z" fill="var(--alps)"/>
			<path d="M900 440L1000 370L1080 330L1140 270L1190 250L1230 214L1270 190L1310 228L1350 214L1400 270L1470 320L1560 370L1680 410L1780 430V440Z" fill="var(--rock)"/>
			<path d="M1270 190L1252 214L1268 210L1280 224L1294 204L1310 228L1296 200Z M1230 214L1218 230L1232 228L1242 238L1246 222Z M1350 214L1338 232L1352 228L1364 240L1366 222Z" fill="var(--snow)"/>
			<path d="M540 440V397H586V440ZM587 440V394H626V440ZM627 440V400H683V440ZM684 440V394H726V440ZM727 440V382H785V440ZM786 440V403H844V440ZM845 440V390H879V440ZM880 440V372H918V440ZM919 440V402H958V440ZM959 440V379H1015V440ZM1016 440V411H1051V440ZM1052 440V391H1087V440ZM1088 440V405H1145V440ZM1146 440V395H1203V440ZM1204 440V410H1253V440ZM1254 440V417H1309V440ZM1310 440V409H1347V440ZM1348 440V415H1387V440ZM1388 440V404H1432V440ZM1433 440V380H1467V440ZM1468 440V391H1507V440ZM1508 440V393H1551V440ZM1552 440V376H1589V440ZM1590 440V417H1637V440ZM1638 440V387H1693V440ZM1694 440V417H1735V440ZM1736 440V414H1774V440ZM1775 440V393H1832V440Z" fill="var(--town2)"/><path d="M538 397L563.0 380L588 397ZM585 394L606.5 371L628 394ZM625 400L655.0 378L685 400ZM682 394L705.0 369L728 394ZM725 382L756.0 368L787 382ZM784 403L815.0 379L846 403ZM843 390L862.0 374L881 390ZM878 372L899.0 353L920 372ZM917 402L938.5 378L960 402ZM957 379L987.0 356L1017 379ZM1014 411L1033.5 388L1053 411ZM1050 391L1069.5 373L1089 391ZM1086 405L1116.5 390L1147 405ZM1144 395L1174.5 370L1205 395ZM1202 410L1228.5 388L1255 410ZM1252 417L1281.5 402L1311 417ZM1308 409L1328.5 384L1349 409ZM1346 415L1367.5 397L1389 415ZM1386 404L1410.0 381L1434 404ZM1431 380L1450.0 357L1469 380ZM1466 391L1487.5 377L1509 391ZM1506 393L1529.5 377L1553 393ZM1550 376L1570.5 361L1591 376ZM1588 417L1613.5 393L1639 417ZM1636 387L1665.5 367L1695 387ZM1692 417L1714.5 402L1737 417ZM1734 414L1755.0 393L1776 414ZM1773 393L1803.5 373L1834 393Z" fill="var(--roof)"/>
			<path d="M640 440V360L648 316L656 360V440ZM672 440V366L680 326L688 366V440Z" fill="var(--town2)"/>
			<rect x="0" y="440" width="1440" height="600" fill="url(#lc-wat)"/>
			<use href="#lc-bridge" transform="translate(0 880) scale(1 -1)" opacity=".22"/>
			<g stroke="var(--wh)" stroke-width="2" opacity=".45" stroke-linecap="round"><path d="M600 470h60M720 486h90M900 464h70M1040 480h120M1260 468h80M1400 486h110M640 520h70M820 534h110M1000 512h80M1180 530h100M1360 516h90M1540 500h80"/></g>
			<use href="#lc-bridge"/>
			<g transform="translate(920 506)"><path d="M0 8Q24 18 46 8Q50-4 38-6H4Z" fill="var(--swan)"/><path d="M40-4Q54-6 50-20Q48-28 56-28" stroke="var(--swan)" stroke-width="5" stroke-linecap="round"/><path d="M58-28l8 2-8 3Z" fill="var(--beak)"/></g>
			<g transform="translate(1000 520) scale(.75)"><path d="M0 8Q24 18 46 8Q50-4 38-6H4Z" fill="var(--swan)"/><path d="M40-4Q54-6 50-20Q48-28 56-28" stroke="var(--swan)" stroke-width="5" stroke-linecap="round"/><path d="M58-28l8 2-8 3Z" fill="var(--beak)"/></g>
			<!-- quay with flower boxes, a lamp taking the status colour, and the pigeons -->
			<rect x="560" y="564" width="1220" height="60" fill="var(--stone)"/>
			<rect x="560" y="562" width="1220" height="6" fill="var(--stone2)"/>
			<rect x="640" y="556" width="36" height="9" rx="1" fill="var(--wood2)"/><ellipse cx="658" cy="553" rx="17" ry="6" fill="var(--leaf)"/><circle cx="648" cy="550" r="3" fill="var(--fl-a)"/><circle cx="658" cy="548" r="3" fill="var(--fl-b)"/><circle cx="668" cy="550" r="3" fill="var(--fl-c)"/><rect x="900" y="556" width="36" height="9" rx="1" fill="var(--wood2)"/><ellipse cx="918" cy="553" rx="17" ry="6" fill="var(--leaf)"/><circle cx="908" cy="550" r="3" fill="var(--fl-a)"/><circle cx="918" cy="548" r="3" fill="var(--fl-b)"/><circle cx="928" cy="550" r="3" fill="var(--fl-c)"/><rect x="1440" y="556" width="36" height="9" rx="1" fill="var(--wood2)"/><ellipse cx="1458" cy="553" rx="17" ry="6" fill="var(--leaf)"/><circle cx="1448" cy="550" r="3" fill="var(--fl-a)"/><circle cx="1458" cy="548" r="3" fill="var(--fl-b)"/><circle cx="1468" cy="550" r="3" fill="var(--fl-c)"/>
			<!-- bridge lanterns -->
			<g class="lamps">
				{#each LAMPS as x, i}
					<g class="lamp" style="animation-delay:{i * 130}ms"><path d="M{x} 372v6" stroke="var(--wood2)" stroke-width="1.5" /><circle cx={x} cy="383" r="9" fill="#ffd77a" opacity=".35" /><circle cx={x} cy="383" r="4" fill="#ffd77a" /></g>
				{/each}
			</g>
			<!-- fog bank (gesture) -->
			<g class="fog" fill="var(--fog)">
				<ellipse cx="700" cy="400" rx="260" ry="46" /><ellipse cx="980" cy="380" rx="280" ry="52" /><ellipse cx="1260" cy="404" rx="270" ry="50" /><ellipse cx="1520" cy="384" rx="250" ry="48" /><ellipse cx="1100" cy="440" rx="500" ry="30" />
			</g>
			<g><rect x="1100" y="500" width="6" height="64" fill="var(--wood2)"/><circle cx="1103" cy="494" r="9" fill="var(--scene-tone, var(--c-signal))"/><circle cx="1103" cy="494" r="20" fill="var(--scene-tone, var(--c-signal))" opacity=".25"/></g>
			<use href="#dm-mascot" transform="translate(1180 532) scale(.1)"/><use href="#dm-mascot" transform="translate(1264 532) scale(.1)"/>
		</g>
		<g class="petals">
			{#if pl.active === 'resolved'}
				{#key pl.key}
					{#each PETALS as q}
						<ellipse class="petal" cx={q.x} cy="410" rx="4" ry="2.4" fill={q.c} style="--dx:{q.dx}px;animation-delay:{q.d}ms" />
					{/each}
				{/key}
			{/if}
		</g>
		<SwissExtras {mood} {pulse} clock={{ x: 1230, y: 412, r: 22 }} board={{ x: 1330, y: 520, w: 150, ground: 563 }} area={{ x: 640, y: 90, w: 1060, h: 350 }} />
<SceneWeather {mood} />
	</svg>
</div>

<style>
	.scene-lucerne {
		--sky1: #a9d0ea;
		--sky2: #f4ecd9;
		--sun: #fff0c4;
		--cloud: #fff;
		--water1: #7fb0c0;
		--water2: #4a8399;
		--alps: #c4d1e0;
		--rock: #93a6bd;
		--snow: #f5f7fb;
		--town2: #cdb99f;
		--roof: #a5604a;
		--wood: #7a5338;
		--wood2: #5a3c28;
		--broof: #98513b;
		--stone: #d9d0c0;
		--stone2: #b5ab99;
		--troof: #a24b3c;
		--dark: #33272a;
		--leaf: #4d8a4f;
		--fl-a: #d94a5a;
		--fl-b: #f27ba0;
		--fl-c: #f2c14e;
		--swan: #fbfbf7;
		--beak: #e08a3a;
		--wh: #fff;
	}
	:global(.dark) .scene-lucerne {
		--sky1: #0b1530;
		--sky2: #262d56;
		--sun: #e8eef8;
		--cloud: #2a3558;
		--water1: #16304e;
		--water2: #0a1428;
		--alps: #1c2a47;
		--rock: #2a3b5b;
		--snow: #8e9ab4;
		--town2: #2b2d44;
		--roof: #3b2a35;
		--wood: #3d2c25;
		--wood2: #271b19;
		--broof: #4a2a2a;
		--stone: #4a4e62;
		--stone2: #383c50;
		--troof: #52292b;
		--dark: #0a0f1d;
		--leaf: #2c5236;
		--fl-a: #a03e4e;
		--fl-b: #b4607f;
		--fl-c: #b8963f;
		--swan: #cdd3df;
		--beak: #a8642b;
		--wh: #9aa0b2;
	}
	.scene-lucerne { --fog: #eef1f4; }
	:global(.dark) .scene-lucerne { --fog: #5a6584; }
	.lamp { --lamp-o: 0.55; opacity: var(--lamp-o); transform-box: fill-box; transform-origin: 50% 0; }
	:global(.dark) .lamp { --lamp-o: 0.95; }
	.fog, .petal { opacity: 0; }
	@media (prefers-reduced-motion: no-preference) {
		.p-down .fog { animation: roll 7s ease-in-out both; }
		.p-down .lamp { animation: lampoff 7s ease-in-out both; }
		.p-up .lamp { animation: lampon 1.4s ease-out both; }
		.petal { transform-box: fill-box; transform-origin: center; animation: petal 3.8s ease-in both; }
	}
	@keyframes roll {
		0% { opacity: 0; transform: translateX(-380px); } 30%, 62% { opacity: 0.92; transform: translateX(0); } 100% { opacity: 0; transform: translateX(380px); }
	}
	@keyframes lampoff { 0%, 100% { opacity: var(--lamp-o); } 20%, 80% { opacity: 0.04; } }
	@keyframes lampon { 0% { opacity: 0; transform: scale(0.3); } 50% { opacity: 1; transform: scale(1.6); } 100% { opacity: var(--lamp-o); transform: scale(1); } }
	@keyframes petal {
		0% { opacity: 1; transform: translate(0, 0) rotate(0); } 80% { opacity: 1; }
		100% { opacity: 0; transform: translate(var(--dx), 150px) rotate(480deg); }
	}
	.scene-lucerne .win {
		fill: #f1bf62;
		opacity: 0;
	}
	:global(.dark) .scene-lucerne .win {
		opacity: .9;
	}
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(180deg, var(--sky1), var(--sky2) 66.7%, var(--water1) 67.5%, var(--water1) 76%, var(--water2));
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
