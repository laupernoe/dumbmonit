<div class="scene scene-tokyo" aria-hidden="true">
  <svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none">
    <defs>
      <pattern id="tokyo-win" width="12" height="14" patternUnits="userSpaceOnUse">
        <rect x="4" y="4" width="5" height="7" rx="1" fill="var(--tokyo-win)" />
      </pattern>
      <linearGradient id="tokyo-fuji" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="var(--tokyo-fuji)" /><stop offset="1" stop-color="var(--tokyo-haze)" />
      </linearGradient>
      <clipPath id="tokyo-tt">
        <path d="M-66 0H-46C-46-26-22-42 0-42C22-42 46-26 46 0H66C54-50 42-95 34-125H24L13-245H10L5-300L2.5-380H-2.5L-5-300L-10-245H-13L-24-125H-34C-42-95-54-50-66 0Z" />
      </clipPath>
    </defs>

    <!-- soleil levant, reflet et étoiles -->
    <circle cx="950" cy="388" r="54" fill="var(--sun)" />
    <circle cx="936" cy="372" r="6" fill="var(--crater)" /><circle cx="962" cy="396" r="4" fill="var(--crater)" />
    <g fill="var(--sun)" opacity=".55"><rect x="920" y="496" width="60" height="6" rx="3" /><rect x="930" y="512" width="40" height="5" rx="2.5" /><rect x="938" y="526" width="24" height="4" rx="2" /></g>
    <g fill="var(--star)"><circle cx="1040" cy="116" r="2" /><circle cx="1260" cy="104" r="2.2" /><circle cx="1560" cy="120" r="2" /><circle cx="1680" cy="190" r="2.2" /><circle cx="1180" cy="170" r="1.8" /><circle cx="1620" cy="250" r="1.8" /></g>
    <g fill="var(--cloud)" opacity=".7"><ellipse cx="1500" cy="150" rx="90" ry="9" /><ellipse cx="1560" cy="172" rx="60" ry="7" /><ellipse cx="1220" cy="330" rx="80" ry="6" /></g>

    <!-- mont Fuji enneigé -->
    <path d="M1000 482C1160 430 1290 316 1380 246L1396 232H1446L1462 246C1550 316 1680 430 1820 482z" fill="url(#tokyo-fuji)" />
    <path d="M1380 246L1396 232H1446L1462 246C1480 262 1494 280 1508 298L1486 308L1470 292L1446 322L1426 294L1406 326L1386 296L1364 316L1350 298C1362 280 1370 262 1380 246z" fill="var(--tokyo-snow)" />

    <!-- second plan : ville dans la brume -->
    <g fill="var(--tokyo-far)">
      <path d="M580 482V436h30v-26h34v40h30v-60h28v50h44v-30h30v46h40v-20h40v40h64v-14h40v-40h30v54h50v-30h44v-50h30v60h54v-28h40v-70h26v90h44v-46h36v70h50v-24h50v-30h40v54h60v-36h50v52h60z" />
    </g>

    <!-- Tokyo Tower -->
    <g transform="translate(1130 482) scale(.93)">
      <path d="M-66 0H-46C-46-26-22-42 0-42C22-42 46-26 46 0H66C54-50 42-95 34-125H24L13-245H10L5-300L2.5-380H-2.5L-5-300L-10-245H-13L-24-125H-34C-42-95-54-50-66 0Z" fill="var(--tokyo-red)" />
      <g clip-path="url(#tokyo-tt)">
        <g fill="var(--tokyo-white)"><rect x="-80" y="-76" width="160" height="20" /><rect x="-80" y="-116" width="160" height="16" /><rect x="-30" y="-190" width="60" height="22" /><rect x="-30" y="-232" width="60" height="16" /><rect x="-30" y="-312" width="60" height="22" /><rect x="-30" y="-352" width="60" height="14" /></g>
        <path d="M-70 0L-40-40M-40 0L-70-40M70 0L40-40M40 0L70-40M-50-56L50-100M50-56L-50-100M-40-125L40-170M40-125L-40-170M-30-170L30-232M30-170L-30-232" stroke="var(--tokyo-red2)" stroke-width="2.4" />
      </g>
      <g fill="var(--tokyo-red)"><rect x="-42" y="-152" width="84" height="26" rx="4" /><rect x="-24" y="-272" width="48" height="28" rx="4" /></g>
      <g fill="var(--tokyo-glass)"><rect x="-34" y="-144" width="68" height="9" rx="2" /><rect x="-17" y="-264" width="34" height="9" rx="2" /></g>
      <path d="M0-380V-396" stroke="var(--tokyo-red)" stroke-width="3" /><circle cy="-398" r="4" fill="var(--tokyo-glass)" />
    </g>

    <!-- immeubles : gauche, puis droite -->
    <g fill="var(--tokyo-b2)">
      <rect x="610" y="372" width="56" height="110" /><rect x="726" y="386" width="64" height="96" /><rect x="850" y="400" width="44" height="82" />
      <rect x="1006" y="384" width="40" height="98" /><rect x="1296" y="362" width="50" height="120" /><rect x="1486" y="402" width="60" height="80" />
    </g>
    <g fill="var(--tokyo-b1)">
      <rect x="672" y="332" width="48" height="150" /><rect x="796" y="344" width="50" height="138" />
      <rect x="1230" y="302" width="60" height="180" /><rect x="1352" y="332" width="70" height="150" /><rect x="1428" y="392" width="52" height="90" /><rect x="1552" y="382" width="48" height="100" />
    </g>
    <g fill="var(--tokyo-b3)"><rect x="1612" y="412" width="56" height="70" /><rect x="1676" y="392" width="50" height="90" /></g>
    <g fill="url(#tokyo-win)">
      <rect x="610" y="372" width="56" height="110" /><rect x="672" y="332" width="48" height="150" /><rect x="726" y="386" width="64" height="96" /><rect x="796" y="344" width="50" height="138" /><rect x="850" y="400" width="44" height="82" />
      <rect x="1006" y="384" width="40" height="98" /><rect x="1230" y="302" width="60" height="180" /><rect x="1296" y="362" width="50" height="120" /><rect x="1352" y="332" width="70" height="150" /><rect x="1428" y="392" width="52" height="90" />
      <rect x="1486" y="402" width="60" height="80" /><rect x="1552" y="382" width="48" height="100" /><rect x="1612" y="412" width="56" height="70" /><rect x="1676" y="392" width="50" height="90" />
    </g>
    <!-- toits : antennes et réservoirs -->
    <g stroke="var(--tokyo-b1)" stroke-width="3"><path d="M696 332v-22M1387 332v-30M1575 382v-16M820 344v-14M1261 302v-16" /></g>
    <g fill="var(--tokyo-b2)"><rect x="740" y="374" width="16" height="12" /><rect x="1500" y="390" width="18" height="12" /><rect x="1302" y="352" width="14" height="10" /></g>
    <g fill="var(--tokyo-red)"><circle cx="1387" cy="300" r="3.5" /><circle cx="696" cy="308" r="3" /></g>

    <!-- berge : cerisiers, torii, quai -->
    <rect x="540" y="482" width="1260" height="8" fill="var(--tokyo-quay)" />
    <g>
      <path d="M1052 482V446M1052 460l-12-12M1052 454l12-10" stroke="var(--tokyo-trunk)" stroke-width="5" stroke-linecap="round" />
      <g fill="var(--tokyo-sakura)"><circle cx="1052" cy="428" r="26" /><circle cx="1030" cy="442" r="18" /><circle cx="1076" cy="440" r="19" /></g>
      <path d="M1636 482V446" stroke="var(--tokyo-trunk)" stroke-width="5" stroke-linecap="round" />
      <g fill="var(--tokyo-sakura)"><circle cx="1636" cy="430" r="24" /><circle cx="1616" cy="444" r="16" /><circle cx="1656" cy="442" r="17" /></g>
    </g>
    <g transform="translate(1334 482)" fill="var(--tokyo-torii)">
      <rect x="-30" y="-80" width="8" height="80" /><rect x="22" y="-80" width="8" height="80" />
      <path d="M-46-86C-30-80 30-80 46-86L50-96C32-88-32-88-50-96z" /><rect x="-26" y="-64" width="52" height="7" /><rect x="-3" y="-82" width="6" height="18" />
    </g>
    <g stroke="var(--tokyo-ripple)" stroke-width="2" stroke-linecap="round" opacity=".6"><path d="M620 514h80M720 548h100M1100 506h110M1280 530h90M1460 504h70M1500 560h100M660 580h80" /></g>

    <!-- yakatabune sur la rivière -->
    <g transform="translate(1160 500)">
      <path d="M0 0H130L116 16H14z" fill="var(--tokyo-trunk)" /><rect x="22" y="-24" width="86" height="24" fill="var(--tokyo-boat)" />
      <path d="M14-24L28-38H102L116-24z" fill="var(--tokyo-torii)" /><rect x="34" y="-18" width="64" height="12" fill="var(--tokyo-glass)" />
      <path d="M14-24H116" stroke="var(--tokyo-trunk)" stroke-width="3" />
      <g fill="var(--tokyo-glass)"><circle cx="30" cy="-18" r="4" /><circle cx="100" cy="-18" r="4" /></g>
    </g>

    <!-- mascottes : sur un toit et au pied du torii -->
    <use href="#dm-mascot" transform="translate(730 326) scale(.2)" />
    <use href="#dm-mascot" transform="translate(1262 422) scale(.2)" />
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
  .scene-tokyo {
    --sky1: #f4b6c4; --sky2: #fde5d4; --sun: #fff1cc; --water1: #bba9cc; --water2: #8f86b3; --cloud: #fff;
    --crater: transparent; --star: transparent;
    --tokyo-fuji: #a79ac6; --tokyo-haze: #e6cfdc; --tokyo-snow: #fff6f2; --tokyo-far: #e0c4d4;
    --tokyo-red: #e5533d; --tokyo-red2: #b93d2e; --tokyo-white: #fff4ea; --tokyo-glass: #ffd9a0;
    --tokyo-b1: #b5a0c6; --tokyo-b2: #9f89b5; --tokyo-b3: #d9a7b8; --tokyo-win: #f8e6ea;
    --tokyo-quay: #c9b2c2; --tokyo-trunk: #7a5d63; --tokyo-sakura: #f29bb5; --tokyo-torii: #d9472f;
    --tokyo-boat: #f4e6d8; --tokyo-ripple: #fff;
  }
  :global(.dark) .scene-tokyo {
    --sky1: #0c1230; --sky2: #261f4d; --sun: #e8eef8; --water1: #171c43; --water2: #0a1028; --cloud: #2a3358;
    --crater: #b9c4dc; --star: #e8eef8;
    --tokyo-fuji: #2c3560; --tokyo-haze: #2a2a52; --tokyo-snow: #8d94b8; --tokyo-far: #232a52;
    --tokyo-red: #f0864a; --tokyo-red2: #b95a32; --tokyo-white: #ffe3b8; --tokyo-glass: #f5c76a;
    --tokyo-b1: #3b4068; --tokyo-b2: #2d3257; --tokyo-b3: #4a4068; --tokyo-win: #f5c76a;
    --tokyo-quay: #3b3e62; --tokyo-trunk: #4a3a4e; --tokyo-sakura: #8a5c86; --tokyo-torii: #d0583f;
    --tokyo-boat: #6b6f90; --tokyo-ripple: #6a74a8;
  }
</style>
