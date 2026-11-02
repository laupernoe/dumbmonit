import type { Reroute } from '@sveltejs/kit';

/** Old bookmarks: the status page editor used to live under Settings. */
export const reroute: Reroute = ({ url }) => {
	const old = url.pathname.match(/^\/settings\/status\/([^/]+)\/?$/);
	if (old) return `/status/${old[1]}`;
};
