/**
 * Whether the window is phone-sized (below Tailwind's `sm`, where the bottom
 * tab bar replaces the top bar). Pages read it to drop what a phone has no
 * room or GPU for — the sky scene, a second column — instead of only hiding
 * it with CSS, so nothing hidden keeps animating behind the scenes.
 */
import { browser } from '$app/env';
import { PHONE_QUERY } from '#lib/ui/motion.js';

class Viewport {
	phone = $state(browser ? window.matchMedia(PHONE_QUERY).matches : false);

	constructor() {
		if (!browser) return;
		const query = window.matchMedia(PHONE_QUERY);
		query.addEventListener('change', () => (this.phone = query.matches));
	}
}

export const viewport = new Viewport();
