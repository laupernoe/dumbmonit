/**
 * Channel delivery errors, made readable.
 *
 * The server reports a failed delivery as a sentence that may carry the
 * service's own JSON body, e.g.
 *   Sending to "ops" failed: the service answered 401: {"message":"401: Unauthorized"}
 * This module splits that into the sentence, the HTTP status and the kind of
 * failure, so the UI can name a fix. The raw string is kept for a Details block.
 * Pure on purpose: no i18n here, the component maps `kind` to a hint.
 */

export type ChannelErrorKind =
	| 'config'
	| 'transport'
	| 'auth'
	| 'not_found'
	| 'rate_limit'
	| 'server'
	| 'rejected'
	| 'unknown';

export interface ReadableChannelError {
	/** What failed, in words, with the service's own message when it sent one. */
	text: string;
	/** HTTP status the service answered with; `null` when none is known. */
	status: number | null;
	kind: ChannelErrorKind;
	/** The untouched input, for the Details block. */
	raw: string;
}

/** Keys under which services put their human-readable explanation. */
const DETAIL_KEYS = ['message', 'error_description', 'error', 'detail', 'description', 'msg', 'reason'];

/** Digs the first human-readable string out of a parsed JSON body. */
function findDetail(value: unknown, depth = 0): string | null {
	if (depth > 3 || value === null || value === undefined) return null;
	if (typeof value === 'string') return value.trim() || null;
	if (Array.isArray(value)) {
		for (const item of value) {
			const found = findDetail(item, depth + 1);
			if (found) return found;
		}
		return null;
	}
	if (typeof value === 'object') {
		const record = value as Record<string, unknown>;
		for (const key of DETAIL_KEYS) {
			if (!(key in record)) continue;
			const found = findDetail(record[key], depth + 1);
			if (found) return found;
		}
	}
	return null;
}

/**
 * Splits `head: {json}` into the text before the JSON and the parsed JSON.
 * `json` is `undefined` when the tail is not a complete JSON document.
 */
function splitJson(raw: string): { head: string; json: unknown } {
	const start = raw.search(/[{[]/);
	if (start === -1) return { head: raw, json: undefined };
	const end = Math.max(raw.lastIndexOf('}'), raw.lastIndexOf(']'));
	if (end <= start) return { head: raw, json: undefined };
	try {
		return { head: raw.slice(0, start), json: JSON.parse(raw.slice(start, end + 1)) };
	} catch {
		return { head: raw, json: undefined };
	}
}

function kindFor(status: number | null, lowered: string): ChannelErrorKind {
	if (status === 0) return 'transport';
	if (status !== null) {
		if (status === 401 || status === 403) return 'auth';
		if (status === 404) return 'not_found';
		if (status === 429) return 'rate_limit';
		if (status >= 500) return 'server';
		return 'rejected';
	}
	if (lowered.includes('configuration') || lowered.includes('unknown channel kind')) return 'config';
	if (lowered.includes('unreachable') || lowered.includes('timed out')) return 'transport';
	return 'unknown';
}

/**
 * Reads a channel error. `httpStatus` is the status of the failed request when
 * the caller has it (0 when the request never completed); otherwise the status
 * is looked for in the text ("the service answered 401").
 */
export function readChannelError(raw: string, httpStatus?: number): ReadableChannelError {
	const input = raw.trim();
	const { head, json } = splitJson(input);
	const detail = findDetail(json);

	const headText = head.trimEnd().replace(/:$/, '').trimEnd();
	const parts: string[] = [];
	if (headText) parts.push(headText);
	if (detail && !headText.endsWith(detail)) parts.push(detail);
	const text = parts.length > 0 ? parts.join(': ') : input;

	const matched = /\b(?:answered|HTTP|status)\s*:?\s*([1-5]\d{2})\b/i.exec(input);
	const status = httpStatus ?? (matched ? Number(matched[1]) : null);

	return {
		text,
		status: status === 0 ? null : status,
		kind: kindFor(status, input.toLowerCase()),
		raw: input
	};
}
