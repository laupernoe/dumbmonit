/**
 * Small motion helpers shared by the effects.
 *
 * Every authored motion asks `reducedMotion()` first: under
 * `prefers-reduced-motion` the state still changes, it just does not move.
 * Only transform and opacity are animated, through the Web Animations API, so
 * nothing here triggers layout.
 */

export function reducedMotion(): boolean {
	return typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** Phones: below Tailwind's `sm` breakpoint, where the bottom tab bar replaces the top bar. */
export const PHONE_QUERY = '(max-width: 639px)';

export function isPhone(): boolean {
	return typeof window !== 'undefined' && window.matchMedia(PHONE_QUERY).matches;
}

/**
 * Whether decorative motion is off: under `prefers-reduced-motion`, and on
 * phones, where entrances, blurs, scrambles and count-ups cost frames on a
 * small GPU for a screen read in a glance. The state still changes; it just
 * does not move. Short feedback (a shake, a bump) keeps `reducedMotion()`.
 */
export function calmMotion(): boolean {
	return reducedMotion() || isPhone();
}

/**
 * A short "no" shake, for a form that refused its input. 320 ms, ±5 px: felt
 * more than seen, and it never moves the text out of reach.
 */
export function shake(element: Element | null | undefined): void {
	if (!element || reducedMotion() || !('animate' in element)) return;
	element.animate(
		[
			{ transform: 'translateX(0)' },
			{ transform: 'translateX(-5px)' },
			{ transform: 'translateX(4px)' },
			{ transform: 'translateX(-3px)' },
			{ transform: 'translateX(2px)' },
			{ transform: 'translateX(0)' }
		],
		{ duration: isPhone() ? 150 : 320, easing: 'cubic-bezier(0.16, 1, 0.3, 1)' }
	);
}

/** A bump: the element swells once and settles, for a count that just went up. */
export function bump(element: Element | null | undefined, scale = 1.28): void {
	if (!element || calmMotion() || !('animate' in element)) return;
	element.animate(
		[{ transform: 'scale(1)' }, { transform: `scale(${scale})`, offset: 0.35 }, { transform: 'scale(1)' }],
		{ duration: 420, easing: 'cubic-bezier(0.22, 1, 0.36, 1)' }
	);
}
