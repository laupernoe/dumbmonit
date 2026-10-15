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

/**
 * Entry prepared for 0.9.0, kept out of RELEASES until the release is cut: the
 * `WHATSNEW_REQUIRE` check needs an entry only for the Cargo.toml version, and
 * the display rules use RELEASES[0] as the current one. At the version bump,
 * add this constant as the first element of RELEASES (and set the date).
 */
export const RELEASE_0_9_0_DRAFT: Release = {
	version: '0.9.0',
	date: '2026-10-15',
	highlights: [
		{
			title: 'Each service is its own device',
			text: 'Docker, backups, ZFS, WireGuard, IPsec and more found by an agent get their own status, alerts and silences under the machine.'
		},
		{
			title: 'A map of your infrastructure',
			text: 'The new Map page draws parents, relays, VMs and network neighbours by itself, shows what is down and what sits behind it, and exports to SVG, PNG or print.'
		},
		{
			title: 'Act on alerts from the notification',
			text: 'Acknowledge or snooze from a button or signed link in the message, and let an outside heartbeat tell you when DumbMonit itself goes quiet.'
		},
		{
			title: 'More devices, and Uptime Kuma import',
			text: 'XCP-ng, Nutanix AHV and OpenVPN servers, KVM and IPsec in the agent, a speed test, and a one-step import of your Uptime Kuma monitors.'
		},
		{
			title: 'Security score, only if you want it',
			text: 'Off by default and shown next to each device, with new checks for Linux, Windows, websites and Synology and a how-to-fix report.'
		}
	]
};

export const RELEASES: Release[] = [
	{
		version: '0.1.0-alpha.8',
		date: '2026-10-14',
		highlights: [
			{
				title: 'Charts are back',
				text: 'Alpha 7 could not read fresh metrics from VictoriaMetrics, so panels stayed empty and monitors showed Unknown. Fixed.'
			},
			{
				title: 'News on the wall',
				text: 'An optional headline ticker and live news channel on the wall, with sources for many countries or any RSS feed of your own.'
			},
			{
				title: 'Quieter website-change alerts',
				text: 'Menus, headers and footers are left out of the comparison, every check that finds changes names the pages, and all changes sit in one list.'
			},
			{
				title: 'Agents pick their modules',
				text: 'Choose which collectors an agent runs when you install it, and Plakar now works when the agent runs as a service.'
			}
		]
	},
	{
		version: '0.1.0-alpha.7',
		date: '2026-10-08',
		highlights: [
			{
				title: 'Push notifications and weekly reports',
				text: 'Alerts as native notifications on your phone or computer, and a weekly summary by e-mail.'
			},
			{
				title: 'An operator role',
				text: 'Acknowledge and snooze alerts without being able to change the configuration.'
			},
			{
				title: 'Status banner for your own website',
				text: 'A script, a WordPress plugin and a PHP include show a banner when services are down. Status pages also get a custom domain, a simple mode and 26 city scenes.'
			},
			{
				title: 'The wall travels',
				text: 'Paris, London, New York, Tokyo and Rome behind the wall, chosen automatically from your time zone.'
			},
			{
				title: 'Translated, and better on phones',
				text: 'The whole interface in eight languages besides English, reworked for small screens, with a button to check for updates.'
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
