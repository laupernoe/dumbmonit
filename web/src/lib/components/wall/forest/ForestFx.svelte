<script lang="ts">
	/**
	 * What moves through the air, as one SVG group in the scene's coordinates:
	 * leaves drifting down for ever, rain and lightning in a storm, and the
	 * gust that swirls the leaves up when an incident is resolved.
	 */
	import { LEAF_PATH, type FallingLeaf, type GustLeaf, type NormalMood, type ForestPulse } from './geometry.js';

	interface Props {
		/** Unique per instance: names the rain pattern. */
		uid: string;
		/** The area rain and lightning cover. */
		x: number;
		y: number;
		w: number;
		h: number;
		mood: NormalMood;
		pulse: ForestPulse;
		leaves: FallingLeaf[];
		gust: GustLeaf[];
		/** Where the gust starts. */
		gustX: number;
		gustY: number;
		/** Where the lightning bolt strikes (top, bottom). */
		boltX: number;
		boltY: number;
		boltH: number;
	}

	let { uid, x, y, w, h, mood, pulse, leaves, gust, gustX, gustY, boltX, boltY, boltH }: Props = $props();
	const storm = $derived(mood === 'critical');
</script>

<defs>
	<pattern id="{uid}-rain" width="60" height="60" patternUnits="userSpaceOnUse">
		<path d="M44 6l-5 22M14 34l-5 22" stroke="var(--f-rain)" stroke-width="1.6" stroke-linecap="round" />
	</pattern>
	<path id="{uid}-leaf" d={LEAF_PATH} />
</defs>

<g class="leaves" class:gale={storm}>
	{#each leaves as l, i (i)}
		<g transform="translate({l.x} {l.y})">
			<g class="fall" style="--fall:{l.fall}px;--dur:{l.dur}s;animation-delay:{l.delay}s">
				<g class="sway" style="--sway:{l.sway}px;--spin:{l.spin}deg;--dur:{l.dur / 3}s;animation-delay:{l.delay}s">
					<use href="#{uid}-leaf" fill="var(--f-c{l.c})" transform="scale({l.size})" />
				</g>
			</g>
		</g>
	{/each}
</g>

{#if storm}
	<g class="rain" transform="translate({x} {y})">
		<rect class="rain-a" x="-60" y="-60" width={w + 120} height={h + 120} fill="url(#{uid}-rain)" />
	</g>
	<g class="bolt-group">
		<rect class="flash" {x} {y} width={w} height={h} fill="var(--f-flash)" />
		<path
			class="bolt"
			d="M{boltX} {boltY}l-14 {boltH * 0.3}l12 0l-18 {boltH * 0.35}l10 0l-14 {boltH * 0.35}"
			fill="none"
			stroke="var(--f-flash)"
			stroke-width="4"
			stroke-linejoin="round"
		/>
	</g>
{/if}

{#if pulse?.kind === 'resolved'}
	{#key pulse.at}
		<g transform="translate({gustX} {gustY})">
			{#each gust as g, i (i)}
				<g
					class="gust"
					style="--dur:{g.dur}s;--delay:{g.delay}s;--x1:{g.p[0][0]}px;--y1:{g.p[0][1]}px;--x2:{g.p[1][0]}px;--y2:{g.p[1][1]}px;--x3:{g.p[2][0]}px;--y3:{g.p[2][1]}px;--x4:{g.p[3][0]}px;--y4:{g.p[3][1]}px;--spin:{g.spin}deg"
				>
					<use href="#{uid}-leaf" fill="var(--f-c{g.c})" transform="scale({g.size * 1.2})" />
				</g>
			{/each}
		</g>
	{/key}
{/if}

<style>
	.fall {
		animation: fall var(--dur) linear infinite;
		opacity: 0;
	}
	.gale .fall {
		animation-duration: calc(var(--dur) * 0.6);
	}
	.sway {
		animation: sway var(--dur) ease-in-out infinite alternate;
	}
	.rain-a {
		animation: rain 0.55s linear infinite;
	}
	.flash {
		opacity: 0;
		animation: flash 9s linear infinite;
	}
	.bolt {
		opacity: 0;
		animation: bolt 9s linear infinite;
	}
	.gust {
		opacity: 0;
		animation: gust var(--dur) cubic-bezier(0.35, 0.1, 0.5, 1) var(--delay) both;
	}
	@keyframes fall {
		0% { transform: translateY(0); opacity: 0; }
		8% { opacity: 0.9; }
		90% { opacity: 0.9; }
		100% { transform: translateY(var(--fall)); opacity: 0; }
	}
	@keyframes sway {
		from { transform: translateX(calc(var(--sway) * -1)) rotate(-40deg); }
		to { transform: translateX(var(--sway)) rotate(var(--spin)); }
	}
	@keyframes rain {
		to { transform: translateY(60px); }
	}
	@keyframes flash {
		0%, 93%, 100% { opacity: 0; }
		94% { opacity: 0.55; }
		95% { opacity: 0.08; }
		96% { opacity: 0.4; }
		98% { opacity: 0; }
	}
	@keyframes bolt {
		0%, 93.5%, 97%, 100% { opacity: 0; }
		94%, 96% { opacity: 1; }
	}
	@keyframes gust {
		0% { transform: translate(0, 0) rotate(0); opacity: 0; }
		10% { opacity: 1; }
		25% { transform: translate(var(--x1), var(--y1)) rotate(calc(var(--spin) * 0.25)); }
		50% { transform: translate(var(--x2), var(--y2)) rotate(calc(var(--spin) * 0.5)); }
		75% { transform: translate(var(--x3), var(--y3)) rotate(calc(var(--spin) * 0.75)); opacity: 1; }
		100% { transform: translate(var(--x4), var(--y4)) rotate(var(--spin)); opacity: 0; }
	}
	@media (prefers-reduced-motion: reduce) {
		.fall, .sway, .rain-a, .flash, .bolt { animation: none; }
		.fall { opacity: 0.8; }
		.gust { animation: none; opacity: 0; }
	}
</style>
