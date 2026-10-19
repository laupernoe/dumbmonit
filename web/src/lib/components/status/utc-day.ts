/**
 * A UTC calendar day (`YYYY-MM-DD`, as the status buckets carry them) in the
 * reader's language, without a time.
 *
 * The formatter is built once per locale: a status page labels every square
 * of every history bar (a hundred services × ninety days), and building an
 * `Intl.DateTimeFormat` costs far more than using one.
 */
const FORMATS = new Map<string, Intl.DateTimeFormat>();

export function formatUtcDay(date: string, locale: string): string {
	const parsed = new Date(`${date}T00:00:00Z`);
	if (Number.isNaN(parsed.getTime())) return date;
	let format = FORMATS.get(locale);
	if (!format) {
		format = new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'short', year: 'numeric', timeZone: 'UTC' });
		FORMATS.set(locale, format);
	}
	return format.format(parsed);
}
