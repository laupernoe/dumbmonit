<script lang="ts">
	/**
	 * Before/after screenshot slider: drag the handle, or the image itself, to
	 * reveal more of one side; fully keyboard-operable as a standard
	 * `role="slider"` (arrow keys, Home/End). The reduced-motion query only
	 * drops the handle's settle animation — dragging itself is never animated.
	 */
	import { reducedMotion } from '$lib/ui';
	import { MoveHorizontal } from 'lucide-svelte';
	import { clampPercent, stepPercent } from './format';

	interface Props {
		beforeSrc: string;
		afterSrc: string;
		beforeLabel: string;
		afterLabel: string;
		class?: string;
	}

	let { beforeSrc, afterSrc, beforeLabel, afterLabel, class: className = '' }: Props = $props();

	let percent = $state(50);
	let dragging = $state(false);
	let container: HTMLDivElement | undefined;

	function percentFromClientX(clientX: number): number {
		if (!container) return percent;
		const rect = container.getBoundingClientRect();
		if (rect.width === 0) return percent;
		return clampPercent(((clientX - rect.left) / rect.width) * 100);
	}

	function startDrag(e: PointerEvent) {
		dragging = true;
		percent = percentFromClientX(e.clientX);
		(e.currentTarget as Element).setPointerCapture?.(e.pointerId);
	}

	function moveDrag(e: PointerEvent) {
		if (!dragging) return;
		percent = percentFromClientX(e.clientX);
	}

	function endDrag() {
		dragging = false;
	}

	function onKeydown(e: KeyboardEvent) {
		const next = stepPercent(percent, e.key);
		if (next === percent) return;
		e.preventDefault();
		percent = next;
	}
</script>

<div
	bind:this={container}
	role="presentation"
	class={`relative w-full touch-none overflow-hidden rounded-[var(--radius-card)] border border-line bg-canvas-deep select-none ${className}`}
	style="aspect-ratio: 16 / 10"
	onpointerdown={startDrag}
	onpointermove={moveDrag}
	onpointerup={endDrag}
	onpointercancel={endDrag}
>
	<img src={afterSrc} alt={afterLabel} class="absolute inset-0 size-full object-contain" draggable="false" />
	<div class="absolute inset-0 overflow-hidden" style={`clip-path: inset(0 ${100 - percent}% 0 0)`}>
		<img src={beforeSrc} alt={beforeLabel} class="absolute inset-0 size-full object-contain" draggable="false" />
	</div>

	<div class="pointer-events-none absolute inset-y-0 w-px bg-surface shadow-[0_0_0_1px_rgb(0_0_0/0.25)]" style={`left: ${percent}%`}></div>

	<div
		role="slider"
		tabindex={0}
		aria-valuenow={Math.round(percent)}
		aria-valuemin={0}
		aria-valuemax={100}
		aria-orientation="horizontal"
		aria-label="Before/after comparison position"
		class={`absolute top-1/2 flex size-9 -translate-x-1/2 -translate-y-1/2 cursor-grab items-center justify-center rounded-full border border-line-strong bg-surface text-ink-2 shadow-lift outline-none focus-visible:ring-2 focus-visible:ring-signal active:scale-95 ${reducedMotion() ? '' : 'transition-transform duration-150 ease-out-expo'}`}
		style={`left: ${percent}%`}
		onpointerdown={startDrag}
		onkeydown={onKeydown}
	>
		<MoveHorizontal class="size-4" aria-hidden="true" />
	</div>

	<span class="pointer-events-none absolute top-2 left-2 rounded-[var(--radius-plate)] border border-line bg-surface/90 px-2 py-0.5 text-[0.75rem] font-semibold text-ink-2 backdrop-blur-sm">
		Before
	</span>
	<span class="pointer-events-none absolute top-2 right-2 rounded-[var(--radius-plate)] border border-line bg-surface/90 px-2 py-0.5 text-[0.75rem] font-semibold text-ink-2 backdrop-blur-sm">
		After
	</span>
</div>
