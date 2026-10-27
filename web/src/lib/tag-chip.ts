/**
 * Text of an option/tag chip. A value is never shown raw when it could break the
 * layout: a PEM certificate becomes "PEM certificate", anything multi-line keeps
 * its first line, and everything is cut to `max` characters with an ellipsis.
 * The full (but still bounded) value stays available for the hover title.
 */
export const TAG_CHIP_MAX = 48;

export function tagValueSummary(value: string, max = TAG_CHIP_MAX): string {
	const text = String(value ?? '').trim();
	if (/-----BEGIN [A-Z ]+-----/.test(text)) return 'PEM';
	const first = text.split(/\r?\n/, 1)[0].trim();
	const multiline = first.length < text.length;
	if (first.length > max) return first.slice(0, max - 1) + '…';
	return multiline ? first + '…' : first;
}

export function tagChip(key: string, value: string): { label: string; title: string } {
	const label = `${key} = ${tagValueSummary(value)}`;
	const raw = String(value ?? '');
	const title = /-----BEGIN [A-Z ]+-----/.test(raw) ? label : `${key} = ${raw.slice(0, 300)}`;
	return { label, title };
}
