import { m } from '#lib/paraglide/messages.js';

/**
 * Translated label of a collector kind. The server serves English labels;
 * kinds the UI has a message for are shown in the user's language, any other
 * kind (new server-side, community pack) keeps the label the server sent.
 */
export function kindLabel(kind: string, serverLabel?: string | null): string {
	const message = (m as unknown as Record<string, (() => string) | undefined>)[`kinds_label_${kind}`];
	return message?.() ?? serverLabel ?? kind;
}
