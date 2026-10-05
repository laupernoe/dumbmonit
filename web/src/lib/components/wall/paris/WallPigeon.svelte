<script lang="ts">
	/**
	 * A pigeon of the wall's Paris scene: the DumbMonit mascot (`Mascot.svelte`,
	 * same geometry) with what a rooftop pigeon needs — eyes that close for a
	 * nap, look somewhere, or go round with surprise; a beret; a protest sign;
	 * an umbrella for the rain; a baguette for the road; slow wings in flight.
	 *
	 * The drawing is 280 × 320 units, inside a larger box (480 × 600) that
	 * leaves room above for the sign and the umbrella; the feet sit at the
	 * bottom centre, so a parent places it by its feet. Colours come from CSS
	 * variables (`--pg-*`) the scene sets per theme; the defaults are the
	 * mascot's own.
	 */
	interface Props {
		class?: string;
		/** Where the pupils look, -1 … 1 on each axis. */
		lookX?: number;
		lookY?: number;
		eyes?: 'open' | 'closed' | 'startled' | 'dizzy';
		hat?: boolean;
		/** Raised beret: the hat tip. */
		tip?: boolean;
		sign?: string | null;
		umbrella?: boolean;
		baguette?: boolean;
		/** Slow wing beats, for flight. */
		flap?: boolean;
		/** Seconds of offset for the blink, so a flock never blinks in sync. */
		blinkOffset?: number;
	}

	let {
		class: className = '',
		lookX = 0,
		lookY = 0,
		eyes = 'open',
		hat = false,
		tip = false,
		sign = null,
		umbrella = false,
		baguette = false,
		flap = false,
		blinkOffset = 0
	}: Props = $props();

	// Pupils at rest look slightly at the viewer, as the mascot does.
	const L = [96, 120];
	const R = [184, 120];
	const pupilL = $derived(
		eyes === 'dizzy' ? 'translate(10px, 12px)' : `translate(${lookX * 22}px, ${lookY * 22}px)`
	);
	const pupilR = $derived(
		eyes === 'dizzy' ? 'translate(-10px, -8px)' : `translate(${lookX * 16}px, ${lookY * 16}px)`
	);
	// A sign this long keeps one line; the board grows with it.
	const signWidth = $derived(sign ? Math.max(220, sign.length * 34 + 70) : 0);
</script>

<svg
	viewBox="-100 -280 480 600"
	class="pigeon {flap ? 'pigeon--flap' : ''} {eyes === 'closed' ? 'pigeon--asleep' : ''} {tip ? 'pigeon--tip' : ''} {className}"
	style:--blink-offset="{blinkOffset}s"
	aria-hidden="true"
>
	{#if sign}
		<!-- The stick passes behind the body; the board is held high. -->
		<g class="sign">
			<path d="M140 -96 L140 220" stroke="var(--pg-stick, #8b6a44)" stroke-width="14" stroke-linecap="round" />
			<rect
				x={140 - signWidth / 2}
				y="-250"
				width={signWidth}
				height="150"
				rx="14"
				fill="var(--pg-sign, #fbf9f4)"
				stroke="var(--pg-line, #1e2640)"
				stroke-width="10"
			/>
			<text
				x="140"
				y="-152"
				text-anchor="middle"
				font-size="66"
				font-weight="700"
				fill="var(--pg-sign-ink, #16213a)"
				font-family="inherit">{sign}</text
			>
		</g>
	{/if}

	{#if umbrella}
		<g class="umbrella">
			<path d="M140 -150 L140 210" stroke="var(--pg-line, #1e2640)" stroke-width="9" stroke-linecap="round" />
			<path d="M140 210 c0 22 -26 22 -26 4" fill="none" stroke="var(--pg-line, #1e2640)" stroke-width="9" stroke-linecap="round" />
			<path
				d="M-40 -40 C-30 -150 50 -200 140 -200 C230 -200 310 -150 320 -40 C290 -60 260 -60 230 -40 C200 -60 170 -60 140 -40 C110 -60 80 -60 50 -40 C20 -60 -10 -60 -40 -40 Z"
				fill="var(--pg-umbrella, #4a5a78)"
				stroke="var(--pg-line, #1e2640)"
				stroke-width="10"
				stroke-linejoin="round"
			/>
			<path d="M140 -200 C110 -150 100 -90 110 -48 M140 -200 C170 -150 180 -90 170 -48" fill="none" stroke="var(--pg-umbrella-rib, rgb(255 255 255 / 0.35))" stroke-width="8" />
		</g>
	{/if}

	<!-- feet -->
	<path d="M96 266h30v20c0 8-6 14-14 14H86c-7 0-11-6-9-12 3-11 10-22 19-22zM184 266h-30v20c0 8 6 14 14 14h26c7 0 11-6 9-12-3-11-10-22-19-22z" fill="var(--pg-beak, #e97b3a)" stroke="var(--pg-line, #1e2640)" stroke-width="12" stroke-linejoin="round" />
	<!-- body -->
	<path d="M140 46c70 0 118 54 118 128 0 62-44 104-118 104S22 236 22 174C22 100 70 46 140 46z" fill="var(--pg-body, #6f83a3)" stroke="var(--pg-line, #1e2640)" stroke-width="12" />
	<!-- wings -->
	<path class="wing wing-l" d="M40 150c-14 40-10 90 18 118" fill="none" stroke="var(--pg-line, #1e2640)" stroke-width="11" stroke-linecap="round" />
	<path class="wing wing-r" d="M240 150c14 40 10 90-18 118" fill="none" stroke="var(--pg-line, #1e2640)" stroke-width="11" stroke-linecap="round" />
	<!-- chest -->
	<path d="M78 190c20 22 46 34 78 34 26 0 44-8 54-20-6 30-34 52-70 52-34 0-56-26-62-66z" fill="var(--pg-chest, #4c9d6f)" />
	<path d="M110 232c30 2 52-8 64-26" fill="none" stroke="var(--pg-line, #1e2640)" stroke-width="10" stroke-linecap="round" />

	{#if baguette}
		<g transform="rotate(-24 140 230)">
			<rect x="10" y="212" width="270" height="40" rx="20" fill="var(--pastry, #d9a35b)" stroke="var(--pg-line, #1e2640)" stroke-width="10" />
			<path d="M60 222 l20 20 M110 222 l20 20 M160 222 l20 20 M210 222 l20 20" stroke="var(--pastry-crust, #b07a3a)" stroke-width="8" stroke-linecap="round" />
		</g>
	{/if}

	<!-- eyes -->
	<circle cx="96" cy="118" r="52" fill="var(--pg-eye, #fff)" stroke="var(--pg-line, #1e2640)" stroke-width="12" />
	<circle cx="184" cy="120" r="42" fill="var(--pg-eye, #fff)" stroke="var(--pg-line, #1e2640)" stroke-width="12" />
	<g class="pupil" class:pupil--small={eyes === 'startled'} style:transform={pupilL}>
		<circle cx={L[0]} cy={L[1]} r="19" fill="var(--pg-pupil, #1e2640)" />
		<circle cx={L[0] + 8} cy={L[1] - 8} r="6" fill="var(--pg-eye, #fff)" />
	</g>
	<g class="pupil" class:pupil--small={eyes === 'startled'} style:transform={pupilR}>
		<circle cx={R[0]} cy={R[1]} r="15" fill="var(--pg-pupil, #1e2640)" />
	</g>
	<!-- eyelids: closed for a nap, otherwise folded up until a blink -->
	<circle cx="96" cy="118" r="52" class="lid" fill="var(--pg-body, #6f83a3)" stroke="var(--pg-line, #1e2640)" stroke-width="12" />
	<circle cx="184" cy="120" r="42" class="lid lid--right" fill="var(--pg-body, #6f83a3)" stroke="var(--pg-line, #1e2640)" stroke-width="12" />
	{#if eyes === 'closed'}
		<path d="M64 126 q32 22 64 0 M158 128 q26 18 52 0" fill="none" stroke="var(--pg-line, #1e2640)" stroke-width="9" stroke-linecap="round" />
	{/if}

	<!-- beak -->
	<path d="M114 150c8-10 44-10 52 0l-26 46z" fill="var(--pg-beak, #e97b3a)" stroke="var(--pg-line, #1e2640)" stroke-width="10" stroke-linejoin="round" />
	<path d="M120 152c6-6 34-6 40 0" fill="none" stroke="var(--pg-eye, #fff)" stroke-width="7" stroke-linecap="round" />

	{#if hat}
		<!-- The beret, worn at an angle, stalk and all. -->
		<g class="beret">
			<path d="M58 66 C52 40 96 18 150 20 C204 22 236 44 222 64 C196 76 96 82 58 66 Z" fill="var(--pg-hat, #262b3d)" stroke="var(--pg-line, #1e2640)" stroke-width="10" stroke-linejoin="round" />
			<path d="M146 20 q4 -18 18 -22" fill="none" stroke="var(--pg-line, #1e2640)" stroke-width="10" stroke-linecap="round" />
		</g>
	{/if}
</svg>

<style>
	.pigeon {
		overflow: visible;
	}
	.pupil {
		transition: transform 900ms cubic-bezier(0.22, 1, 0.36, 1);
	}
	.pupil circle {
		transform-box: fill-box;
		transform-origin: center;
		transition: transform 300ms cubic-bezier(0.22, 1, 0.36, 1);
	}
	.pupil--small circle {
		transform: scale(0.5);
	}
	.lid {
		transform-box: fill-box;
		transform-origin: top;
		transform: scaleY(0);
		animation: blink 7.3s ease-in-out infinite;
		animation-delay: calc(var(--blink-offset, 0s) * -1);
	}
	.lid--right {
		animation-delay: calc(var(--blink-offset, 0s) * -1 + 40ms);
	}
	.pigeon--asleep .lid {
		animation: none;
		transform: scaleY(1);
	}
	@keyframes blink {
		0%,
		94% {
			transform: scaleY(0);
		}
		96% {
			transform: scaleY(1);
		}
		98%,
		100% {
			transform: scaleY(0);
		}
	}
	.wing {
		transform-box: fill-box;
		transform-origin: top center;
	}
	/* Slow, soft beats: a glide with a lazy flap, never a buzz. */
	.pigeon--flap .wing-l {
		animation: flap-l 900ms ease-in-out infinite;
	}
	.pigeon--flap .wing-r {
		animation: flap-r 900ms ease-in-out infinite;
	}
	@keyframes flap-l {
		50% {
			transform: rotate(32deg);
		}
	}
	@keyframes flap-r {
		50% {
			transform: rotate(-32deg);
		}
	}
	.beret {
		transform-box: fill-box;
		transform-origin: 30% 100%;
		transition: transform 900ms cubic-bezier(0.22, 1, 0.36, 1);
	}
	.pigeon--tip .beret {
		transform: translate(18px, -46px) rotate(-16deg);
	}
	.sign {
		animation: sign-bob 3.6s ease-in-out infinite;
	}
	@keyframes sign-bob {
		50% {
			transform: translateY(-10px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.lid,
		.wing,
		.sign {
			animation: none !important;
		}
		.pupil,
		.beret {
			transition: none;
		}
	}
</style>
