/**
 * Pip's tips — the mascot pigeon's little speech bubble, context-aware per
 * route. Unlike `overview/tips.ts` (the "Did you know?" strip, shown inline
 * on the Overview only), these pop up uninvited, in a corner, on any page of
 * the signed-in app; so they stay short, calm, and occasionally funny rather
 * than another feature list.
 *
 * Every line here names a real capability — verified against the code it
 * describes — because a mascot that lies about the product is worse than no
 * mascot. Device-kind setup guidance is not written here: `Pip.svelte` reads
 * it straight from `GET /api/collectors` (`CollectorInfo.setup`), so a new
 * integration pack gets a tutorial tip for free.
 *
 * Shown state and "don't show again" live in this browser only, every access
 * guarded: storage can be missing, blocked, or full, and Pip just stays quiet
 * about it.
 */

export type Area = 'overview' | 'devices' | 'device' | 'alerts' | 'settings' | 'status' | 'general';

export interface Tip {
	id: string;
	/** What Pip says, one or two short sentences. */
	text: string;
	/** Button label; omitted for a tip that is just advice or a joke. */
	action?: string;
	href?: string;
	/** Only offered to admins: a viewer could not follow it through. */
	admin?: boolean;
}

export const AREA_TIPS: Record<Area, Tip[]> = {
	overview: [
		{
			id: 'briefing',
			text: 'This page is a briefing, not a dashboard: a sky, "since you last looked" in sentences, and "needs you" as tiles. No device list here on purpose — that lives on Devices.',
			action: 'See the devices',
			href: '/targets'
		},
		{
			id: 'week-ahead',
			text: '"The week ahead" shows what is due on its day — a maintenance window, a renewal. A quiet week shrinks to one line so it never begs for attention it does not need.'
		},
		{
			id: 'sky-weather',
			text: 'I live the weather with you up there: I nap on the wire on a quiet night, startle when something breaks, and loop in confetti once the sky clears. Try breaking something. (Please don’t.)'
		},
		{
			id: 'palette-anywhere',
			text: 'Ctrl K or ⌘K opens the command palette from anywhere, even here, even mid-scroll. No menu diving required.',
			action: 'Open the palette',
			href: 'palette'
		},
		{
			id: 'last-7-days',
			text: '"Last 7 days" boils the week down to three numbers: how long everything stayed up, the quietest device, and the one that paged you the most. Petty, but fair.'
		}
	],
	devices: [
		{
			id: 'rack-order',
			text: 'The rack sorts itself: trouble floats to the top, everything else keeps the order you gave it. Children sit indented under their parent, always.'
		},
		{
			id: 'folders',
			text: 'Drag a device into a folder to group it — a site, a rack, a client. Double-click a folder’s name to rename it in place.'
		},
		{
			id: 'parent-child',
			text: 'A VM or container under its hypervisor inherits trouble quietly: if the host is down, its guests are marked suppressed instead of each paging you separately.'
		},
		{
			id: 'network-scan',
			text: 'Scanning the network finds what is already answering before you type a single IP address by hand.',
			action: 'Scan my network',
			href: '/targets/new?scan=1',
			admin: true
		},
		{
			id: 'relay-agent',
			text: 'An agent can watch a whole second network for you — a client’s office, a site behind a NAT — reporting home over an outbound connection only.',
			action: 'Watch a remote site',
			href: '/targets/new?kind=agent&via=relay',
			admin: true
		}
	],
	device: [
		{
			id: 'probe-now',
			text: '"Probe now" asks this device right now, live, instead of waiting for its next scheduled check — the fastest way to find out why it has gone quiet.'
		},
		{
			id: 'silence',
			text: 'Expecting noise — a reboot, a firmware update? Silence this device for a while and its alerts stay quiet without disabling the checks themselves.'
		},
		{
			id: 'secrets',
			text: 'Credentials never come back once saved. Editing this device without touching the password field just keeps the one already stored — nothing round-trips to your screen.'
		},
		{
			id: 'timeline',
			text: 'The timeline below the faceplate is this device’s whole story: every alert that opened and closed on it, in order.'
		},
		{
			id: 'docker',
			text: 'On an agent with Docker, you can restart or update a container from right here — no terminal required.'
		}
	],
	alerts: [
		{
			id: 'suppressed',
			text: 'An alert marked "suppressed" is not being ignored — its parent is already down, and I am not paging you twice for the same outage.'
		},
		{
			id: 'learning',
			text: 'A new baseline rule stays silent for its first 14 days: it is still learning what normal looks like for this device before it dares to alert on it.'
		},
		{
			id: 'snooze',
			text: 'Snoozing an alert buys it a fixed window of quiet; it comes back on its own once that window ends — no need to remember to re-enable it.'
		},
		{
			id: 'ignore-device',
			text: 'A rule firing on a device that will just never comply? Ignore that rule for that one device — the rule still watches everywhere else.'
		},
		{
			id: 'clear-history',
			text: '"Clear all resolved" sweeps the closed alerts out of your history in one go, once you are done reading them.'
		}
	],
	settings: [
		{
			id: 'packs',
			text: 'A device type is just a YAML file — an integration pack. Install one to teach me a device I do not already know.',
			action: 'Integration packs',
			href: '/settings#packs',
			admin: true
		},
		{
			id: 'assistant',
			text: 'Claude, ChatGPT or any MCP client can read this bulletin — and with a write token, silence a device or run a probe for you.',
			action: 'Connect an assistant',
			href: '/settings#assistant',
			admin: true
		},
		{
			id: '2fa',
			text: 'Two-factor authentication is one QR code away, right in Account & security — worth it for the one account that can see every credential on this server.',
			action: 'Account & security',
			href: '/settings#security'
		},
		{
			id: 'backup',
			text: 'The backup bundle is encrypted, but it still holds every credential this server knows. Keep `/data/secret.key` as carefully as the bundle itself.',
			action: 'Back up this instance',
			href: '/settings#backup',
			admin: true
		},
		{
			id: 'wall-appearance',
			text: 'Wall mode is tuned in Appearance — including an OLED-friendly night theme for a screen that never turns off.',
			action: 'Open wall mode',
			href: '/wall'
		}
	],
	status: [
		{
			id: 'public-page',
			text: 'A status page answers "is it down?" before anyone has to ask — pick the services, share the link, done.',
			action: 'Create a status page',
			href: '/status',
			admin: true
		},
		{
			id: 'embed',
			text: 'Add `/embed` to a status page’s link and it drops into an iframe neatly — handy for an internal wiki or a company intranet.'
		},
		{
			id: 'incidents',
			text: 'Posting an incident update here keeps everyone off your chat, politely. Maintenance windows show as planned, not as a surprise outage.',
			admin: true
		},
		{
			id: 'subscribe',
			text: 'Visitors can subscribe to a status page for email updates; the unsubscribe link in every one of those emails works without them signing in anywhere.'
		},
		{
			id: 'theme-param',
			text: 'A status page accepts `?theme=light` or `?theme=dark` in its URL, so an embed can match the page that hosts it instead of guessing.'
		}
	],
	general: [
		{
			id: 'palette-everywhere',
			text: 'The keyboard shortcut for the command palette still works even with the hint gone from the nav — Ctrl K, or ⌘K on a Mac.',
			action: 'Open the palette',
			href: 'palette'
		},
		{
			id: 'docs',
			text: 'Longer explanations live on Read the Docs — this bulletin keeps its own screens short on purpose.',
			action: 'Read the docs',
			href: 'https://dumbmonit.readthedocs.io/en/latest/'
		},
		{
			id: 'security-score',
			text: 'Every device gets a security score from what it actually exposes — not a guess, a reading.',
			action: 'Open Security',
			href: '/security'
		},
		{
			id: 'about-me',
			text: 'I am a pigeon. I live in your monitoring tool. I have made my peace with this.'
		},
		{
			id: 'quiet-option',
			text: 'If I am more than you want, the × below puts me away for this visit — or tell me to stop for good and I will not bring it up again.'
		}
	]
};

/** Route → area, for everything Pip has an opinion about. `null`: no tip context (Pip stays quiet there, e.g. a public page it is never mounted on anyway). */
export function areaForPath(pathname: string): Area {
	if (pathname === '/') return 'overview';
	if (pathname === '/targets') return 'devices';
	if (pathname.startsWith('/targets/')) return 'device';
	if (pathname.startsWith('/alerts')) return 'alerts';
	if (pathname.startsWith('/settings')) return 'settings';
	if (pathname.startsWith('/status')) return 'status';
	return 'general';
}

const HIDDEN_KEY = 'dumbmonit-pip-hidden';
const NEXT_KEY = 'dumbmonit-pip-next';

export function pipHidden(): boolean {
	try {
		return localStorage.getItem(HIDDEN_KEY) === '1';
	} catch {
		return false;
	}
}

export function hidePipForGood(): void {
	try {
		localStorage.setItem(HIDDEN_KEY, '1');
	} catch {
		// This visit only, then: the storage would not take it.
	}
}

/** The tip index to show for this area right now; advancing it is separate (`storeNextTip`). */
export function takeTipIndex(area: Area): number {
	try {
		const raw = JSON.parse(localStorage.getItem(NEXT_KEY) ?? '{}');
		const value = Number(raw?.[area]);
		return Number.isInteger(value) && value >= 0 ? value : 0;
	} catch {
		return 0;
	}
}

/** Remembers the tip after `index` as the next one to show for this area. */
export function storeNextTip(area: Area, index: number): void {
	try {
		const raw = JSON.parse(localStorage.getItem(NEXT_KEY) ?? '{}');
		raw[area] = index;
		localStorage.setItem(NEXT_KEY, JSON.stringify(raw));
	} catch {
		// Nothing to do: the rotation restarts from the first tip next time.
	}
}
