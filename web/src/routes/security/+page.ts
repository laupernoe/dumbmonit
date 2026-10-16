/**
 * The former Security page: the score now sits next to each device, and is
 * switched on or off in Settings. Old links and bookmarks land on that switch.
 */
import { redirect } from '@sveltejs/kit';

export function load(): never {
	redirect(307, '/settings#security-score');
}
