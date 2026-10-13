<script lang="ts">
	/**
	 * The Mausoleum of Halicarnassus: a tall podium with a frieze, an Ionic colonnade, a stepped pyramid roof topped by a quadriga, between cypresses.
	 * `mood` is the weather (ok: clear, warn: clouds, critical: rain and
	 * lightning; `calm|clouded|storm` accepted too). `pulse` plays one gesture
	 * whenever its `at` changes: down topples the golden quadriga off the roof (earthquake), up drops it back with a bounce, resolved makes the chariot rear up twice amid golden sparkles.
	 * Fills its positioned parent; the headline zone (top left) is left clear.
	 * Reduced motion freezes every animation.
	 */
	import SceneWeather from './SceneWeatherB.svelte';
	import { createGesture, moodLevel, type Mood, type Pulse } from './sceneGesture.svelte.js';

	interface Props {
		mood?: Mood;
		pulse?: Pulse;
	}
	let { mood = 'ok', pulse = null }: Props = $props();
	const g = createGesture(() => pulse);
	const level = $derived(moodLevel(mood));
	const spark = 'M0-9L2.4-2.4L9 0L2.4 2.4L0 9L-2.4 2.4L-9 0L-2.4-2.4z';
</script>

<div class="scene scene-halicarnassus" class:p-down={g.kind === 'down'} class:p-up={g.kind === 'up'} class:p-resolved={g.kind === 'resolved'} aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
<defs><linearGradient id="hl-sea" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--water1)"/><stop offset="1" stop-color="var(--water2)"/></linearGradient></defs>
<g fill="var(--star)"><circle cx="1010" cy="118" r="2"/><circle cx="1250" cy="104" r="2.2"/><circle cx="1560" cy="120" r="2"/><circle cx="1650" cy="230" r="2.2"/><circle cx="1130" cy="190" r="1.8"/><circle cx="1440" cy="160" r="1.8"/><circle cx="780" cy="200" r="2"/></g>
<circle cx="1470" cy="190" r="46" fill="var(--sun)"/>
<g fill="var(--cloud)" opacity=".75"><ellipse cx="850" cy="150" rx="80" ry="12"/><ellipse cx="900" cy="164" rx="50" ry="9"/><ellipse cx="1620" cy="290" rx="80" ry="11"/></g>
<path d="M560 470C660 420 760 400 860 436C940 400 1010 390 1080 440L1080 480H560Z" fill="var(--hill)"/>
<path d="M1300 480C1380 420 1480 392 1580 430C1650 410 1720 420 1780 450V480Z" fill="var(--hill)"/>
<rect x="560" y="458" width="1220" height="24" fill="url(#hl-sea)" opacity=".8"/>
<rect x="560" y="478" width="1220" height="180" fill="var(--ground)"/>
<path d="M560 480H1780" stroke="var(--ground2)" stroke-width="3"/>
<!-- precinct wall -->
<rect x="860" y="466" width="640" height="14" fill="var(--wall)"/><g stroke="var(--wall2)" stroke-width="2"><path d="M860 473H1500"/></g>
<!-- mausoleum -->
<g>
<rect x="1040" y="466" width="280" height="14" fill="var(--m2)"/><rect x="1052" y="455" width="256" height="12" fill="var(--m1)"/>
<rect x="1066" y="350" width="228" height="106" fill="var(--pod)"/>
<rect x="1060" y="342" width="240" height="8" fill="var(--m1)"/>
<rect x="1066" y="366" width="228" height="5" fill="var(--m1)"/><rect x="1066" y="392" width="228" height="5" fill="var(--m1)"/>
<rect x="1066" y="371" width="228" height="21" fill="var(--frieze)"/>
<g fill="var(--m1)"><rect x="1080" y="376" width="3.5" height="11" rx="1.5"/><rect x="1090.6" y="376" width="3.5" height="11" rx="1.5"/><rect x="1101.2" y="376" width="3.5" height="11" rx="1.5"/><rect x="1111.8" y="376" width="3.5" height="11" rx="1.5"/><rect x="1122.4" y="376" width="3.5" height="11" rx="1.5"/><rect x="1133" y="376" width="3.5" height="11" rx="1.5"/><rect x="1143.6" y="376" width="3.5" height="11" rx="1.5"/><rect x="1154.2" y="376" width="3.5" height="11" rx="1.5"/><rect x="1164.8" y="376" width="3.5" height="11" rx="1.5"/><rect x="1175.4" y="376" width="3.5" height="11" rx="1.5"/><rect x="1186" y="376" width="3.5" height="11" rx="1.5"/><rect x="1196.6" y="376" width="3.5" height="11" rx="1.5"/><rect x="1207.2" y="376" width="3.5" height="11" rx="1.5"/><rect x="1217.8" y="376" width="3.5" height="11" rx="1.5"/><rect x="1228.4" y="376" width="3.5" height="11" rx="1.5"/><rect x="1239" y="376" width="3.5" height="11" rx="1.5"/><rect x="1249.6" y="376" width="3.5" height="11" rx="1.5"/><rect x="1260.2" y="376" width="3.5" height="11" rx="1.5"/><rect x="1270.8" y="376" width="3.5" height="11" rx="1.5"/><rect x="1281.4" y="376" width="3.5" height="11" rx="1.5"/></g>
<g fill="var(--pod2)"><rect x="1066" y="397" width="228" height="10" opacity=".5"/><rect x="1084" y="410" width="36" height="42"/><rect x="1240" y="410" width="36" height="42"/></g>
<path d="M1162 456V418H1198V456Z" fill="var(--door)"/>
<g stroke="var(--m2)" stroke-width="2"><path d="M1150 456h60M1154 450h52M1158 444h44"/></g>
<rect x="1070" y="284" width="220" height="58" fill="var(--shade)"/>
<rect x="1078" y="284" width="7" height="58" fill="var(--m1)"/><rect x="1076" y="279" width="11" height="5" fill="var(--m1)"/><circle cx="1076.5" cy="283" r="2.4" fill="var(--m1)"/><circle cx="1086.5" cy="283" r="2.4" fill="var(--m1)"/><rect x="1096.9" y="284" width="7" height="58" fill="var(--m1)"/><rect x="1094.9" y="279" width="11" height="5" fill="var(--m1)"/><circle cx="1095.4" cy="283" r="2.4" fill="var(--m1)"/><circle cx="1105.4" cy="283" r="2.4" fill="var(--m1)"/><rect x="1115.8" y="284" width="7" height="58" fill="var(--m1)"/><rect x="1113.8" y="279" width="11" height="5" fill="var(--m1)"/><circle cx="1114.3" cy="283" r="2.4" fill="var(--m1)"/><circle cx="1124.3" cy="283" r="2.4" fill="var(--m1)"/><rect x="1134.7" y="284" width="7" height="58" fill="var(--m1)"/><rect x="1132.7" y="279" width="11" height="5" fill="var(--m1)"/><circle cx="1133.2" cy="283" r="2.4" fill="var(--m1)"/><circle cx="1143.2" cy="283" r="2.4" fill="var(--m1)"/><rect x="1153.6" y="284" width="7" height="58" fill="var(--m1)"/><rect x="1151.6" y="279" width="11" height="5" fill="var(--m1)"/><circle cx="1152.1" cy="283" r="2.4" fill="var(--m1)"/><circle cx="1162.1" cy="283" r="2.4" fill="var(--m1)"/><rect x="1172.5" y="284" width="7" height="58" fill="var(--m1)"/><rect x="1170.5" y="279" width="11" height="5" fill="var(--m1)"/><circle cx="1171" cy="283" r="2.4" fill="var(--m1)"/><circle cx="1181" cy="283" r="2.4" fill="var(--m1)"/><rect x="1191.4" y="284" width="7" height="58" fill="var(--m1)"/><rect x="1189.4" y="279" width="11" height="5" fill="var(--m1)"/><circle cx="1189.9" cy="283" r="2.4" fill="var(--m1)"/><circle cx="1199.9" cy="283" r="2.4" fill="var(--m1)"/><rect x="1210.3" y="284" width="7" height="58" fill="var(--m1)"/><rect x="1208.3" y="279" width="11" height="5" fill="var(--m1)"/><circle cx="1208.8" cy="283" r="2.4" fill="var(--m1)"/><circle cx="1218.8" cy="283" r="2.4" fill="var(--m1)"/><rect x="1229.2" y="284" width="7" height="58" fill="var(--m1)"/><rect x="1227.2" y="279" width="11" height="5" fill="var(--m1)"/><circle cx="1227.7" cy="283" r="2.4" fill="var(--m1)"/><circle cx="1237.7" cy="283" r="2.4" fill="var(--m1)"/><rect x="1248.1" y="284" width="7" height="58" fill="var(--m1)"/><rect x="1246.1" y="279" width="11" height="5" fill="var(--m1)"/><circle cx="1246.6" cy="283" r="2.4" fill="var(--m1)"/><circle cx="1256.6" cy="283" r="2.4" fill="var(--m1)"/><rect x="1267" y="284" width="7" height="58" fill="var(--m1)"/><rect x="1265" y="279" width="11" height="5" fill="var(--m1)"/><circle cx="1265.5" cy="283" r="2.4" fill="var(--m1)"/><circle cx="1275.5" cy="283" r="2.4" fill="var(--m1)"/><rect x="1285.9" y="284" width="7" height="58" fill="var(--m1)"/><rect x="1283.9" y="279" width="11" height="5" fill="var(--m1)"/><circle cx="1284.4" cy="283" r="2.4" fill="var(--m1)"/><circle cx="1294.4" cy="283" r="2.4" fill="var(--m1)"/>
<rect x="1062" y="272" width="236" height="12" fill="var(--m1)"/><rect x="1062" y="272" width="236" height="5" fill="var(--frieze)"/>
<rect x="1056" y="262" width="248" height="10" fill="var(--m2)"/>
<rect x="1068" y="258.3" width="224" height="4.05" fill="var(--m1)"/><rect x="1071.4" y="254.5" width="217.2" height="4.05" fill="var(--m2)"/><rect x="1074.8" y="250.8" width="210.3" height="4.05" fill="var(--m1)"/><rect x="1078.2" y="247" width="203.5" height="4.05" fill="var(--m2)"/><rect x="1081.7" y="243.3" width="196.7" height="4.05" fill="var(--m1)"/><rect x="1085.1" y="239.5" width="189.8" height="4.05" fill="var(--m2)"/><rect x="1088.5" y="235.8" width="183" height="4.05" fill="var(--m1)"/><rect x="1091.9" y="232" width="176.2" height="4.05" fill="var(--m2)"/><rect x="1095.3" y="228.3" width="169.4" height="4.05" fill="var(--m1)"/><rect x="1098.7" y="224.5" width="162.5" height="4.05" fill="var(--m2)"/><rect x="1102.2" y="220.8" width="155.7" height="4.05" fill="var(--m1)"/><rect x="1105.6" y="217" width="148.9" height="4.05" fill="var(--m2)"/><rect x="1109" y="213.3" width="142" height="4.05" fill="var(--m1)"/><rect x="1112.4" y="209.5" width="135.2" height="4.05" fill="var(--m2)"/><rect x="1115.8" y="205.8" width="128.4" height="4.05" fill="var(--m1)"/><rect x="1119.2" y="202" width="121.5" height="4.05" fill="var(--m2)"/><rect x="1122.6" y="198.3" width="114.7" height="4.05" fill="var(--m1)"/><rect x="1126.1" y="194.5" width="107.9" height="4.05" fill="var(--m2)"/><rect x="1129.5" y="190.8" width="101.1" height="4.05" fill="var(--m1)"/><rect x="1132.9" y="187" width="94.2" height="4.05" fill="var(--m2)"/><rect x="1136.3" y="183.3" width="87.4" height="4.05" fill="var(--m1)"/><rect x="1139.7" y="179.5" width="80.6" height="4.05" fill="var(--m2)"/><rect x="1143.1" y="175.8" width="73.7" height="4.05" fill="var(--m1)"/><rect x="1146.5" y="172" width="66.9" height="4.05" fill="var(--m2)"/>
<rect x="1160" y="160" width="40" height="12" fill="var(--m2)"/>
<g class="hl-quad"><g fill="var(--gold)" transform="translate(1180 162) scale(1.5) translate(-1180 -162)"><g transform="translate(0 0)"><ellipse cx="1188" cy="143" rx="15" ry="5.5"/><path d="M1198 140L1210 126L1219 128L1220 133L1213 134L1205 146Z"/><path d="M1180 148l-1 9M1196 148l1 9" stroke="var(--gold)" stroke-width="2.4"/></g><g transform="translate(5 0)"><ellipse cx="1188" cy="143" rx="15" ry="5.5"/><path d="M1198 140L1210 126L1219 128L1220 133L1213 134L1205 146Z"/><path d="M1180 148l-1 9M1196 148l1 9" stroke="var(--gold)" stroke-width="2.4"/></g><g transform="translate(10 0)"><ellipse cx="1188" cy="143" rx="15" ry="5.5"/><path d="M1198 140L1210 126L1219 128L1220 133L1213 134L1205 146Z"/><path d="M1180 148l-1 9M1196 148l1 9" stroke="var(--gold)" stroke-width="2.4"/></g><g transform="translate(15 0)"><ellipse cx="1188" cy="143" rx="15" ry="5.5"/><path d="M1198 140L1210 126L1219 128L1220 133L1213 134L1205 146Z"/><path d="M1180 148l-1 9M1196 148l1 9" stroke="var(--gold)" stroke-width="2.4"/></g><rect x="1160" y="140" width="28" height="12" rx="2"/><circle cx="1166" cy="153" r="8" stroke="var(--gold2)" stroke-width="2.4" fill="none"/><circle cx="1166" cy="153" r="1.8"/><path d="M1169 140c0-9 4-13 7-13s6 4 6 13z"/><circle cx="1176" cy="122" r="3.6"/></g></g>
</g>
<path d="M1020 330C1033 390 1033 450 1027.8 480H1012.2C1007 450 1007 390 1020 330Z" fill="var(--tree)"/><path d="M1344 320C1357 384 1357 448 1351.8 480H1336.2C1331 448 1331 384 1344 320Z" fill="var(--tree)"/><path d="M1000 380C1010 420 1010 460 1006 480H994C990 460 990 420 1000 380Z" fill="var(--tree)"/><path d="M1368 370C1378 414 1378 458 1374 480H1362C1358 458 1358 414 1368 370Z" fill="var(--tree)"/><path d="M960 410C968 438 968 466 964.8 480H955.2C952 466 952 438 960 410Z" fill="var(--tree)"/><path d="M1410 400C1419 432 1419 464 1415.4 480H1404.6C1401 464 1401 432 1410 400Z" fill="var(--tree)"/>
<g fill="var(--grass)" opacity=".8"><ellipse cx="1180" cy="500" rx="190" ry="8"/><ellipse cx="1500" cy="520" rx="150" ry="7"/><ellipse cx="860" cy="526" rx="170" ry="7"/></g>
<g class="hl-sparks" fill="#ffd978">
<path d={spark} transform="translate(1130 130)" /><path d={spark} transform="translate(1235 120) scale(.9)" /><path d={spark} transform="translate(1180 96) scale(1.2)" /><path d={spark} transform="translate(1260 160) scale(.7)" />
</g>
<g class="hl-dust" fill="var(--m2)"><ellipse cx="1150" cy="250" rx="30" ry="12" /><ellipse cx="1210" cy="246" rx="26" ry="10" /><ellipse cx="1180" cy="236" rx="22" ry="9" /></g>
<SceneWeather {level} bolt={1180} />
	</svg>
</div>

<style>
	.scene { position: absolute; inset: 0; overflow: hidden; background: linear-gradient(180deg, var(--sky1), var(--sky2) 73.5%, var(--water1) 74.4%, var(--water1) 80%, var(--water2)); }
	svg { position: absolute; inset: 0; width: 100%; height: 100%; -webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%); mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%); }
	.scene-halicarnassus {
		--sky1: #f1cf9e; --sky2: #fbe9cd; --sun: #fff0c4; --star: transparent; --cloud: #fff; --water1: #8fbfc4; --water2: #6fa4ae; --hill: #c9b690; --ground: #dcc79a; --ground2: #c9b280; --grass: #b8b87a; --wall: #e9dcc0; --wall2: #d4c4a2; --m1: #f6efde; --m2: #e2d6bd; --pod: #e8d9b4; --pod2: #d3c19a; --frieze: #5d86a6; --shade: #a98f69; --door: #6c5a44; --gold: #d9a63a; --gold2: #b8872a; --tree: #4c6d4e;
	}
	:global(.dark) .scene-halicarnassus {
		--sky1: #0d1229; --sky2: #2a2142; --sun: #e8eef8; --star: #e8eef8; --cloud: #2a3658; --water1: #1b2a4c; --water2: #0e1730; --hill: #2a2b4a; --ground: #262541; --ground2: #3a3858; --grass: #1f3a40; --wall: #5c5772; --wall2: #4a4660; --m1: #9aa0b8; --m2: #7a7f9a; --pod: #6f6a86; --pod2: #58546e; --frieze: #3d5a86; --shade: #2b2842; --door: #14122a; --gold: #e8bd4f; --gold2: #b88f32; --tree: #17383f;
	}
	.hl-quad { transform-box: fill-box; transform-origin: 50% 100%; }
	.hl-sparks, .hl-dust { opacity: 0; }
	@media (prefers-reduced-motion: no-preference) {
		.p-down .hl-quad { animation: hl-fall 5s ease-in both; }
		.p-down .hl-dust { animation: hl-puff 3s ease-out 0.9s both; }
		.p-up .hl-quad { animation: hl-land 2.6s cubic-bezier(0.3, 0, 0.3, 1) both; }
		.p-resolved .hl-quad { animation: hl-rear 3.6s ease-in-out both; }
		.p-resolved .hl-sparks { animation: hl-twinkle 1.8s ease-in-out 3 both; }
	}
	@keyframes hl-fall { 0% { transform: none; } 12% { transform: rotate(-6deg); } 20% { transform: rotate(5deg); } 45% { transform: translate(14px, 70px) rotate(38deg); opacity: 1; } 60%, 88% { transform: translate(18px, 90px) rotate(60deg); opacity: 0; } 100% { transform: none; opacity: 1; } }
	@keyframes hl-puff { 0% { opacity: 0.8; transform: translateY(0) scale(0.5); } 100% { opacity: 0; transform: translateY(-30px) scale(1.8); } }
	@keyframes hl-land { 0% { transform: translateY(-90px); opacity: 0; } 50% { transform: translateY(0); opacity: 1; } 65% { transform: translateY(-14px); } 80%, 100% { transform: none; opacity: 1; } }
	@keyframes hl-rear { 0%, 100% { transform: none; } 20% { transform: rotate(-12deg) translateY(-10px); } 40% { transform: none; } 60% { transform: rotate(-12deg) translateY(-10px); } 80% { transform: none; } }
	@keyframes hl-twinkle { 0%, 100% { opacity: 0; } 50% { opacity: 1; } }
</style>
