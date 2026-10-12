<div class="scene scene-matterhorn" aria-hidden="true">
  <svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none">
    <defs>
      <clipPath id="mh-clip">
        <path d="M700 490L860 380L960 330L1040 280L1090 214L1130 162L1160 120L1174 98L1190 112L1210 172L1242 242L1272 302L1332 362L1424 414L1580 490z" />
      </clipPath>
      <!-- chalet : origine = pied, coin gauche, 72 de large -->
      <g id="mh-chalet">
        <rect y="-40" width="72" height="40" fill="var(--mh-wood)" />
        <path d="M0-26h72M0-13h72" stroke="var(--mh-wood2)" stroke-width="2" />
        <path d="M-7-40L36-66L79-40z" fill="var(--mh-roof)" />
        <path d="M-2-42L36-64L74-42L68-42L36-58L4-42z" fill="var(--mh-snow)" />
        <path d="M12-30h14v14H12zM46-30h14v14H46z" fill="var(--mh-win)" />
        <path d="M8-8h56v3H8z" fill="var(--mh-wood2)" />
      </g>
    </defs>

    <!-- astre, étoiles et nuages -->
    <circle cx="1560" cy="190" r="42" fill="var(--sun)" />
    <circle cx="1546" cy="180" r="7" fill="var(--crater)" /><circle cx="1574" cy="202" r="5" fill="var(--crater)" />
    <g fill="var(--star)"><circle cx="720" cy="130" r="2" /><circle cx="900" cy="108" r="2.4" /><circle cx="1400" cy="120" r="2" /><circle cx="1660" cy="132" r="1.8" /><circle cx="1000" cy="230" r="1.8" /><circle cx="1710" cy="260" r="2.2" /><circle cx="820" cy="210" r="1.8" /></g>
    <g fill="var(--cloud)" opacity=".75"><ellipse cx="760" cy="150" rx="64" ry="12" /><ellipse cx="792" cy="139" rx="38" ry="12" /><ellipse cx="1470" cy="150" rx="58" ry="11" /></g>

    <!-- chaîne lointaine -->
    <path d="M560 490V360L650 300L720 350L800 290L880 360L940 400V490zM1440 490L1520 380L1580 330L1650 390L1720 340L1780 400V490z" fill="var(--mh-far)" />

    <!-- le Cervin -->
    <path d="M700 490L860 380L960 330L1040 280L1090 214L1130 162L1160 120L1174 98L1190 112L1210 172L1242 242L1272 302L1332 362L1424 414L1580 490z" fill="var(--mh-rock)" />
    <g clip-path="url(#mh-clip)">
      <path d="M1174 98L1190 112L1210 172L1242 242L1272 302L1332 362L1424 414L1580 490H1250L1214 360L1190 230z" fill="var(--mh-rock2)" />
      <path d="M1000 90H1300V270L1262 240L1240 282L1214 226L1190 272L1166 218L1140 262L1116 214L1090 250L1062 206L1030 240z" fill="var(--mh-snow)" />
      <path d="M1174 90H1300V270L1262 240L1240 282L1214 226L1190 272L1186 200z" fill="var(--mh-snow2)" />
      <path d="M1100 300L1150 262L1196 326M1010 350L1070 316M1300 400L1262 350M1180 400L1220 340" stroke="var(--mh-rock2)" stroke-width="3" stroke-linecap="round" opacity=".7" />
    </g>
    <!-- panache de nuage -->
    <g fill="var(--cloud)" opacity=".8"><ellipse cx="1240" cy="100" rx="70" ry="9" /><ellipse cx="1300" cy="90" rx="46" ry="7" /></g>

    <!-- prairie et voie du train -->
    <path d="M560 470C700 452 900 470 1100 462S1560 450 1780 466V620H560z" fill="var(--mh-meadow)" />
    <path d="M560 500L1780 478" stroke="var(--mh-rail)" stroke-width="3" />
    <path d="M560 504L1780 482" stroke="var(--mh-rail)" stroke-width="1.5" stroke-dasharray="3 6" />
    <g transform="translate(1000 490) rotate(-1)">
      <rect x="-70" y="-24" width="62" height="22" rx="3" fill="var(--mh-train)" /><rect x="-4" y="-24" width="62" height="22" rx="3" fill="var(--mh-train)" />
      <path d="M-62-18h46M2-18h46" stroke="var(--mh-glass)" stroke-width="7" /><path d="M-70-8h128" stroke="#fbf7ee" stroke-width="2" />
    </g>

    <!-- pins -->
    <g fill="var(--mh-pine)">
      <path d="M640 520L656 472L672 520zM664 524L682 478L700 524zM1480 520L1496 466L1512 520zM1504 524L1520 482L1538 524zM1706 520L1724 468L1742 520z" />
      <path d="M1056 520L1072 476L1088 520z" />
    </g>

    <!-- village : chalets et clocher -->
    <g transform="translate(0 540)">
      <use href="#mh-chalet" x="600" /><use href="#mh-chalet" x="690" /><use href="#mh-chalet" x="1130" /><use href="#mh-chalet" x="1220" />
      <use href="#mh-chalet" x="1330" /><use href="#mh-chalet" x="1420" /><use href="#mh-chalet" x="1620" />
      <g transform="translate(900 0)">
        <rect x="0" y="-50" width="64" height="50" fill="var(--mh-stone)" />
        <rect x="32" y="-92" width="34" height="92" fill="var(--mh-stone)" />
        <path d="M30-92L49-150L68-92z" fill="var(--mh-spire)" />
        <circle cx="49" cy="-76" r="7" fill="var(--mh-win)" /><path d="M42-40h12v40H42z" fill="var(--mh-wood2)" />
      </g>
    </g>
    <rect x="560" y="540" width="1220" height="80" fill="var(--mh-ground)" />

    <!-- mascottes : sur un toit et dans la rue -->
    <use href="#dm-mascot" transform="translate(1448 462) scale(.1)" />
    <use href="#dm-mascot" transform="translate(1000 540) scale(.1)" />
  </svg>
</div>

<style>
  .scene {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: linear-gradient(180deg, var(--sky1), var(--sky2) 70%, var(--mh-meadow) 72%, var(--mh-ground));
  }
  .scene svg {
    display: block;
    width: 100%;
    height: 100%;
    -webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
    mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
  }
  .scene-matterhorn {
    --sky1: #a9cdee; --sky2: #eef3f6; --sun: #fff0b8; --cloud: #fff; --crater: transparent; --star: transparent;
    --mh-far: #b9c8dd; --mh-rock: #8d93a6; --mh-rock2: #646b84; --mh-snow: #fff; --mh-snow2: #d3deee;
    --mh-meadow: #8fba7a; --mh-ground: #6f9a63; --mh-pine: #2f5d47; --mh-rail: #5d5a58;
    --mh-train: #c8302a; --mh-glass: #cfe3f2;
    --mh-wood: #a8774f; --mh-wood2: #7f573a; --mh-roof: #6a5546; --mh-win: #f4dea0; --mh-stone: #d9d2c4; --mh-spire: #7c8696;
  }
  :global(.dark) .scene-matterhorn {
    --sky1: #0a1226; --sky2: #1d2b4f; --sun: #e8eef8; --cloud: #2a3658; --crater: #b9c4dc; --star: #e8eef8;
    --mh-far: #1c2848; --mh-rock: #333c58; --mh-rock2: #232b45; --mh-snow: #8e9cc0; --mh-snow2: #5c6a90;
    --mh-meadow: #1f3a36; --mh-ground: #162b2c; --mh-pine: #10282a; --mh-rail: #3c4560;
    --mh-train: #d0504a; --mh-glass: #f5c76a;
    --mh-wood: #4a3a3a; --mh-wood2: #382c30; --mh-roof: #2c3350; --mh-win: #f5c76a; --mh-stone: #5a6380; --mh-spire: #3b4766;
  }
</style>
