<script lang="ts">
  /**
   * Autumn forest banner: a misty wood of orange and red trees, a stag
   * grazing on the bank of a stream, toadstools and drifting leaves.
   *
   * `mood` is the weather (clear and sunny, clouds, then a storm with rain and
   * lightning); `pulse` plays one gesture each time `at` changes: `down` the
   * stag snaps its head up and freezes, `up` it goes back to grazing,
   * `resolved` a gust swirls the leaves. Frozen under reduced motion.
   */
  import ForestDeer from '#lib/components/wall/forest/ForestDeer.svelte';
  import ForestTrees from '#lib/components/wall/forest/ForestTrees.svelte';
  import ForestFx from '#lib/components/wall/forest/ForestFx.svelte';
  import Mushroom from '#lib/components/wall/forest/Mushroom.svelte';
  import {
    makeTrees,
    makeLeaves,
    makeGust,
    makeStars,
    normalizeMood,
    type ForestMood,
    type ForestPulse
  } from '#lib/components/wall/forest/geometry.js';

  let { mood = 'ok', pulse = null }: { mood?: ForestMood; pulse?: ForestPulse } = $props();
  const weather = $derived(normalizeMood(mood));

  const FAR = makeTrees(11, 26, 540, 1800, 420, 120, 190);
  const MID = makeTrees(23, 14, 560, 1790, 466, 150, 230);
  const NEAR = [...makeTrees(37, 1, 640, 700, 500, 330, 340), ...makeTrees(41, 1, 1600, 1680, 500, 320, 330)];
  const LEAVES = makeLeaves(5, 14, 620, 1740, 130, 380);
  const GUST = makeGust(9, 12, 520, 260);
  const STARS = makeStars(3, 26, 1220, 300);
</script>

<div class="scene scene-forest mood-{weather}" aria-hidden="true">
  <svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none">
    <defs>
      <linearGradient id="forest-mist" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="var(--f-mist)" stop-opacity="0" />
        <stop offset=".5" stop-color="var(--f-mist)" />
        <stop offset="1" stop-color="var(--f-mist)" stop-opacity="0" />
      </linearGradient>
      <linearGradient id="forest-water" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="var(--f-water)" />
        <stop offset="1" stop-color="var(--f-water2)" />
      </linearGradient>
      <linearGradient id="forest-shaft" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="var(--f-sun)" stop-opacity=".5" />
        <stop offset="1" stop-color="var(--f-sun)" stop-opacity="0" />
      </linearGradient>
    </defs>

    <!-- étoiles, astre et nuages -->
    <g fill="var(--f-star)">
      {#each STARS as s, i (i)}<circle cx={560 + s.x} cy={80 + s.y} r={s.r} />{/each}
    </g>
    <g class="sun">
      <circle cx="1490" cy="196" r="92" fill="var(--f-sun)" opacity=".22" />
      <circle cx="1490" cy="196" r="38" fill="var(--f-sun)" />
    </g>
    <g class="shafts" fill="url(#forest-shaft)">
      <path d="M1470 220L1010 470H1090L1500 230z" />
      <path d="M1500 230L1240 480H1310L1530 236z" />
      <path d="M1450 214L800 470H860L1480 224z" opacity=".6" />
    </g>
    <g class="clouds" fill="var(--f-cloud)">
      <g class="cloud-a" opacity=".6"><ellipse cx="1010" cy="150" rx="70" ry="13" /><ellipse cx="1040" cy="139" rx="40" ry="13" /></g>
      <g class="cloud-b"><ellipse cx="1330" cy="130" rx="110" ry="20" /><ellipse cx="1380" cy="112" rx="64" ry="22" /><ellipse cx="1620" cy="168" rx="96" ry="18" /><ellipse cx="1590" cy="152" rx="50" ry="18" /></g>
      <g class="cloud-c" fill="var(--f-cloud-storm)"><ellipse cx="860" cy="120" rx="150" ry="28" /><ellipse cx="1120" cy="100" rx="170" ry="30" /><ellipse cx="1480" cy="104" rx="180" ry="30" /><ellipse cx="1700" cy="130" rx="120" ry="26" /></g>
    </g>

    <!-- forêt : lointaine, brumeuse, puis proche -->
    <g opacity=".5"><ForestTrees trees={FAR} /></g>
    <rect x="540" y="330" width="1260" height="130" fill="url(#forest-mist)" class="mist" />
    <g opacity=".82"><ForestTrees trees={MID} /></g>
    <rect x="540" y="400" width="1260" height="90" fill="url(#forest-mist)" class="mist mist-2" />

    <!-- ruisseau puis berge -->
    <rect x="540" y="500" width="1260" height="130" fill="url(#forest-water)" />
    <g class="reflect" fill="var(--f-c1)" opacity=".35">
      <rect x="760" y="534" width="22" height="64" rx="11" /><rect x="1330" y="536" width="26" height="56" rx="13" fill="var(--f-c2)" /><rect x="1560" y="534" width="20" height="60" rx="10" fill="var(--f-c3)" /><rect x="940" y="536" width="18" height="50" rx="9" fill="var(--f-c3)" />
    </g>
    <g class="ripples" stroke="var(--f-glint)" stroke-width="2" stroke-linecap="round" opacity=".5">
      <path d="M600 556h80M820 584h110M1120 560h90M1300 596h120M1500 568h100M700 604h70" />
    </g>
    <g fill="var(--f-ground2)"><ellipse cx="1060" cy="572" rx="26" ry="8" /><ellipse cx="1110" cy="586" rx="18" ry="6" /></g>
    <path d="M540 460C760 450 960 470 1200 458S1620 452 1800 466V524C1560 530 1360 514 1130 526S760 516 540 526z" fill="var(--f-ground)" />
    <path d="M540 466C760 456 960 476 1200 464S1620 458 1800 472" stroke="var(--f-grass)" stroke-width="5" stroke-linecap="round" opacity=".7" />

    <ForestTrees trees={NEAR} />

    <!-- souche et mascotte, champignons, cerf -->
    <path d="M992 494V472h44v22z" fill="var(--f-trunk)" /><ellipse cx="1014" cy="472" rx="22" ry="4" fill="var(--f-ground2)" />
    <use href="#dm-mascot" transform="translate(1000 442) scale(.1)" />
    <Mushroom x={820} y={494} glow /><Mushroom x={846} y={498} s={0.7} glow /><Mushroom x={1450} y={492} s={0.9} glow />
    <g transform="translate(1180 488)"><ForestDeer {pulse} /></g>

    <ForestFx
      uid="forest-s"
      x={560}
      y={80}
      w={1220}
      h={540}
      mood={weather}
      {pulse}
      leaves={LEAVES}
      gust={GUST}
      gustX={1120}
      gustY={480}
      boltX={1400}
      boltY={90}
      boltH={340}
    />
    <rect x="560" y="80" width="1220" height="540" fill="#16202a" class="veil" />
  </svg>
</div>

<style>
  .scene {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: linear-gradient(180deg, var(--f-sky1), var(--f-sky2) 70%, var(--f-water));
  }
  .scene svg {
    display: block;
    width: 100%;
    height: 100%;
    -webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
    mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
  }
  .scene-forest {
    --f-sky1: #c9d3cf; --f-sky2: #f3e3c4; --f-sun: #fff0c4; --f-cloud: #fffaf0; --f-cloud-storm: #8d97a1;
    --f-c1: #d9722a; --f-c2: #b53a26; --f-c3: #e4a935; --f-c4: #9c5324; --f-trunk: #5a3d2c;
    --f-mist: rgb(255 246 228 / .65); --f-ground: #8b6b3c; --f-ground2: #6d5530; --f-grass: #b08a42;
    --f-water: #8fb0b0; --f-water2: #5f8788; --f-glint: #fff;
    --f-deer: #a8693a; --f-deer2: #7a4a2a; --f-belly: #f1dcc0; --f-antler: #e8d6b0; --f-ink: #241810;
    --f-cap: #c8402f; --f-stem: #f2e6d0; --f-spot: #fff; --f-glow: transparent;
    --f-rain: rgb(80 95 110 / .5); --f-flash: #fff; --f-star: transparent;
  }
  :global(.dark) .scene-forest {
    --f-sky1: #0a121c; --f-sky2: #1d2b33; --f-sun: #e8e4d0; --f-cloud: #2a3a46; --f-cloud-storm: #1a232c;
    --f-c1: #8c4a1e; --f-c2: #6a2a22; --f-c3: #8a6a28; --f-c4: #5e3418; --f-trunk: #1e140f;
    --f-mist: rgb(120 150 170 / .2); --f-ground: #2b2214; --f-ground2: #1d170d; --f-grass: #4a3b1c;
    --f-water: #1c3540; --f-water2: #0e1d26; --f-glint: #9fc2d0;
    --f-deer: #6a4128; --f-deer2: #46291a; --f-belly: #a8947a; --f-antler: #b9a98a; --f-ink: #000;
    --f-cap: #b8453a; --f-stem: #cfc5b0; --f-spot: #ddd; --f-glow: rgb(120 220 170 / .35);
    --f-rain: rgb(170 190 210 / .4); --f-flash: #dfe9ff; --f-star: #e8eef8;
  }
  .scene-forest.mood-critical {
    --f-cloud: var(--f-cloud-storm);
  }
  /* The weather: each layer eases between moods. */
  .sun, .shafts, .cloud-a, .cloud-b, .cloud-c, .veil, .mist {
    transition: opacity 1.2s ease;
  }
  .shafts { opacity: .3; }
  .mood-warn .sun { opacity: .35; }
  .mood-warn .shafts, .mood-critical .shafts, .mood-critical .sun { opacity: 0; }
  .cloud-b, .cloud-c { opacity: 0; }
  .mood-warn .cloud-b, .mood-critical .cloud-b { opacity: .92; }
  .mood-critical .cloud-c { opacity: .95; }
  .veil { opacity: 0; pointer-events: none; }
  .mood-warn .veil { opacity: .12; }
  .mood-critical .veil { opacity: .32; }
  .mood-warn .mist { opacity: 1; }
  .mist { opacity: .8; }
  .mist-2 { animation: drift 40s ease-in-out infinite alternate; }
  .cloud-a, .cloud-b { animation: drift 70s ease-in-out infinite alternate; }
  .ripples { animation: ripple 6s ease-in-out infinite alternate; }
  .reflect { animation: wobble 5s ease-in-out infinite alternate; }
  @keyframes drift { from { transform: translateX(-24px); } to { transform: translateX(24px); } }
  @keyframes ripple { from { transform: translateX(-14px); } to { transform: translateX(14px); } }
  @keyframes wobble { from { transform: translateX(-3px); } to { transform: translateX(3px); } }
  @media (prefers-reduced-motion: reduce) {
    .mist-2, .cloud-a, .cloud-b, .ripples, .reflect { animation: none; }
  }
</style>
