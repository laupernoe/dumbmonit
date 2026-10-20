/**
 * A relay's site and the world map: the location that stands for that site
 * (its `site`, or failing that its name), compared without case or spaces
 * around — the way the server matches topology sites to locations.
 */
export function locationForSite<L extends { name: string; site: string }>(locations: readonly L[], site: string | null | undefined): L | undefined {
	const wanted = site?.trim().toLowerCase();
	if (!wanted) return undefined;
	return (
		locations.find((l) => l.site.trim().toLowerCase() === wanted) ??
		locations.find((l) => l.name.trim().toLowerCase() === wanted)
	);
}
