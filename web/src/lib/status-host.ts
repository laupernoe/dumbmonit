/**
 * The status page this origin is dedicated to, if any.
 *
 * When a request arrives on a status page's public domain (`status.example.com`
 * pointed at DumbMonit by a reverse proxy), the server serves the app with
 * `<meta name="dumbmonit-status-page" content="<slug>">` and answers nothing
 * else on that host. The root layout then shows that page at `/`, without the
 * session check, the nav or anything else of the instance.
 *
 * Read once at startup: the host does not change during a visit.
 */
function readHostedSlug(): string | null {
	if (typeof document === 'undefined') return null;
	const content = document
		.querySelector('meta[name="dumbmonit-status-page"]')
		?.getAttribute('content');
	return content && /^[a-z0-9-]{2,40}$/.test(content) ? content : null;
}

export const hostedStatusSlug: string | null = readHostedSlug();
