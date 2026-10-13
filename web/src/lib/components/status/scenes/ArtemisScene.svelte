<script lang="ts">
	/**
	 * The Temple of Artemis at Ephesus: a vast Ionic temple on its stepped platform, cypresses and a hill behind, a sacred flame in a brazier on each side.
	 * `down`: the flames die; `up`: they are relit; `resolved`: they flare and doves fly up from the pediment.
	 *
	 * `mood` is the weather (ok/calm: clear, warn/clouded: cloud, critical/storm:
	 * rain and lightning). `pulse` plays one gesture whenever its `at` changes.
	 * Fills its positioned parent, the headline zone (top left) is left clear.
	 * Reduced motion keeps the still image.
	 */
	import SceneWeather from './SceneWeatherA.svelte';
	import SceneSparkles from './SceneSparkles.svelte';
	import { createPulse } from './pulseA.svelte';
	import { level, type Mood, type PulseProp } from './weather';

	let { mood = 'ok', pulse = null }: { mood?: Mood; pulse?: PulseProp } = $props();
	const p = createPulse(() => pulse);
	const wx = $derived(level(mood));
</script>

<div
	class="scene scene-artemis"
	class:p-down={p.active === 'down'}
	class:p-up={p.active === 'up'}
	class:p-resolved={p.active === 'resolved'}
	aria-hidden="true"
>
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<g id="ar-cypress"><path d="M0 0V-10C-14-30-12-70 0-120C12-70 14-30 0-10z" fill="var(--cypress)" /></g>
			<linearGradient id="ar-grass" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--grass1)" /><stop offset="1" stop-color="var(--grass2)" />
			</linearGradient>
		</defs>

		<!-- sun / moon, stars, clouds -->
		<circle cx="1000" cy="150" r="52" fill="var(--sun-halo)" opacity=".5" />
		<circle cx="1000" cy="150" r="34" fill="var(--sun)" />
		<circle cx="988" cy="140" r="5" fill="var(--crater)" /><circle cx="1012" cy="160" r="4" fill="var(--crater)" />
		<g fill="var(--star)"><circle cx="780" cy="126" r="2" /><circle cx="1180" cy="112" r="2.2" /><circle cx="1400" cy="130" r="1.8" /><circle cx="1540" cy="104" r="2" /><circle cx="1690" cy="150" r="2.2" /><circle cx="850" cy="200" r="1.8" /><circle cx="1480" cy="180" r="1.6" /></g>
		<g fill="var(--cloud)" opacity=".75"><ellipse cx="1500" cy="150" rx="100" ry="9" /><ellipse cx="1560" cy="166" rx="58" ry="7" /><ellipse cx="760" cy="196" rx="70" ry="7" /></g>

		<!-- Mount Pion and far hills -->
		<path d="M560 482V400C640 390 700 340 800 330C880 322 940 360 1040 380C1140 396 1240 400 1340 396C1460 392 1560 372 1680 380C1740 384 1780 392 1780 392V482z" fill="var(--hill-far)" />
		<path d="M560 482V438C680 424 780 410 900 424C1000 436 1100 440 1240 436C1400 430 1560 420 1780 430V482z" fill="var(--hill)" />
		<g fill="var(--cypress-far)"><use href="#ar-cypress" transform="translate(720 440) scale(.5)" /><use href="#ar-cypress" transform="translate(750 438) scale(.42)" /><use href="#ar-cypress" transform="translate(1690 436) scale(.5)" /></g>

		<!-- the temple -->
		<g>
			<!-- cella and shadow behind the colonnade -->
			<rect x="976" y="296" width="648" height="156" fill="var(--cella)" />
			<path d="M1276 452V372h48v80z" fill="var(--door)" />
			<g fill="var(--lamp)" opacity="var(--lamp-on)"><rect x="1290" y="400" width="20" height="52" rx="2" /></g>
			<path d="M1025.8 452L1027.8 306H1040.8L1042.8 452z" fill="var(--m-back)" /><path d="M1031.8 452V306M1036.8 452V306" stroke="var(--m-flute-b)" stroke-width="1.4" opacity=".7" /><path d="M1114.4 452L1116.4 306H1129.4L1131.4 452z" fill="var(--m-back)" /><path d="M1120.4 452V306M1125.4 452V306" stroke="var(--m-flute-b)" stroke-width="1.4" opacity=".7" /><path d="M1202.9 452L1204.9 306H1217.9L1219.9 452z" fill="var(--m-back)" /><path d="M1208.9 452V306M1213.9 452V306" stroke="var(--m-flute-b)" stroke-width="1.4" opacity=".7" /><path d="M1380.1 452L1382.1 306H1395.1L1397.1 452z" fill="var(--m-back)" /><path d="M1386.1 452V306M1391.1 452V306" stroke="var(--m-flute-b)" stroke-width="1.4" opacity=".7" /><path d="M1468.6 452L1470.6 306H1483.6L1485.6 452z" fill="var(--m-back)" /><path d="M1474.6 452V306M1479.6 452V306" stroke="var(--m-flute-b)" stroke-width="1.4" opacity=".7" /><path d="M1557.2 452L1559.2 306H1572.2L1574.2 452z" fill="var(--m-back)" /><path d="M1563.2 452V306M1568.2 452V306" stroke="var(--m-flute-b)" stroke-width="1.4" opacity=".7" />
			<rect x="1022.3" y="302" width="24" height="7" fill="var(--m-back)" /><rect x="1110.9" y="302" width="24" height="7" fill="var(--m-back)" /><rect x="1199.4" y="302" width="24" height="7" fill="var(--m-back)" /><rect x="1376.6" y="302" width="24" height="7" fill="var(--m-back)" /><rect x="1465.1" y="302" width="24" height="7" fill="var(--m-back)" /><rect x="1553.7" y="302" width="24" height="7" fill="var(--m-back)" />
			<path d="M979.0 452L981.0 306H999.0L1001.0 452z" fill="var(--m)" /><path d="M986.7 452V306M993.3 452V306" stroke="var(--m-flute)" stroke-width="1.4" opacity=".7" /><path d="M1067.6 452L1069.6 306H1087.6L1089.6 452z" fill="var(--m)" /><path d="M1075.2 452V306M1081.9 452V306" stroke="var(--m-flute)" stroke-width="1.4" opacity=".7" /><path d="M1156.1 452L1158.1 306H1176.1L1178.1 452z" fill="var(--m)" /><path d="M1163.8 452V306M1170.5 452V306" stroke="var(--m-flute)" stroke-width="1.4" opacity=".7" /><path d="M1244.7 452L1246.7 306H1264.7L1266.7 452z" fill="var(--m)" /><path d="M1252.4 452V306M1259.0 452V306" stroke="var(--m-flute)" stroke-width="1.4" opacity=".7" /><path d="M1333.3 452L1335.3 306H1353.3L1355.3 452z" fill="var(--m)" /><path d="M1341.0 452V306M1347.6 452V306" stroke="var(--m-flute)" stroke-width="1.4" opacity=".7" /><path d="M1421.9 452L1423.9 306H1441.9L1443.9 452z" fill="var(--m)" /><path d="M1429.5 452V306M1436.2 452V306" stroke="var(--m-flute)" stroke-width="1.4" opacity=".7" /><path d="M1510.4 452L1512.4 306H1530.4L1532.4 452z" fill="var(--m)" /><path d="M1518.1 452V306M1524.8 452V306" stroke="var(--m-flute)" stroke-width="1.4" opacity=".7" /><path d="M1599.0 452L1601.0 306H1619.0L1621.0 452z" fill="var(--m)" /><path d="M1606.7 452V306M1613.3 452V306" stroke="var(--m-flute)" stroke-width="1.4" opacity=".7" />
			<rect x="976.0" y="446" width="28" height="6" fill="var(--m2)" /><rect x="1064.6" y="446" width="28" height="6" fill="var(--m2)" /><rect x="1153.1" y="446" width="28" height="6" fill="var(--m2)" /><rect x="1241.7" y="446" width="28" height="6" fill="var(--m2)" /><rect x="1330.3" y="446" width="28" height="6" fill="var(--m2)" /><rect x="1418.9" y="446" width="28" height="6" fill="var(--m2)" /><rect x="1507.4" y="446" width="28" height="6" fill="var(--m2)" /><rect x="1596.0" y="446" width="28" height="6" fill="var(--m2)" />
			<rect x="973.0" y="299" width="34" height="6" fill="var(--m2)" /><circle cx="977.0" cy="308" r="5.5" fill="var(--m)" /><circle cx="1003.0" cy="308" r="5.5" fill="var(--m)" /><circle cx="977.0" cy="308" r="2" fill="var(--m-flute)" /><circle cx="1003.0" cy="308" r="2" fill="var(--m-flute)" /><rect x="977.0" y="304" width="26" height="5" fill="var(--m)" /><rect x="1061.6" y="299" width="34" height="6" fill="var(--m2)" /><circle cx="1065.6" cy="308" r="5.5" fill="var(--m)" /><circle cx="1091.6" cy="308" r="5.5" fill="var(--m)" /><circle cx="1065.6" cy="308" r="2" fill="var(--m-flute)" /><circle cx="1091.6" cy="308" r="2" fill="var(--m-flute)" /><rect x="1065.6" y="304" width="26" height="5" fill="var(--m)" /><rect x="1150.1" y="299" width="34" height="6" fill="var(--m2)" /><circle cx="1154.1" cy="308" r="5.5" fill="var(--m)" /><circle cx="1180.1" cy="308" r="5.5" fill="var(--m)" /><circle cx="1154.1" cy="308" r="2" fill="var(--m-flute)" /><circle cx="1180.1" cy="308" r="2" fill="var(--m-flute)" /><rect x="1154.1" y="304" width="26" height="5" fill="var(--m)" /><rect x="1238.7" y="299" width="34" height="6" fill="var(--m2)" /><circle cx="1242.7" cy="308" r="5.5" fill="var(--m)" /><circle cx="1268.7" cy="308" r="5.5" fill="var(--m)" /><circle cx="1242.7" cy="308" r="2" fill="var(--m-flute)" /><circle cx="1268.7" cy="308" r="2" fill="var(--m-flute)" /><rect x="1242.7" y="304" width="26" height="5" fill="var(--m)" /><rect x="1327.3" y="299" width="34" height="6" fill="var(--m2)" /><circle cx="1331.3" cy="308" r="5.5" fill="var(--m)" /><circle cx="1357.3" cy="308" r="5.5" fill="var(--m)" /><circle cx="1331.3" cy="308" r="2" fill="var(--m-flute)" /><circle cx="1357.3" cy="308" r="2" fill="var(--m-flute)" /><rect x="1331.3" y="304" width="26" height="5" fill="var(--m)" /><rect x="1415.9" y="299" width="34" height="6" fill="var(--m2)" /><circle cx="1419.9" cy="308" r="5.5" fill="var(--m)" /><circle cx="1445.9" cy="308" r="5.5" fill="var(--m)" /><circle cx="1419.9" cy="308" r="2" fill="var(--m-flute)" /><circle cx="1445.9" cy="308" r="2" fill="var(--m-flute)" /><rect x="1419.9" y="304" width="26" height="5" fill="var(--m)" /><rect x="1504.4" y="299" width="34" height="6" fill="var(--m2)" /><circle cx="1508.4" cy="308" r="5.5" fill="var(--m)" /><circle cx="1534.4" cy="308" r="5.5" fill="var(--m)" /><circle cx="1508.4" cy="308" r="2" fill="var(--m-flute)" /><circle cx="1534.4" cy="308" r="2" fill="var(--m-flute)" /><rect x="1508.4" y="304" width="26" height="5" fill="var(--m)" /><rect x="1593.0" y="299" width="34" height="6" fill="var(--m2)" /><circle cx="1597.0" cy="308" r="5.5" fill="var(--m)" /><circle cx="1623.0" cy="308" r="5.5" fill="var(--m)" /><circle cx="1597.0" cy="308" r="2" fill="var(--m-flute)" /><circle cx="1623.0" cy="308" r="2" fill="var(--m-flute)" /><rect x="1597.0" y="304" width="26" height="5" fill="var(--m)" />
			<!-- steps -->
			<rect x="964" y="452" width="672" height="10" fill="var(--step1)" />
			<rect x="952" y="462" width="696" height="10" fill="var(--step2)" />
			<rect x="940" y="472" width="720" height="10" fill="var(--step3)" />
			<g fill="var(--step-shade)" opacity=".55"><rect x="964" y="460" width="672" height="2" /><rect x="952" y="470" width="696" height="2" /><rect x="940" y="480" width="720" height="2" /></g>
			<!-- entablature -->
			<rect x="966" y="278" width="668" height="22" fill="var(--m)" />
			<path d="M966 288H1634M966 294H1634" stroke="var(--m-flute)" stroke-width="1.4" opacity=".8" />
			<rect x="962" y="262" width="676" height="16" fill="var(--m2)" />
			<g fill="var(--frieze)" opacity=".75"><rect x="964" y="262" width="12" height="12" rx="2" /><rect x="992" y="262" width="12" height="12" rx="2" /><rect x="1020" y="262" width="12" height="12" rx="2" /><rect x="1048" y="262" width="12" height="12" rx="2" /><rect x="1076" y="262" width="12" height="12" rx="2" /><rect x="1104" y="262" width="12" height="12" rx="2" /><rect x="1132" y="262" width="12" height="12" rx="2" /><rect x="1160" y="262" width="12" height="12" rx="2" /><rect x="1188" y="262" width="12" height="12" rx="2" /><rect x="1216" y="262" width="12" height="12" rx="2" /><rect x="1244" y="262" width="12" height="12" rx="2" /><rect x="1272" y="262" width="12" height="12" rx="2" /><rect x="1300" y="262" width="12" height="12" rx="2" /><rect x="1328" y="262" width="12" height="12" rx="2" /><rect x="1356" y="262" width="12" height="12" rx="2" /><rect x="1384" y="262" width="12" height="12" rx="2" /><rect x="1412" y="262" width="12" height="12" rx="2" /><rect x="1440" y="262" width="12" height="12" rx="2" /><rect x="1468" y="262" width="12" height="12" rx="2" /><rect x="1496" y="262" width="12" height="12" rx="2" /><rect x="1524" y="262" width="12" height="12" rx="2" /><rect x="1552" y="262" width="12" height="12" rx="2" /><rect x="1580" y="262" width="12" height="12" rx="2" /><rect x="1608" y="262" width="12" height="12" rx="2" /><rect x="1636" y="262" width="12" height="12" rx="2" /></g>
			<rect x="962" y="256" width="676" height="7" fill="var(--m)" />
			<g fill="var(--m-flute)" opacity=".8"><rect x="958" y="276" width="5" height="5" /><rect x="968" y="276" width="5" height="5" /><rect x="978" y="276" width="5" height="5" /><rect x="988" y="276" width="5" height="5" /><rect x="998" y="276" width="5" height="5" /><rect x="1008" y="276" width="5" height="5" /><rect x="1018" y="276" width="5" height="5" /><rect x="1028" y="276" width="5" height="5" /><rect x="1038" y="276" width="5" height="5" /><rect x="1048" y="276" width="5" height="5" /><rect x="1058" y="276" width="5" height="5" /><rect x="1068" y="276" width="5" height="5" /><rect x="1078" y="276" width="5" height="5" /><rect x="1088" y="276" width="5" height="5" /><rect x="1098" y="276" width="5" height="5" /><rect x="1108" y="276" width="5" height="5" /><rect x="1118" y="276" width="5" height="5" /><rect x="1128" y="276" width="5" height="5" /><rect x="1138" y="276" width="5" height="5" /><rect x="1148" y="276" width="5" height="5" /><rect x="1158" y="276" width="5" height="5" /><rect x="1168" y="276" width="5" height="5" /><rect x="1178" y="276" width="5" height="5" /><rect x="1188" y="276" width="5" height="5" /><rect x="1198" y="276" width="5" height="5" /><rect x="1208" y="276" width="5" height="5" /><rect x="1218" y="276" width="5" height="5" /><rect x="1228" y="276" width="5" height="5" /><rect x="1238" y="276" width="5" height="5" /><rect x="1248" y="276" width="5" height="5" /><rect x="1258" y="276" width="5" height="5" /><rect x="1268" y="276" width="5" height="5" /><rect x="1278" y="276" width="5" height="5" /><rect x="1288" y="276" width="5" height="5" /><rect x="1298" y="276" width="5" height="5" /><rect x="1308" y="276" width="5" height="5" /><rect x="1318" y="276" width="5" height="5" /><rect x="1328" y="276" width="5" height="5" /><rect x="1338" y="276" width="5" height="5" /><rect x="1348" y="276" width="5" height="5" /><rect x="1358" y="276" width="5" height="5" /><rect x="1368" y="276" width="5" height="5" /><rect x="1378" y="276" width="5" height="5" /><rect x="1388" y="276" width="5" height="5" /><rect x="1398" y="276" width="5" height="5" /><rect x="1408" y="276" width="5" height="5" /><rect x="1418" y="276" width="5" height="5" /><rect x="1428" y="276" width="5" height="5" /><rect x="1438" y="276" width="5" height="5" /><rect x="1448" y="276" width="5" height="5" /><rect x="1458" y="276" width="5" height="5" /><rect x="1468" y="276" width="5" height="5" /><rect x="1478" y="276" width="5" height="5" /><rect x="1488" y="276" width="5" height="5" /><rect x="1498" y="276" width="5" height="5" /><rect x="1508" y="276" width="5" height="5" /><rect x="1518" y="276" width="5" height="5" /><rect x="1528" y="276" width="5" height="5" /><rect x="1538" y="276" width="5" height="5" /><rect x="1548" y="276" width="5" height="5" /><rect x="1558" y="276" width="5" height="5" /><rect x="1568" y="276" width="5" height="5" /><rect x="1578" y="276" width="5" height="5" /><rect x="1588" y="276" width="5" height="5" /><rect x="1598" y="276" width="5" height="5" /><rect x="1608" y="276" width="5" height="5" /><rect x="1618" y="276" width="5" height="5" /><rect x="1628" y="276" width="5" height="5" /><rect x="1638" y="276" width="5" height="5" /></g>
			<!-- pediment -->
			<path d="M948 256L1300 192L1652 256z" fill="var(--m)" />
			<path d="M984 250L1300 200L1616 250z" fill="var(--tympanum)" />
			<g fill="var(--m)" opacity=".95"><circle cx="1300" cy="226" r="9" /><path d="M1290 250v-14h20v14zM1236 250l10-12h18l8 12zM1336 250l8-12h18l10 12zM1190 250l14-8 16 8zM1380 250l16-8 14 8z" /><rect x="1140" y="244" width="36" height="6" rx="3" /><rect x="1424" y="244" width="36" height="6" rx="3" /></g>
			<path d="M948 256L1300 192L1652 256" stroke="var(--m2)" stroke-width="5" stroke-linejoin="round" />
			<!-- roof tiles and acroteria -->
			<path d="M954 248L1300 185L1646 248" stroke="var(--roof)" stroke-width="6" />
			<path d="M1300 186v-14M1290 186c0-14 20-14 20 0M1300 172c-8-6-8-16 0-22c8 6 8 16 0 22z" stroke="var(--roof)" stroke-width="3" stroke-linecap="round" />
			<circle cx="1300" cy="162" r="6" fill="var(--roof)" />
			<circle cx="948" cy="250" r="7" fill="var(--roof)" /><circle cx="1652" cy="250" r="7" fill="var(--roof)" />
		</g>

		<!-- terrace and garden ground -->
		<rect x="560" y="482" width="1220" height="140" fill="url(#ar-grass)" />
		<path d="M1230 482H1370L1480 620H1120z" fill="var(--path)" />
		<path d="M560 520C700 506 860 508 940 522C1100 536 1500 536 1660 520C1720 512 1760 512 1780 516V620H560z" fill="var(--grass2)" opacity=".5" />
		<g stroke="var(--blade)" stroke-width="2" stroke-linecap="round" opacity=".6"><path d="M640 540h40M760 566h60M880 530h50M1700 546h60M1560 580h70M720 596h50" /></g>

		<!-- cypresses -->
		<use href="#ar-cypress" transform="translate(900 490) scale(1.35)" />
		<use href="#ar-cypress" transform="translate(936 488) scale(1.1)" />
		<use href="#ar-cypress" transform="translate(1690 490) scale(1.4)" />
		<use href="#ar-cypress" transform="translate(1728 488) scale(1.05)" />
		<!-- olive trees -->
		<g fill="var(--olive)"><ellipse cx="840" cy="500" rx="26" ry="16" /><ellipse cx="1790" cy="506" rx="26" ry="14" /></g>
		<g stroke="var(--trunk)" stroke-width="4" stroke-linecap="round"><path d="M840 500v14M1790 506v12" /></g>
		<g class="braz" transform="translate(990 472) scale(1.7)">
			<path d="M-9-18h18l-4 18h-10z" fill="var(--door)" /><rect x="-12" y="-21" width="24" height="4" rx="1.5" fill="var(--door)" />
			<g class="bflame">
				<circle cy="-32" r="17" fill="#f2a53a" opacity=".3" />
				<path d="M0-21C-8-27-6-35 0-44C6-35 8-27 0-21z" fill="#f2a53a" /><path d="M0-22C-4-26-3-31 0-36C3-31 4-26 0-22z" fill="#fff0a8" />
			</g>
		</g>
		<g class="braz" transform="translate(1610 472) scale(1.7)">
			<path d="M-9-18h18l-4 18h-10z" fill="var(--door)" /><rect x="-12" y="-21" width="24" height="4" rx="1.5" fill="var(--door)" />
			<g class="bflame">
				<circle cy="-32" r="17" fill="#f2a53a" opacity=".3" />
				<path d="M0-21C-8-27-6-35 0-44C6-35 8-27 0-21z" fill="#f2a53a" /><path d="M0-22C-4-26-3-31 0-36C3-31 4-26 0-22z" fill="#fff0a8" />
			</g>
		</g>
		<g class="doves" fill="none" stroke="#fffaf0" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round">
			<path class="dv dv1" d="M1284 182q8-12 16 0q8-12 16 0" /><path class="dv dv2" d="M1306 196q6-9 12 0q6-9 12 0" /><path class="dv dv3" d="M1270 196q5-8 10 0q5-8 10 0" />
		</g>

		<SceneWeather level={wx} />
		{#if p.active === 'resolved'}
			{#key p.key}
				<SceneSparkles x={1300} y={200} colors={['#f2c14e', '#fff0a8', '#fff']} count={12} spread={130} />
			{/key}
		{/if}
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
	.scene-artemis {
		--sky1: #9ec7e0;
		--sky2: #f4e8d2;
		--sun: #fff5d2;
		--sun-halo: #ffe7b0;
		--cloud: #fff;
		--star: transparent;
		--crater: transparent;
		--hill-far: #b9cbb6;
		--hill: #9db98d;
		--cypress: #3d6b46;
		--cypress-far: #7ea083;
		--m: #f5ecd9;
		--m2: #dfd0b3;
		--m-back: #cdbb9b;
		--m-flute: #c8b595;
		--m-flute-b: #a8946f;
		--cella: #8b7456;
		--door: #4b3a2c;
		--lamp: #f5c76a;
		--lamp-on: 0;
		--step1: #efe3cb;
		--step2: #e1d1b2;
		--step3: #d3c19f;
		--step-shade: #9d8761;
		--frieze: #c8a27a;
		--tympanum: #6f94b8;
		--roof: #c4724b;
		--grass1: #a9bf86;
		--grass2: #7fa06b;
		--path: #e9dcc0;
		--blade: #5f8250;
		--olive: #7a9a6a;
		--trunk: #6b5238;
	}
	:global(.dark) .scene-artemis {
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
		--cypress-far: #1a2f40;
		--m: #8b92a8;
		--m2: #6d7690;
		--m-back: #4d5671;
		--m-flute: #6d7690;
		--m-flute-b: #3a4260;
		--cella: #1c2036;
		--door: #0a0f1d;
		--lamp: #f5c76a;
		--lamp-on: 1;
		--step1: #7a8299;
		--step2: #667088;
		--step3: #535c75;
		--step-shade: #2a3048;
		--frieze: #4d5671;
		--tympanum: #2c4a80;
		--roof: #5c4458;
		--grass1: #1a2a3a;
		--grass2: #0e1626;
		--path: #3a4260;
		--blade: #2a4a4a;
		--olive: #1f3a3a;
		--trunk: #14182a;
	}

	.bflame { transform-box: fill-box; transform-origin: 50% 100%; }
	.dv { opacity: 0; }
	@media (prefers-reduced-motion: no-preference) {
		.bflame { animation: ar-flicker 1.8s ease-in-out infinite alternate; }
		.p-down .bflame { animation: ar-out 4.4s ease-in both; }
		.p-up .bflame { animation: ar-in 2.8s ease-out both; }
		.p-resolved .bflame { animation: ar-flare 3s ease-out both; }
		.p-resolved .dv { animation: ar-fly 3.6s ease-out both; }
		.p-resolved .dv2 { animation-delay: 0.3s; }
		.p-resolved .dv3 { animation-delay: 0.6s; }
	}
	@keyframes ar-flicker { to { transform: scale(1.06, 0.94); } }
	@keyframes ar-out {
		0% { transform: none; opacity: 1; } 15% { transform: scale(0.7, 0.55); } 28% { transform: scale(1, 0.9); }
		50%, 85% { transform: scale(0.1, 0.05); opacity: 0; } 100% { transform: none; opacity: 1; }
	}
	@keyframes ar-in {
		0% { transform: scale(0.1, 0.05); opacity: 0; } 30% { transform: scale(0.6, 0.7); opacity: 1; } 45% { transform: scale(0.3, 0.3); }
		70% { transform: scale(1.2, 1.3); } 100% { transform: none; }
	}
	@keyframes ar-flare { 30% { transform: scale(1.6, 1.9); } 100% { transform: none; } }
	@keyframes ar-fly {
		0% { opacity: 0; transform: translate(0, 0); } 15% { opacity: 1; } 80% { opacity: 1; }
		100% { opacity: 0; transform: translate(120px, -150px); }
	}
</style>
