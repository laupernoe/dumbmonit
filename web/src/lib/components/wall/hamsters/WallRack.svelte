<script lang="ts">
	/**
	 * One server rack with its hamster wheel on top, in its own units (150 wide,
	 * 190 tall, origin at the middle of its foot; the scene scales it). Rack-level
	 * gags live here: the lights going dark, the zap and the smoke of a "down", the
	 * energy drink and the sprint of an "up". The hamster's own acrobatics are in
	 * `WallHamster`. Reduced motion is handled by the scene.
	 */
	import WallHamster from './WallHamster.svelte';
	import { HUB_UP } from './geometry.js';

	interface Props {
		index: number;
		level: 0 | 1 | 2;
		gesture: 'down' | 'up' | 'resolved' | null;
		/** This rack is the one the gag happens to. */
		victim: boolean;
		/** Night: the wheel is empty, the hamsters are in bed. */
		asleep: boolean;
		startled: boolean;
	}

	let { index, level, gesture, victim, asleep, startled }: Props = $props();

	const LED = ['var(--led-g)', 'var(--led-a)', 'var(--led-b)'];
	const pose = $derived(
		gesture === 'resolved'
			? 'dance'
			: victim && gesture === 'down'
				? 'faint'
				: victim && gesture === 'up'
					? 'wake'
					: startled
						? 'startle'
						: 'run'
	);
</script>

<g
	class="rack m{level}"
	class:down={victim && gesture === 'down'}
	class:up={victim && gesture === 'up'}
	class:asleep
	style:--i={index}
>
	<ellipse cx="0" cy="4" rx="96" ry="9" fill="var(--shadow)" />
	<rect x="-75" y="-190" width="150" height="190" rx="7" fill="var(--rack)" stroke="var(--rack2)" stroke-width="4" />
	{#each [0, 1, 2, 3] as u (u)}
		<g transform="translate(0 {-182 + u * 45})">
			<rect x="-67" y="0" width="134" height="39" rx="4" fill="var(--unit)" />
			<path d="M-58 11h52M-58 19h52M-58 27h52" stroke="var(--rack2)" stroke-width="2.4" stroke-linecap="round" />
			<circle cx="-62" cy="5" r="1.6" fill="var(--rack2)" />
			<circle cx="62" cy="34" r="1.6" fill="var(--rack2)" />
			<circle class="led" cx="38" cy="14" r="4.2" fill={LED[(index + u) % 3]} style:animation-delay="{(index * 4 + u) * 0.43}s" />
			<circle class="led" cx="52" cy="14" r="4.2" fill={LED[(index + u + 1) % 3]} style:animation-delay="{(index * 4 + u) * 0.29 + 0.7}s" />
			<rect class="led" x="32" y="25" width="28" height="4" rx="2" fill={LED[(index + u + 2) % 3]} style:animation-delay="{(index * 4 + u) * 0.51}s" />
		</g>
	{/each}
	<rect class="dimmer" x="-75" y="-190" width="150" height="190" rx="7" fill="#0a0f1d" />

	<!-- Wheel on a frame, the dynamo, the cable. -->
	<path d="M0 -{HUB_UP} L-34 -190M0 -{HUB_UP} L34 -190" stroke="var(--wheel)" stroke-width="5" stroke-linecap="round" />
	<path d="M20 -246q40 10 36 56" stroke="var(--cable)" stroke-width="3.5" stroke-linecap="round" />
	<g transform="translate(0 -{HUB_UP})">
		<circle class="aura" r="62" fill="var(--aura)" />
		<g class="wheel">
			<circle r="50" stroke="var(--wheel)" stroke-width="5" />
			<circle r="46" fill="var(--wheel-in)" opacity=".45" />
			{#each [0, 45, 90, 135] as a (a)}
				<path d="M0 -48V48" stroke="var(--wheel)" stroke-width="2.4" transform="rotate({a})" />
			{/each}
		</g>
		<g class="slot" transform="translate(0 10)">
			<WallHamster {pose} beat={index} />
		</g>
		<rect x="-9" y="-9" width="18" height="18" rx="4" fill="var(--rack2)" />
		<path d="M1.5 -6l-5 7h4l-1 5 5-7h-4z" fill="#ffe94a" />
		<path class="zap" d="M-30 -48l10 14-9 4 13 16M26 -40l-9 12 10 3-10 14" stroke="#ffe94a" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round" />
		{#if level === 2 && !asleep}
			<path class="spark" d="M-16 -4l8-10 3 9 8-8" stroke="#ffe94a" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" style:animation-delay="{index * 0.13}s" />
		{/if}
		{#if level >= 1 && !asleep}
			<path class="drip" d="M16 -4q-5 7 0 10t0-10z" fill="var(--sweat)" style:animation-delay="{index * 0.2}s" />
		{/if}
		{#if level === 2 && !asleep}
			<path class="drip" d="M26 0q-5 7 0 10t0-10z" fill="var(--sweat)" style:animation-delay="{0.22 + index * 0.2}s" />
			<path class="drip" d="M6 2q-5 7 0 10t0-10z" fill="var(--sweat)" style:animation-delay="{0.4 + index * 0.2}s" />
		{/if}
		<g class="dizzy-fade">
			<g class="dizzy">
				<path d="M-26 -18l3 6 6 1-5 4 1 6-5-3-5 3 1-6-5-4 6-1z" fill="#ffe94a" transform="translate(4 -8) scale(.7)" />
				<path d="M-26 -18l3 6 6 1-5 4 1 6-5-3-5 3 1-6-5-4 6-1z" fill="#ffe94a" transform="translate(34 6) scale(.55)" />
				<path d="M-26 -18l3 6 6 1-5 4 1 6-5-3-5 3 1-6-5-4 6-1z" fill="#ffe94a" transform="translate(-10 16) scale(.5)" />
			</g>
		</g>
		<path class="speed" d="M-92 -10h30M-100 8h38M-90 26h28" stroke="var(--wheel)" stroke-width="3" stroke-linecap="round" />
		<g class="smoke">
			<circle cx="-4" cy="-60" r="7" fill="var(--smoke)" />
			<circle cx="8" cy="-80" r="9" fill="var(--smoke)" />
			<circle cx="-2" cy="-102" r="11" fill="var(--smoke)" />
		</g>
	</g>
</g>

<!-- The colleague and the energy drink: only in the "up" gag, only by day. -->
{#if victim && gesture === 'up' && !asleep}
	<g transform="translate(112 0)">
		<g class="courier">
			<g transform="translate(0 -37) scale(-.8 .8)"><WallHamster pose="courier" /></g>
			<g class="held" transform="translate(-26 -22)">
				{@render can()}
			</g>
		</g>
	</g>
	<g transform="translate(30 -{HUB_UP - 20})">
		<g class="fly-x">
			<g class="fly-y">
				<g class="fly-r">{@render can()}</g>
				<circle class="fizz" cx="-3" cy="-12" r="2" fill="#fff" />
				<circle class="fizz f2" cx="3" cy="-14" r="1.6" fill="#fff" />
			</g>
		</g>
	</g>
{/if}

{#snippet can()}
	<rect x="-5" y="-9" width="10" height="18" rx="2" fill="var(--can)" />
	<rect x="-5" y="-9" width="10" height="4" rx="1.5" fill="var(--can-top)" />
	<path d="M1.5 -3l-4 6h3l-1 4 4-6h-3z" fill="#ffe94a" />
{/snippet}

<style>
	.m0 {
		--run: 0.5s;
		--turn: 4.4s;
	}
	.m1 {
		--run: 0.3s;
		--turn: 2.4s;
	}
	.m2 {
		--run: 0.17s;
		--turn: 1.2s;
	}
	.wheel,
	.dizzy {
		transform-box: fill-box;
		transform-origin: center;
	}
	.wheel {
		animation: turn var(--turn) linear infinite;
	}
	.asleep .wheel {
		animation: none;
	}
	.slot {
		transition: opacity 3s ease;
	}
	.asleep .slot {
		opacity: 0;
	}
	.asleep .dizzy-fade {
		display: none;
	}
	.led {
		animation: blink 2.4s ease-in-out infinite;
	}
	.asleep .led {
		animation-duration: 6s;
	}
	.zap,
	.dimmer,
	.aura,
	.speed,
	.smoke,
	.dizzy-fade,
	.fizz {
		opacity: 0;
	}
	.drip {
		animation: drip 1.1s ease-in infinite;
	}
	.spark {
		animation: flicker 0.35s steps(2) infinite;
	}
	.dizzy {
		animation: turn 1.5s linear infinite;
	}

	/* --- Down: the lights go out, a zap, a little smoke. */
	.down .wheel {
		animation: wheel-stop 7s ease-out 1;
	}
	.down .led {
		animation: leds-off 7s linear 1;
	}
	.down .dimmer {
		animation: dim 7s ease-in-out 1;
	}
	.down .zap {
		animation: zap 7s steps(1) 1;
	}
	.down .smoke {
		animation: smoke 7s ease-out 1;
	}
	.down .dizzy-fade {
		animation: show-mid 7s steps(1) 1;
	}
	.down .drip,
	.up .drip {
		animation: none;
		opacity: 0;
	}

	/* --- Up: the lights stay dark until the drink lands, then full sprint. */
	.up .wheel {
		animation: wheel-sprint 8s linear 1;
	}
	.up .led {
		animation: leds-on 8s linear 1;
	}
	.up .dimmer {
		animation: dim-up 8s linear 1;
	}
	.up .aura {
		animation: aura 8s ease-in-out 1;
	}
	.up .speed {
		animation: speed 0.18s linear 4.8s infinite;
	}
	.courier {
		animation: courier 8s ease-in-out 1 both;
	}
	.held {
		animation: held 8s steps(1) 1 both;
	}
	.fly-x {
		animation: fly-x 8s linear 1 both;
	}
	.fly-y {
		animation: fly-y 8s linear 1 both;
	}
	.fly-r {
		animation: fly-r 8s linear 1 both;
	}
	.fizz {
		animation: fizz 8s linear 1 both;
	}
	.fizz.f2 {
		animation-delay: 0.15s;
	}

	@keyframes turn {
		to {
			transform: rotate(360deg);
		}
	}
	@keyframes blink {
		0%,
		100% {
			opacity: 1;
		}
		45% {
			opacity: 0.35;
		}
		60% {
			opacity: 1;
		}
	}
	@keyframes drip {
		0% {
			opacity: 0;
			transform: translate(0, 0);
		}
		15% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(14px, 34px);
		}
	}
	@keyframes flicker {
		50% {
			opacity: 0.1;
		}
	}
	@keyframes wheel-stop {
		from {
			transform: rotate(0);
		}
		22%,
		to {
			transform: rotate(135deg);
		}
	}
	@keyframes show-mid {
		0% {
			opacity: 0;
		}
		16%,
		90% {
			opacity: 1;
		}
		91%,
		to {
			opacity: 0;
		}
	}
	@keyframes leds-off {
		0%,
		8% {
			opacity: 1;
		}
		11%,
		85% {
			opacity: 0.05;
		}
		88% {
			opacity: 1;
		}
		91% {
			opacity: 0.1;
		}
		94%,
		to {
			opacity: 1;
		}
	}
	@keyframes dim {
		0%,
		8% {
			opacity: 0;
		}
		14%,
		85% {
			opacity: 0.38;
		}
		to {
			opacity: 0;
		}
	}
	@keyframes zap {
		0%,
		7% {
			opacity: 0;
		}
		8%,
		11% {
			opacity: 1;
		}
		12%,
		13% {
			opacity: 0;
		}
		14%,
		17% {
			opacity: 1;
		}
		18%,
		to {
			opacity: 0;
		}
	}
	@keyframes smoke {
		0%,
		18% {
			opacity: 0;
			transform: translateY(14px);
		}
		35% {
			opacity: 1;
		}
		80%,
		to {
			opacity: 0;
			transform: translateY(-24px);
		}
	}
	@keyframes wheel-sprint {
		0%,
		44% {
			transform: rotate(135deg);
			animation-timing-function: ease-in;
		}
		60% {
			transform: rotate(270deg);
			animation-timing-function: linear;
		}
		to {
			transform: rotate(2070deg);
		}
	}
	@keyframes leds-on {
		0%,
		58% {
			opacity: 0.05;
		}
		60% {
			opacity: 1;
		}
		62% {
			opacity: 0.1;
		}
		64%,
		to {
			opacity: 1;
		}
	}
	@keyframes dim-up {
		0%,
		58% {
			opacity: 0.38;
		}
		62%,
		to {
			opacity: 0;
		}
	}
	@keyframes aura {
		0%,
		58% {
			opacity: 0;
		}
		64% {
			opacity: 1;
		}
		92% {
			opacity: 0.8;
		}
		to {
			opacity: 0;
		}
	}
	@keyframes speed {
		from {
			opacity: 0.9;
			transform: translateX(0);
		}
		to {
			opacity: 0;
			transform: translateX(-26px);
		}
	}
	@keyframes courier {
		0% {
			transform: translateX(300px);
		}
		27%,
		64% {
			transform: translateX(0);
		}
		to {
			transform: translateX(320px);
		}
	}
	@keyframes held {
		0%,
		27% {
			opacity: 1;
		}
		28%,
		to {
			opacity: 0;
		}
	}
	@keyframes fly-x {
		0%,
		29% {
			transform: translateX(56px);
			opacity: 0;
		}
		30% {
			opacity: 1;
		}
		40%,
		63% {
			transform: translateX(0);
			opacity: 1;
		}
		66%,
		to {
			transform: translateX(0);
			opacity: 0;
		}
	}
	@keyframes fly-y {
		0%,
		29% {
			transform: translateY(202px);
			animation-timing-function: ease-out;
		}
		35% {
			transform: translateY(-40px);
			animation-timing-function: ease-in;
		}
		40%,
		to {
			transform: translateY(0);
		}
	}
	@keyframes fly-r {
		0%,
		29% {
			transform: rotate(0);
		}
		40% {
			transform: rotate(540deg);
		}
		43% {
			transform: rotate(-105deg);
		}
		55% {
			transform: rotate(-118deg) translateY(2px);
		}
		60%,
		to {
			transform: rotate(-105deg);
		}
	}
	@keyframes fizz {
		0%,
		42% {
			opacity: 0;
			transform: translateY(0);
		}
		46% {
			opacity: 1;
		}
		58%,
		to {
			opacity: 0;
			transform: translateY(-22px);
		}
	}
</style>
