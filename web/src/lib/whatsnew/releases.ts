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
		version: '0.14.0',
		date: '2026-10-28',
		highlights: [
			{
				title: 'Homelab or Enterprise',
				text: 'Pick how DumbMonit looks in Settings → General: the familiar interface made simpler, or a dark console with a context panel. A setup wizard walks a new instance through its first channel, monitors and agent.'
			},
			{
				title: 'Teams, single sign-on and on-call',
				text: 'Teams with rights per folder, SAML and SCIM, and built-in on-call schedules with escalation, replacements and quiet hours.'
			},
			{
				title: 'Several clients on one instance',
				text: 'MSP mode puts clients above teams: each sees only its monitors, alerts, dashboards and reports, with a monthly SLA report in its colours.'
			},
			{
				title: 'Dashboards, logs and configuration as code',
				text: 'Drag-and-drop dashboards and a metrics explorer, central logs with syslog and SNMP traps, and YAML files with a CLI and a Terraform provider.'
			},
			{
				title: 'Many more integrations',
				text: 'Cloud, network, storage, applications and databases, agentless Windows, OpenTelemetry, any Prometheus exporter, tickets and runbooks.'
			}
		]
	},
	{
		version: '0.13.6',
		date: '2026-10-25',
		highlights: [
			{
				title: 'Encrypted agent channel',
				text: 'Agents report on port 8443 with TLS 1.3, post-quantum key exchange and a certificate per machine, and only trust your server. Open 8443 next to 8080; existing agents switch by themselves once updated.'
			},
			{
				title: 'CVE & vulnerabilities',
				text: 'The new Security page lists the known CVEs of the software you monitor and says, per device, whether each one is vulnerable, patched or to check. Off by default: turn it on in Security.'
			},
			{
				title: 'Deploy to Active Directory',
				text: 'Settings → Agents → Deploy with Active Directory downloads a script that creates one group policy: every server of the chosen OU installs the agent and shows up here.'
			},
			{
				title: 'A map that explains your network',
				text: 'Proxmox clusters, backups to PBS, firewalls first, subnets and VLANs, and services hung under the VM that runs them.'
			},
			{
				title: 'Richer status pages',
				text: 'SEO settings, whole groups, public names, a 24 h to 90 day history with a day view, and past incidents.'
			}
		]
	},
	{
		version: '0.13.5',
		date: '2026-10-21',
		highlights: [
			{
				title: 'Remote sites in one step',
				text: 'Watch a remote site gives an install command that sets up a relay by itself, and any agent can be switched to relay mode from its page. Scan a site’s network or set Reached through on many monitors at once.'
			},
			{
				title: 'Dismiss an alert in one click',
				text: 'Dismiss makes an alert go away and stay quiet until the problem clears. Undo it from the Dismissed view.'
			},
			{
				title: 'Simpler setup guides',
				text: 'Each monitor type shows short steps in the product’s own web interface, the exact permissions to grant and the network ports to open.'
			},
			{
				title: 'Saved credentials stay saved',
				text: 'Editing a monitor shows the token ID or user name and keeps the saved secret when you leave it empty.'
			},
			{
				title: 'Logs, imports and the wall',
				text: 'Download DumbMonit’s logs or send them to Graylog, Loki, VictoriaLogs, Elasticsearch or syslog; import Uptime Kuma 2 from kuma.db; arrange the wall by drag and drop, with a new Text widget.'
			}
		]
	},
	{
		version: '0.13.0',
		date: '2026-10-19',
		highlights: [
			{
				title: 'Discover everything on your network',
				text: 'Scan my network now finds every device that answers and recognises over sixty products — Proxmox, NAS, firewalls, Home Assistant, Immich and more — shown as a map of addresses and their services.'
			},
			{
				title: 'Add in one click',
				text: 'Add on a discovered service opens the monitor form already filled in: only the credentials are left to type.'
			},
			{
				title: 'Scan remote sites',
				text: 'Pick a relay agent in Scan from and it scans the network it sits on, results streaming back as they come.'
			},
			{
				title: 'Clearer status page settings',
				text: 'The settings of a status page use the full width and are grouped into clearer sections.'
			}
		]
	},
	{
		version: '0.12.0',
		date: '2026-10-19',
		highlights: [
			{
				title: 'Your sites on a world map',
				text: 'Create locations in Settings, place your monitors there, and see the state of every site at a glance in Map → World. Nearby sites group together.'
			},
			{
				title: 'OPNsense live map',
				text: 'Turn on the live map of an OPNsense monitor to see where blocked and passed connections come from over the last 15 minutes, with top countries, rules and ports.'
			},
			{
				title: 'A calmer, clearer wall',
				text: 'No more carousels or tickers: problems stay readable whatever their number, and widgets sit in small, quiet bars you arrange yourself in Settings.'
			},
			{
				title: 'Status banner for your website',
				text: 'At the bottom of a status page’s settings: ready-to-copy HTML and PHP snippets and a one-click WordPress plugin download.'
			},
			{
				title: 'Signed releases',
				text: 'Every file of a release now comes with a signature you can check.'
			}
		]
	},
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
