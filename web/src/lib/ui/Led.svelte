<script lang="ts">
	/**
	 * The status LED on a faceplate. Steady teal breathes when reporting;
	 * amber or red blinks a fault code (two short drop-outs, then a pause)
	 * when something is wrong; ghost is unlit when disabled or never probed.
	 * Always paired with a text label somewhere in the row.
	 *
	 * `boot` flickers the LED on once, like a unit powering up in the rack;
	 * `bootDelay` lines that up with the row's entrance. The glow is its own
	 * layer animated by opacity alone, so a rack of forty breathing LEDs
	 * repaints nothing.
	 */
	interface Props {
		tone: 'signal' | 'advisory' | 'warning' | 'ghost' | 'info';
		blink?: boolean;
		size?: 'sm' | 'md' | 'lg';
		/** Flicker on once when it first appears. */
		boot?: boolean;
		/** Milliseconds before the boot flicker, to follow a staggered entrance. */
		bootDelay?: number;
		class?: string;
		label?: string;
	}

	let { tone, blink = false, size = 'md', boot = false, bootDelay = 0, class: className = '', label }: Props = $props();

	const COLOR: Record<Props['tone'], string> = {
		signal: 'var(--c-signal)',
		advisory: 'var(--c-advisory)',
		warning: 'var(--c-warning)',
		info: 'var(--c-info)',
		ghost: 'var(--c-ghost)'
	};
	const SIZE = { sm: '0.5rem', md: '0.625rem', lg: '0.875rem' };

	const mode = $derived(blink ? 'fault' : tone === 'signal' ? 'breathe' : tone === 'ghost' ? 'off' : 'steady');
</script>

<span
	class={`led inline-block shrink-0 rounded-full ${boot && tone !== 'ghost' ? 'led--boot' : ''} ${className}`}
	data-mode={mode}
	style:--led={COLOR[tone]}
	style:--led-glow={tone === 'ghost' ? 'transparent' : COLOR[tone]}
	style:--led-delay={`${bootDelay}ms`}
	style:width={SIZE[size]}
	style:height={SIZE[size]}
	role={label ? 'img' : undefined}
	aria-label={label}
	aria-hidden={label ? undefined : 'true'}
></span>

<style>
	.led {
		position: relative;
		background: radial-gradient(circle at 35% 35%, rgb(255 255 255 / 0.55), transparent 45%), var(--led);
		box-shadow: inset 0 0 0 1px rgb(0 0 0 / 0.12);
	}
	/* The glow: a soft disc behind the lens, faded in and out by opacity only. */
	.led::after {
		content: '';
		position: absolute;
		inset: -6px;
		border-radius: 9999px;
		background: radial-gradient(circle, var(--led-glow) 0%, transparent 66%);
		opacity: 0;
		pointer-events: none;
		transition:
			opacity 260ms var(--ease-out-expo),
			transform 260ms var(--ease-out-expo);
	}
	.led[data-mode='breathe']::after {
		animation: led-glow 3.2s ease-in-out infinite;
	}
	.led[data-mode='steady']::after,
	.led[data-mode='fault']::after {
		opacity: 0.45;
	}
	.led[data-mode='fault'] {
		animation: led-fault 1.6s linear infinite;
	}

	/* Power-on: a short flicker, then whatever the tone does. */
	.led--boot {
		animation: led-boot 640ms linear both;
		animation-delay: var(--led-delay, 0ms);
	}
	.led--boot[data-mode='fault'] {
		animation:
			led-boot 640ms linear both,
			led-fault 1.6s linear infinite;
		animation-delay: var(--led-delay, 0ms), calc(var(--led-delay, 0ms) + 640ms);
	}

	@keyframes led-glow {
		0%,
		100% {
			opacity: 0.15;
		}
		50% {
			opacity: 0.9;
		}
	}
	/* A fault code: lit, two short drop-outs, lit again. Under 2 Hz, never a strobe. */
	@keyframes led-fault {
		0%,
		58% {
			opacity: 1;
		}
		58.01%,
		66% {
			opacity: 0.2;
		}
		66.01%,
		76% {
			opacity: 1;
		}
		76.01%,
		84% {
			opacity: 0.2;
		}
		84.01%,
		100% {
			opacity: 1;
		}
	}
	@keyframes led-boot {
		0% {
			opacity: 0.12;
		}
		22% {
			opacity: 1;
		}
		34% {
			opacity: 0.3;
		}
		52%,
		100% {
			opacity: 1;
		}
	}
</style>
