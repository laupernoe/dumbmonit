<script lang="ts">
	/**
	 * Swiss calendar extras as an SVG group to drop inside a scene's <svg>:
	 * snow in winter, fireworks on 1 August, and on request the chocolate
	 * Escalade cauldron that cracks open (Geneva, around 12 December). The day
	 * comes from `day` (see `resolveSwissDay`). Decorative only. Under
	 * reduced motion the snow and fireworks are still pictures and the cauldron
	 * stays whole with its crack drawn.
	 */
	import { reducedMotion } from '#lib/ui/index.js';
	import type { SwissDay } from '#lib/scene/swiss.js';

	interface Area {
		x: number;
		y: number;
		w: number;
		h: number;
	}
	interface Props {
		day: SwissDay;
		/** Where flakes fall and fireworks burst. */
		area: Area;
		/** Show the Escalade cauldron here when it is the Escalade (Geneva only). */
		marmite?: { x: number; y: number; s?: number } | null;
		/** Scene units per flake, to match the scene's scale (the wall's stage is bigger). */
		unit?: number;
		paused?: boolean;
	}
	let { day, area, marmite = null, unit = 1, paused = false }: Props = $props();

	// Without motion the flakes hang scattered in the sky instead of lining up at the top.
	const still = reducedMotion();

	function rng(seed: number) {
		let s = seed;
		return () => {
			s = (s * 1664525 + 1013904223) % 4294967296;
			return s / 4294967296;
		};
	}
	const FLAKES = (() => {
		const r = rng(11);
		return Array.from({ length: 56 }, () => ({
			x: r(),
			y: r(),
			r: 0.8 + r() * 1.7,
			dur: 7 + r() * 8,
			delay: -r() * 14,
			sway: 6 + r() * 14,
			o: 0.55 + r() * 0.4
		}));
	})();

	const COLORS = ['#eb0000', '#ffc41f', '#d01a1a', '#ffb000', '#eb0000'];
	const BURSTS = [
		{ x: 0.2, y: 0.28, d: 0 },
		{ x: 0.5, y: 0.16, d: 1.4 },
		{ x: 0.78, y: 0.3, d: 2.7 },
		{ x: 0.36, y: 0.4, d: 4 },
		{ x: 0.64, y: 0.12, d: 5.1 }
	];
	const RAYS = Array.from({ length: 16 }, (_, i) => {
		const a = (i / 16) * Math.PI * 2;
		const c = Math.cos(a);
		const s = Math.sin(a);
		return `M${(c * 10).toFixed(1)} ${(s * 10).toFixed(1)}L${(c * 38).toFixed(1)} ${(s * 38).toFixed(1)}`;
	}).join('');
	const SPARKS = Array.from({ length: 16 }, (_, i) => {
		const a = ((i + 0.5) / 16) * Math.PI * 2;
		return { x: Math.cos(a) * 46, y: Math.sin(a) * 46 };
	});
</script>

{#if !paused}
	{#if day.winter}
		<g class="snow" fill="#ffffff" pointer-events="none">
			{#each FLAKES as f, i (i)}
				<circle
					cx={area.x + f.x * area.w}
					cy={still ? area.y + f.y * area.h : area.y}
					r={f.r * unit}
					opacity={f.o}
					style:--fall="{area.h}px"
					style:--sway="{f.sway * unit}px"
					style:animation-duration="{f.dur}s"
					style:animation-delay="{f.delay}s"
				/>
			{/each}
		</g>
	{/if}

	{#if day.nationalDay}
		<g class="fireworks" pointer-events="none">
			{#each BURSTS as b, i (i)}
				<g transform="translate({area.x + b.x * area.w} {area.y + b.y * area.h}) scale({unit * 1.3})">
					<g class="fw" style:color={COLORS[i]} style:animation-delay="{b.d}s">
						<path d={RAYS} stroke="currentColor" stroke-width="3" stroke-linecap="round" />
						{#each SPARKS as s, k (k)}
							<circle cx={s.x} cy={s.y} r="2.6" fill="currentColor" />
						{/each}
					</g>
				</g>
			{/each}
		</g>
	{/if}

	{#if day.escalade && marmite}
		<g class="marmite" transform="translate({marmite.x} {marmite.y}) scale({marmite.s ?? 1})" pointer-events="none">
			<defs>
				<clipPath id="sw-mar-l"><rect x="-70" y="-80" width="70" height="170" /></clipPath>
				<clipPath id="sw-mar-r"><rect x="0" y="-80" width="70" height="170" /></clipPath>
				<symbol id="sw-bowl" overflow="visible">
					<path d="M-42 -26H42C42 22 22 44 0 44C-22 44 -42 22 -42 -26z" fill="#5b3321" />
					<path d="M-42 -26H42V-16H-42z" fill="#eb0000" />
					<path d="M-40 -10H40" stroke="#f1c24a" stroke-width="3" />
					<path d="M-32 8C-26 28 -12 38 0 38" stroke="#7a4a30" stroke-width="3" stroke-linecap="round" opacity=".8" />
					<path d="M-52 -26H52" stroke="#3f2214" stroke-width="7" stroke-linecap="round" />
					<path d="M-26 44l-6 14M26 44l6 14" stroke="#3f2214" stroke-width="6" stroke-linecap="round" />
				</symbol>
			</defs>
			<g class="veg">
				<circle cx="-18" cy="-36" r="9" fill="#e8742a" />
				<circle cx="2" cy="-42" r="10" fill="#6aa84f" />
				<circle cx="20" cy="-34" r="8" fill="#d63b3b" />
				<circle cx="-4" cy="-30" r="7" fill="#f4eed8" />
			</g>
			<g clip-path="url(#sw-mar-l)"><g class="half left"><use href="#sw-bowl" /></g></g>
			<g clip-path="url(#sw-mar-r)"><g class="half right"><use href="#sw-bowl" /></g></g>
			<path class="crack" d="M0 -26L-5 -10L4 4L-4 20L2 44" stroke="#1d0f08" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" fill="none" pathLength="100" />
		</g>
	{/if}
{/if}

<style>
	.snow circle {
		transform-box: fill-box;
		opacity: 0.8;
	}
	.fw {
		opacity: 0.85;
	}
	.crack {
		stroke-dasharray: 100;
		stroke-dashoffset: 0;
	}
	@media (prefers-reduced-motion: no-preference) {
		.snow circle {
			animation: sw-fall linear infinite;
		}
		.fw {
			opacity: 0;
			transform-box: fill-box;
			transform-origin: center;
			animation: sw-burst 6.5s ease-out infinite;
		}
		.half {
			animation: sw-split 14s ease-in-out infinite;
		}
		.half.left {
			transform-origin: 0 0;
		}
		.crack {
			animation: sw-crack 14s ease-in-out infinite;
		}
		.veg {
			animation: sw-veg 14s ease-in infinite;
		}
	}
	@keyframes sw-fall {
		0% { transform: translate(0, 0); }
		50% { transform: translate(var(--sway), calc(var(--fall) * 0.5)); }
		100% { transform: translate(0, var(--fall)); }
	}
	@keyframes sw-burst {
		0% { opacity: 0; transform: scale(0.15); }
		6% { opacity: 1; }
		40% { opacity: 1; transform: scale(1.15); }
		55%, 100% { opacity: 0; transform: scale(1.4) translateY(10px); }
	}
	@keyframes sw-crack {
		0%, 22% { stroke-dashoffset: 100; opacity: 1; }
		48%, 70% { stroke-dashoffset: 0; opacity: 1; }
		74%, 100% { stroke-dashoffset: 0; opacity: 0; }
	}
	@keyframes sw-split {
		0%, 50% { transform: translate(0, 0) rotate(0); opacity: 1; }
		60%, 86% { transform: translate(var(--dx, 0), 20px) rotate(var(--rot, 0)); opacity: 1; }
		94%, 100% { transform: translate(var(--dx, 0), 30px) rotate(var(--rot, 0)); opacity: 0; }
	}
	.half.left { --dx: -26px; --rot: -22deg; }
	.half.right { --dx: 26px; --rot: 22deg; }
	@keyframes sw-veg {
		0%, 50% { transform: translateY(0); opacity: 1; }
		58% { transform: translateY(-30px); }
		86% { transform: translateY(30px); opacity: 1; }
		94%, 100% { transform: translateY(40px); opacity: 0; }
	}
</style>
