/**
 * Requests in flight, shared.
 *
 * Several views of a page often ask for the same data at the same moment (the
 * target list, the alert history, the same metric query). While the first
 * request has not answered, the others join it: one network request, one
 * server-side query, an answer for everybody. Nothing is kept once it has
 * answered, so the next call always reads fresh data.
 *
 * The network request is cancelled only when every caller that joined it has
 * given up: one view leaving a page does not break another's request.
 */
interface Flight<T> {
	promise: Promise<T>;
	controller: AbortController;
	/** Callers still waiting for the answer. */
	waiters: number;
}

const flights = new Map<string, Flight<unknown>>();

function abortError(): DOMException {
	return new DOMException('The operation was aborted.', 'AbortError');
}

/**
 * Runs `start` once for all the callers that ask for `key` at the same time.
 * `start` receives the signal to give to its `fetch`; `signal` is this caller's.
 */
export function shareFlight<T>(
	key: string,
	start: (signal: AbortSignal) => Promise<T>,
	signal?: AbortSignal
): Promise<T> {
	if (signal?.aborted) return Promise.reject(abortError());

	let flight = flights.get(key) as Flight<T> | undefined;
	if (!flight) {
		const controller = new AbortController();
		const created: Flight<T> = { promise: start(controller.signal), controller, waiters: 0 };
		flight = created;
		flights.set(key, created);
		const forget = () => {
			if (flights.get(key) === created) flights.delete(key);
		};
		created.promise.then(forget, forget);
	}

	const joined = flight;
	joined.waiters += 1;
	return new Promise<T>((resolve, reject) => {
		const leave = () => signal?.removeEventListener('abort', onAbort);
		const onAbort = () => {
			joined.waiters -= 1;
			if (joined.waiters === 0) {
				joined.controller.abort();
				if (flights.get(key) === joined) flights.delete(key);
			}
			reject(abortError());
		};
		signal?.addEventListener('abort', onAbort, { once: true });
		joined.promise.then(
			(value) => {
				leave();
				resolve(value);
			},
			(cause) => {
				leave();
				reject(cause);
			}
		);
	});
}
