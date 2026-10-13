<script lang="ts">
	/**
	 * Hong Kong by night — the Central skyline under Victoria Peak, the Star
	 * Ferry crossing the harbour, neon signs and a red taxi on our side of the
	 * water, two DumbMonit pigeons on a rooftop, and a harbour buoy whose lamp
	 * takes the page's status colour (`--scene-tone`, set by the page).
	 *
	 * `mood` is the weather (ok: clear night, warn: cloud, critical: rain and
	 * lightning). `pulse` plays one gesture whenever its `at` changes: the
	 * neon dies on `down`, flickers back on `up`, and `resolved` lights
	 * fireworks over the harbour. Fills its positioned parent; the headline
	 * zone (top left) is left clear. Reduced motion freezes everything.
	 */
	import { tick, untrack } from 'svelte';

	interface Props {
		mood?: 'ok' | 'warn' | 'critical';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}
	let { mood = 'ok', pulse = null }: Props = $props();

	function rng(seed: number) {
		let s = seed;
		return () => {
			s = (s * 1664525 + 1013904223) % 4294967296;
			return s / 4294967296;
		};
	}

	// Skyline: [x, width, top]. Taller toward Central on the right.
	const make = (seed: number, from: number, to: number, lo: number, hi: number) => {
		const r = rng(seed);
		const out: [number, number, number][] = [];
		for (let x = from; x < to; ) {
			const w = 26 + Math.round(r() * 30);
			const t = (x - 560) / 1220;
			const h = lo + r() * (hi - lo) * (0.5 + t);
			out.push([x, w, Math.round(470 - h)]);
			x += w + Math.round(r() * 4);
		}
		return out;
	};
	const far = make(11, 590, 1790, 60, 150);
	const near = make(29, 600, 1790, 40, 110);

	const RAYS = Array.from({ length: 14 }, (_, i) => {
		const a = (i / 14) * Math.PI * 2;
		return `M${Math.round(Math.cos(a) * 12)} ${Math.round(Math.sin(a) * 12)}L${Math.round(Math.cos(a) * 46)} ${Math.round(Math.sin(a) * 46)}`;
	}).join('');
	const DROPS = (() => {
		const r = rng(5);
		let d = '';
		for (let i = 0; i < 90; i++) {
			const x = 540 + Math.round(r() * 1300);
			const y = 60 + Math.round(r() * 600);
			d += `M${x} ${y}l-8 22`;
		}
		return d;
	})();
	const BURSTS = [
		{ x: 1090, y: 220, c: 'var(--n-amber)', d: 0 },
		{ x: 1250, y: 170, c: 'var(--n-pink)', d: 500 },
		{ x: 1420, y: 240, c: 'var(--n-cyan)', d: 1000 }
	];
	const REFL = Array.from({ length: 22 }, (_, i) => ({
		x: 640 + i * 52 + (i % 3) * 9,
		w: 3 + (i % 4),
		c: ['--n-red', '--n-cyan', '--win', '--n-pink', '--win'][i % 5],
		d: (i * 370) % 3200
	}));

	// Replay the gesture each time `at` changes (not for a pulse already there at mount).
	let active = $state<'down' | 'up' | 'resolved' | null>(null);
	let playKey = $state(0);
	let seen: number | null = untrack(() => pulse?.at ?? null);
	let timer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const p = pulse;
		if (!p || p.at === seen) return;
		seen = p.at;
		active = null;
		clearTimeout(timer);
		void tick().then(() => {
			active = p.kind;
			playKey++;
			timer = setTimeout(() => (active = null), p.kind === 'resolved' ? 6500 : 5000);
		});
	});
	$effect(() => () => clearTimeout(timer));
</script>

<div
	class="scene scene-hongkong"
	class:p-down={active === 'down'}
	class:p-up={active === 'up'}
	class:m-warn={mood === 'warn'}
	class:m-crit={mood === 'critical'}
	aria-hidden="true"
>
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<pattern id="hk-win" width="10" height="12" patternUnits="userSpaceOnUse">
				<rect x="3" y="3" width="4" height="5" fill="var(--win)" />
			</pattern>
			<pattern id="hk-win2" width="14" height="10" patternUnits="userSpaceOnUse">
				<rect x="3" y="3" width="6" height="3" fill="var(--win2)" />
			</pattern>
			<linearGradient id="hk-sky" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--sky1)" /><stop offset=".6" stop-color="var(--sky2)" /><stop offset="1" stop-color="var(--sky3)" />
			</linearGradient>
			<linearGradient id="hk-sea" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--sea1)" /><stop offset="1" stop-color="var(--sea2)" />
			</linearGradient>
			<symbol id="hk-burst" overflow="visible"><path d={RAYS} stroke="currentColor" stroke-width="3" stroke-linecap="round" /></symbol>
		</defs>

		<rect x="540" y="60" width="1260" height="560" fill="url(#hk-sky)" />
		<g class="stars" fill="var(--star)"><circle cx="1000" cy="120" r="1.8" /><circle cx="1160" cy="140" r="1.5" /><circle cx="1320" cy="104" r="2" /><circle cx="1560" cy="130" r="1.6" /><circle cx="1700" cy="190" r="1.8" /><circle cx="900" cy="200" r="1.4" /><circle cx="1450" cy="180" r="1.4" /></g>
		<g class="moon"><circle cx="1090" cy="170" r="22" fill="var(--moon)" /><circle cx="1100" cy="162" r="20" fill="var(--sky2)" opacity=".92" /></g>

		<!-- Victoria Peak -->
		<path d="M960 472C1080 436 1190 340 1290 290C1340 266 1420 272 1480 304C1580 354 1700 424 1790 472z" fill="var(--hill)" />
		<path d="M1304 284v-26" stroke="var(--far)" stroke-width="3" /><circle cx="1304" cy="256" r="3" fill="var(--n-red)" class="neon" />

		<!-- far towers -->
		<g fill="var(--far)">
			{#each far as [x, w, t]}<rect {x} y={t} width={w} height={472 - t} />{/each}
		</g>
		<g fill="url(#hk-win2)" class="lights">
			{#each far as [x, w, t]}<rect {x} y={t} width={w} height={472 - t} />{/each}
		</g>

		<!-- Central landmarks -->
		<g>
			<path d="M1310 472V300l22-60 22 60v172z" fill="var(--tower)" /><path d="M1332 240V478" stroke="var(--far)" stroke-width="2" />
			<path d="M1332 240v-30" stroke="var(--tower)" stroke-width="3" />
			<path d="M1400 472V230h44v242z" fill="var(--tower)" /><path d="M1422 230v-52" stroke="var(--tower)" stroke-width="3" />
			<g class="neon"><rect x="1400" y="262" width="44" height="6" fill="var(--n-cyan)" /><rect x="1400" y="274" width="44" height="6" fill="var(--n-pink)" /><circle cx="1422" cy="178" r="3.5" fill="var(--n-red)" /></g>
			<path d="M1500 472V168h42v304z" fill="var(--tower)" /><path d="M1500 168l8-14h26l8 14z" fill="var(--tower)" /><path d="M1521 154v-26" stroke="var(--tower)" stroke-width="3" />
			<circle class="neon" cx="1521" cy="126" r="3.5" fill="var(--n-red)" />
			<rect x="1500" y="168" width="42" height="304" fill="url(#hk-win)" class="lights" />
			<rect x="1400" y="230" width="44" height="242" fill="url(#hk-win)" class="lights" />
			<rect x="1310" y="300" width="44" height="172" fill="url(#hk-win)" class="lights" />
		</g>

		<!-- Laser beams of the Symphony of Lights -->
		<g class="neon beams" stroke-width="3" stroke-linecap="round" opacity=".8">
			<path d="M1522 124L1700 40" stroke="var(--n-cyan)" /><path d="M1422 176L1250 90" stroke="var(--n-pink)" />
		</g>

		<g fill="var(--near)">
			{#each near as [x, w, t]}<rect {x} y={t + 40} width={w} height={432 - t} />{/each}
		</g>
		<g fill="url(#hk-win)" class="lights">
			{#each near as [x, w, t]}<rect {x} y={t + 40} width={w} height={432 - t} />{/each}
		</g>
		<rect x="540" y="470" width="1260" height="6" fill="var(--quay)" />

		<!-- harbour -->
		<rect x="540" y="476" width="1260" height="150" fill="url(#hk-sea)" />
		<g opacity=".4">
			{#each REFL as r}<rect class="refl" x={r.x} y="480" width={r.w} height="64" rx="2" style="fill:var({r.c});animation-delay:-{r.d}ms" />{/each}
		</g>
		<g stroke="var(--ripple)" stroke-width="2" stroke-linecap="round" opacity=".45"><path d="M680 520h70M900 548h90M1160 510h100M1420 540h80M1600 514h90M780 585h100" /></g>

		<!-- the Star Ferry -->
		<g class="ferry">
			<g class="bob">
				<path d="M-62 0H64L52 17H-50z" fill="var(--hull)" />
				<path d="M-58 -5H60v5H-58z" fill="var(--ferry-white)" />
				<rect x="-48" y="-22" width="92" height="17" fill="var(--ferry-white)" />
				<rect x="-36" y="-40" width="64" height="18" fill="var(--ferry-white)" /><path d="M-40 -40h72l-6-6h-60z" fill="var(--hull)" />
				<g class="neon" fill="var(--win)"><rect x="-42" y="-18" width="14" height="8" /><rect x="-22" y="-18" width="14" height="8" /><rect x="-2" y="-18" width="14" height="8" /><rect x="18" y="-18" width="14" height="8" /><rect x="-30" y="-36" width="12" height="8" /><rect x="-12" y="-36" width="12" height="8" /><rect x="6" y="-36" width="12" height="8" /></g>
				<path d="M-2 -46v-18" stroke="var(--hull)" stroke-width="3" /><circle cx="-2" cy="-66" r="3.2" fill="var(--n-amber)" class="neon" />
				<path d="M-64 6C-90 8-110 14-132 20M-62 12C-86 14-102 20-118 26" stroke="var(--ripple)" stroke-width="2" stroke-linecap="round" opacity=".6" />
			</g>
		</g>

		<!-- buoy: the page's status colour -->
		<g transform="translate(1130 566)"><path d="M-8 0h16l-3 -14h-10z" fill="var(--hull)" /><path d="M0 -14v-12" stroke="var(--hull)" stroke-width="2" /><circle cy="-30" r="5" style="fill:var(--scene-tone, var(--n-green))" /><circle cy="-30" r="11" style="fill:var(--scene-tone, var(--n-green))" opacity=".25" /></g>

		<!-- our bank: a building with neon, the street, a red taxi -->
		<g>
			<rect x="640" y="380" width="170" height="240" fill="var(--bank)" />
			<rect x="632" y="372" width="186" height="10" fill="var(--bank2)" />
			<rect x="650" y="400" width="150" height="200" fill="url(#hk-win)" class="lights" />
			<g class="neon sign-a">
				<rect x="810" y="396" width="26" height="92" rx="3" fill="none" stroke="var(--n-red)" stroke-width="4" />
				<text x="823" y="432" text-anchor="middle" font-size="22" font-weight="700" fill="var(--n-red)">酒</text>
				<text x="823" y="462" text-anchor="middle" font-size="22" font-weight="700" fill="var(--n-red)">家</text>
			</g>
			<g class="neon sign-b">
				<rect x="780" y="500" width="64" height="26" rx="13" fill="none" stroke="var(--n-cyan)" stroke-width="4" />
				<text x="812" y="519" text-anchor="middle" font-size="16" font-weight="700" fill="var(--n-cyan)">茶 餐 廳</text>
			</g>
			<g class="neon sign-c"><rect x="660" y="540" width="100" height="5" rx="2.5" fill="var(--n-pink)" /><rect x="676" y="552" width="68" height="5" rx="2.5" fill="var(--n-amber)" /></g>
			<use href="#dm-mascot" transform="translate(690 342) scale(.1)" />
			<use href="#dm-mascot" transform="translate(766 342) scale(-.1 .1)" />
		</g>
		<rect x="540" y="590" width="1260" height="40" fill="var(--street)" />
		<path d="M540 604h1260" stroke="var(--lane)" stroke-width="2" stroke-dasharray="26 22" />
		<g transform="translate(980 600)">
			<path d="M-46 0v-14c0-4 3-6 7-7l14-3 10-12h32l10 12 14 3c4 1 7 3 7 7v14z" fill="var(--taxi)" />
			<path d="M-20 -24l9-10h26l9 10z" fill="var(--taxi-glass)" />
			<rect x="-8" y="-40" width="16" height="6" rx="2" fill="var(--win)" class="neon" />
			<circle cx="-26" cy="0" r="8" fill="var(--street)" /><circle cx="30" cy="0" r="8" fill="var(--street)" />
			<circle class="neon" cx="-44" cy="-10" r="3" fill="var(--win)" /><circle class="neon" cx="50" cy="-10" r="3" fill="var(--n-red)" />
		</g>

		<!-- weather -->
		<g class="clouds">
			<ellipse cx="900" cy="150" rx="150" ry="22" /><ellipse cx="1040" cy="132" rx="110" ry="20" />
			<ellipse cx="1360" cy="120" rx="170" ry="24" /><ellipse cx="1620" cy="160" rx="140" ry="22" /><ellipse cx="1200" cy="220" rx="130" ry="18" />
		</g>
		<rect class="veil" x="540" y="60" width="1260" height="560" />
		<path class="rain" d={DROPS} />
		<path class="bolt" d="M1480 80l-26 90 22-4-30 110 56-120-26 4z" />
		<rect class="flash" x="540" y="60" width="1260" height="560" />

		{#if active === 'resolved'}
			{#key playKey}
				{#each BURSTS as b}
					<g transform="translate({b.x} {b.y})" style="color:{b.c}"><g class="burst" style="animation-delay:{b.d}ms"><use href="#hk-burst" /></g></g>
				{/each}
			{/key}
		{/if}
	</svg>
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: var(--sky1);
	}
	.scene svg {
		display: block;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 80%, transparent 99%);
	}
	.scene-hongkong {
		--sky1: #1e2a5c; --sky2: #5b4a8a; --sky3: #e58c86; --star: #fff; --moon: #fdf3d8;
		--hill: #2f3866; --far: #3d4478; --tower: #262b57; --near: #1d2147; --quay: #161a3a;
		--sea1: #2a2f66; --sea2: #10143a; --ripple: #aab4ee; --hull: #14233a; --ferry-white: #f1f0e6;
		--win: #ffd98a; --win2: #8fe6ff; --bank: #1a1d3f; --bank2: #2a2e58; --street: #12142e; --lane: #6b6f9e;
		--taxi: #e0342f; --taxi-glass: #ffd98a;
		--n-red: #ff4d6d; --n-cyan: #3ee8ff; --n-pink: #ff62d6; --n-green: #6bff9a; --n-amber: #ffb84a;
	}
	:global(.dark) .scene-hongkong {
		--sky1: #05071a; --sky2: #14153a; --sky3: #3b1f4d; --moon: #e8e4d6;
		--hill: #0f1230; --far: #12163a; --tower: #0b0e28; --near: #090b21; --quay: #06081a;
		--sea1: #0c1130; --sea2: #040618; --ripple: #5b66b0; --hull: #0a1426; --ferry-white: #cfd2d8;
		--win: #ffc25c; --win2: #5fd3ee; --bank: #0b0d24; --bank2: #1a1d44; --street: #070817; --lane: #3d4170;
		--taxi: #b8252a; --taxi-glass: #e8a850;
	}

	.neon {
		filter: drop-shadow(0 0 3px currentColor);
	}
	.sign-a { color: var(--n-red); }
	.sign-b { color: var(--n-cyan); }
	.sign-c { color: var(--n-pink); }
	.beams { transform-box: fill-box; transform-origin: 50% 100%; }

	.clouds { fill: var(--sky2); opacity: 0; transition: opacity 2s; }
	.m-warn .clouds { opacity: 0.75; }
	.m-crit .clouds { opacity: 0.95; fill: #121631; }
	.veil { fill: #060818; opacity: 0; transition: opacity 2s; }
	.m-warn .veil { opacity: 0.12; }
	.m-crit .veil { opacity: 0.38; }
	.stars, .moon { transition: opacity 2s; }
	.m-warn .stars, .m-crit .stars, .m-crit .moon { opacity: 0.25; }
	.rain { stroke: var(--ripple); stroke-width: 1.6; stroke-linecap: round; opacity: 0; transition: opacity 2s; }
	.m-crit .rain { opacity: 0.45; }
	.bolt { fill: #fff; opacity: 0; }
	.flash { fill: #cfd8ff; opacity: 0; }
	.ferry { transform: translate(1000px, 518px); }
	.burst { opacity: 0; }

	@media (prefers-reduced-motion: no-preference) {
		.ferry { animation: sail 80s linear infinite; }
		.bob { animation: bob 4s ease-in-out infinite; }
		.refl { animation: shimmer 3.2s ease-in-out infinite alternate; }
		.beams { animation: sweep 9s ease-in-out infinite alternate; }
		.m-crit .rain { animation: fall 0.7s linear infinite; }
		.m-crit .flash { animation: flash 8s linear infinite; }
		.m-crit .bolt { animation: bolt 8s linear infinite; }
		.p-down .neon, .p-down .lights { animation: blackout 5s ease-out both; }
		.p-up .neon, .p-up .lights { animation: relight 2.4s ease-out both; }
		.burst { transform-box: fill-box; transform-origin: center; animation: burst 2.6s ease-out both; }
	}
	@keyframes sail { from { transform: translate(600px, 518px); } to { transform: translate(1780px, 518px); } }
	@keyframes bob { 50% { transform: translateY(2.5px) rotate(0.6deg); } }
	@keyframes shimmer { to { opacity: 0.25; transform: scaleY(0.8); } }
	@keyframes sweep { to { transform: rotate(10deg); } }
	@keyframes fall { to { transform: translate(-16px, 44px); } }
	@keyframes flash {
		0%, 91%, 100% { opacity: 0; } 92% { opacity: 0.55; } 93% { opacity: 0.05; } 94% { opacity: 0.4; } 96% { opacity: 0; }
	}
	@keyframes bolt { 0%, 91.5%, 94.5%, 100% { opacity: 0; } 92%, 94% { opacity: 0.95; } }
	@keyframes blackout {
		0% { opacity: 1; } 8% { opacity: 0.15; } 14% { opacity: 0.9; } 22%, 80% { opacity: 0.04; } 100% { opacity: 1; }
	}
	@keyframes relight {
		0% { opacity: 0.04; } 14% { opacity: 1; } 22% { opacity: 0.1; } 34% { opacity: 1; } 42% { opacity: 0.3; } 54%, 100% { opacity: 1; }
	}
	@keyframes burst {
		0% { opacity: 1; transform: scale(0.1); } 70% { opacity: 1; } 100% { opacity: 0; transform: scale(1.6) translateY(14px); }
	}
</style>
