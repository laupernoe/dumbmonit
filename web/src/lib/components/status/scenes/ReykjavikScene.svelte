<script lang="ts">
  /**
   * Reykjavik for the public status banner: Hallgrimskirkja on its hill, the
   * colourful corrugated houses, the harbour, Esja across the bay and the
   * northern lights at night.
   *
   * `mood` is the weather (ok: clear, warn: clouds, critical: rain and
   * lightning; the wall's calm / clouded / storm are accepted too). `pulse` is
   * the scene's gesture, replayed whenever `at` changes:
   *  - down: the lights go out, the aurora collapses and the church goes dark,
   *    then both flicker back;
   *  - up: the aurora surges, ribbons swelling and brightening over the town;
   *  - resolved: rings of green light ripple out from the spire while the
   *    aurora shifts through violet.
   */
  type Mood = 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
  interface Props {
    mood?: Mood;
    pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
  }
  let { mood = 'ok', pulse = null }: Props = $props();

  const level = $derived(mood === 'warn' || mood === 'clouded' ? 1 : mood === 'critical' || mood === 'storm' ? 2 : 0);

  // The gesture plays for a few seconds, then the scene settles back.
  let playing = $state<'down' | 'up' | 'resolved' | null>(null);
  $effect(() => {
    const p = pulse;
    playing = null;
    if (!p) return;
    void p.at;
    const start = requestAnimationFrame(() => (playing = p.kind));
    const stop = setTimeout(() => (playing = null), 5600);
    return () => {
      cancelAnimationFrame(start);
      clearTimeout(stop);
    };
  });

  function rng(seed: number) {
    let a = seed;
    return () => {
      a = (a + 0x6d2b79f5) >>> 0;
      let t = a;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }
  const r = rng(64);
  function row(x0: number, x1: number, base: number, hMin: number, hMax: number, wMin: number, wMax: number) {
    const out: { x: number; w: number; h: number; base: number; wall: number; roof: number; cols: number }[] = [];
    let x = x0;
    while (x < x1) {
      const w = Math.round(wMin + r() * (wMax - wMin));
      const h = Math.round(hMin + r() * (hMax - hMin));
      out.push({ x, w, h, base, wall: Math.floor(r() * 6), roof: Math.floor(r() * 4), cols: Math.max(1, Math.floor(w / 20)) });
      x += w + Math.round(r() * 3);
    }
    return out;
  }
  const BACK = row(590, 1230, 368, 26, 42, 30, 46);
  const FRONT = row(470, 1230, 394, 36, 64, 40, 62);
  const COLS = [200, 158, 120, 90, 66, 48, 34];
  const RAIN = (() => {
    const g = rng(7);
    let d = '';
    for (let i = 0; i < 90; i++) {
      const x = Math.round(g() * 1260);
      const y = Math.round(g() * 540) - 540;
      d += `M${x} ${y}l-6 22M${x} ${y + 540}l-6 22`;
    }
    return d;
  })();
  const STARS = [[1010, 60, 2], [730, 40, 1.8], [150, 90, 1.6], [420, 70, 2], [1150, 120, 1.8], [90, 200, 1.8], [560, 110, 1.5], [880, 90, 1.6], [330, 40, 1.5], [1190, 40, 1.6]];
</script>

<div class="scene scene-reykjavik mood-{level}" class:p-down={playing === 'down'} class:p-up={playing === 'up'} class:p-resolved={playing === 'resolved'} aria-hidden="true">
  <svg viewBox="0 0 1220 540" preserveAspectRatio="xMidYMid slice" fill="none">
    <defs>
      <linearGradient id="rey-a1" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--rey-aur1)" stop-opacity="0" /><stop offset="1" stop-color="var(--rey-aur1)" stop-opacity=".85" /></linearGradient>
      <linearGradient id="rey-a2" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--rey-aur2)" stop-opacity="0" /><stop offset="1" stop-color="var(--rey-aur2)" stop-opacity=".7" /></linearGradient>
      <linearGradient id="rey-ray" gradientUnits="userSpaceOnUse" x1="0" y1="110" x2="0" y2="250"><stop offset="0" stop-color="var(--rey-aur1)" stop-opacity="0" /><stop offset="1" stop-color="var(--rey-aur1)" stop-opacity=".75" /></linearGradient>
      <linearGradient id="rey-refl" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--rey-aur1)" stop-opacity=".35" /><stop offset="1" stop-color="var(--rey-aur1)" stop-opacity="0" /></linearGradient>
      <filter id="rey-blur" x="-10%" y="-30%" width="120%" height="160%"><feGaussianBlur stdDeviation="5" /></filter>
    </defs>

    <!-- stars, sun or moon -->
    <g fill="var(--star)">{#each STARS as s, i (i)}<circle cx={s[0]} cy={s[1]} r={s[2]} />{/each}</g>
    <circle cx="250" cy="170" r="30" fill="var(--sun)" class="sun" />

    <!-- northern lights: curtains, rays and their reflection in the bay -->
    <g class="aurora">
      <g class="sway">
        <g filter="url(#rey-blur)">
          <path d="M100 200C260 120 380 210 560 140S880 90 1100 170L1100 250C900 170 760 240 560 220S260 280 100 260Z" fill="url(#rey-a1)" />
          <path d="M300 120C460 60 640 130 800 80S1000 60 1180 110L1180 160C1000 120 860 170 700 160S440 170 300 170Z" fill="url(#rey-a2)" />
        </g>
        <g stroke="url(#rey-ray)" stroke-width="2.4" stroke-linecap="round">
          {#each Array(26) as _, i (i)}<path d="M{150 + i * 37} {245 - ((i * 7) % 5) * 10}V{150 + ((i * 11) % 6) * 14}" />{/each}
        </g>
        <path d="M200 395H980V470H200z" fill="url(#rey-refl)" />
      </g>
    </g>

    <!-- clouds -->
    <g class="clouds" fill="var(--cloud)">
      <ellipse cx="300" cy="120" rx="130" ry="20" /><ellipse cx="380" cy="100" rx="70" ry="20" />
      <ellipse cx="880" cy="160" rx="150" ry="22" /><ellipse cx="960" cy="140" rx="80" ry="20" />
      <ellipse cx="600" cy="70" rx="110" ry="16" /><ellipse cx="1100" cy="90" rx="90" ry="14" />
    </g>

    <!-- Esja, flat-topped, with snow in its gullies -->
    <path d="M0 392V330L90 300L170 285L240 262L380 252L560 258L700 276L820 300L960 316L1100 306L1220 326V392z" fill="var(--rey-mtn)" />
    <path d="M0 392V352L110 330L250 318L420 322L600 314L760 330L930 340L1100 336L1220 350V392z" fill="var(--rey-mtn2)" />
    <path d="M250 266l10 34 10-30zM330 254l8 40 12-38zM440 254l6 34 12-32zM560 260l8 36 10-34zM650 270l8 30 10-26zM780 292l8 22 8-20z" fill="var(--rey-snow)" opacity=".8" />

    <!-- the hill and Hallgrimskirkja -->
    <path d="M470 394C560 380 640 328 760 324C900 320 1010 352 1140 362L1220 368V394z" fill="var(--rey-ground)" />
    <g transform="translate(790 326) scale(.76)">
      <ellipse cy="-4" rx="190" ry="40" fill="var(--glow)" class="chlit" />
      <rect x="-156" y="-30" width="52" height="30" fill="var(--rey-church2)" /><rect x="104" y="-30" width="52" height="30" fill="var(--rey-church2)" />
      {#each COLS as h, i (i)}
        <rect x={-20 - 13 * (i + 1)} y={-h} width="13" height={h} fill="var(--rey-church)" stroke="var(--rey-church2)" stroke-width="1.4" />
        <rect x={20 + 13 * i} y={-h} width="13" height={h} fill="var(--rey-church)" stroke="var(--rey-church2)" stroke-width="1.4" />
      {/each}
      <rect x="-20" y="-238" width="40" height="238" fill="var(--rey-church)" /><rect x="2" y="-238" width="18" height="238" fill="var(--rey-church2)" opacity=".55" />
      <rect x="-27" y="-272" width="54" height="36" fill="var(--rey-church)" /><rect x="-30" y="-276" width="60" height="6" fill="var(--rey-church2)" />
      <path d="M-27 -272L0 -344L27 -272z" fill="var(--rey-church)" /><path d="M0 -344L27 -272H8z" fill="var(--rey-church2)" opacity=".55" />
      <path d="M0 -344V-362M-6 -354H6" stroke="var(--rey-church2)" stroke-width="2.4" />
      <g class="chlit" fill="var(--window-lit)"><path d="M-14 -262v-18h6v18zM-3 -262v-18h6v18zM8 -262v-18h6v18zM-4 -200v-40h8v40zM-4 -150v-30h8v30z" /></g>
      <!-- resolved: rings of light from the spire -->
      <g class="rings" stroke="var(--rey-aur1)" stroke-width="3"><circle cy="-330" r="40" /><circle cy="-330" r="40" /><circle cy="-330" r="40" /></g>
    </g>
    <use href="#dm-mascot" transform="translate(668 273) scale(.1)" />

    <!-- colourful houses -->
    {#each [BACK, FRONT] as rowOf, k (k)}
      {#each rowOf as h, i (i)}
        <g>
          <rect x={h.x} y={h.base - h.h} width={h.w} height={h.h} fill="var(--rey-h{h.wall})" />
          <path d="M{h.x - 2} {h.base - h.h}L{h.x + h.w / 2} {h.base - h.h - h.w * 0.36}L{h.x + h.w + 2} {h.base - h.h}z" fill="var(--rey-r{h.roof})" />
          {#each Array(h.cols) as _, c (c)}
            <rect x={h.x + (h.w / h.cols) * (c + 0.5) - 3.5} y={h.base - h.h + 8} width="7" height="9" fill="var(--rey-win)" class="win" />
            {#if h.h > 48}<rect x={h.x + (h.w / h.cols) * (c + 0.5) - 3.5} y={h.base - h.h + 26} width="7" height="9" fill="var(--rey-win)" class="win" />{/if}
          {/each}
        </g>
      {/each}
    {/each}

    <!-- quay, ripples, fishing boat and a sailboat -->
    <rect x="0" y="394" width="1220" height="6" fill="var(--rey-quay)" opacity=".7" />
    <g stroke="var(--rey-ripple)" stroke-width="2" stroke-linecap="round" opacity=".5"><path d="M60 450h90M300 480h110M520 440h80M700 500h120M930 460h90M180 520h120M420 520h100" /></g>
    <g transform="translate(330 438)">
      <path d="M0 0H110L96 18H14z" fill="var(--rey-hull)" /><rect x="60" y="-24" width="30" height="24" fill="var(--rey-r1)" /><rect x="66" y="-18" width="18" height="9" fill="var(--rey-win)" />
      <path d="M30 0V-48M30-48L74-8" stroke="var(--rey-mast)" stroke-width="2.4" />
    </g>
    <g transform="translate(130 470)"><path d="M0 0H60L52 12H8z" fill="var(--rey-hull)" /><path d="M30 0V-52L58-8z" fill="var(--rey-sail)" /></g>
    <use href="#dm-mascot" transform="translate(430 364) scale(.1)" />

    <!-- storm -->
    <g class="rainfall"><path d={RAIN} stroke="var(--rey-rain)" stroke-width="1.6" stroke-linecap="round" /></g>
    <path class="bolt" d="M360 0L335 120L368 118L322 250" stroke="#fff6c8" stroke-width="4" stroke-linejoin="round" />
    <rect class="flash" width="1220" height="540" fill="#fff" />
  </svg>
</div>

<style>
  .scene {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: linear-gradient(180deg, var(--sky1), var(--sky2) 72.5%, var(--water1) 73.3%, var(--water1) 79%, var(--water2));
  }
  .scene svg {
    display: block;
    width: 100%;
    height: 100%;
    -webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
    mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
  }
  .scene-reykjavik {
    --sky1: #b9d3ec; --sky2: #eef0ee; --sun: #fbefc9; --water1: #9fbbd2; --water2: #7396b4; --cloud: #fff; --star: transparent;
    --glow: transparent; --window-lit: #f6d27a;
    --rey-aur1: #4fe3a2; --rey-aur2: #b07cf0; --rey-aur-op: 0; --rey-aur-peak: .55;
    --rey-mtn: #8da3c4; --rey-mtn2: #a9bad2; --rey-snow: #f4f7fb; --rey-ground: #8fae8a;
    --rey-church: #f1ede6; --rey-church2: #bdb7ae;
    --rey-h0: #d9473b; --rey-h1: #f2c14a; --rey-h2: #3f78b5; --rey-h3: #f4efe6; --rey-h4: #4ea083; --rey-h5: #e98b4f;
    --rey-r0: #b0372e; --rey-r1: #3b5f8f; --rey-r2: #5a6c80; --rey-r3: #8a3b3b;
    --rey-win: rgb(255 255 255 / .75); --rey-quay: #7d8ea2; --rey-ripple: #fff; --rey-hull: #2f4a73; --rey-mast: #55402e; --rey-sail: #fbf7ee;
    --rey-rain: rgb(70 84 110 / .5);
  }
  :global(.dark) .scene-reykjavik {
    --sky1: #050a1c; --sky2: #16223f; --sun: #e8eef8; --water1: #14264a; --water2: #08122a; --cloud: #28345a; --star: #e8eef8;
    --glow: rgb(245 200 110 / .35); --window-lit: #f5c76a;
    --rey-aur-op: .85; --rey-aur-peak: 1;
    --rey-mtn: #2a3a62; --rey-mtn2: #1f2d52; --rey-snow: #8da0c8; --rey-ground: #14233f;
    --rey-church: #9aa3bd; --rey-church2: #4e5878;
    --rey-h0: #8c3434; --rey-h1: #8f7a34; --rey-h2: #2d4f80; --rey-h3: #8a889a; --rey-h4: #2f6657; --rey-h5: #8a5230;
    --rey-r0: #5a2323; --rey-r1: #1f3050; --rey-r2: #30394f; --rey-r3: #4a2630;
    --rey-win: #f5c76a; --rey-quay: #26335a; --rey-ripple: #5f7aa8; --rey-hull: #1b2a4a; --rey-mast: #6b5a48; --rey-sail: #8b92a8;
    --rey-rain: rgb(159 176 200 / .35);
  }

  /* weather */
  .clouds { opacity: 0; transition: opacity 1s; }
  .mood-1 .clouds { opacity: .8; }
  .mood-2 .clouds { opacity: 1; fill: var(--rey-rain); }
  .mood-2 .sun { opacity: 0; }
  .mood-1 .sun { opacity: .5; }
  .aurora { opacity: var(--rey-aur-op); transform-box: fill-box; transform-origin: 50% 100%; }
  .mood-1 .aurora { opacity: calc(var(--rey-aur-op) * .55); }
  .mood-2 .aurora { opacity: calc(var(--rey-aur-op) * .15); }
  .sway { animation: sway 18s ease-in-out infinite alternate; transform-box: fill-box; transform-origin: 50% 100%; }
  @keyframes sway { from { transform: translateX(-14px) skewX(-2deg); } to { transform: translateX(14px) skewX(2deg) scaleY(1.08); } }
  .rainfall, .bolt, .flash, .rings { display: none; }
  .mood-2 .rainfall { display: block; animation: fall .9s linear infinite; }
  @keyframes fall { to { transform: translateY(540px); } }
  .mood-2 .bolt, .mood-2 .flash { display: block; opacity: 0; animation: bolt 9s linear infinite; }
  .mood-2 .flash { animation-name: flash; }
  @keyframes bolt { 0%, 52%, 58%, 100% { opacity: 0; } 53%, 55% { opacity: 1; } 54% { opacity: .2; } }
  @keyframes flash { 0%, 52%, 58%, 100% { opacity: 0; } 53% { opacity: .5; } 54% { opacity: .1; } 55% { opacity: .35; } }

  /* pulse, down: the lights go out and flicker back */
  .p-down .aurora { animation: a-out 5.4s ease both; }
  .p-down .chlit, .p-down .win { animation: l-out 5.4s steps(1, end) both; }
  @keyframes a-out { 0% { opacity: var(--rey-aur-op); } 14%, 78% { opacity: 0; transform: scaleY(.15); } 100% { opacity: var(--rey-aur-op); } }
  @keyframes l-out { 0%, 8% { opacity: 1; } 9%, 70% { opacity: 0; } 74% { opacity: 1; } 77% { opacity: 0; } 82%, 100% { opacity: 1; } }
  /* up: the aurora surges */
  .p-up .aurora { animation: a-up 5s ease-in-out both; }
  @keyframes a-up { 0% { opacity: var(--rey-aur-op); } 35% { opacity: var(--rey-aur-peak); transform: scaleY(1.45); } 100% { opacity: var(--rey-aur-op); } }
  /* resolved: rings from the spire, aurora through violet */
  .p-resolved .aurora { animation: a-res 5.4s ease-in-out both; }
  @keyframes a-res { 0%, 100% { opacity: var(--rey-aur-op); filter: hue-rotate(0); } 40% { opacity: var(--rey-aur-peak); filter: hue-rotate(75deg); } }
  .p-resolved .rings { display: block; }
  .rings circle { transform-box: fill-box; transform-origin: center; animation: ring 3.6s ease-out both; opacity: 0; }
  .rings circle:nth-child(2) { animation-delay: .7s; }
  .rings circle:nth-child(3) { animation-delay: 1.4s; }
  @keyframes ring { 0% { transform: scale(.15); opacity: .9; } 100% { transform: scale(3.2); opacity: 0; } }

  @media (prefers-reduced-motion: reduce) {
    .scene :global(*) { animation: none !important; }
    .mood-2 .bolt, .mood-2 .flash { display: none; }
  }
</style>
