/**
 * Where an old destination now lives: a tab of the page that absorbed it.
 * `/map` became `/targets?tab=map`, `/actions` became `/alerts?tab=automation`,
 * and so on. The query string of the old address rides along (a filter, a
 * monitor id), `extra` adds what the old address carried in its hash, and the
 * hash is kept unless `extra` consumed it.
 */
export function tabUrl(
	path: string,
	tab: string,
	from: { searchParams: URLSearchParams; hash?: string },
	extra: Record<string, string> = {},
	defaultTab = ''
): string {
	const params = new URLSearchParams(from.searchParams);
	if (tab === defaultTab) params.delete('tab');
	else params.set('tab', tab);
	for (const [key, value] of Object.entries(extra)) params.set(key, value);
	const query = params.toString();
	return `${path}${query ? `?${query}` : ''}${from.hash ?? ''}`;
}
