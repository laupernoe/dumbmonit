<div class="scene scene-newyork" aria-hidden="true">
  <svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none">
    <defs>
      <!-- fenêtres des tours (cellule 12 x 10) et des immeubles de brique (16 x 18) -->
      <pattern id="ny-win" width="12" height="10" patternUnits="userSpaceOnUse"><rect x="3" y="2" width="6" height="6" fill="var(--ny-glass)" /></pattern>
      <pattern id="ny-bwin" width="16" height="18" patternUnits="userSpaceOnUse"><rect x="4" y="4" width="8" height="11" rx="1" fill="var(--ny-bwin)" /></pattern>
      <clipPath id="ny-esb">
        <path d="M-80 0V-86H-58V-168H-42V-230H-30V-276H-20V-292H-14V-308H-8V-322H-2V-380H2V-322H8V-308H14V-292H20V-276H30V-230H42V-168H58V-86H80V0Z" />
      </clipPath>
      <g id="ny-fire"><path d="M0-70h24M0-44h24M0-18h24M2-70L22-44M22-44L2-18" stroke="var(--ny-brick2)" stroke-width="2.4" /></g>
    </defs>

    <!-- astre bas sur l'horizon et nuages -->
    <circle cx="900" cy="346" r="58" fill="var(--sun)" />
    <circle cx="884" cy="330" r="7" fill="var(--crater)" /><circle cx="916" cy="360" r="5" fill="var(--crater)" />
    <g fill="var(--star)"><circle cx="1010" cy="120" r="2" /><circle cx="1300" cy="104" r="2.2" /><circle cx="1560" cy="120" r="2" /><circle cx="1660" cy="210" r="2.2" /><circle cx="1130" cy="190" r="1.8" /><circle cx="1500" cy="170" r="1.8" /></g>
    <g fill="var(--cloud)" opacity=".7"><ellipse cx="1530" cy="150" rx="80" ry="10" /><ellipse cx="1580" cy="170" rx="54" ry="8" /><ellipse cx="1000" cy="150" rx="50" ry="8" /></g>

    <!-- rive lointaine et pont de Brooklyn -->
    <g fill="var(--ny-far)"><path d="M560 482V450h40v-18h30v26h44v-14h36v22h50v-30h28v30h60v-12h44v24h70v-16h60z" /><rect x="1060" y="372" width="60" height="110" /><rect x="1400" y="346" width="40" height="136" /><rect x="1458" y="396" width="90" height="86" /></g>
    <g stroke="var(--ny-bridge)" stroke-width="1.6" fill="none">
      <path d="M760 360L690 450M760 360L718 450M760 360L746 450M760 360L776 450M760 360L804 450M760 360L832 450M960 360L890 450M960 360L918 450M960 360L946 450M960 360L976 450M960 360L1004 450M960 360L1032 450" />
      <path d="M640 456Q700 440 760 358Q860 430 960 358Q1020 440 1080 456" stroke-width="3" />
    </g>
    <g fill="var(--ny-bridge)">
      <rect x="560" y="450" width="520" height="7" />
      <path d="M744 462V352q16-16 32 0V462zM944 462V352q16-16 32 0V462z" />
    </g>
    <g fill="var(--sky2)" opacity=".8"><path d="M752 462v-34q8-16 16 0v34zM952 462v-34q8-16 16 0v34z" /></g>

    <!-- gratte-ciel : second plan bleu-gris -->
    <g fill="var(--ny-blue2)"><rect x="1100" y="292" width="56" height="190" /><rect x="1296" y="316" width="54" height="166" /><rect x="1480" y="326" width="60" height="156" /><rect x="1556" y="286" width="48" height="196" /></g>
    <g fill="url(#ny-win)"><rect x="1100" y="292" width="56" height="190" /><rect x="1296" y="316" width="54" height="166" /><rect x="1480" y="326" width="60" height="156" /><rect x="1556" y="286" width="48" height="196" /></g>

    <!-- Chrysler Building -->
    <g transform="translate(1420 482)">
      <path d="M-34 0V-150H-26V-232C-26-270-10-288 0-300C10-288 26-270 26-232V-150H34V0Z" fill="var(--ny-blue1)" />
      <path d="M-26-232C-26-270-10-288 0-300C10-288 26-270 26-232z" fill="var(--ny-silver)" />
      <path d="M-18-236C-18-262-6-280 0-290M18-236C18-262 6-280 0-290M-9-238C-9-256-3-270 0-278M9-238C9-256 3-270 0-278" stroke="var(--ny-blue1)" stroke-width="2.4" />
      <path d="M0-300V-340" stroke="var(--ny-silver)" stroke-width="3" />
      <rect x="-26" y="-232" width="52" height="82" fill="url(#ny-win)" /><rect x="-34" y="-150" width="68" height="150" fill="url(#ny-win)" />
    </g>

    <!-- Empire State Building -->
    <g transform="translate(1210 482)">
      <path d="M-80 0V-86H-58V-168H-42V-230H-30V-276H-20V-292H-14V-308H-8V-322H-2V-380H2V-322H8V-308H14V-292H20V-276H30V-230H42V-168H58V-86H80V0Z" fill="var(--ny-stone)" />
      <g clip-path="url(#ny-esb)">
        <rect x="-80" y="-380" width="160" height="380" fill="url(#ny-win)" opacity=".75" />
        <rect x="0" y="-380" width="80" height="380" fill="var(--ny-stone2)" opacity=".55" />
        <path d="M-80-86H80M-58-168H58M-42-230H42M-30-276H30" stroke="var(--ny-stone)" stroke-width="3" />
      </g>
      <path d="M-14-292H14M-8-308H8" stroke="var(--ny-silver)" stroke-width="3" /><circle cy="-336" r="3" fill="var(--ny-beacon)" />
    </g>

    <!-- immeubles de brique, escaliers de secours et château d'eau -->
    <g fill="var(--ny-brick)">
      <rect x="1020" y="372" width="70" height="110" /><rect x="1096" y="390" width="60" height="92" /><rect x="1300" y="356" width="64" height="126" />
      <rect x="1368" y="388" width="56" height="94" /><rect x="1428" y="368" width="70" height="114" /><rect x="1502" y="392" width="60" height="90" /><rect x="1566" y="364" width="70" height="118" />
    </g>
    <g fill="var(--ny-brick2)"><rect x="1016" y="366" width="78" height="8" /><rect x="1092" y="384" width="68" height="8" /><rect x="1296" y="350" width="72" height="8" /><rect x="1424" y="362" width="78" height="8" /><rect x="1562" y="358" width="78" height="8" /><rect x="1364" y="382" width="64" height="8" /><rect x="1498" y="386" width="68" height="8" /></g>
    <g fill="url(#ny-bwin)"><rect x="1020" y="376" width="70" height="106" /><rect x="1096" y="394" width="60" height="88" /><rect x="1300" y="360" width="64" height="122" /><rect x="1368" y="392" width="56" height="90" /><rect x="1428" y="372" width="70" height="110" /><rect x="1502" y="396" width="60" height="86" /><rect x="1566" y="368" width="70" height="114" /></g>
    <use href="#ny-fire" x="1062" y="478" /><use href="#ny-fire" x="1466" y="478" /><use href="#ny-fire" x="1602" y="478" />
    <g transform="translate(1126 384)">
      <path d="M-14 0V-14M14 0V-14M-14 -2L14 -14M14 -2L-14 -14" stroke="var(--ny-iron)" stroke-width="2" />
      <rect x="-12" y="-40" width="24" height="26" fill="var(--ny-wood)" /><path d="M-12-30h24M-12-22h24" stroke="var(--ny-iron)" stroke-width="2" /><path d="M-15-40L0-54L15-40z" fill="var(--ny-brick2)" />
    </g>

    <!-- quai, ponton, arbres et rides -->
    <rect x="540" y="482" width="1260" height="6" fill="var(--ny-far)" opacity=".8" /><rect x="1000" y="482" width="800" height="8" fill="var(--ny-deck)" />
    <rect x="900" y="482" width="104" height="6" fill="var(--ny-wood)" /><path d="M910 488v14M950 488v14M994 488v14" stroke="var(--ny-wood)" stroke-width="5" />
    <g fill="var(--ny-tree)"><circle cx="1166" cy="468" r="14" /><circle cx="1262" cy="470" r="12" /><circle cx="1556" cy="470" r="13" /></g>
    <g stroke="var(--ny-ripple)" stroke-width="2" stroke-linecap="round" opacity=".55"><path d="M600 516h90M1100 520h110M1360 512h80M1500 548h110M700 560h100M1180 570h90" /></g>

    <!-- remorqueur -->
    <g transform="translate(700 500)">
      <path d="M0 0H120L106 16H12z" fill="var(--ny-brick2)" /><rect x="14" y="-10" width="52" height="10" fill="var(--ny-silver)" />
      <rect x="24" y="-26" width="32" height="16" fill="var(--ny-silver)" /><rect x="30" y="-22" width="20" height="7" fill="var(--ny-glass)" />
      <rect x="76" y="-24" width="12" height="24" fill="var(--ny-iron)" /><rect x="76" y="-18" width="12" height="5" fill="var(--ny-brick)" />
      <g fill="var(--ny-silver)" opacity=".7"><circle cx="86" cy="-34" r="7" /><circle cx="97" cy="-44" r="9" /></g>
    </g>

    <!-- mascottes : sur un toit de brique et sur le ponton -->
    <use href="#dm-mascot" transform="translate(1324 320) scale(.1)" />
    <use href="#dm-mascot" transform="translate(946 452) scale(.1)" />
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
  .scene-newyork {
    --sky1: #adc0dc; --sky2: #f4e0cd; --sun: #ffd7a2; --water1: #94abc3; --water2: #647f9f; --cloud: #fff;
    --crater: transparent; --star: transparent;
    --ny-far: #c0cadb; --ny-bridge: #a79fa8; --ny-stone: #d6cfc8; --ny-stone2: #aeb0bc; --ny-glass: #7683a3;
    --ny-blue1: #8c9cb8; --ny-blue2: #9aa9c2; --ny-silver: #e4e9f1; --ny-beacon: #c8402f;
    --ny-brick: #b45a40; --ny-brick2: #93442f; --ny-bwin: #4b4458; --ny-iron: #3b3f55; --ny-wood: #8a5a3c;
    --ny-deck: #9a8a7c; --ny-tree: #7aa183; --ny-ripple: #fff;
  }
  :global(.dark) .scene-newyork {
    --sky1: #0b1229; --sky2: #232a4e; --sun: #e8eef8; --water1: #16294a; --water2: #0a1428; --cloud: #2a3658;
    --crater: #b9c4dc; --star: #e8eef8;
    --ny-far: #1d2848; --ny-bridge: #2e3a60; --ny-stone: #5d6783; --ny-stone2: #333b58; --ny-glass: #f5c76a;
    --ny-blue1: #3b4666; --ny-blue2: #2f3957; --ny-silver: #aeb8d2; --ny-beacon: #ff6b5a;
    --ny-brick: #6b3a44; --ny-brick2: #522c37; --ny-bwin: #f5c76a; --ny-iron: #0d1222; --ny-wood: #5a4048;
    --ny-deck: #3a4260; --ny-tree: #1f4a4f; --ny-ripple: #5f7aa8;
  }
</style>
