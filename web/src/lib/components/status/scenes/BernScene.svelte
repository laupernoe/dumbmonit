<script lang="ts">
  /**
   * Bern — the Zytglogge clock tower in the old town, arcades, the Aare, the
   * Alps behind. `mood` is the weather (clear, clouded, storm with rain and
   * lightning). `pulse` plays one gesture when `at` changes: on `down` the
   * clock stops and its dial goes dark, on `up` the hands spin and the dial
   * relights, on `resolved` the tower bell rings with sound waves and
   * confetti. Reduced motion freezes everything.
   */
  import SceneWeather from './SceneWeatherS.svelte';
  import { createPulse, type SceneMood, type ScenePulse } from './scenePulseS.svelte.js';

  interface Props {
    mood?: SceneMood;
    pulse?: ScenePulse;
  }
  let { mood = 'ok', pulse = null }: Props = $props();
  const pl = createPulse(() => pulse, { down: 7000, up: 4000, resolved: 6500 });
  const CONFETTI = Array.from({ length: 22 }, (_, i) => ({
    x: 1060 + ((i * 53) % 220),
    c: ['#d52b1e', '#fff', '#e0b341'][i % 3],
    d: ((i * 211) % 1800) + 700,
    dx: ((i * 37) % 90) - 45
  }));
</script>

<div
  class="scene scene-bern"
  class:p-down={pl.active === 'down'}
  class:p-up={pl.active === 'up'}
  class:p-resolved={pl.active === 'resolved'}
  aria-hidden="true"
>
  <svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none">
    <defs>
      <!-- arcades de la vieille ville : un arc tous les 30, alignés sur le sol y=482 -->
      <pattern id="bern-arch" x="0" y="438" width="30" height="44" patternUnits="userSpaceOnUse">
        <path d="M6 44V20a9 9 0 0 1 18 0V44z" fill="var(--bern-dark)" />
      </pattern>
      <pattern id="bern-win" x="0" y="0" width="26" height="30" patternUnits="userSpaceOnUse"><rect x="9" y="8" width="8" height="13" rx="1" fill="var(--bern-win)" /></pattern>
      <!-- drapeau suisse 1 x 1, origine = coin haut gauche -->
      <g id="bern-flag"><rect width="24" height="24" fill="#d52b1e" /><path d="M10 5h4v5h5v4h-5v5h-4v-5H5v-4h5z" fill="#fff" /></g>
    </defs>

    <!-- astre, étoiles, nuages -->
    <circle cx="1500" cy="190" r="48" fill="var(--sun)" />
    <circle cx="1484" cy="178" r="7" fill="var(--crater)" /><circle cx="1514" cy="204" r="5" fill="var(--crater)" />
    <g fill="var(--star)"><circle cx="1010" cy="118" r="2" /><circle cx="1330" cy="104" r="2.2" /><circle cx="1620" cy="130" r="2" /><circle cx="1700" cy="250" r="2.2" /><circle cx="700" cy="170" r="1.8" /><circle cx="1000" cy="230" r="1.8" /></g>
    <g fill="var(--cloud)" opacity=".75"><ellipse cx="760" cy="146" rx="80" ry="10" /><ellipse cx="810" cy="164" rx="50" ry="8" /><ellipse cx="1590" cy="290" rx="60" ry="8" /></g>

    <!-- Alpes : Eiger, Mönch, Jungfrau, calottes enneigées -->
    <path d="M560 400L650 340L700 372L790 296L840 346L930 262L1000 340L1070 300L1130 372L1200 330L1280 380L1370 320L1450 372L1540 302L1620 366L1700 330L1780 380V482H560z" fill="var(--bern-alps)" />
    <g fill="var(--bern-snow)"><path d="M790 296L766 322L784 316L798 330L812 318L822 324z" /><path d="M930 262L902 298L920 290L934 306L950 292L962 300z" /><path d="M1540 302L1518 330L1534 324L1546 336L1560 324L1566 330z" /><path d="M1370 320L1352 340L1368 336L1380 346L1390 336z" /></g>

    <!-- Bundeshaus au loin : aile, tambour, coupole de cuivre, drapeau -->
    <g>
      <rect x="650" y="404" width="170" height="78" fill="var(--bern-far)" /><rect x="660" y="394" width="150" height="12" fill="var(--bern-far)" />
      <rect x="716" y="372" width="38" height="26" fill="var(--bern-far)" />
      <path d="M714 372C714 330 756 330 756 372z" fill="var(--bern-copper)" />
      <rect x="732" y="322" width="4" height="16" fill="var(--bern-copper)" /><use href="#bern-flag" transform="translate(737 304) scale(.5)" />
      <rect x="740" y="304" width="1.4" height="22" fill="var(--bern-dark)" />
      <path d="M670 482V430M690 482V430M710 482V430M760 482V430M780 482V430M800 482V430" stroke="var(--bern-stone2)" stroke-width="4" opacity=".6" />
    </g>

    <!-- vieille ville gauche : maisons à arcades, toits de tuiles -->
    <g>
      <rect x="560" y="388" width="110" height="94" fill="var(--bern-sand)" /><rect x="560" y="388" width="110" height="50" fill="url(#bern-win)" /><rect x="560" y="438" width="110" height="44" fill="url(#bern-arch)" />
      <path d="M552 388L574 350H658L678 388z" fill="var(--bern-tile)" />
      <rect x="826" y="404" width="120" height="78" fill="var(--bern-sand2)" /><rect x="826" y="404" width="120" height="34" fill="url(#bern-win)" /><rect x="826" y="438" width="120" height="44" fill="url(#bern-arch)" />
      <path d="M818 404L842 366H932L954 404z" fill="var(--bern-tile)" />
      <rect x="950" y="380" width="104" height="102" fill="var(--bern-sand)" /><rect x="950" y="380" width="104" height="58" fill="url(#bern-win)" /><rect x="950" y="438" width="104" height="44" fill="url(#bern-arch)" />
      <path d="M942 380L966 340H1040L1062 380z" fill="var(--bern-tile2)" />
      <rect x="1000" y="318" width="5" height="26" fill="var(--bern-dark)" /><use href="#bern-flag" transform="translate(1005 318) scale(.6)" />
    </g>

    <!-- Zytglogge : fût de molasse, cadran astronomique, cadran bleu et or, lanterne, toit de tuiles et flèche -->
    <g>
      <rect x="1118" y="220" width="104" height="262" fill="var(--bern-stone)" />
      <rect x="1118" y="220" width="12" height="262" fill="var(--bern-stone2)" opacity=".7" />
      <rect x="1108" y="214" width="124" height="10" fill="var(--bern-tile2)" />
      <!-- porte voûtée au pied -->
      <path d="M1148 482V440a22 22 0 0 1 44 0V482z" fill="var(--bern-dark)" />
      <!-- cadran astronomique -->
      <circle cx="1170" cy="388" r="26" fill="var(--bern-gold)" /><circle cx="1170" cy="388" r="21" fill="var(--bern-dial)" />
      <circle cx="1170" cy="388" r="13" fill="none" stroke="var(--bern-gold)" stroke-width="1.5" /><circle cx="1170" cy="388" r="5" fill="var(--bern-gold)" />
      <path d="M1170 367V409M1149 388H1191" stroke="var(--bern-gold)" stroke-width="1" />
      <!-- grand cadran -->
      <circle cx="1170" cy="288" r="34" fill="var(--bern-gold)" /><circle cx="1170" cy="288" r="29" fill="var(--bern-dial)" />
      <g stroke="var(--bern-gold)" stroke-width="2" stroke-linecap="round"><path d="M1170 262v6M1170 308v6M1144 288h6M1190 288h6M1152 270l3 3M1185 303l3 3M1188 270l-3 3M1155 303l-3 3" /></g>
      <circle class="dialdim" cx="1170" cy="288" r="29" fill="#14121c" />
      <g class="hand hand-m"><circle cx="1170" cy="288" r="30" /><path d="M1170 288V266" stroke="var(--bern-gold)" stroke-width="3" stroke-linecap="round" /></g>
      <g class="hand hand-h"><circle cx="1170" cy="288" r="30" /><path d="M1170 288L1182 296" stroke="var(--bern-gold)" stroke-width="3" stroke-linecap="round" /></g>
      <circle cx="1170" cy="288" r="3" fill="var(--bern-gold)" />
      <!-- étage supérieur, lanterne à arcades -->
      <rect x="1126" y="170" width="88" height="46" fill="var(--bern-stone)" /><rect x="1120" y="164" width="100" height="8" fill="var(--bern-stone2)" />
      <path d="M1140 214V190a8 8 0 0 1 16 0V214zM1184 214V190a8 8 0 0 1 16 0V214z" fill="var(--bern-win)" />
      <g class="bell"><rect x="1169" y="176" width="2" height="8" fill="var(--bern-dark)" /><path d="M1162 206c0-10 3-20 8-20s8 10 8 20z" fill="var(--bern-gold)" /><circle cx="1170" cy="208" r="2.4" fill="var(--bern-dark)" /></g>
      <g class="waves" stroke="var(--bern-gold)" stroke-width="3" stroke-linecap="round" fill="none">
        <path d="M1104 186q-12 14 0 28M1090 178q-20 22 0 44" /><path d="M1236 186q12 14 0 28M1250 178q20 22 0 44" />
      </g>
      <!-- tourelles d'angle et toit pyramidal -->
      <path d="M1112 166L1126 126L1140 166z" fill="var(--bern-tile)" /><path d="M1200 166L1214 126L1228 166z" fill="var(--bern-tile)" />
      <path d="M1126 166L1170 74L1214 166z" fill="var(--bern-tile)" />
      <path d="M1170 74V166" stroke="var(--bern-tile2)" stroke-width="2" opacity=".6" />
      <rect x="1169" y="44" width="2" height="34" fill="var(--bern-gold)" /><circle cx="1170" cy="60" r="3.5" fill="var(--bern-gold)" />
      <rect x="1170" y="30" width="2" height="22" fill="var(--bern-dark)" /><use href="#bern-flag" transform="translate(1172 30) scale(.7)" />
    </g>

    <!-- vieille ville droite -->
    <g>
      <rect x="1250" y="396" width="116" height="86" fill="var(--bern-sand2)" /><rect x="1250" y="396" width="116" height="42" fill="url(#bern-win)" /><rect x="1250" y="438" width="116" height="44" fill="url(#bern-arch)" />
      <path d="M1242 396L1266 358H1352L1374 396z" fill="var(--bern-tile)" />
      <rect x="1376" y="382" width="124" height="100" fill="var(--bern-sand)" /><rect x="1376" y="382" width="124" height="56" fill="url(#bern-win)" /><rect x="1376" y="438" width="124" height="44" fill="url(#bern-arch)" />
      <path d="M1368 382L1392 344H1486L1508 382z" fill="var(--bern-tile2)" />
      <rect x="1520" y="406" width="140" height="76" fill="var(--bern-sand2)" /><rect x="1520" y="406" width="140" height="32" fill="url(#bern-win)" /><rect x="1520" y="438" width="140" height="44" fill="url(#bern-arch)" />
      <path d="M1512 406L1536 370H1644L1668 406z" fill="var(--bern-tile)" />
    </g>

    <!-- quai de molasse et l'Aare qui s'enroule -->
    <rect x="540" y="482" width="1260" height="8" fill="var(--bern-stone2)" />
    <path d="M540 520C700 500 820 548 980 530S1260 510 1420 536S1700 540 1800 526" stroke="var(--bern-aare)" stroke-width="26" stroke-linecap="round" opacity=".55" />
    <g stroke="var(--bern-ripple)" stroke-width="2" stroke-linecap="round" opacity=".5"><path d="M620 560h90M820 574h110M1060 552h100M1280 580h120M1500 556h90M700 596h70M1180 604h110" /></g>
    <g fill="var(--bern-tree)"><circle cx="600" cy="478" r="14" /><circle cx="1440" cy="480" r="12" /><circle cx="1710" cy="478" r="15" /></g>

    <!-- mascottes : sur le toit de gauche et au bord de l'eau -->
    <use href="#dm-mascot" transform="translate(1486 316) scale(.1)" />
    <use href="#dm-mascot" transform="translate(880 452) scale(.1)" />
    <g class="confetti">
      {#if pl.active === 'resolved'}
        {#key pl.key}
          {#each CONFETTI as c}
            <rect class="conf" x={c.x} y="130" width="7" height="5" fill={c.c} style="--dx:{c.dx}px;animation-delay:{c.d}ms" />
          {/each}
        {/key}
      {/if}
    </g>
    <SceneWeather {mood} />
  </svg>
</div>

<style>
  .scene {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: linear-gradient(180deg, var(--sky1), var(--sky2) 73.5%, var(--water1) 74.4%, var(--water1) 80%, var(--water2));
  }
  .scene svg {
    display: block;
    width: 100%;
    height: 100%;
    -webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
    mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
  }
  .hand, .bell { transform-box: fill-box; }
  .hand { transform-origin: 50% 50%; }
  .hand circle { fill: none; }
  .waves { transform-box: fill-box; transform-origin: center; }
  .bell { transform-origin: 50% 0; }
  .dialdim, .waves, .conf { opacity: 0; }
  @media (prefers-reduced-motion: no-preference) {
    .hand-m { animation: tick 120s linear infinite; }
    .hand-h { animation: tick 1440s linear infinite; }
    .p-down .hand-m, .p-down .hand-h { animation-play-state: paused; }
    .p-down .dialdim { animation: stopped 7s ease-in-out both; }
    .p-up .hand-m { animation: whirl 3s cubic-bezier(0.2, 0.7, 0.3, 1) both; }
    .p-up .hand-h { animation: whirl-h 3s cubic-bezier(0.2, 0.7, 0.3, 1) both; }
    .p-up .dialdim { animation: relit 1.4s ease-out both; }
    .p-resolved .bell { animation: ring 1.1s ease-in-out 5 both; }
    .p-resolved .waves { animation: pulse 1.1s ease-out 5 both; }
    .conf { transform-box: fill-box; transform-origin: center; animation: confetti 3.4s ease-in both; }
  }
  @keyframes tick { to { transform: rotate(360deg); } }
  @keyframes whirl { to { transform: rotate(1080deg); } }
  @keyframes whirl-h { to { transform: rotate(90deg); } }
  @keyframes stopped { 0% { opacity: 0; } 8% { opacity: 0.7; } 14% { opacity: 0.25; } 22%, 80% { opacity: 0.8; } 100% { opacity: 0; } }
  @keyframes relit { 0% { opacity: 0.8; } 30% { opacity: 0.1; } 45% { opacity: 0.5; } 100% { opacity: 0; } }
  @keyframes ring { 0%, 100% { transform: rotate(0); } 25% { transform: rotate(22deg); } 75% { transform: rotate(-22deg); } }
  @keyframes pulse { 0% { opacity: 0.9; transform: scale(0.96); } 100% { opacity: 0; transform: scale(1.1); } }
  @keyframes confetti {
    0% { opacity: 1; transform: translate(0, 0) rotate(0); } 85% { opacity: 1; }
    100% { opacity: 0; transform: translate(var(--dx), 340px) rotate(540deg); }
  }
  .scene-bern {
    --sky1: #a9c8e6; --sky2: #eaf1f3; --sun: #fff2c6; --water1: #8fc0b8; --water2: #5f9a98; --cloud: #fff;
    --crater: transparent; --star: transparent;
    --bern-stone: #cdc6a8; --bern-stone2: #aaa38a; --bern-sand: #d6cfb4; --bern-sand2: #c9c3a8; --bern-dark: #5a5a52; --bern-win: #6c7068;
    --bern-tile: #c4503c; --bern-tile2: #a63f30; --bern-gold: #e0b341; --bern-dial: #2f55a4;
    --bern-alps: #b9c7dc; --bern-snow: #fff; --bern-far: #c5c8c0; --bern-copper: #6aa592;
    --bern-aare: #6bb3ad; --bern-ripple: #fff; --bern-tree: #6f9a5c;
  }
  :global(.dark) .scene-bern {
    --sky1: #0b1230; --sky2: #26325a; --sun: #e8eef8; --water1: #1c2a44; --water2: #0d1424; --cloud: #2a3658;
    --crater: #b9c4dc; --star: #e8eef8;
    --bern-stone: #6d6c76; --bern-stone2: #504f5c; --bern-sand: #5f5d68; --bern-sand2: #68646f; --bern-dark: #1d1b27; --bern-win: #f5c76a;
    --bern-tile: #6d3340; --bern-tile2: #52283a; --bern-gold: #f0c85a; --bern-dial: #213a78;
    --bern-alps: #2a3558; --bern-snow: #a5b4d4; --bern-far: #353a58; --bern-copper: #3d6a6a;
    --bern-aare: #3a6d8c; --bern-ripple: #7a9cc8; --bern-tree: #1f4a4f;
  }
</style>
