/**
 * A burst of paper slips, for the few moments that earn one: the first device
 * added, the sky clearing after trouble. Never for an ordinary click.
 *
 * One fixed canvas over the page, created for the burst and removed after it
 * (about 1.8 s), pointer events off, so nothing underneath is blocked. The
 * slips are cut from the chart's own inks — signal, advisory, info — plus the
 * pigeon's beak and chest. Under `prefers-reduced-motion` nothing is drawn.
 */
import { reducedMotion } from '../motion';

export interface ConfettiOptions {
	/** Burst origin in viewport pixels; defaults to the top centre of the viewport. */
	x?: number;
	y?: number;
	/** Number of slips. */
	count?: number;
	/** Spread of the burst, in degrees around straight up. */
	spread?: number;
}

interface Slip {
	x: number;
	y: number;
	vx: number;
	vy: number;
	angle: number;
	spin: number;
	flip: number;
	flipSpeed: number;
	w: number;
	h: number;
	color: string;
	round: boolean;
}

const DURATION_MS = 1800;
const GRAVITY = 0.0011; // px / ms²
const DRAG = 0.0016; // fraction of speed lost per ms

function palette(): string[] {
	const css = getComputedStyle(document.documentElement);
	const read = (name: string, fallback: string) => css.getPropertyValue(name).trim() || fallback;
	return [
		read('--c-signal', '#0f8f86'),
		read('--c-advisory', '#b7791f'),
		read('--c-info', '#2f6db5'),
		'#e97b3a', // the pigeon's beak
		'#4c9d6f', // its chest patch
		read('--c-signal', '#0f8f86')
	];
}

export function confetti(options: ConfettiOptions = {}): void {
	if (typeof window === 'undefined' || reducedMotion() || document.hidden) return;
	const width = window.innerWidth;
	const height = window.innerHeight;
	const originX = options.x ?? width / 2;
	const originY = options.y ?? Math.min(160, height * 0.2);
	const count = options.count ?? 70;
	const spread = ((options.spread ?? 110) * Math.PI) / 180;

	const canvas = document.createElement('canvas');
	const dpr = Math.min(window.devicePixelRatio || 1, 2);
	canvas.width = width * dpr;
	canvas.height = height * dpr;
	canvas.setAttribute('aria-hidden', 'true');
	Object.assign(canvas.style, {
		position: 'fixed',
		inset: '0',
		width: `${width}px`,
		height: `${height}px`,
		pointerEvents: 'none',
		zIndex: '60'
	});
	document.body.appendChild(canvas);
	const ctx = canvas.getContext('2d');
	if (!ctx) {
		canvas.remove();
		return;
	}
	ctx.scale(dpr, dpr);

	const colors = palette();
	const slips: Slip[] = Array.from({ length: count }, (_, i) => {
		const angle = -Math.PI / 2 + (Math.random() - 0.5) * spread;
		const speed = 0.55 + Math.random() * 0.75; // px / ms
		return {
			x: originX,
			y: originY,
			vx: Math.cos(angle) * speed,
			vy: Math.sin(angle) * speed,
			angle: Math.random() * Math.PI,
			spin: (Math.random() - 0.5) * 0.02,
			flip: Math.random() * Math.PI,
			flipSpeed: 0.008 + Math.random() * 0.012,
			w: 5 + Math.random() * 4,
			h: 8 + Math.random() * 6,
			color: colors[i % colors.length],
			round: i % 5 === 0
		};
	});

	const start = performance.now();
	let last = start;
	const frame = (now: number) => {
		const dt = Math.min(32, now - last);
		last = now;
		const elapsed = now - start;
		ctx.clearRect(0, 0, width, height);
		// The last third fades out: the slips leave, they do not vanish.
		const alpha = elapsed < DURATION_MS * 0.66 ? 1 : Math.max(0, 1 - (elapsed - DURATION_MS * 0.66) / (DURATION_MS * 0.34));
		ctx.globalAlpha = alpha;
		for (const slip of slips) {
			slip.vx -= slip.vx * DRAG * dt;
			slip.vy -= slip.vy * DRAG * dt;
			slip.vy += GRAVITY * dt;
			slip.x += slip.vx * dt;
			slip.y += slip.vy * dt;
			slip.angle += slip.spin * dt;
			slip.flip += slip.flipSpeed * dt;
			ctx.save();
			ctx.translate(slip.x, slip.y);
			ctx.rotate(slip.angle);
			// A paper slip turning over: its height follows the cosine of the flip.
			ctx.scale(1, Math.cos(slip.flip));
			ctx.fillStyle = slip.color;
			if (slip.round) {
				ctx.beginPath();
				ctx.arc(0, 0, slip.w / 2, 0, Math.PI * 2);
				ctx.fill();
			} else {
				ctx.fillRect(-slip.w / 2, -slip.h / 2, slip.w, slip.h);
			}
			ctx.restore();
		}
		if (elapsed < DURATION_MS && !document.hidden) requestAnimationFrame(frame);
		else canvas.remove();
	};
	requestAnimationFrame(frame);
}
