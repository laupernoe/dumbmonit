<script lang="ts">
	/**
	 * What the panoramic windows look at: sky, sun or moon, stars, drifting
	 * clouds, snowy peaks, hills, a lake with chalets, a Landwasser-like
	 * viaduct with a red train, fir-covered slopes and telegraph poles, all
	 * scrolling at their own speed (parallax). A nested <svg> of W x H user
	 * units at (x, y); its parent supplies the colours (`--sky-*`, `--tr-*`).
	 *
	 * The speed is one number eased towards a target, so a pulse is a gesture
	 * in speed: `down` brakes to a stop and holds a few seconds, `up` pulls
	 * away with a surge, `resolved` comes out of a tunnel into bright light.
	 * Weather by `level`: 0 clear, 1 clouds and a dim veil, 2 storm (rain on
	 * the glass, lightning). Reduced motion: a still picture, no gestures.
	 */
	import { untrack, type Snippet } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import { buildLandscape, type MoodLevel, type Pulse, type PulseKind } from './geometry.js';

	interface Props {
		x: number;
		y: number;
		W: number;
		H: number;
		level: MoodLevel;
		pulse?: Pulse;
		paused?: boolean;
		/** Local to this box. */
		sun?: { x: number; y: number } | null;
		moon?: { x: number; y: number } | null;
		idp?: string;
	}

	let { x, y, W, H, level, pulse = null, paused = false, sun = null, moon = null, idp = 'tr' }: Props = $props();

	const motion = !reducedMotion();
	const geo = $derived(buildLandscape(W, H));
	const u = $derived(geo.u);

	// --- Speed ----------------------------------------------------------------------

	const els: (SVGGElement | undefined)[] = [];
	let world: SVGGElement | undefined;
	const offsets = [0, 0, 0, 0, 0, 0, 0];
	let speed = 1;
	let target = 1;
	let tau = 1.2;
	let phase = 0;
	let timers: ReturnType<typeof setTimeout>[] = [];

	function later(fn: () => void, ms: number) {
		timers.push(setTimeout(fn, ms));
	}

	function play(kind: PulseKind) {
		for (const t of timers.splice(0)) clearTimeout(t);
		if (kind === 'down') {
			// Brakes: the landscape slows to a stop, waits, then the train sets off again.
			target = 0;
			tau = 0.55;
			later(() => {
				target = 1;
				tau = 1.8;
			}, 5000);
		} else if (kind === 'up') {
			// Sets off again: a surge, then cruising speed.
			target = 2.3;
			tau = 0.7;
			later(() => {
				target = 1;
				tau = 1.4;
			}, 1600);
		} else {
			// Out of the tunnel: a little faster, into the light.
			target = 1.6;
			tau = 0.9;
			later(() => {
				target = 1;
				tau = 1.6;
			}, 2800);
		}
	}

	let seenAt: number | null | undefined = untrack(() => pulse?.at);
	let tunnelKey = $state(0);
	$effect(() => {
		const p = pulse;
		if (!p || p.at === seenAt) return;
		seenAt = p.at;
		if (!motion) return;
		play(p.kind);
		if (p.kind === 'resolved') tunnelKey += 1;
	});

	$effect(() => {
		if (!motion || paused) return;
		const layers = geo.layers;
		const unit = geo.u;
		let raf = 0;
		let last = performance.now();
		const frame = (now: number) => {
			const dt = Math.min(0.1, (now - last) / 1000);
			last = now;
			speed += (target - speed) * (1 - Math.exp(-dt / tau));
			if (Math.abs(target - speed) < 0.002) speed = target;
			for (let i = 0; i < layers.length; i++) {
				offsets[i] = (offsets[i] + layers[i].v * unit * speed * dt) % layers[i].w;
				els[i]?.setAttribute('transform', `translate(${(-offsets[i]).toFixed(2)} 0)`);
			}
			// The rails: a faint sway that dies with the speed.
			phase += dt * 9 * speed;
			world?.setAttribute('transform', `translate(0 ${(Math.sin(phase) * 0.55 * unit * Math.min(speed, 1.3)).toFixed(2)})`);
			raf = requestAnimationFrame(frame);
		};
		raf = requestAnimationFrame(frame);
		return () => cancelAnimationFrame(raf);
	});

	$effect(() => () => {
		for (const t of timers.splice(0)) clearTimeout(t);
	});

	const FIR = 'M0-100L12-66H7L17-36H9L22-6H3V0H-3V-6H-22L-9-36H-17L-7-66H-12Z';
	const veilOpacity = $derived(level === 2 ? 0.34 : level === 1 ? 0.16 : 0);
	const cx = $derived(W / 2);
	const cy = $derived(H * 0.5);
</script>

{#snippet layer(i: number, body: Snippet)}
	<g bind:this={els[i]}>
		{#each [0, 1] as k (k)}
			<g transform="translate({k * geo.layers[i].w} 0)">{@render body()}</g>
		{/each}
	</g>
{/snippet}

{#snippet fir(f: { x: number; y: number; s: number; v: 0 | 1 })}
	<use href="#{idp}-fir" class="f{f.v}" transform="translate({f.x} {f.y}) scale({f.s})" />
{/snippet}

{#snippet clouds()}
	{#each geo.clouds as c, i (i)}
		<g class="cloud" class:cloud--on={level >= c.lv} transform="translate({c.x} {c.y}) scale({c.s})">
			<path d="M-70 0C-74-18-46-26-34-16C-28-40 12-44 22-20C42-32 70-16 62 0Z" />
		</g>
	{/each}
{/snippet}

{#snippet far()}
	<path d={geo.far.d} fill="var(--tr-far)" />
	<path d={geo.far.caps} fill="var(--tr-snow)" />
{/snippet}

{#snippet mid()}
	<path d={geo.back.d} fill="var(--tr-back)" />
	<path d={geo.mid.d} fill="var(--tr-mid)" />
	{#each geo.mid.firs as f, i (i)}{@render fir(f)}{/each}
{/snippet}

{#snippet lake()}
	<rect x="0" y={geo.lake.y - 6 * u} width={geo.layers[3].w} height={H} fill="var(--tr-meadow)" />
	<path d={geo.lake.d} fill="var(--tr-lake)" />
	{#each geo.lake.glints as g, i (i)}
		<rect class="glint" style:animation-delay="{g.d}s" x={g.x} y={g.y} width={g.w} height={1.6 * u} rx={u} fill="var(--tr-lake-hi)" />
	{/each}
	{#each geo.lake.chalets as c, i (i)}
		<g transform="translate({c.x} {c.y}) scale({c.s})">
			<rect x="-9" y="-9" width="18" height="9" fill="var(--tr-wood)" />
			<path d="M-12-9L0-18L12-9Z" fill="var(--tr-wood-2)" />
			<rect x="-2" y="-6" width="4" height="6" fill="var(--tr-wood-2)" />
		</g>
	{/each}
{/snippet}

{#snippet viaduct()}
	{@const v = geo.via}
	<path d={v.rockL} fill="var(--tr-rock)" />
	<path d={v.rockR} fill="var(--tr-rock)" />
	<path d={v.portal} fill="var(--tr-tunnel)" />
	{#each v.firs as f, i (i)}{@render fir(f)}{/each}
	<path d={v.piers} fill="var(--tr-stone)" />
	<path d={v.shade} fill="var(--tr-stone-shade)" />
	<path d={v.rings} fill="var(--tr-stone)" />
	<rect x={v.deck.x} y={v.deck.y} width={v.deck.w} height={v.deck.h} fill="var(--tr-stone)" />
	<rect x={v.deck.x} y={v.deck.y + v.deck.h - u * 1.5} width={v.deck.w} height={u * 1.5} fill="var(--tr-stone-shade)" />
	{#each v.cars as c, i (i)}
		<rect x={c.x} y={c.y} width={c.w} height={c.h} rx={u * 1.5} fill="var(--tr-train)" />
		<rect x={c.x + c.w * 0.1} y={c.y + c.h * 0.2} width={c.w * 0.8} height={c.h * 0.32} fill="var(--tr-light)" opacity="0.8" />
	{/each}
{/snippet}

{#snippet near()}
	<path d={geo.near.back} fill="var(--tr-near-2)" />
	{#each geo.near.firs as f, i (i)}{@render fir(f)}{/each}
	<path d={geo.near.front} fill="var(--tr-near)" />
{/snippet}

{#snippet poles()}
	{@const p = geo.poles}
	{#each p.xs as px, i (i)}
		{@const nx = (p.xs[i + 1] ?? p.w) }
		{#each [-14, 0, 14] as dx (dx)}
			<path
				d="M{px + dx * u} {p.top + 7 * u}Q{(px + nx) / 2 + dx * u} {p.top + 42 * u} {nx + dx * u} {p.top + 7 * u}"
				fill="none"
				stroke="var(--tr-pole)"
				stroke-width={0.9 * u}
				opacity="0.75"
			/>
		{/each}
		<rect x={px - 1.8 * u} y={p.top} width={3.6 * u} height={p.bottom - p.top} fill="var(--tr-pole)" />
		<rect x={px - 18 * u} y={p.top + 5 * u} width={36 * u} height={3 * u} fill="var(--tr-pole)" />
	{/each}
{/snippet}

<svg {x} {y} width={W} height={H} viewBox="0 0 {W} {H}" class="land lv{level}" focusable="false">
	<defs>
		<linearGradient id="{idp}-sky" x1="0" y1="0" x2="0" y2="1">
			<stop offset="0" style="stop-color:var(--sky-top)" />
			<stop offset="1" style="stop-color:var(--sky-bottom)" />
		</linearGradient>
		<path id="{idp}-fir" d={FIR} />
		<pattern id="{idp}-rain" width="180" height="135" patternUnits="userSpaceOnUse">
			<g stroke="var(--rain, rgb(255 255 255 / 0.45))" stroke-width="1.4" stroke-linecap="round" fill="none">
				<path d="M30 20l-26 19.5M110 6l-26 19.5M150 70l-26 19.5M64 92l-26 19.5M170 118l-26 19.5" />
			</g>
		</pattern>
		<radialGradient id="{idp}-hole">
			<stop offset="0" stop-color="#000" />
			<stop offset="0.62" stop-color="#000" />
			<stop offset="1" stop-color="#fff" />
		</radialGradient>
		<radialGradient id="{idp}-glow">
			<stop offset="0" stop-color="#fff" stop-opacity="1" />
			<stop offset="0.55" stop-color="#fff" stop-opacity="0.75" />
			<stop offset="1" stop-color="#fff" stop-opacity="0" />
		</radialGradient>
	</defs>

	<rect width={W} height={H} fill="url(#{idp}-sky)" />
	<g class="stars">
		{#each geo.stars as s, i (i)}<circle cx={s.x} cy={s.y} r={s.r} fill="var(--star, #fff)" />{/each}
	</g>
	{#if sun}
		<g class="body" style:opacity="var(--tr-sun-o, 1)">
			<circle cx={sun.x} cy={sun.y} r={74 * u} fill="var(--sun-halo, transparent)" />
			<circle cx={sun.x} cy={sun.y} r={30 * u} fill="var(--sun)" />
		</g>
	{/if}
	{#if moon}
		<g class="body" style:opacity="var(--tr-moon-o, 1)">
			<circle cx={moon.x} cy={moon.y} r={44 * u} fill="var(--moon)" opacity="0.14" />
			<circle cx={moon.x} cy={moon.y} r={22 * u} fill="var(--moon)" />
		</g>
	{/if}

	<g bind:this={world}>
		{@render layer(0, clouds)}
		{@render layer(1, far)}
		{@render layer(2, mid)}
		{@render layer(3, lake)}
		{@render layer(4, viaduct)}
		{@render layer(5, near)}
		{@render layer(6, poles)}
	</g>

	<rect width={W} height={H} fill="var(--tr-tint, transparent)" opacity="var(--tr-tint-o, 0)" />
	<rect class="veil" width={W} height={H} fill="var(--tr-veil, #566075)" style:opacity={veilOpacity} />

	{#if level === 2}
		<g class="storm">
			<g class="rain"><rect x="-180" y="-135" width={W + 360} height={H + 270} fill="url(#{idp}-rain)" /></g>
			{#each geo.drops as d, i (i)}
				<ellipse
					class="drop"
					cx={d.x}
					cy={d.y}
					rx={d.r}
					ry={d.r * 1.5}
					fill="#fff"
					style:--dy="{d.dy}px"
					style:animation-duration="{d.dur}s"
					style:animation-delay="{d.delay}s"
				/>
			{/each}
			<rect class="flash" width={W} height={H} fill="#f4f6ff" />
		</g>
	{/if}

	{#if tunnelKey > 0}
		{#key tunnelKey}
			<g class="tunnel">
				<mask id="{idp}-tm-{tunnelKey}" maskUnits="userSpaceOnUse" x="0" y="0" width={W} height={H}>
					<rect width={W} height={H} fill="#fff" />
					<circle class="hole" {cx} {cy} r="60" fill="url(#{idp}-hole)" />
				</mask>
				<rect class="dark" width={W} height={H} fill="var(--tr-tunnel, #0b0c12)" mask="url(#{idp}-tm-{tunnelKey})" />
				<circle class="lamp" {cx} {cy} r="60" fill="url(#{idp}-glow)" />
				<rect class="glare" width={W} height={H} fill="var(--tr-light, #fff8e4)" />
			</g>
		{/key}
	{/if}
</svg>

<style>
	.land {
		overflow: hidden;
	}
	.f0 {
		fill: var(--tr-fir);
	}
	.f1 {
		fill: var(--tr-fir-2);
	}
	.stars {
		opacity: var(--tr-star-o, 0);
	}
	.cloud {
		opacity: 0;
		fill: var(--cloud, #fff);
		transition:
			opacity 3s ease,
			fill 3s ease;
	}
	.cloud--on {
		opacity: 0.9;
	}
	.lv2 .cloud {
		fill: var(--cloud-storm, #8a93a6);
	}
	.veil {
		transition: opacity 3s ease;
	}
	.glint {
		animation: glint 4.5s ease-in-out infinite;
		opacity: 0.2;
	}
	.rain {
		animation: rain 0.5s linear infinite;
	}
	.drop {
		opacity: 0;
		animation: drop 8s ease-in infinite;
	}
	.flash {
		opacity: 0;
		animation: flash 9s linear infinite;
	}
	.tunnel .dark {
		animation: tunnel-dark 3.6s linear forwards;
	}
	.tunnel .hole,
	.tunnel .lamp {
		transform-box: fill-box;
		transform-origin: center;
		transform: scale(0.02);
	}
	.tunnel .hole {
		animation: tunnel-hole 3.6s linear forwards;
	}
	.tunnel .lamp {
		animation: tunnel-hole 3.6s linear forwards, tunnel-lamp 3.6s linear forwards;
	}
	.tunnel .glare {
		opacity: 0;
		animation: tunnel-glare 3.6s linear forwards;
	}

	@media (prefers-reduced-motion: reduce) {
		.glint,
		.rain,
		.drop,
		.flash {
			animation: none;
		}
	}

	@keyframes glint {
		0%,
		100% {
			opacity: 0.15;
		}
		50% {
			opacity: 0.7;
		}
	}
	@keyframes rain {
		to {
			transform: translate(-180px, 135px);
		}
	}
	@keyframes drop {
		0% {
			opacity: 0;
			transform: translateY(0);
		}
		12% {
			opacity: 0.5;
		}
		100% {
			opacity: 0;
			transform: translateY(var(--dy));
		}
	}
	@keyframes flash {
		0%,
		84%,
		100% {
			opacity: 0;
		}
		85% {
			opacity: 0.55;
		}
		87% {
			opacity: 0.04;
		}
		88.5% {
			opacity: 0.4;
		}
		93% {
			opacity: 0;
		}
	}
	/* Darkness falls, a spark of light appears far ahead, grows, and floods the glass. */
	@keyframes tunnel-dark {
		0% {
			opacity: 0;
		}
		8%,
		80% {
			opacity: 1;
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes tunnel-hole {
		0%,
		8% {
			transform: scale(0.02);
		}
		40% {
			transform: scale(0.09);
		}
		85% {
			transform: scale(30);
		}
		100% {
			transform: scale(30);
		}
	}
	@keyframes tunnel-lamp {
		0%,
		8% {
			opacity: 0;
		}
		14%,
		70% {
			opacity: 1;
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes tunnel-glare {
		0%,
		55% {
			opacity: 0;
		}
		75% {
			opacity: 0.85;
		}
		100% {
			opacity: 0;
		}
	}
</style>
