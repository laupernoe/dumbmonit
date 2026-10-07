/**
 * What's new: the few headline changes of each release, shown once in a small
 * window after an update. The full detail stays in CHANGELOG.md and on the
 * GitHub release. Newest first; only the versions listed here can be shown.
 *
 * Before tagging vX.Y.Z, add its entry here (see CONTRIBUTING.md, "Releasing").
 * Three to five highlights, short, in English.
 */
export interface Highlight {
	title: string;
	text: string;
}

export interface Release {
	/** Exactly the version the server reports (`0.1.0-alpha.7`, no leading `v`). */
	version: string;
	/** ISO date of the release. */
	date: string;
	highlights: Highlight[];
}

export const RELEASES: Release[] = [
	{
		version: '0.1.0-alpha.7',
		date: '2026-10-08',
		highlights: [
			{
				title: 'A city scene on status pages',
				text: 'Pick up to six of Venice, Paris, Tokyo, New York, London and Rome to sit behind the banner, and rotate them every visit, minute, ten minutes or hour.'
			},
			{
				title: 'Translations are here',
				text: 'The interface now ships in French, German, Spanish, Italian, Portuguese, Brazilian Portuguese, Russian and Simplified Chinese, with more on Weblate.'
			},
			{
				title: 'Simpler device pages',
				text: 'A direct change button, and the security score folded away until you want it.'
			},
			{
				title: 'Calmer weather',
				text: 'Alerts you dismissed or snoozed no longer colour the forecast on the overview.'
			}
		]
	}
];

export function releaseFor(version: string): Release | undefined {
	return RELEASES.find((release) => release.version === version);
}

export function releaseNotesUrl(version: string): string {
	return `https://github.com/laupernoe/dumbmonit/releases/tag/v${encodeURIComponent(version)}`;
}
