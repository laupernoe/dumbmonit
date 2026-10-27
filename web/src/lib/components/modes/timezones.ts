/** IANA time zones the browser knows, the browser's own first; a short fallback on old engines. */
export function timeZones(): string[] {
	const own = browserTimeZone();
	let all: string[] = [];
	try {
		const intl = Intl as unknown as { supportedValuesOf?: (key: string) => string[] };
		all = intl.supportedValuesOf?.('timeZone') ?? [];
	} catch {
		all = [];
	}
	if (all.length === 0) all = ['UTC', 'Europe/London', 'Europe/Paris', 'Europe/Zurich', 'America/New_York', 'America/Los_Angeles', 'Asia/Tokyo'];
	return [own, ...all.filter((zone) => zone !== own)].filter(Boolean);
}

export function browserTimeZone(): string {
	try {
		return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
	} catch {
		return 'UTC';
	}
}
