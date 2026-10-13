<script lang="ts">
  /**
   * Kyoto: the golden pavilion on its pond, torii in the water, stone and
   * paper lanterns. Cherry blossom Feb-Jun, red maples the rest of the year.
   *
   * `mood` is the weather (ok: clear, warn: clouds, critical: rain and
   * lightning). `pulse` plays the lanterns' gesture whenever `at` changes:
   * down snuffs them out one by one, up relights them with a flare, resolved
   * sends a gust of petals (or maple leaves) across the pond.
   */
  interface Props {
    mood?: 'ok' | 'warn' | 'critical';
    pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
  }
  let { mood = 'ok', pulse = null }: Props = $props();

  const month = new Date().getMonth();
  const spring = month >= 1 && month <= 5;

  const toro = [1040, 1360, 1640];
  const chochin = [780, 835, 890, 945];
  const posts = [0, 1, 2, 3, 4, 5, 6, 7, 8];
  // A maple leaf is a five-pointed star, a petal an ellipse.
  const fallD = spring
    ? 'M0-3C3-3 4 0 0 3C-4 0-3-3 0-3Z'
    : 'M0-5L1.5-1.5 5-2 2.5 1 3.5 5 0 3-3.5 5-2.5 1-5-2-1.5-1.5Z';
  const drift = [
    { x: 700, d: 0, t: 14 }, { x: 860, d: -5, t: 17 }, { x: 1010, d: -9, t: 15 },
    { x: 1250, d: -3, t: 18 }, { x: 1420, d: -11, t: 13 }, { x: 1590, d: -7, t: 16 },
    { x: 1730, d: -2, t: 19 }
  ];
  const gust = Array.from({ length: 16 }, (_, i) => i);
  const wob = (i: number) => Math.sin(i * 1.6) * 3;
</script>

<div
  class="scene scene-kyoto"
  class:spring
  class:autumn={!spring}
  class:mood-warn={mood === 'warn'}
  class:mood-critical={mood === 'critical'}
  aria-hidden="true"
>
  <svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none">
    <defs>
      <g id="ky-pav">
        <rect x="-122" y="-6" width="244" height="6" fill="var(--ky-stone)" />
        <rect x="-104" y="-62" width="208" height="56" fill="var(--ky-white)" />
        {#each posts as p (p)}<rect x={-104 + p * 25.5} y="-62" width="4" height="56" fill="var(--ky-wood)" />{/each}
        <rect x="-104" y="-16" width="208" height="4" fill="var(--ky-wood)" />
        <path d="M-132-56Q-70-58 -52-84H52Q70-58 132-56Q70-52 0-52Q-70-52 -132-56Z" fill="var(--ky-roof)" />
        <rect x="-84" y="-128" width="168" height="44" fill="var(--ky-gold)" />
        <rect x="-84" y="-128" width="168" height="6" fill="var(--ky-gold2)" />
        <g fill="var(--ky-win)"><rect x="-68" y="-114" width="30" height="22" rx="2" /><rect x="-15" y="-114" width="30" height="22" rx="2" /><rect x="38" y="-114" width="30" height="22" rx="2" /></g>
        <path d="M-106-126Q-56-128 -40-154H40Q56-128 106-126Q56-122 0-122Q-56-122 -106-126Z" fill="var(--ky-roof)" />
        <rect x="-58" y="-190" width="116" height="36" fill="var(--ky-gold)" />
        <rect x="-58" y="-190" width="116" height="5" fill="var(--ky-gold2)" />
        <g fill="var(--ky-win)"><path d="M-40-148V-170Q-30-180 -20-170V-148Z" /><path d="M-10-148V-170Q0-180 10-170V-148Z" /><path d="M20-148V-170Q30-180 40-170V-148Z" /></g>
        <path d="M-86-188Q-40-190 -20-222L-6-246H6L20-222Q40-190 86-188Q40-184 0-184Q-40-184 -86-188Z" fill="var(--ky-roof)" />
        <path d="M0-250L-7-262Q0-272 7-262Z" fill="var(--ky-gold)" /><path d="M0-250V-246" stroke="var(--ky-gold2)" stroke-width="2" />
      </g>
      <pattern id="ky-rain" width="40" height="60" patternUnits="userSpaceOnUse" patternTransform="rotate(12)">
        <path d="M8 6v22M28 34v22" stroke="var(--ky-rain)" stroke-width="1.6" stroke-linecap="round" />
      </pattern>
    </defs>

    <!-- soleil / lune, étoiles, nuages -->
    <g class="sun"><circle cx="1000" cy="330" r="50" fill="var(--sun)" /></g>
    <g fill="var(--star)"><circle cx="1040" cy="116" r="2" /><circle cx="1260" cy="104" r="2.2" /><circle cx="1560" cy="120" r="2" /><circle cx="1680" cy="190" r="2.2" /><circle cx="1180" cy="170" r="1.8" /><circle cx="740" cy="160" r="1.8" /></g>
    <g fill="var(--cloud)" opacity=".7"><ellipse cx="1500" cy="150" rx="90" ry="9" /><ellipse cx="1560" cy="172" rx="60" ry="7" /><ellipse cx="800" cy="220" rx="80" ry="7" /></g>

    <!-- collines de Kitayama -->
    <path d="M560 400C640 360 700 380 780 350C860 320 940 360 1020 340C1120 312 1200 350 1300 336C1400 322 1480 360 1560 340C1640 322 1700 350 1780 370V490H560Z" fill="var(--ky-hill)" />
    <path d="M560 440C680 410 760 430 860 412C980 392 1100 430 1220 416C1340 402 1460 436 1580 418C1660 408 1720 424 1780 430V490H560Z" fill="var(--ky-hill2)" />

    <!-- pins et arbres de saison -->
    <g fill="var(--ky-pine)">
      <path d="M1060 482V440" stroke="var(--ky-trunk)" stroke-width="5" />
      <ellipse cx="1060" cy="426" rx="38" ry="12" /><ellipse cx="1052" cy="406" rx="28" ry="10" /><ellipse cx="1064" cy="388" rx="18" ry="8" fill="var(--ky-pine2)" />
      <path d="M1304 482V446" stroke="var(--ky-trunk)" stroke-width="5" />
      <ellipse cx="1304" cy="432" rx="34" ry="11" /><ellipse cx="1312" cy="414" rx="24" ry="9" /><ellipse cx="1302" cy="398" rx="15" ry="7" fill="var(--ky-pine2)" />
    </g>
    {#each [670, 990, 1430, 1715] as x, i (x)}
      <path d="M{x} 482V444M{x} 462l-14-14M{x} 456l12-12" stroke="var(--ky-trunk)" stroke-width="6" stroke-linecap="round" />
      <g fill="var(--ky-leaf)">
        <circle cx={x} cy={426 - (i % 2) * 6} r="30" /><circle cx={x - 24} cy="446" r="20" /><circle cx={x + 24} cy="444" r="21" />
      </g>
      <g fill="var(--ky-leaf2)"><circle cx={x + 8} cy={418 - (i % 2) * 6} r="14" /><circle cx={x - 22} cy="440" r="9" /></g>
    {/each}

    <!-- île, pavillon d'or et son reflet -->
    <rect x="1040" y="482" width="280" height="12" rx="4" fill="var(--ky-bank)" />
    <g transform="translate(1180 482) scale(1 -1)" opacity=".26"><use href="#ky-pav" /></g>
    <g transform="translate(1180 482)"><use href="#ky-pav" /></g>
    <path d="M560 482H1040L1000 494H560Z" fill="var(--ky-bank)" />

    <!-- torii dans l'eau -->
    <g transform="translate(1560 500)" fill="var(--ky-torii)">
      <rect x="-42" y="-118" width="11" height="118" /><rect x="31" y="-118" width="11" height="118" />
      <path d="M-66-122Q0-110 66-122L70-136Q0-124 -70-136Z" /><rect x="-50" y="-98" width="100" height="8" /><rect x="-4" y="-116" width="8" height="18" />
      <rect x="-44" y="-8" width="15" height="8" fill="var(--ky-lamp)" /><rect x="29" y="-8" width="15" height="8" fill="var(--ky-lamp)" />
    </g>
    <g stroke="var(--ky-ripple)" stroke-width="2" stroke-linecap="round" opacity=".55"><path d="M620 514h80M720 548h100M1120 506h90M1270 540h110M1460 524h70M1500 570h100M1650 512h70M660 584h80" /></g>

    <!-- lanternes : la clé `pulse.at` rejoue le geste -->
    {#key pulse?.at ?? 0}
      <g class="lamps" class:lamps-down={pulse?.kind === 'down'} class:lamps-up={pulse?.kind === 'up'} class:lamps-resolved={pulse?.kind === 'resolved'}>
        <path d="M740 482V424M980 482V424" stroke="var(--ky-wood)" stroke-width="5" stroke-linecap="round" />
        <path d="M740 428Q860 452 980 428" stroke="var(--ky-wood)" stroke-width="2.4" />
        {#each chochin as x, i (x)}
          <g style="--i:{i}">
            <circle class="glow" cx={x} cy={452 + wob(i)} r="26" fill="var(--ky-glow)" />
            <ellipse cx={x} cy={452 + wob(i)} rx="9" ry="12" fill="var(--ky-chochin)" />
            <rect class="fire" x={x - 5} y={448 + wob(i)} width="10" height="8" rx="3" fill="var(--ky-fire)" />
            <path d="M{x - 5} {440 + wob(i)}h10M{x - 5} {464 + wob(i)}h10" stroke="var(--ky-lamp)" stroke-width="2" />
          </g>
        {/each}
        {#each toro as x, i (x)}
          <g transform="translate({x} 482)" style="--i:{i + 4}">
            <circle class="glow" cy="-43" r="30" fill="var(--ky-glow)" />
            <rect x="-9" y="-14" width="18" height="14" fill="var(--ky-stone)" /><rect x="-3.5" y="-36" width="7" height="22" fill="var(--ky-stone)" />
            <rect x="-10" y="-54" width="20" height="19" fill="var(--ky-lamp)" /><rect class="fire" x="-6" y="-50" width="12" height="11" fill="var(--ky-fire)" />
            <path d="M-18-54L0-68 18-54Z" fill="var(--ky-stone)" /><circle cy="-70" r="2.6" fill="var(--ky-stone)" />
          </g>
        {/each}
        <!-- l'élan du pavillon quand tout se rallume -->
        <ellipse class="flare" cx="1180" cy="380" rx="150" ry="130" fill="var(--ky-gold)" />
        {#if pulse?.kind === 'resolved'}
          {#each gust as i (i)}
            <g transform="translate(0 {150 + ((i * 53) % 330)})">
              <path class="gust" d={fallD} fill="var(--ky-fall)" style="--dy:{((i % 5) - 2) * 36}px;animation-delay:{i * 0.16}s" />
            </g>
          {/each}
        {/if}
      </g>
    {/key}

    <!-- pétales / feuilles qui tombent -->
    {#each drift as f, i (i)}
      <g transform="translate({f.x} 90)">
        <path class="fall" d={fallD} fill="var(--ky-fall)" style="--dx:{f.d * 12}px;animation-duration:{f.t}s;animation-delay:-{i * 2.3}s" />
      </g>
    {/each}

    <!-- mascotte au pied du pavillon -->
    <use href="#dm-mascot" transform="translate(1346 452) scale(.1)" />

    <!-- météo : voile, nuages, pluie, éclair -->
    <rect class="gloom" x="560" y="80" width="1220" height="540" fill="var(--ky-storm)" />
    <g class="wx-clouds" fill="var(--ky-wxcloud)">
      <ellipse cx="900" cy="150" rx="190" ry="34" /><ellipse cx="1100" cy="128" rx="150" ry="30" /><ellipse cx="1400" cy="156" rx="210" ry="36" /><ellipse cx="1640" cy="132" rx="150" ry="28" />
    </g>
    <rect class="rain" x="480" y="-40" width="1400" height="720" fill="url(#ky-rain)" />
    <rect class="bolt" x="560" y="80" width="1220" height="540" fill="#fff" />
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
  .scene-kyoto {
    --sun: #fff1cc; --star: transparent; --cloud: #fff;
    --ky-hill: #b9c4b8; --ky-hill2: #9fb3a0; --ky-pine: #4f7a58; --ky-pine2: #5f8c66; --ky-trunk: #6e5546;
    --ky-gold: #e9bd3c; --ky-gold2: #c4932a; --ky-roof: #5b4036; --ky-white: #f4ead8; --ky-wood: #7a5a44; --ky-win: #7d5a2a;
    --ky-torii: #d9472f; --ky-stone: #a9a79f; --ky-lamp: #56504c; --ky-fire: #ffd57a; --ky-glow: rgb(255 200 110 / .18);
    --ky-chochin: #e5533d; --ky-ripple: #fff; --ky-bank: #8e9382;
    --ky-rain: rgb(60 76 104 / .5); --ky-storm: #2a3048; --ky-wxcloud: #f2f2f4;
  }
  .scene-kyoto.spring { --sky1: #f6c9d8; --sky2: #fdeede; --water1: #a9c9cf; --water2: #7ba2b0; --ky-leaf: #f4a7c0; --ky-leaf2: #fbd0df; --ky-fall: #f7b6cc; }
  .scene-kyoto.autumn { --sky1: #f0b98f; --sky2: #fde6c8; --water1: #b0bfb2; --water2: #84988f; --ky-leaf: #d8452c; --ky-leaf2: #f08a2e; --ky-fall: #d9502e; }
  :global(.dark) .scene-kyoto {
    --sun: #e8eef8; --star: #e8eef8; --cloud: #2a3358;
    --ky-hill: #1d2748; --ky-hill2: #17203c; --ky-pine: #1c3a36; --ky-pine2: #24483f; --ky-trunk: #3a2e3a;
    --ky-gold: #f0c04a; --ky-gold2: #b88a2a; --ky-roof: #2a2030; --ky-white: #3a3550; --ky-wood: #4a3a46; --ky-win: #ffd27a;
    --ky-torii: #d0583f; --ky-stone: #4b4f6c; --ky-lamp: #2a2838; --ky-fire: #ffc766; --ky-glow: rgb(255 180 80 / .38);
    --ky-chochin: #d9553f; --ky-ripple: #6a74a8; --ky-bank: #2a2f4e;
    --ky-rain: rgb(170 190 230 / .4); --ky-storm: #05070f; --ky-wxcloud: #1a2038;
  }
  :global(.dark) .scene-kyoto.spring { --sky1: #0c1230; --sky2: #30244f; --water1: #171c43; --water2: #0a1028; --ky-leaf: #8a5c86; --ky-leaf2: #b07aa0; --ky-fall: #c78aae; }
  :global(.dark) .scene-kyoto.autumn { --sky1: #0c1230; --sky2: #2c2048; --water1: #171c43; --water2: #0a1028; --ky-leaf: #8c3a30; --ky-leaf2: #b8622c; --ky-fall: #c0502e; }

  /* Weather */
  .gloom, .wx-clouds, .rain, .bolt { opacity: 0; transition: opacity 2s ease; pointer-events: none; }
  .sun { transition: opacity 2s; }
  .mood-warn .gloom { opacity: .1; }
  .mood-critical .gloom { opacity: .3; }
  .mood-warn .wx-clouds { opacity: .85; }
  .mood-critical .wx-clouds { opacity: 1; }
  .mood-warn .sun { opacity: .5; }
  .mood-critical .sun { opacity: 0; }
  .mood-critical .rain { opacity: 1; animation: ky-rain .7s linear infinite; }
  .mood-critical .bolt { animation: ky-bolt 9s linear infinite; }
  .wx-clouds { animation: ky-cloud 40s ease-in-out infinite alternate; }
  @keyframes ky-rain { to { transform: translateY(60px); } }
  @keyframes ky-bolt {
    0%, 62%, 66%, 100% { opacity: 0; }
    63% { opacity: .5; } 64% { opacity: .05; } 65% { opacity: .35; }
  }
  @keyframes ky-cloud { to { transform: translateX(40px); } }

  /* Falling petals: each sits in a translated group, so the keyframes are relative */
  .fall { animation: ky-fall 16s linear infinite; }
  @keyframes ky-fall {
    from { transform: translate(0, 0) rotate(0); opacity: 0; }
    8%, 92% { opacity: .9; }
    to { transform: translate(var(--dx), 400px) rotate(300deg); opacity: 0; }
  }

  /* Lantern gestures */
  .flare { opacity: 0; }
  .lamps :global(.glow), .lamps :global(.fire) { transform-box: fill-box; transform-origin: center; }
  .lamps-down .glow, .lamps-down .fire { animation: ky-off 4s ease-in-out calc(var(--i, 0) * .22s) backwards; }
  .lamps-up .glow { animation: ky-on 3s ease-out calc(var(--i, 0) * .16s) backwards; }
  .lamps-up .fire { animation: ky-on-fire 3s ease-out calc(var(--i, 0) * .16s) backwards; }
  .lamps-up .flare { animation: ky-flare 3.4s ease-out 1s both; }
  .lamps-resolved .glow { animation: ky-on 2.4s ease-out calc(var(--i, 0) * .1s) backwards; }
  .lamps-resolved .flare { animation: ky-flare 3s ease-out .4s both; }
  @keyframes ky-off { 0% { opacity: 1; } 12%, 70% { opacity: 0; } 100% { opacity: 1; } }
  @keyframes ky-on { 0% { opacity: 0; transform: scale(.4); } 35% { opacity: 1; transform: scale(1.7); } 100% { opacity: 1; transform: scale(1); } }
  @keyframes ky-on-fire { 0% { opacity: 0; } 35%, 100% { opacity: 1; } }
  @keyframes ky-flare { 0% { opacity: 0; } 30% { opacity: .35; } 100% { opacity: 0; } }
  .gust { animation: ky-gust 5.5s ease-in-out both; }
  @keyframes ky-gust {
    from { transform: translate(560px, 0) rotate(0); opacity: 0; }
    10% { opacity: 1; }
    to { transform: translate(1780px, var(--dy)) rotate(720deg); opacity: 0; }
  }
  @media (prefers-reduced-motion: reduce) {
    .scene :global(*) { animation: none !important; transition: none !important; }
    .fall, .gust { display: none; }
  }
</style>
