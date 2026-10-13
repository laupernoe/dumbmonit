<script lang="ts">
	/**
	 * Swiss railway station clock, as an SVG group to drop inside a scene's <svg>:
	 * white face, black bars and hands, a red seconds hand ending in a disc.
	 * The seconds hand laps in 58 s then waits at 12 for the minute to turn, the
	 * way the real ones do. Time is Europe/Zurich. Drawn in a 200 x 200 box
	 * centred on (x, y) and scaled to a radius of `r` scene units.
	 * Reduced motion: the hand moves once a second instead of sweeping.
	 */
	import { reducedMotion } from '#lib/ui/index.js';
	import { clockAngles } from '#lib/scene/swiss.js';

	interface Props {
		x: number;
		y: number;
		r?: number;
		/** The tab is hidden: nothing is scheduled. */
		paused?: boolean;
		/** Scene y of the ground: a post reaches down to it. */
		ground?: number | null;
	}
	let { x, y, r = 40, paused = false, ground = null }: Props = $props();

	const motion = !reducedMotion();
	let angles = $state(clockAngles(new Date(), motion));

	$effect(() => {
		if (paused) return;
		let frame = 0;
		let timer: ReturnType<typeof setInterval> | undefined;
		const tickNow = () => (angles = clockAngles(new Date(), motion));
		tickNow();
		if (motion) {
			const loop = () => {
				tickNow();
				frame = requestAnimationFrame(loop);
			};
			frame = requestAnimationFrame(loop);
		} else {
			timer = setInterval(tickNow, 1000);
		}
		return () => {
			cancelAnimationFrame(frame);
			clearInterval(timer);
		};
	});

	const MINUTE_TICKS = Array.from({ length: 60 }, (_, i) => i).filter((i) => i % 5 !== 0);
	const HOUR_BARS = Array.from({ length: 12 }, (_, i) => i);
</script>

{#if ground != null}
	<rect x={x - r * 0.06} y={y} width={r * 0.12} height={Math.max(0, ground - y)} fill="#3d424b" />
{/if}
<g class="sbb-clock" transform="translate({x} {y}) scale({r / 100})">
	<circle r="102" fill="#1a1a1a" opacity=".25" cy="3" />
	<circle r="100" fill="#1d1d1d" />
	<circle r="94" fill="#fbfbf8" />
	<g stroke="#161616" stroke-linecap="butt">
		{#each MINUTE_TICKS as i (i)}
			<line
				x1="0"
				y1="-90"
				x2="0"
				y2="-84"
				stroke-width="2"
				transform="rotate({i * 6})"
			/>
		{/each}
		{#each HOUR_BARS as i (i)}
			<line x1="0" y1="-91" x2="0" y2="-70" stroke-width="7" transform="rotate({i * 30})" />
		{/each}
	</g>
	<g fill="#161616">
		<path d="M-5 14L-4 -52L0 -58L4 -52L5 14z" transform="rotate({angles.hour})" />
		<path d="M-4 16L-3 -80L0 -85L3 -80L4 16z" transform="rotate({angles.minute})" />
	</g>
	<g transform="rotate({angles.second})">
		<rect x="-1.6" y="-62" width="3.2" height="92" fill="var(--sbb-red, #eb0000)" />
		<circle cy="-66" r="10" fill="var(--sbb-red, #eb0000)" />
	</g>
	<circle r="5" fill="var(--sbb-red, #eb0000)" />
</g>
