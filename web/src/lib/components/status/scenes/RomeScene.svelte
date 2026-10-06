<div class="scene scene-rome" aria-hidden="true">
  <svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none">
    <defs>
      <!-- arcades du Colisée : 3 niveaux de 56, alignés sur y=314 ; attique en dessous du bandeau -->
      <pattern id="rome-arch" x="960" y="314" width="32" height="56" patternUnits="userSpaceOnUse">
        <path d="M7 56V22a9 9 0 0 1 18 0V56z" fill="var(--rome-arch)" /><rect x="14" y="10" width="4" height="5" fill="var(--rome-stone3)" />
      </pattern>
      <pattern id="rome-attic" x="960" y="292" width="32" height="22" patternUnits="userSpaceOnUse"><rect x="12" y="6" width="8" height="11" fill="var(--rome-arch)" /></pattern>
      <pattern id="rome-pal" width="22" height="26" patternUnits="userSpaceOnUse"><rect x="7" y="6" width="8" height="13" rx="1" fill="var(--rome-win)" /></pattern>
      <clipPath id="rome-inner"><path d="M1250 482V318H1330V336H1380V360H1440V482z" /></clipPath>
      <clipPath id="rome-shell"><path d="M960 482V300H1280L1300 312L1316 330L1344 338L1362 372L1396 380L1440 416V482z" /></clipPath>
      <!-- pin parasol : origine = pied -->
      <g id="rome-pine">
        <path d="M0 0C3-40-5-80 0-120" stroke="var(--rome-trunk)" stroke-width="5" stroke-linecap="round" />
        <ellipse cy="-128" rx="46" ry="13" fill="var(--rome-pine)" /><ellipse cx="-24" cy="-116" rx="26" ry="9" fill="var(--rome-pine)" /><ellipse cx="26" cy="-118" rx="24" ry="9" fill="var(--rome-pine)" />
      </g>
    </defs>

    <!-- astre, étoiles, nuage -->
    <circle cx="1400" cy="196" r="50" fill="var(--sun)" />
    <circle cx="1384" cy="184" r="7" fill="var(--crater)" /><circle cx="1414" cy="210" r="5" fill="var(--crater)" />
    <g fill="var(--star)"><circle cx="1010" cy="118" r="2" /><circle cx="1250" cy="104" r="2.2" /><circle cx="1560" cy="120" r="2" /><circle cx="1640" cy="230" r="2.2" /><circle cx="1180" cy="190" r="1.8" /><circle cx="1500" cy="250" r="1.8" /></g>
    <g fill="var(--cloud)" opacity=".75"><ellipse cx="1590" cy="146" rx="80" ry="10" /><ellipse cx="1640" cy="164" rx="50" ry="8" /><ellipse cx="1190" cy="150" rx="56" ry="8" /></g>

    <!-- second plan : toits ocre et basilique Saint-Pierre -->
    <path d="M560 482V440h40v-14h30v10h40v-20h34v24h50v-12h40z" fill="var(--rome-far)" />
    <path d="M1440 482V430h50v-16h36v22h50v-14h60v18h60v-10h60v32z" fill="var(--rome-far)" />
    <g fill="var(--rome-peter)">
      <rect x="680" y="410" width="120" height="72" /><rect x="688" y="398" width="104" height="14" /><rect x="716" y="350" width="48" height="50" />
      <path d="M716 350C716 306 728 288 740 280C752 288 764 306 764 350z" /><rect x="736" y="262" width="8" height="20" /><path d="M736 264L740 254L744 264z" />
      <path d="M690 398C690 378 704 372 712 372C720 372 726 378 728 398zM752 398C754 378 760 372 768 372C776 372 790 378 790 398z" />
      <rect x="739" y="238" width="2" height="18" fill="var(--rome-gold)" /><rect x="735" y="244" width="10" height="2" fill="var(--rome-gold)" />
    </g>
    <path d="M696 482V424M716 482V424M736 482V424M756 482V424M776 482V424" stroke="var(--rome-stone2)" stroke-width="5" opacity=".6" />

    <!-- palazzi, campanile -->
    <g>
      <rect x="610" y="394" width="92" height="88" fill="var(--rome-wall)" /><rect x="610" y="394" width="92" height="88" fill="url(#rome-pal)" /><path d="M604 394L616 378H696L708 394z" fill="var(--rome-roof)" />
      <rect x="806" y="400" width="104" height="82" fill="var(--rome-wall2)" /><rect x="806" y="400" width="104" height="82" fill="url(#rome-pal)" /><path d="M800 400L814 384H902L916 400z" fill="var(--rome-roof)" />
      <rect x="1456" y="404" width="90" height="78" fill="var(--rome-wall)" /><rect x="1456" y="404" width="90" height="78" fill="url(#rome-pal)" /><path d="M1450 404L1464 388H1538L1552 404z" fill="var(--rome-roof)" />
      <rect x="924" y="346" width="30" height="136" fill="var(--rome-wall)" /><path d="M932 366v-14a7 7 0 0 1 14 0v14z" fill="var(--rome-arch)" /><path d="M920 346L939 318L958 346z" fill="var(--rome-roof)" />
    </g>

    <!-- Colisée : coquille extérieure, partie intérieure à nu, brèche et éboulis -->
    <g clip-path="url(#rome-inner)">
      <rect x="1250" y="318" width="190" height="164" fill="var(--rome-stone3)" />
      <rect x="1250" y="318" width="190" height="164" fill="url(#rome-arch)" opacity=".75" />
    </g>
    <g clip-path="url(#rome-shell)">
      <rect x="960" y="292" width="480" height="190" fill="var(--rome-stone)" />
      <rect x="960" y="314" width="480" height="168" fill="url(#rome-arch)" /><rect x="960" y="292" width="480" height="22" fill="url(#rome-attic)" />
      <g fill="var(--rome-stone2)"><rect x="960" y="312" width="480" height="5" /><rect x="960" y="368" width="480" height="5" /><rect x="960" y="424" width="480" height="5" /><rect x="960" y="292" width="480" height="5" /><rect x="960" y="476" width="480" height="6" /></g>
    </g>
    <g fill="var(--rome-stone2)"><ellipse cx="1412" cy="476" rx="22" ry="8" /><ellipse cx="1380" cy="480" rx="14" ry="6" /><ellipse cx="1340" cy="478" rx="10" ry="5" /></g>

    <!-- pins, ruines (colonnes et entablement) et fontaine -->
    <use href="#rome-pine" x="650" y="482" /><use href="#rome-pine" x="1486" y="482" /><use href="#rome-pine" x="1660" y="482" />
    <g transform="translate(0 482)" fill="var(--rome-stone)">
      <path d="M826-128H922L918-120L910-114L902-112H826z" fill="var(--rome-stone2)" /><rect x="830" y="-116" width="80" height="6" fill="var(--rome-stone2)" />
      <g><rect x="835" y="-110" width="14" height="104" /><rect x="863" y="-110" width="14" height="104" /><rect x="891" y="-110" width="14" height="104" /></g>
      <g fill="var(--rome-stone2)"><rect x="831" y="-114" width="22" height="8" rx="2" /><rect x="859" y="-114" width="22" height="8" rx="2" /><rect x="887" y="-114" width="22" height="8" rx="2" /><rect x="831" y="-8" width="22" height="8" /><rect x="859" y="-8" width="22" height="8" /><rect x="887" y="-8" width="22" height="8" /></g>
      <path d="M842-100V-8M870-100V-8M898-100V-8" stroke="var(--rome-stone2)" stroke-width="2" opacity=".7" />
      <rect x="930" y="-16" width="38" height="16" rx="4" /><rect x="976" y="-12" width="26" height="12" rx="3" fill="var(--rome-stone2)" />
    </g>
    <g transform="translate(1570 482)">
      <path d="M-76-16H76Q76 0 56 0H-56Q-76 0-76-16z" fill="var(--rome-stone)" /><ellipse cy="-16" rx="76" ry="9" fill="var(--rome-stone2)" /><ellipse cy="-16" rx="68" ry="6" fill="var(--rome-water)" />
      <rect x="-8" y="-62" width="16" height="48" fill="var(--rome-stone)" /><path d="M-34-62Q0-30 34-62z" fill="var(--rome-stone)" /><ellipse cy="-62" rx="34" ry="5" fill="var(--rome-water)" />
      <path d="M0-66V-98M0-66Q-18-100-40-66M0-66Q18-100 40-66" stroke="var(--rome-water)" stroke-width="3" stroke-linecap="round" />
    </g>

    <!-- sol de la piazza : bordure et pavés -->
    <rect x="540" y="482" width="1260" height="7" fill="var(--rome-stone2)" />
    <g stroke="var(--rome-pave)" stroke-width="2" stroke-linecap="round" opacity=".55"><path d="M600 512h100M780 540h120M1040 506h130M1260 534h100M1480 512h110M700 576h90M1160 570h130M1400 560h80" /></g>

    <!-- mascottes : sur les ruines et au pied du Colisée -->
    <use href="#dm-mascot" transform="translate(842 296) scale(.2)" />
    <use href="#dm-mascot" transform="translate(1440 422) scale(.2)" />
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
  .scene-rome {
    --sky1: #f1c79a; --sky2: #fbe9cb; --sun: #fff2c6; --water1: #dcb98b; --water2: #c4965f; --cloud: #fff;
    --crater: transparent; --star: transparent;
    --rome-stone: #ecd6aa; --rome-stone2: #d3b886; --rome-stone3: #bc9a68; --rome-arch: #8a6647; --rome-far: #ebd2a8; --rome-peter: #e8d9ba;
    --rome-wall: #e7b98a; --rome-wall2: #efcca0; --rome-roof: #c6724b; --rome-win: #8a6647; --rome-gold: #d9a63a;
    --rome-pine: #6f905b; --rome-trunk: #86684a; --rome-water: #a9d0d6; --rome-pave: #fff;
  }
  :global(.dark) .scene-rome {
    --sky1: #0d1229; --sky2: #2a2142; --sun: #e8eef8; --water1: #26213a; --water2: #120f20; --cloud: #2a3658;
    --crater: #b9c4dc; --star: #e8eef8;
    --rome-stone: #7e7791; --rome-stone2: #605a75; --rome-stone3: #453f5a; --rome-arch: #d9a94d; --rome-far: #2a2745; --rome-peter: #3a3a5e;
    --rome-wall: #655a74; --rome-wall2: #70647d; --rome-roof: #4a3a53; --rome-win: #f5c76a; --rome-gold: #e8bd4f;
    --rome-pine: #1f4a4f; --rome-trunk: #3f3646; --rome-water: #7a9cc8; --rome-pave: #5f5a88;
  }
</style>
