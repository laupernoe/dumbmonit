<script lang="ts">
	/**
	 * Tilt — a card that leans toward the cursor on a gimbal and settles flat
	 * when it leaves. Svelte 5 port of the React Bits "TiltedCard" component
	 * (CSS transform only, no dependencies, no canvas). Pair it with
	 * `Spotlight` for the glow; this one only does the lean and the lift.
	 * Under `prefers-reduced-motion` the card stays flat — the lean is
	 * decoration, nothing the reader needs to see move.
	 */
	import type { Snippet } from 'svelte';
	import { reducedMotion } from '../motion';

	interface Props {
		children: Snippet;
		class?: string;
		tag?: 'div' | 'a' | 'button' | 'article' | 'li';
		href?: string;
		/** Maximum lean, in degrees, reached at the card's edge. */
		maxTilt?: number;
		/** Scale reached at full lean; 1 disables the lift. */
		scale?: number;
		/** Extra inline style, merged with the computed transform (never overwritten by it). */
		style?: string;
		[key: string]: unknown;
	}

	let {
		children,
		class: className = '',
		tag = 'div',
		href,
		maxTilt = 5,
		scale = 1.012,
		style: extraStyle,
		...rest
	}: Props = $props();

	let rotateX = $state(0);
	let rotateY = $state(0);
	let hovering = $state(false);

	function move(e: PointerEvent) {
		if (!hovering || reducedMotion()) return;
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		if (rect.width === 0 || rect.height === 0) return;
		const px = (e.clientX - rect.left) / rect.width - 0.5;
		const py = (e.clientY - rect.top) / rect.height - 0.5;
		rotateX = -py * 2 * maxTilt;
		rotateY = px * 2 * maxTilt;
	}
	function enter() {
		if (!reducedMotion()) hovering = true;
	}
	function leave() {
		hovering = false;
		rotateX = 0;
		rotateY = 0;
	}

	const transform = $derived(
		hovering
			? `perspective(900px) rotateX(${rotateX}deg) rotateY(${rotateY}deg) scale(${scale})`
			: 'perspective(900px) rotateX(0) rotateY(0) scale(1)'
	);
	// Built by hand, not the `style:` directive: a caller's own `style` (an
	// indent on a nested device row, say) must merge with the transform, not
	// race it for the last write to the attribute.
	const fullStyle = $derived(`transform: ${transform};${extraStyle ? ` ${extraStyle}` : ''}`);
</script>

<svelte:element
	this={tag}
	{href}
	class={`[transform-style:preserve-3d] transition-transform duration-200 ease-out-expo will-change-transform ${className}`}
	style={fullStyle}
	onpointermove={move}
	onpointerenter={enter}
	onpointerleave={leave}
	{...rest}
>
	{@render children()}
</svelte:element>
