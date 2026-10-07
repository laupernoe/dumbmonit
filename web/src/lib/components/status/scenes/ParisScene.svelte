<div class="scene scene-paris" aria-hidden="true">
  <svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none">
    <defs>
      <!-- fenêtres haussmanniennes : une trame de 32 x 28 -->
      <pattern id="paris-win" width="32" height="28" patternUnits="userSpaceOnUse">
        <path d="M9 23V9a7 7 0 0 1 14 0v14z" fill="var(--paris-win)" />
      </pattern>
      <clipPath id="paris-eif">
        <path d="M-95 0H-58C-58-30-30-52 0-52C30-52 58-30 58 0H95C80-30 64-56 58-78C48-100 40-125 34-150C26-200 16-280 9-345H-9C-16-280-26-200-34-150C-40-125-48-100-58-78C-64-56-80-30-95 0Z" />
      </clipPath>
      <!-- immeubles : A (haut) et B (bas) ; origine = pied, coin gauche -->
      <g id="paris-bldg-a">
        <rect y="-118" width="96" height="118" fill="var(--paris-stone)" />
        <rect y="-112" width="96" height="112" fill="url(#paris-win)" />
        <path d="M0-61h96v3H0zM0-5h96v3H0z" fill="var(--paris-trim)" />
        <rect y="-12" width="96" height="12" fill="var(--paris-stone2)" />
        <path d="M-4-118L8-152H88L100-118z" fill="var(--paris-zinc)" />
        <path d="M-4-118H100v4H-4z" fill="var(--paris-zinc2)" />
        <path d="M17-152v-16h9v16zM68-152v-20h9v20z" fill="var(--paris-stone2)" />
        <path d="M22-130h12v12H22zM42-130h12v12H42zM62-130h12v12H62z" fill="var(--paris-stone)" />
        <path d="M24-124h8v6h-8zM44-124h8v6h-8zM64-124h8v6h-8z" fill="var(--paris-win)" />
      </g>
      <g id="paris-bldg-b">
        <rect y="-90" width="96" height="90" fill="var(--paris-stone)" />
        <rect y="-84" width="96" height="84" fill="url(#paris-win)" />
        <path d="M0-33h96v3H0z" fill="var(--paris-trim)" />
        <rect y="-12" width="96" height="12" fill="var(--paris-stone2)" />
        <path d="M-4-90L8-122H88L100-90z" fill="var(--paris-zinc2)" />
        <path d="M-4-90H100v4H-4z" fill="var(--paris-zinc)" />
        <path d="M70-122v-14h8v14z" fill="var(--paris-stone2)" />
        <path d="M24-112h12v12H24zM60-112h12v12H60z" fill="var(--paris-stone)" />
        <path d="M26-107h8v7h-8zM62-107h8v7h-8z" fill="var(--paris-win)" />
      </g>
    </defs>

    <!-- astre, étoiles et nuage -->
    <circle cx="1470" cy="196" r="44" fill="var(--sun)" />
    <circle cx="1456" cy="186" r="7" fill="var(--crater)" /><circle cx="1484" cy="208" r="5" fill="var(--crater)" />
    <g fill="var(--star)"><circle cx="1010" cy="130" r="2" /><circle cx="1360" cy="118" r="2.4" /><circle cx="1560" cy="130" r="2" /><circle cx="1330" cy="250" r="1.8" /><circle cx="1620" cy="260" r="2.2" /><circle cx="1090" cy="104" r="1.8" /><circle cx="1220" cy="96" r="2" /></g>
    <g fill="var(--cloud)" opacity=".7"><ellipse cx="1000" cy="132" rx="64" ry="13" /><ellipse cx="1030" cy="121" rx="38" ry="13" /><ellipse cx="1640" cy="170" rx="60" ry="11" /></g>

    <!-- second plan : butte Montmartre, Sacré-Cœur et toits lointains -->
    <g fill="var(--paris-far)">
      <path d="M540 482V396C620 366 700 340 780 346C860 352 910 420 960 482z" />
      <path d="M940 482V420h40v-10h50v16h60v-14h50v18h60v-10h70v20h80v-14h70v12h90v-10h80v30h60v14h60v-10h50v22z" />
    </g>
<g transform="translate(704 350)" fill="var(--paris-church)">
      <rect x="-60" y="-14" width="120" height="16" /><rect x="-26" y="-34" width="52" height="22" />
      <path d="M-26-34C-26-76 26-76 26-34z" /><rect x="-3" y="-96" width="6" height="22" />
      <path d="M-58-14C-58-40-26-40-26-14zM26-14C26-40 58-40 58-14z" />
      <rect x="62" y="-52" width="16" height="52" /><path d="M62-52C62-66 78-66 78-52z" />
    </g>

    <!-- Tour Eiffel, trois niveaux -->
    <g transform="translate(1230 482) scale(.92)">
      <path d="M-95 0H-58C-58-30-30-52 0-52C30-52 58-30 58 0H95C80-30 64-56 58-78C48-100 40-125 34-150C26-200 16-280 9-345H-9C-16-280-26-200-34-150C-40-125-48-100-58-78C-64-56-80-30-95 0Z" fill="var(--paris-iron)" />
      <g clip-path="url(#paris-eif)" stroke="var(--paris-lat)" stroke-width="2" opacity=".6">
        <path d="M-100 0L-52-40M-52 0L-100-40M100 0L52-40M52 0L100-40M-100-40L-52-78M-52-40L-100-78M100-40L52-78M52-40L100-78" />
        <path d="M-70-96L70-150M70-96L-70-150M0-78V-150M-40-150L40-196M40-150L-40-196M-40-196L40-244M40-196L-40-244M-40-244L40-292M40-244L-40-292M-40-292L40-340M40-292L-40-340M-50-120H50M-50-170H50M-50-220H50M-50-268H50M-50-316H50" />
      </g>
      <g fill="var(--paris-iron2)">
        <rect x="-68" y="-84" width="136" height="9" rx="2" /><rect x="-42" y="-158" width="84" height="8" rx="2" />
        <rect x="-16" y="-354" width="32" height="9" rx="2" /><rect x="-8" y="-370" width="16" height="16" rx="3" />
        <path d="M-2.5-370h5L1-398h-2z" /><path d="M-64-84v-8h128v8z" opacity=".6" />
      </g>
      <path d="M0-398V-412M0-410l12 3-12 3z" stroke="var(--paris-iron2)" stroke-width="2" />
    </g>
    <!-- Champ-de-Mars : arbres -->
    <g fill="var(--paris-green)">
      <circle cx="1118" cy="464" r="20" /><circle cx="1150" cy="472" r="14" /><circle cx="1310" cy="472" r="14" /><circle cx="1342" cy="464" r="20" />
    </g>

    <!-- bâtiments haussmanniens : rive gauche puis rive droite -->
    <g transform="translate(0 482)">
      <use href="#paris-bldg-a" x="600" /><use href="#paris-bldg-b" x="696" /><use href="#paris-bldg-a" x="792" />
      <use href="#paris-bldg-b" x="888" /><use href="#paris-bldg-a" x="984" />
      <use href="#paris-bldg-a" x="1380" /><use href="#paris-bldg-b" x="1476" /><use href="#paris-bldg-a" x="1572" /><use href="#paris-bldg-b" x="1668" />
    </g>

    <!-- quai de Seine, réverbère et rides -->
    <rect x="540" y="482" width="1260" height="8" fill="var(--paris-stone2)" />
    <path d="M1098 482V440M1098 440q0-8 8-8M1092 432h12" stroke="var(--paris-iron)" stroke-width="3" stroke-linecap="round" />
    <g stroke="var(--paris-ripple)" stroke-width="2" stroke-linecap="round" opacity=".6"><path d="M600 512h70M760 540h110M1260 510h90M1420 536h120M1180 560h100M640 580h90" /></g>

    <!-- bateau-mouche -->
    <g transform="translate(930 498)">
      <path d="M0 4H150L136 20H12z" fill="var(--paris-boat)" />
      <path d="M12 20H136l-3 5H15z" fill="var(--paris-zinc2)" />
      <path d="M16 4V-8H134V4z" fill="var(--paris-boat)" />
      <path d="M22 -2V-18H128V-2z" fill="var(--paris-glass)" />
      <path d="M52-18V-2M80-18V-2M108-18V-2" stroke="var(--paris-boat)" stroke-width="3" />
      <path d="M30-18l8-8H112l8 8z" fill="var(--paris-boat)" /><path d="M92-26V-38l12 5z" fill="var(--paris-flag)" />
    </g>

    <!-- mascottes : sur le quai et sur un toit -->
    <use href="#dm-mascot" transform="translate(1034 452) scale(.1)" />
    <use href="#dm-mascot" transform="translate(1432 300) scale(.1)" />
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
  .scene-paris {
    --sky1: #c6dbf0; --sky2: #f9eddb; --sun: #ffe3a3; --water1: #a8c5d8; --water2: #7da2ba; --cloud: #fff;
    --crater: transparent; --star: transparent;
    --paris-stone: #f1e7d3; --paris-stone2: #dccfb4; --paris-trim: #c9b48c; --paris-win: #5a6684;
    --paris-zinc: #7f8da6; --paris-zinc2: #66748e; --paris-iron: #735a4d; --paris-iron2: #98806f; --paris-lat: #f9eddb;
    --paris-far: #c5d2e4; --paris-church: #f3efe8; --paris-green: #8fb597; --paris-ripple: #fff;
    --paris-boat: #fbf7ee; --paris-glass: #8fb2c9; --paris-flag: #c8402f;
  }
  :global(.dark) .scene-paris {
    --sky1: #0b1229; --sky2: #1b2850; --sun: #e8eef8; --water1: #162a4b; --water2: #0a1428; --cloud: #2a3658;
    --crater: #b9c4dc; --star: #e8eef8;
    --paris-stone: #727b96; --paris-stone2: #565f7a; --paris-trim: #444d68; --paris-win: #f5c76a;
    --paris-zinc: #3b4766; --paris-zinc2: #2e3856; --paris-iron: #c9984a; --paris-iron2: #e8bd4f; --paris-lat: #1b2850;
    --paris-far: #1c2848; --paris-church: #2d3b60; --paris-green: #1f4a4f; --paris-ripple: #5f7aa8;
    --paris-boat: #8b92a8; --paris-glass: #f5c76a; --paris-flag: #ff6b5a;
  }
</style>
