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
		version: '0.11.0',
		date: '2026-10-19',
		highlights: [
			{
				title: 'Sign in with LDAP or Active Directory',
				text: 'People sign in with their directory account on the usual sign-in page; groups map to DumbMonit roles.'
			},
			{
				title: 'Pick your blocklists',
				text: 'Mail server and IP range monitors need no API key: tick the lists to query, add your own DNSBL zones, and open MXToolbox for a second opinion.'
			},
			{
				title: 'Alerts at a glance',
				text: 'A bell with the number of active alerts now shows on each monitor, in the Monitors views and on the monitor page.'
			},
			{
				title: 'A down parent hides its children',
				text: 'When a switch or a host is down, whatever the cause, you get one alert instead of one per device behind it.'
			},
			{
				title: 'Now MIT-licensed',
				text: 'DumbMonit moves from Apache-2.0 to the MIT license. It is free and will stay free.'
			}
		]
	},
	{
		version: '0.10.0',
		date: '2026-10-18',
		highlights: [
			{
				title: 'A wall that tells you more',
				text: 'The weather under the clock, and cards in rotation for crypto prices, stocks, exchange rates, the latest critical CVEs and your Claude and OpenAI usage, plus news pictures from your RSS feeds.'
			},
			{
				title: 'Is your mail server blocklisted?',
				text: 'Two new monitors check a mail domain or a public IP range against Spamhaus, Barracuda, SpamCop and more, with reverse DNS, SPF and DMARC, and the delisting page of every list that lists you.'
			},
			{
				title: 'Quieter maintenance',
				text: 'An alert that recovers during a silence or a maintenance window no longer sends a notification; a switch in Settings brings it back.'
			},
			{
				title: 'Made for phones',
				text: 'The Overview fits one screen, lists scroll under a fixed toolbar, wide tables become cards and nothing scrolls sideways any more.'
			}
		]
	},
	{
		version: '0.9.0',
		date: '2026-10-16',
		highlights: [
			{
				title: 'Simpler Alerts and Monitors',
				text: 'Alerts is a calm list, one line per problem with Ack and Open on it. What you watch is now called a monitor, and the Monitors page has one toolbar and a Select mode for bulk actions.'
			},
			{
				title: 'Act on alerts from the notification',
				text: 'Acknowledge or snooze from a button or a signed link in the message, on ntfy, Telegram, Web Push, Discord, Slack, e-mail and more.'
			},
			{
				title: 'A monitor per service, and a map',
				text: 'Docker, backups, ZFS, WireGuard and the like found by an agent each get their own status and alerts. The Map page draws parents, relays and VMs by itself.'
			},
			{
				title: 'Who watches the watcher',
				text: 'An outgoing heartbeat tells an outside service that DumbMonit is alive, new rules watch its own health, Docker shows the container as healthy, and a bind-mounted /data just works.'
			},
			{
				title: 'Easier to start, more to monitor',
				text: 'Import your Uptime Kuma monitors in one step, plus XCP-ng, Nutanix AHV, OpenVPN, KVM and IPsec, a speed test, and a security score you turn on only if you want it.'
			}
		]
	},
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
