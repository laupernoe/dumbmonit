<script lang="ts">
	/**
	 * One hamster, facing right, feet on the line y = 36 and body around the
	 * origin. Its `pose` is the whole story:
	 *   run      jogs (pace from the inherited --run)
	 *   faint    tips over legs-up with crossed-out eyes, then gets back up (7 s)
	 *   wake     starts legs-up, springs up at 3.2 s, sprints from 3.6 s (8 s)
	 *   dance    a hat and a victory hop (set `beat` to stagger the dancers)
	 *   sleep    eyes shut, breathing slowly
	 *   courier  a cap, jogging on the spot
	 *   startle  one big hop
	 * Reduced motion is handled by the scene (it kills every animation).
	 */
	interface Props {
		pose?: 'run' | 'faint' | 'wake' | 'dance' | 'sleep' | 'courier' | 'startle';
		beat?: number;
		/** Eyes open even though asleep: a problem woke this one. */
		alert?: boolean;
	}

	let { pose = 'run', beat = 0, alert = false }: Props = $props();
</script>

<g class="ham {pose}" class:alert style:--beat="{beat * 0.12}s">
	<g class="flip">
		<g class="bob">
			<g class="leg b">
				<ellipse cx="-9" cy="30" rx="5" ry="7" fill="var(--fur2)" />
				<ellipse cx="-7" cy="36" rx="6.5" ry="3" fill="var(--belly)" />
			</g>
			<ellipse cx="0" cy="18" rx="22" ry="17" fill="var(--fur)" />
			<path d="M-18 10c6-8 18-10 28-6" stroke="var(--fur2)" stroke-width="5" stroke-linecap="round" />
			<ellipse cx="5" cy="26" rx="15" ry="8" fill="var(--belly)" />
			<g class="leg a">
				<ellipse cx="11" cy="30" rx="5" ry="7" fill="var(--fur2)" />
				<ellipse cx="14" cy="36" rx="6.5" ry="3" fill="var(--belly)" />
			</g>
			<circle cx="20" cy="9" r="12.5" fill="var(--fur)" />
			<circle cx="27" cy="14" r="6.5" fill="var(--belly)" />
			<circle cx="24" cy="15" r="3" fill="var(--blush)" opacity=".7" />
			<circle cx="12" cy="-1" r="5" fill="var(--fur2)" />
			<circle cx="12" cy="-1" r="2.6" fill="var(--blush)" />
			<g class="eye-open">
				<circle cx="24" cy="7" r="2.7" fill="var(--ink)" />
				<circle cx="24.9" cy="6.1" r=".9" fill="#fff" />
			</g>
			<path class="eye-x" d="M21.5 4.5l5 5m0-5l-5 5" stroke="var(--ink)" stroke-width="1.8" stroke-linecap="round" />
			<path class="eye-z" d="M21.5 7.5q2.7 2.6 5.4 0" stroke="var(--ink)" stroke-width="1.6" stroke-linecap="round" />
			<circle cx="32" cy="11" r="2.1" fill="var(--blush)" />
			<path d="M29 15q2.5 3 5.5 1" stroke="var(--ink)" stroke-width="1.2" stroke-linecap="round" />
			<path d="M30 12l8-2M30 13l8 2" stroke="var(--ink)" stroke-width=".7" opacity=".6" />
			{#if pose === 'courier'}
				<path d="M10 2c1-11 22-13 26-1z" fill="var(--cap)" />
				<rect x="27" y="-2" width="11" height="3.2" rx="1.6" fill="var(--cap)" />
			{:else if pose === 'dance'}
				<path d="M15 -3l6-19 7 19z" fill="var(--party)" />
				<circle cx="21" cy="-23" r="2.4" fill="var(--led-a)" />
			{/if}
		</g>
	</g>
</g>

<style>
	.flip,
	.bob,
	.leg {
		transform-box: fill-box;
	}
	.bob {
		transform-origin: 50% 100%;
	}
	.flip {
		transform-origin: 50% 58%;
	}
	.leg {
		transform-origin: 50% 0;
	}
	.eye-x,
	.eye-z {
		opacity: 0;
	}
	.bob {
		animation: bob var(--run, 0.5s) ease-in-out infinite alternate;
	}
	.leg.a {
		animation: step var(--run, 0.5s) ease-in-out infinite alternate;
	}
	.leg.b {
		animation: step var(--run, 0.5s) ease-in-out infinite alternate-reverse;
	}

	/* Faint: tips over, legs twitching, then gets up. */
	.faint .flip {
		animation: faint 7s ease-out 1;
	}
	.faint .bob {
		animation: none;
	}
	.faint .leg.a {
		animation: twitch 0.55s ease-in-out 1.2s infinite alternate;
	}
	.faint .leg.b {
		animation: twitch 0.55s ease-in-out 1.2s infinite alternate-reverse;
	}
	.faint .eye-open {
		animation: hide-mid 7s steps(1) 1;
	}
	.faint .eye-x {
		animation: show-mid 7s steps(1) 1;
	}

	/* Wake: lying dead, an energy drink arrives, full sprint. */
	.wake .flip {
		animation: wake 8s ease-out 1;
	}
	.wake .bob {
		animation: bob 0.08s ease-in-out 3.6s infinite alternate;
	}
	.wake .leg.a {
		animation: step 0.08s ease-in-out 3.6s infinite alternate;
	}
	.wake .leg.b {
		animation: step 0.08s ease-in-out 3.6s infinite alternate-reverse;
	}
	.wake .eye-open {
		animation: show-late 8s steps(1) 1;
	}
	.wake .eye-x {
		animation: hide-late 8s steps(1) 1;
	}

	.dance .flip {
		animation: dance 0.66s ease-in-out calc(var(--beat) + 0.1s) 6 both;
	}
	.dance .bob,
	.dance .leg {
		animation-duration: 0.12s;
	}
	.startle .flip {
		animation: dance 0.5s ease-out 2;
	}

	.courier .bob,
	.courier .leg {
		animation-duration: 0.22s;
	}

	.sleep .leg {
		animation: none;
	}
	.sleep .bob {
		animation: breathe 3.6s ease-in-out infinite alternate;
		animation-delay: calc(var(--beat) * -3);
	}
	.sleep .eye-open {
		opacity: 0;
	}
	.sleep .eye-z {
		opacity: 1;
	}
	.sleep.alert .eye-open {
		opacity: 1;
	}
	.sleep.alert .eye-z {
		opacity: 0;
	}

	@keyframes bob {
		to {
			transform: translateY(-2.5px);
		}
	}
	@keyframes breathe {
		to {
			transform: translateY(1.5px) scale(1.02, 0.97);
		}
	}
	@keyframes step {
		from {
			transform: rotate(-38deg);
		}
		to {
			transform: rotate(38deg);
		}
	}
	@keyframes twitch {
		from {
			transform: rotate(-12deg);
		}
		to {
			transform: rotate(12deg);
		}
	}
	@keyframes faint {
		0%,
		8% {
			transform: rotate(0);
		}
		14% {
			transform: rotate(-22deg) translateY(-5px);
		}
		20% {
			transform: rotate(150deg) translateY(-9px);
		}
		24%,
		86% {
			transform: rotate(180deg) translateY(0);
		}
		90% {
			transform: rotate(250deg) translateY(-14px);
		}
		95%,
		to {
			transform: rotate(360deg) translateY(0);
		}
	}
	@keyframes wake {
		0%,
		38% {
			transform: rotate(180deg) translateY(0);
		}
		42% {
			transform: rotate(270deg) translateY(-26px);
		}
		47%,
		to {
			transform: rotate(360deg) translateY(0);
		}
	}
	@keyframes hide-mid {
		0% {
			opacity: 1;
		}
		16%,
		90% {
			opacity: 0;
		}
		91%,
		to {
			opacity: 1;
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
	@keyframes show-late {
		0%,
		39% {
			opacity: 0;
		}
		40%,
		to {
			opacity: 1;
		}
	}
	@keyframes hide-late {
		0%,
		39% {
			opacity: 1;
		}
		40%,
		to {
			opacity: 0;
		}
	}
	@keyframes dance {
		0%,
		to {
			transform: translateY(0) rotate(0);
		}
		25% {
			transform: translateY(-16px) rotate(-12deg);
		}
		50% {
			transform: translateY(0) rotate(0);
		}
		75% {
			transform: translateY(-16px) rotate(12deg);
		}
	}
</style>
