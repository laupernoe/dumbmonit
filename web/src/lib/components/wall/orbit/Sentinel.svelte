<!--
	The sentinel: a small decorative satellite standing in for a device. It is
	drawn around (0, 0) in local units (about 70 wide); the caller places and
	scales it. Colours come from `--orb-hull`, `--orb-panel`, `--orb-ok`,
	`--orb-bad`. Its pulse is replayed whenever `at` changes:
	  down     the lamp stutters out and the satellite tumbles off its heading
	  up       it rights itself with a little overshoot and pings three rings
	  resolved a wide green ring and four sparkles
	Reduced motion keeps the final pose, without movement.
-->
<script lang="ts">
	interface Props {
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}
	let { pulse = null }: Props = $props();
</script>

{#key pulse?.at ?? 0}
	<g class="sat {pulse ? `sat--${pulse.kind}` : ''}">
		{#if pulse?.kind === 'up'}
			<circle class="ring r1" r="6" />
			<circle class="ring r2" r="6" />
			<circle class="ring r3" r="6" />
		{:else if pulse?.kind === 'resolved'}
			<circle class="ring big" r="6" />
			<path class="spark s1" d="M-30-22l2 5 5 2-5 2-2 5-2-5-5-2 5-2z" />
			<path class="spark s2" d="M32-26l2 5 5 2-5 2-2 5-2-5-5-2 5-2z" />
			<path class="spark s3" d="M-40 16l1.6 4 4 1.6-4 1.6-1.6 4-1.6-4-4-1.6 4-1.6z" />
			<path class="spark s4" d="M38 18l1.6 4 4 1.6-4 1.6-1.6 4-1.6-4-4-1.6 4-1.6z" />
		{/if}
		<g class="tilt">
			<path d="M-9 -6L-34 -6M9 -6L34 -6" stroke="var(--orb-hull, #9aa6c0)" stroke-width="1.4" />
			<rect x="-36" y="-12" width="24" height="12" rx="1" fill="var(--orb-panel, #3f6fc4)" />
			<rect x="12" y="-12" width="24" height="12" rx="1" fill="var(--orb-panel, #3f6fc4)" />
			<path
				d="M-30 -12v12M-24 -12v12M-18 -12v12M18 -12v12M24 -12v12M30 -12v12"
				stroke="var(--orb-hull, #9aa6c0)"
				stroke-width=".5"
				opacity=".7"
			/>
			<rect x="-9" y="-9" width="18" height="14" rx="2.5" fill="var(--orb-hull, #cfd6e6)" />
			<rect x="-9" y="-9" width="18" height="4" rx="2" fill="#fff" opacity=".35" />
			<path d="M-5 -9Q0 -20 5 -9" fill="none" stroke="var(--orb-hull, #cfd6e6)" stroke-width="1.2" />
			<circle class="lamp" cx="0" cy="-1" r="2.4" />
		</g>
	</g>
{/key}

<style>
	.spark {
		fill: #eafff2;
		opacity: 0;
		transform-box: fill-box;
		transform-origin: center;
	}
	.lamp {
		fill: var(--orb-ok, #5fd39a);
		animation: lamp-blink 3.2s ease-in-out infinite;
	}
	.ring {
		fill: none;
		stroke: var(--orb-ok, #5fd39a);
		stroke-width: 1.2;
		opacity: 0;
	}

	.sat--down .tilt {
		transform: translate(10px, 14px) rotate(28deg);
		animation: tumble 2.6s cubic-bezier(0.3, 0, 0.3, 1) both;
	}
	.sat--down .lamp {
		fill: var(--orb-bad, #ff6a55);
		opacity: 0.45;
		animation: lamp-fail 2.6s steps(1, end) both;
	}
	.sat--up .tilt {
		animation: right-up 1.7s cubic-bezier(0.3, 1.5, 0.5, 1) both;
	}
	.sat--up .ring {
		animation: ping 2.4s ease-out both;
	}
	.sat--up .r2 {
		animation-delay: 0.45s;
	}
	.sat--up .r3 {
		animation-delay: 0.9s;
	}
	.sat--resolved .ring {
		animation: ping 3s ease-out both;
	}
	.sat--resolved .spark {
		animation: twinkle 1.8s ease-in-out both;
	}
	.sat--resolved .s2 {
		animation-delay: 0.25s;
	}
	.sat--resolved .s3 {
		animation-delay: 0.5s;
	}
	.sat--resolved .s4 {
		animation-delay: 0.75s;
	}

	@keyframes lamp-blink {
		0%,
		70%,
		100% {
			opacity: 1;
		}
		80% {
			opacity: 0.25;
		}
	}
	@keyframes lamp-fail {
		0% {
			opacity: 1;
		}
		12% {
			opacity: 0.1;
		}
		24% {
			opacity: 1;
		}
		36% {
			opacity: 0.1;
		}
		48% {
			opacity: 1;
		}
		60%,
		100% {
			opacity: 0.45;
		}
	}
	@keyframes tumble {
		from {
			transform: none;
		}
		to {
			transform: translate(10px, 14px) rotate(28deg);
		}
	}
	@keyframes right-up {
		from {
			transform: translate(10px, 14px) rotate(28deg);
		}
		to {
			transform: none;
		}
	}
	@keyframes ping {
		from {
			opacity: 0.9;
			transform: scale(1);
		}
		to {
			opacity: 0;
			transform: scale(9);
		}
	}
	@keyframes twinkle {
		0%,
		100% {
			opacity: 0;
			transform: scale(0.2);
		}
		40% {
			opacity: 1;
			transform: scale(1);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.sat * {
			animation: none !important;
		}
		.ring {
			display: none;
		}
	}
</style>
