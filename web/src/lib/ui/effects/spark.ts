/**
 * The green burst at the click point: eight short strokes flying out of where
 * the pointer landed (or the middle of the control, from the keyboard).
 *
 * Drawn on a throw-away canvas fixed over the page, so it never changes the
 * layout of what was clicked, is never clipped by an `overflow: hidden`
 * button, and stays out of the accessibility tree and of the pointer's way.
 * Under `prefers-reduced-motion` nothing is drawn; the click itself is
 * untouched either way.
 *
 * `sparkle` is the action form, for a primary button (`use:sparkle`). A click
 * sparks once: the innermost listener marks the event, so a `ClickSpark`
 * wrapped around a primary `Button` does not burst twice.
 */
import { reducedMotion } from '../motion';

export interface SparkOptions {
	/** Strokes in the burst. */
	count?: number;
	/** Length of a stroke at the start, in CSS pixels. */
	size?: number;
	/** How far the strokes travel, in CSS pixels. */
	radius?: number;
	/** Length of the burst, in milliseconds. */
	duration?: number;
	/** A colour, or a `var(--token)` read from the document when the burst starts. */
	color?: string;
	/** False turns the burst off (a secondary button sharing the code path). */
	enabled?: boolean;
}

const DEFAULTS = { count: 8, size: 9, radius: 18, duration: 420, color: 'var(--c-signal)' };

/** Clicks that already burst, so an outer wrapper does not burst again. */
const sparked = new WeakSet<Event>();

function resolveColor(color: string): string {
	if (!color.startsWith('var(')) return color;
	const name = color.slice(4, -1).trim();
	return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || '#2fd17c';
}

/** Draws one burst centred on a viewport point; returns at once, cleans up after itself. */
export function burstAt(x: number, y: number, options: SparkOptions = {}): void {
	if (typeof document === 'undefined' || reducedMotion()) return;
	const { count, size, radius, duration, color } = { ...DEFAULTS, ...options };
	const reach = radius + size + 4;
	const box = reach * 2;
	const ratio = window.devicePixelRatio || 1;

	const canvas = document.createElement('canvas');
	canvas.width = Math.ceil(box * ratio);
	canvas.height = Math.ceil(box * ratio);
	canvas.setAttribute('aria-hidden', 'true');
	Object.assign(canvas.style, {
		position: 'fixed',
		left: `${x - reach}px`,
		top: `${y - reach}px`,
		width: `${box}px`,
		height: `${box}px`,
		pointerEvents: 'none',
		zIndex: '90'
	});
	const ctx = canvas.getContext('2d');
	if (!ctx) return;
	document.body.appendChild(canvas);
	ctx.scale(ratio, ratio);
	const stroke = resolveColor(color);
	const start = performance.now();

	function frame(now: number) {
		if (!ctx) return;
		const t = Math.min(1, (now - start) / duration);
		const eased = t * (2 - t);
		const dist = eased * radius;
		const len = size * (1 - eased);
		ctx.clearRect(0, 0, box, box);
		ctx.strokeStyle = stroke;
		ctx.lineWidth = 2;
		ctx.lineCap = 'round';
		for (let i = 0; i < count; i += 1) {
			const angle = (2 * Math.PI * i) / count;
			const cos = Math.cos(angle);
			const sin = Math.sin(angle);
			ctx.beginPath();
			ctx.moveTo(reach + dist * cos, reach + dist * sin);
			ctx.lineTo(reach + (dist + len) * cos, reach + (dist + len) * sin);
			ctx.stroke();
		}
		if (t < 1) requestAnimationFrame(frame);
		else canvas.remove();
	}
	requestAnimationFrame(frame);
}

/** Bursts for this click unless an inner control already did; returns whether it did. */
export function sparkFor(event: MouseEvent, element: Element, options: SparkOptions = {}): boolean {
	if (options.enabled === false || sparked.has(event)) return false;
	sparked.add(event);
	let x = event.clientX;
	let y = event.clientY;
	// A keyboard activation (Enter, Space) is a click at 0,0: burst from the middle.
	if (event.detail === 0 || (x === 0 && y === 0)) {
		const rect = element.getBoundingClientRect();
		x = rect.left + rect.width / 2;
		y = rect.top + rect.height / 2;
	}
	burstAt(x, y, options);
	return true;
}

/** Svelte action: `<button use:sparkle>` or `use:sparkle={{ enabled: primary }}`. */
export function sparkle(node: HTMLElement, options: SparkOptions = {}) {
	let current = options;
	function onclick(event: MouseEvent) {
		// A disabled control never fires `click`, but an `aria-disabled` link might.
		if (node.getAttribute('aria-disabled') === 'true') return;
		sparkFor(event, node, current);
	}
	node.addEventListener('click', onclick);
	return {
		update(next: SparkOptions = {}) {
			current = next;
		},
		destroy() {
			node.removeEventListener('click', onclick);
		}
	};
}
