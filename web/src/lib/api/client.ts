/**
 * The single HTTP client of the interface.
 *
 * Every network access goes through here: no `fetch` may be written inside a
 * component. This guarantees uniform error handling, consistent user-facing
 * messages, and a single place to change if the API evolves.
 */

import { m } from '#lib/paraglide/messages.js';
import { notifyMutation } from './mutations';

/** Normalised error, always carrying a message that can be displayed as is. */
export class ApiError extends Error {
	/** HTTP status, or 0 if the request never completed (network down, server off). */
	readonly status: number;
	/** True when the route does not exist on the server yet (404 on an anticipated endpoint). */
	readonly missing: boolean;
	/** True when the public demo refused a change (403 with `demo: true`). */
	readonly demo: boolean;

	/** Set on the 401 of a private status page: what the visitor has to provide. */
	access: 'password' | 'users' | null = null;

	constructor(message: string, status: number, missing = false, demo = false) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
		this.missing = missing;
		this.demo = demo;
	}

	/** Concrete advice shown to the user below the error message. */
	get hint(): string {
		if (this.status === 0) {
			return m.api_hint_network();
		}
		if (this.status === 401) {
			return m.api_hint_unauthorized();
		}
		if (this.demo) {
			return m.api_hint_demo();
		}
		if (this.status === 403) {
			return m.api_hint_forbidden();
		}
		if (this.status === 404) {
			return m.api_hint_not_found();
		}
		if (this.status === 429) {
			return m.api_hint_rate_limited();
		}
		if (this.status === 409) {
			return m.api_hint_conflict();
		}
		if (this.status >= 500) {
			return m.api_hint_server();
		}
		return m.api_hint_invalid();
	}
}

/** Turns any thrown value into a displayable `ApiError`. */
export function toApiError(cause: unknown): ApiError {
	if (cause instanceof ApiError) return cause;
	if (cause instanceof Error) return new ApiError(cause.message, 0);
	return new ApiError(m.api_error_unexpected(), 0);
}

interface RequestOptions {
	method?: 'GET' | 'POST' | 'PUT' | 'DELETE';
	body?: unknown;
	query?: Record<string, string | number | boolean | undefined>;
	signal?: AbortSignal;
	/**
	 * Marks a route not yet implemented on the server. A 404 then becomes an
	 * `ApiError` with `missing = true`, which the caller can degrade gracefully
	 * instead of displaying as an outage.
	 */
	anticipated?: boolean;
	/**
	 * Prevents the automatic redirect to the sign-in screen on a 401.
	 *
	 * Reserved for routes where a 401 is an expected business response rather
	 * than an expired session: wrong password at sign-in, wrong current password
	 * when changing it. Without this, a simple typo would kick the user out of
	 * the page they are on.
	 */
	allowUnauthorized?: boolean;
}

/**
 * Single reaction to a 401 received on any route.
 *
 * The auth store registers itself here at application start-up. It is the only
 * place in the interface that decides what to do with a lost session: no
 * component has to watch HTTP status codes itself.
 */
type UnauthorizedHandler = () => void;

let unauthorizedHandler: UnauthorizedHandler | null = null;

/** Installs the 401 reaction. Called once, from the auth store. */
export function setUnauthorizedHandler(handler: UnauthorizedHandler | null): void {
	unauthorizedHandler = handler;
}

const BASE = '/api';

/**
 * MSP mode: the client picked in the header, sent with every request. The
 * server narrows lists to it for the MSP administrator, and never widens
 * anyone's access with it.
 */
let clientView: number | null = null;

export function setClientView(id: number | null): void {
	clientView = id;
}

export function getClientView(): number | null {
	return clientView;
}

/** The header carrying the picked client, to spread into a `fetch` made by hand. */
export function clientViewHeaders(): Record<string, string> {
	return clientView === null ? {} : { 'x-dumbmonit-client': String(clientView) };
}

/** Builds the final URL, skipping query parameters that are not set. */
function buildUrl(path: string, query?: RequestOptions['query']): string {
	const url = `${BASE}${path}`;
	if (!query) return url;
	const params = new URLSearchParams();
	for (const [key, value] of Object.entries(query)) {
		if (value !== undefined && value !== '') params.set(key, String(value));
	}
	const encoded = params.toString();
	return encoded ? `${url}?${encoded}` : url;
}

/**
 * Single reaction to a change refused by the public demo (403 with `demo: true`).
 *
 * Installed by the demo store: every refused action says why, even on the
 * paths whose caller would otherwise swallow the error.
 */
type DemoRefusalHandler = (message: string) => void;

let demoRefusalHandler: DemoRefusalHandler | null = null;

/** Installs the demo refusal reaction. Called once, from the demo store. */
export function setDemoRefusalHandler(handler: DemoRefusalHandler | null): void {
	demoRefusalHandler = handler;
}

/** Extracts the error message from the `{ "error": "..." }` body returned by the server. */
async function readError(
	response: Response
): Promise<{ message: string; demo: boolean; access: 'password' | 'users' | null }> {
	const fallback = { message: m.api_error_status({ status: response.status }), demo: false, access: null };
	try {
		const text = await response.text();
		if (!text) return fallback;
		const parsed: unknown = JSON.parse(text);
		if (parsed && typeof parsed === 'object' && 'error' in parsed) {
			const { error: message, demo, access } = parsed as { error: unknown; demo?: unknown; access?: unknown };
			if (typeof message === 'string' && message.trim()) {
				return {
					message,
					demo: demo === true,
					access: access === 'password' || access === 'users' ? access : null
				};
			}
		}
		return fallback;
	} catch {
		return fallback;
	}
}

/**
 * Executes a request and returns the decoded body.
 *
 * `T` is `void` for 204 responses (deletion), which have no body.
 */
export async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
	const {
		method = 'GET',
		body,
		query,
		signal,
		anticipated = false,
		allowUnauthorized = false
	} = options;

	let response: Response;
	try {
		response = await fetch(buildUrl(path, query), {
			method,
			signal,
			headers: {
				// Proof of origin for cookie-authenticated writes (anti-CSRF): a
				// third-party page cannot add this header without CORS consent.
				'x-requested-with': 'DumbMonit',
				...clientViewHeaders(),
				...(body === undefined
					? {}
					: { 'content-type': body instanceof Blob ? 'application/octet-stream' : 'application/json' })
			},
			// A file is sent as is (an upload); anything else travels as JSON.
			body: body === undefined ? undefined : body instanceof Blob ? body : JSON.stringify(body)
		});
	} catch (cause) {
		// A cancelled request is not an outage: propagate it as is so the caller
		// can ignore it silently.
		if (cause instanceof DOMException && cause.name === 'AbortError') throw cause;
		throw new ApiError(m.api_error_unreachable(), 0);
	}

	if (!response.ok) {
		const { message, demo, access } = await readError(response);
		// The session has expired, or the instance has just been protected: notify
		// once, at the client level, rather than screen by screen.
		if (response.status === 401 && !allowUnauthorized) unauthorizedHandler?.();
		const refusedByDemo = demo && response.status === 403;
		if (refusedByDemo) demoRefusalHandler?.(message);
		const failure = new ApiError(message, response.status, anticipated && response.status === 404, refusedByDemo);
		failure.access = access;
		throw failure;
	}

	notifyMutation(path, method);

	if (response.status === 204) return undefined as T;

	const text = await response.text();
	if (!text) return undefined as T;
	try {
		return JSON.parse(text) as T;
	} catch {
		throw new ApiError(m.api_error_unreadable(), response.status);
	}
}
