<script lang="ts">
	/**
	 * The storm that every outdoor wall scene shares. A scene draws its own
	 * clouds and rain; this layer sits over it (under the board) and makes the
	 * critical state read from across the room: the sky goes down a step,
	 * and now and then lightning lights the clouds — a soft double flash and
	 * a bolt, never faster than a slow pulse, never full-screen white.
	 *
	 * Indoor and space scenes do not mount it (see `STORMLESS` in the wall
	 * page). Reduced motion keeps the darker sky and drops the lightning;
	 * a hidden tab stops the timers.
	 */
	import { reducedMotion } from '#lib/ui/index.js';
	import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

	interface Props {
		theme: SceneTheme;
		paused?: boolean;
	}

	let { theme, paused = false }: Props = $props();

	const motion = !reducedMotion();

	/** Where the bolt strikes (% of the width) and which of the two shapes. */
	let strike = $state<{ x: number; shape: 0 | 1; key: number } | null>(null);

	$effect(() => {
		if (!motion || paused) return;
		let alive = true;
		let timer: ReturnType<typeof setTimeout>;
		let key = 0;
		const schedule = (ms: number) => {
			timer = setTimeout(() => {
				if (!alive) return;
				// Keep clear of the left third, where the board's text sits.
				strike = { x: 38 + Math.random() * 58, shape: Math.random() < 0.5 ? 0 : 1, key: ++key };
				timer = setTimeout(() => {
					if (alive) strike = null;
				}, 1200);
				schedule(7000 + Math.random() * 7000);
			}, ms);
		};
		schedule(1500 + Math.random() * 2500);
		return () => {
			alive = false;
			clearTimeout(timer);
			strike = null;
		};
	});
</script>

<div class="storm storm--{theme}" aria-hidden="true">
	<div class="dark"></div>
	{#if strike}
		{#key strike.key}
			<div class="flash" style:--x="{strike.x}%"></div>
			<svg class="bolt" style:left="{strike.x}%" viewBox="0 0 60 220" fill="none">
				{#if strike.shape === 0}
					<path d="M34 0 L18 74 L32 78 L12 150 L28 154 L8 220" />
				{:else}
					<path d="M28 0 L40 62 L24 68 L42 138 L26 144 L46 220" />
				{/if}
			</svg>
		{/key}
	{/if}
</div>

<style>
	.storm {
		position: absolute;
		inset: 0;
		pointer-events: none;
		overflow: hidden;
	}
	/* The sky a step down: darkest at the top, gone by the ground. */
	.dark {
		position: absolute;
		inset: 0;
		background: linear-gradient(to bottom, rgb(14 20 38 / 0.5) 0%, rgb(14 20 38 / 0.34) 45%, transparent 85%);
		animation: dim 2s ease both;
		/* Over the monument and the open sky; the board's text on the left keeps its contrast. */
		mask-image: linear-gradient(to right, transparent 18%, #000 40%, #000 70%, rgb(0 0 0 / 0.5) 100%);
	}
	.storm--dark .dark {
		background: linear-gradient(to bottom, rgb(2 4 12 / 0.45) 0%, rgb(2 4 12 / 0.28) 45%, transparent 85%);
	}
	.storm--oled .dark {
		background: linear-gradient(to bottom, rgb(0 0 0 / 0.4) 0%, rgb(0 0 0 / 0.22) 45%, transparent 85%);
	}
	/* The clouds lit from inside: a wide soft glow around the strike. */
	.flash {
		position: absolute;
		inset: 0;
		background: radial-gradient(ellipse 55% 60% at var(--x) 10%, rgb(235 240 255 / 0.7), rgb(200 215 255 / 0.25) 45%, transparent 75%);
		opacity: 0;
		animation: flash 1.1s linear both;
	}
	.storm--oled .flash {
		opacity: 0;
		background: radial-gradient(ellipse 45% 50% at var(--x) 10%, rgb(200 212 245 / 0.4), transparent 70%);
	}
	.bolt {
		position: absolute;
		top: 0;
		width: 6%;
		max-width: 90px;
		height: 38%;
		margin-left: -3%;
		stroke: #f4f7ff;
		stroke-width: 2.4;
		stroke-linejoin: bevel;
		stroke-linecap: round;
		filter: drop-shadow(0 0 6px rgb(190 210 255 / 0.95)) drop-shadow(0 0 16px rgb(160 185 255 / 0.7));
		opacity: 0;
		animation: bolt 1.1s linear both;
	}
	@keyframes dim {
		from {
			opacity: 0;
		}
	}
	@keyframes flash {
		0% {
			opacity: 0;
		}
		6% {
			opacity: 1;
		}
		16% {
			opacity: 0.25;
		}
		26% {
			opacity: 0.85;
		}
		60%,
		100% {
			opacity: 0;
		}
	}
	@keyframes bolt {
		0% {
			opacity: 0;
		}
		6% {
			opacity: 1;
		}
		18% {
			opacity: 0.3;
		}
		26% {
			opacity: 1;
		}
		50%,
		100% {
			opacity: 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.dark {
			animation: none;
		}
	}
</style>
