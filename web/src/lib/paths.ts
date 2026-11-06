/**
 * Base path of the app. Empty on the normal build (served at the root of the
 * binary); `/demo` on the static demo, which lives under a sub-path. Set at
 * build time by `VITE_BASE_PATH`, in step with `kit.paths.base`
 * (see `vite.config.ts`).
 *
 * Internal links and `goto` calls are written root-relative (`/targets/3`) and
 * go through `withBase`; `page.url.pathname` carries the base, so route
 * comparisons go through `routePath`. With an empty base all three are the
 * identity.
 */
export const BASE: string = (import.meta.env.VITE_BASE_PATH as string | undefined) ?? '';

/** Paths the server owns (API, downloads, service worker): never under the base. */
const SERVER_PREFIXES = ['/api/', '/download/', '/sw.js'];

/**
 * Prefixes an internal root-relative path with the base. Idempotent (a path
 * already under the base is returned as is) and leaves anything else alone:
 * absolute URLs, `#anchors`, `//host`, server-owned paths, non-strings.
 */
export function withBase<T>(path: T): T {
	if (!BASE || typeof path !== 'string') return path;
	if (!path.startsWith('/') || path.startsWith('//')) return path;
	if (path === BASE || path.startsWith(`${BASE}/`) || path.startsWith(`${BASE}?`) || path.startsWith(`${BASE}#`)) {
		return path;
	}
	if (SERVER_PREFIXES.some((prefix) => path.startsWith(prefix))) return path;
	return (BASE + path) as T;
}

/** The pathname of a URL without the base: what the routes and the nav compare against. */
export function stripBase(pathname: string): string {
	if (!BASE) return pathname;
	if (pathname === BASE) return '/';
	return pathname.startsWith(`${BASE}/`) ? pathname.slice(BASE.length) : pathname;
}

/** `stripBase(url.pathname)`. */
export function routePath(url: { pathname: string }): string {
	return stripBase(url.pathname);
}
