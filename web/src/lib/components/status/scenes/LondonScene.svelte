<div class="scene scene-london" aria-hidden="true">
  <svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none">
    <defs>
      <!-- fenêtres gothiques (16 x 28) et pinacles (32 x 16) du palais -->
      <pattern id="lon-win" width="16" height="28" patternUnits="userSpaceOnUse"><path d="M5 25V10Q8 3 11 10V25z" fill="var(--lon-win)" /></pattern>
      <pattern id="lon-pin" width="32" height="18" patternUnits="userSpaceOnUse"><path d="M13 18V9L16 0L19 9V18z" fill="var(--lon-stone2)" /></pattern>
      <linearGradient id="lon-mist" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--sky2)" stop-opacity="0" /><stop offset="1" stop-color="var(--sky2)" stop-opacity=".5" /></linearGradient>
      <g id="lon-spokes"><path d="M-104 0H104M0-104V104M-74-74L74 74M74-74L-74 74" /></g>
      <!-- tour de Tower Bridge : origine = pied, centre -->
      <g id="lon-tower">
        <rect x="-22" y="-130" width="44" height="130" fill="var(--lon-bridge)" />
        <rect x="-26" y="-150" width="52" height="22" fill="var(--lon-bridge)" />
        <path d="M-30-150L0-196L30-150z" fill="var(--lon-roof)" />
        <path d="M-30-150V-168L-25-178L-20-168V-150zM20-150V-168L25-178L30-168V-150z" fill="var(--lon-roof)" />
        <path d="M-14-96v-22a4 4 0 0 1 8 0v22zM6-96v-22a4 4 0 0 1 8 0v22zM-14-60v-22a4 4 0 0 1 8 0v22zM6-60v-22a4 4 0 0 1 8 0v22z" fill="var(--lon-win)" />
        <path d="M-22-20h44M-22-130h44" stroke="var(--lon-trim)" stroke-width="3" />
      </g>
    </defs>

    <!-- soleil voilé et nuages -->
    <circle cx="1046" cy="330" r="34" fill="var(--sun)" opacity=".85" />
    <g fill="var(--star)"><circle cx="1330" cy="110" r="2" /><circle cx="1560" cy="130" r="2.2" /><circle cx="1650" cy="230" r="2" /><circle cx="1260" cy="196" r="1.8" /><circle cx="1470" cy="102" r="1.8" /></g>
    <g fill="var(--cloud)" opacity=".75"><ellipse cx="1420" cy="140" rx="90" ry="14" /><ellipse cx="1470" cy="124" rx="52" ry="14" /><ellipse cx="1630" cy="190" rx="70" ry="10" /><ellipse cx="1330" cy="236" rx="60" ry="8" /></g>

    <!-- London Eye, en retrait -->
    <g transform="translate(1486 360)">
      <path d="M0 0L-34 122M0 0L34 122" stroke="var(--lon-far2)" stroke-width="5" />
      <circle r="108" stroke="var(--lon-far2)" stroke-width="4" /><circle r="100" stroke="var(--lon-far2)" stroke-width="1.5" />
      <g stroke="var(--lon-far2)" stroke-width="1.4"><use href="#lon-spokes" /><use href="#lon-spokes" transform="rotate(22.5)" /></g>
      <circle r="112" stroke="var(--lon-far3)" stroke-width="9" stroke-linecap="round" stroke-dasharray="0.1 43.98" />
      <circle r="6" fill="var(--lon-far2)" />
    </g>

    <!-- palais de Westminster -->
    <g>
      <rect x="1196" y="398" width="236" height="84" fill="var(--lon-stone)" />
      <rect x="1196" y="398" width="236" height="84" fill="url(#lon-win)" />
      <rect x="1196" y="382" width="236" height="18" fill="url(#lon-pin)" />
      <path d="M1196 398H1432V404H1196z" fill="var(--lon-stone2)" />
      <path d="M1196 398L1212 388H1416L1432 398z" fill="var(--lon-roof)" />
      <rect x="1280" y="338" width="34" height="62" fill="var(--lon-stone)" /><path d="M1276 340L1297 296L1318 340z" fill="var(--lon-roof)" /><rect x="1296" y="278" width="2" height="20" fill="var(--lon-gold)" />
      <path d="M1288 392v-30a4 4 0 0 1 8 0v30zM1300 392v-30a4 4 0 0 1 8 0v30z" fill="var(--lon-win)" />
      <!-- tour Victoria -->
      <rect x="1410" y="296" width="44" height="186" fill="var(--lon-stone)" /><rect x="1406" y="290" width="52" height="12" fill="var(--lon-stone2)" />
      <path d="M1406 290V272h6v18zM1452 290V272h6v18z" fill="var(--lon-stone2)" /><path d="M1414 290V282L1432 266L1450 282V290z" fill="var(--lon-roof)" />
      <path d="M1426 330v-24a6 6 0 0 1 12 0v24zM1426 390v-30a6 6 0 0 1 12 0v30z" fill="var(--lon-win)" />
    </g>

    <!-- Big Ben (tour Elizabeth) -->
    <g transform="translate(1160 482) scale(.96)">
      <rect x="-26" y="-205" width="52" height="205" fill="var(--lon-stone)" />
      <path d="M-14-205V0M14-205V0" stroke="var(--lon-stone2)" stroke-width="2" />
      <rect x="-30" y="-22" width="60" height="22" fill="var(--lon-stone2)" />
      <path d="M-4-170h8v22h-8zM-4-120h8v22h-8zM-4-70h8v22h-8z" fill="var(--lon-win)" />
      <rect x="-34" y="-285" width="68" height="82" fill="var(--lon-stone)" /><rect x="-36" y="-206" width="72" height="6" fill="var(--lon-stone2)" />
      <path d="M-38-285V-302L-34-312L-30-302V-285zM30-285V-302L34-312L38-302V-285z" fill="var(--lon-stone2)" />
      <circle cy="-246" r="27" fill="var(--lon-gold)" /><circle cy="-246" r="22" fill="var(--lon-face)" />
      <path d="M0-266v6M0-232v6M-20-246h6M14-246h6M0-246V-262M0-246L10-240" stroke="var(--lon-hands)" stroke-width="2.6" stroke-linecap="round" />
      <rect x="-28" y="-318" width="56" height="34" fill="var(--lon-stone2)" />
      <path d="M-20-286v-18a5 5 0 0 1 10 0v18zM-5-286v-18a5 5 0 0 1 10 0v18zM10-286v-18a5 5 0 0 1 10 0v18z" fill="var(--lon-win)" />
      <path d="M-36-318L-7-356H7L36-318z" fill="var(--lon-roof)" /><path d="M-7-356L-2-370H2L7-356z" fill="var(--lon-roof)" />
      <rect x="-1.5" y="-392" width="3" height="24" fill="var(--lon-gold)" /><circle cy="-374" r="3.6" fill="var(--lon-gold)" />
    </g>

    <!-- Tower Bridge : deux tours, passerelle haute, chaînes, tablier et autobus -->
    <g>
      <path d="M646 462Q720 462 770 390M1094 462Q1020 462 970 390" stroke="var(--lon-bridge)" stroke-width="3.5" />
      <path d="M770 392Q700 440 650 462M970 392Q1040 440 1090 462" stroke="var(--lon-bridge)" stroke-width="1.6" />
      <rect x="812" y="364" width="116" height="8" fill="var(--lon-trim)" />
      <path d="M814 372L842 364L870 372L898 364L926 372" stroke="var(--lon-bridge)" stroke-width="2" />
      <rect x="640" y="460" width="460" height="8" fill="var(--lon-bridge)" />
      <rect x="640" y="468" width="460" height="3" fill="var(--lon-stone2)" opacity=".6" />
      <use href="#lon-tower" x="790" y="482" /><use href="#lon-tower" x="950" y="482" />
      <g transform="translate(862 444)"><rect width="30" height="14" rx="2" fill="var(--lon-bus)" /><path d="M4 4h5v4H4zM12 4h5v4h-5zM20 4h5v4h-5z" fill="var(--lon-glass)" /></g>
    </g>

    <!-- brume basse, quai, cabine, arbres -->
    <rect x="540" y="454" width="1260" height="28" fill="url(#lon-mist)" />
    <rect x="540" y="482" width="1260" height="8" fill="var(--lon-quay)" />
    <g transform="translate(1086 482)"><rect x="-9" y="-36" width="18" height="36" rx="3" fill="var(--lon-bus)" /><rect x="-6" y="-31" width="12" height="20" fill="var(--lon-glass)" /><path d="M-9-38h18" stroke="var(--lon-bus)" stroke-width="3" /></g>
    <g fill="var(--lon-tree)"><circle cx="1130" cy="466" r="14" /><circle cx="1448" cy="470" r="14" /><circle cx="1230" cy="470" r="10" /></g>
    <g stroke="var(--lon-ripple)" stroke-width="2" stroke-linecap="round" opacity=".55"><path d="M620 520h90M1100 512h110M1300 536h90M1480 510h100M720 566h120M1180 566h100" /></g>

    <!-- bateau sur la Tamise -->
    <g transform="translate(1250 500)">
      <path d="M0 2H132L118 18H12z" fill="var(--lon-boat)" /><path d="M12 18H118l-2 4H14z" fill="var(--lon-bridge)" />
      <rect x="22" y="-14" width="86" height="16" fill="var(--lon-boat)" /><rect x="30" y="-9" width="70" height="8" fill="var(--lon-glass)" />
      <rect x="40" y="-26" width="48" height="12" fill="var(--lon-boat)" /><rect x="46" y="-23" width="36" height="6" fill="var(--lon-glass)" />
      <path d="M64-26V-40M64-40l12 4-12 4z" stroke="var(--lon-bus)" stroke-width="2" fill="var(--lon-bus)" />
    </g>

    <!-- mascottes : sur le quai et sur le toit du palais -->
    <use href="#dm-mascot" transform="translate(1018 452) scale(.1)" />
    <use href="#dm-mascot" transform="translate(1336 370) scale(.1)" />
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
  .scene-london {
    --sky1: #c5bfd9; --sky2: #eee6ea; --sun: #fbf0dd; --water1: #a5b2c6; --water2: #7b8ba7; --cloud: #fff;
    --crater: transparent; --star: transparent;
    --lon-stone: #dccba2; --lon-stone2: #c2ae82; --lon-roof: #66777f; --lon-gold: #d9aa3b; --lon-face: #fffaf0; --lon-hands: #3a3f52;
    --lon-win: #6f6a7e; --lon-bridge: #7d92bd; --lon-trim: #eef1f8; --lon-far2: #b7b0cc; --lon-far3: #a79fc2;
    --lon-bus: #c8402f; --lon-glass: #a9c3d6; --lon-quay: #b8aeb0; --lon-tree: #8aa88b; --lon-boat: #f6f1ea; --lon-ripple: #fff;
  }
  :global(.dark) .scene-london {
    --sky1: #0c1229; --sky2: #1e2548; --sun: #e8eef8; --water1: #16294a; --water2: #0a1428; --cloud: #2a3658;
    --crater: #b9c4dc; --star: #e8eef8;
    --lon-stone: #6f7494; --lon-stone2: #555b7c; --lon-roof: #36425e; --lon-gold: #e8bd4f; --lon-face: #f5c76a; --lon-hands: #0d1222;
    --lon-win: #f5c76a; --lon-bridge: #44558a; --lon-trim: #8e9cc8; --lon-far2: #36406a; --lon-far3: #5a668f;
    --lon-bus: #ff6b5a; --lon-glass: #f5c76a; --lon-quay: #343c5a; --lon-tree: #1f4a4f; --lon-boat: #8b92a8; --lon-ripple: #5f7aa8;
  }
</style>
