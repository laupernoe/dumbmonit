<script lang="ts">
	/**
	 * The stag, as an SVG group standing on (0, 0), facing left. It grazes with
	 * its head down; on `down` it snaps its head up and freezes, on `up` it
	 * relaxes and goes back to grazing. `alert` holds the raised head (the wall
	 * does while problems are open). Colours come from the --f-* properties.
	 */
	import type { ForestPulse } from './geometry.js';

	let { alert = false, pulse = null }: { alert?: boolean; pulse?: ForestPulse } = $props();

	const gesture = $derived(pulse?.kind === 'down' || pulse?.kind === 'up' ? pulse.kind : null);
</script>

{#key pulse?.at}
	<g class="deer" class:alert class:g-down={gesture === 'down'} class:g-up={gesture === 'up'}>
		<g class="legs" stroke="var(--f-deer2)" stroke-width="5" stroke-linecap="round" fill="none">
			<path d="M-26 -50L-25 -26L-27 -2M-14 -50L-13 -26L-14 -2" />
			<path d="M42 -52L46 -30L40 -2M54 -54L56 -28L52 -2" opacity=".85" />
		</g>
		<path d="M57 -74Q70 -82 67 -66Q62 -64 57 -66Z" fill="var(--f-belly)" class="tail" />
		<ellipse cx="14" cy="-62" rx="47" ry="20" fill="var(--f-deer)" />
		<ellipse cx="14" cy="-51" rx="38" ry="8" fill="var(--f-belly)" opacity=".7" />
		<g fill="var(--f-belly)" opacity=".5">
			<circle cx="0" cy="-70" r="2" /><circle cx="18" cy="-74" r="2" /><circle cx="34" cy="-68" r="2" /><circle cx="26" cy="-62" r="1.8" />
		</g>
		<g transform="translate(-22 -70)">
			<g class="neck">
				<path d="M-11 8L11 8L5 -58L-6 -58Z" fill="var(--f-deer)" />
				<g transform="translate(0 -58)">
					<g class="head">
						<path class="ear" d="M3-8Q12-27 17-23Q15-12 8-5Z" fill="var(--f-deer2)" />
						<path d="M8-8C-6-13-22-3-32 6C-34 10-30 13-26 12C-14 14 0 10 8 6Z" fill="var(--f-deer)" />
						<path d="M-30 7l-3 3 4 2z" fill="var(--f-deer2)" />
						<circle cx="-6" cy="-2" r="1.9" fill="var(--f-ink)" />
						<path d="M2-9L8-30M6-22L-4-33M8-30L19-38M7-24L17-25" stroke="var(--f-antler)" stroke-width="2.6" stroke-linecap="round" fill="none" />
					</g>
				</g>
			</g>
		</g>
	</g>
{/key}

<style>
	.neck {
		--rest: -127deg;
		transform: rotate(var(--rest));
		transform-origin: 0 0;
		transition: transform 0.9s ease-in-out;
	}
	.head {
		--rest: 52deg;
		transform: rotate(var(--rest));
		transform-origin: 0 0;
		transition: transform 0.9s ease-in-out;
	}
	.alert .neck {
		--rest: -26deg;
	}
	.alert .head {
		--rest: 30deg;
	}
	.ear {
		transform-origin: 3px -8px;
	}
	.tail {
		transform-origin: 57px -70px;
	}
	/* Grazing: the head nibbles, an ear twitches, the tail flicks. */
	.deer:not(.alert):not(.g-down):not(.g-up) .head {
		animation: nibble 6s ease-in-out infinite;
	}
	.deer:not(.g-down) .ear {
		animation: twitch 7s ease-in-out infinite;
	}
	.tail {
		animation: flick 9s ease-in-out infinite;
	}
	/* Down: the head snaps up and the stag holds perfectly still. */
	.g-down .neck {
		animation: neck-down 7s ease-in-out both;
	}
	.g-down .head {
		animation: head-down 7s ease-in-out both;
	}
	/* Up: it relaxes, lowers its head and starts grazing again. */
	.g-up .neck {
		animation: neck-up 3.6s ease-in-out both;
	}
	.g-up .head {
		animation: head-up 3.6s ease-in-out both;
	}
	@keyframes nibble {
		0%, 55%, 100% { transform: rotate(52deg); }
		65% { transform: rotate(60deg); }
		75% { transform: rotate(50deg); }
		85% { transform: rotate(58deg); }
	}
	@keyframes twitch {
		0%, 90%, 100% { transform: rotate(0); }
		94% { transform: rotate(-16deg); }
	}
	@keyframes flick {
		0%, 92%, 100% { transform: rotate(0); }
		95% { transform: rotate(-18deg); }
	}
	@keyframes neck-down {
		0% { transform: rotate(var(--rest)); }
		10% { transform: rotate(-20deg); }
		14%, 82% { transform: rotate(-26deg); }
		100% { transform: rotate(var(--rest)); }
	}
	@keyframes head-down {
		0% { transform: rotate(var(--rest)); }
		10% { transform: rotate(34deg); }
		14%, 82% { transform: rotate(30deg); }
		100% { transform: rotate(var(--rest)); }
	}
	@keyframes neck-up {
		0%, 14% { transform: rotate(-26deg); }
		60% { transform: rotate(-127deg); }
		68% { transform: rotate(-120deg); }
		100% { transform: rotate(var(--rest)); }
	}
	@keyframes head-up {
		0%, 14% { transform: rotate(30deg); }
		60% { transform: rotate(52deg); }
		100% { transform: rotate(var(--rest)); }
	}
	@media (prefers-reduced-motion: reduce) {
		.neck, .head, .ear, .tail { animation: none !important; transition: none; }
	}
</style>
