/**
 * Presentation knowledge about collector kinds.
 *
 * The list of kinds itself comes from the server; only the icon, the
 * grouping, the search words and the shortcuts into the agent are decided
 * here, with a neutral fallback for a kind this build does not know.
 */
import type { Icon as LucideIcon } from 'lucide-svelte';
import {
	Antenna,
	Container,
	DatabaseBackup,
	Disc3,
	Layers,
	PackageCheck,
	Thermometer,
	Cog,
	Archive,
	AtSign,
	Boxes,
	BrickWall,
	Cable,
	Cpu,
	Database,
	Globe,
	HardDrive,
	HeartPulse,
	LayoutGrid,
	Mail,
	MailCheck,
	Network,
	Plug,
	Radio,
	Server,
	ServerCog,
	ShieldCheck,
	ChartLine,
	Logs,
	ScrollText,
	Cloud,
	Images,
	FileText,
	Film,
	Tv,
	ShieldBan,
	BatteryCharging,
	Router,
	House,
	DatabaseZap,
	Leaf,
	Inbox,
	ShieldAlert,
	ShipWheel,
	ArchiveRestore,
	Waypoints,
	Route,
	Split,
	CalendarClock,
	Puzzle,
	BrickWallFire,
	BrickWallShield,
	CloudBackup,
	RadioTower,
	Clock
} from 'lucide-svelte';
import type { CollectorInfo } from '$lib/api';
import { isPackKind } from '$lib/api/packs';
import { isUptimeKind, PUSH_KIND } from '$lib/format';

const KIND_ICON: Record<string, typeof LucideIcon> = {
	snmp: Network,
	proxmox: Boxes,
	pbs: Archive,
	synology: HardDrive,
	agent: Cpu,
	http: Globe,
	tcp: Plug,
	dns: AtSign,
	ping: Radio,
	tls: ShieldCheck,
	ntp: Clock,
	smtp: Mail,
	postgres: Database,
	mysql: Database,
	mqtt: Antenna,
	websocket: Cable,
	push: HeartPulse,
	pdm: LayoutGrid,
	pmg: MailCheck,
	opnsense: BrickWall,
	truenas: HardDrive,
	redfish: ServerCog,
	mdaemon: Mail,
	securitygateway: MailCheck,
	victoriametrics: ChartLine,
	victorialogs: ScrollText,
	loki: Logs,
	graylog: ScrollText,
	nextcloud: Cloud,
	immich: Images,
	paperless: FileText,
	jellyfin: Film,
	plex: Tv,
	pihole: ShieldBan,
	adguard: ShieldBan,
	nut: BatteryCharging,
	mikrotik: Router,
	unifi: Router,
	homeassistant: House,
	vsphere: Layers,
	redis: DatabaseZap,
	mongodb: Leaf,
	rabbitmq: Inbox,
	crowdsec: ShieldAlert,
	traefik: Waypoints,
	caddy: Route,
	nginx: Route,
	apache: Route,
	npm: Split,
	domain: CalendarClock,
	kubernetes: ShipWheel,
	pfsense: BrickWallFire,
	unraid: HardDrive,
	veeam: CloudBackup,
	tailscale: Network,
	fortigate: BrickWallShield,
	sophos: ShieldCheck
};

export function kindIcon(kind: string): typeof LucideIcon {
	return KIND_ICON[kind] ?? (isPackKind(kind) ? Puzzle : Server);
}

/** Kinds that describe a machine the collector polls, in display order. */
const DEVICE_KINDS = ['snmp', 'proxmox', 'vsphere', 'pbs', 'pdm', 'pmg', 'synology', 'truenas', 'unraid', 'opnsense', 'pfsense', 'fortigate', 'sophos', 'unifi', 'homeassistant', 'mikrotik', 'nut', 'redfish', 'pihole', 'adguard', 'mdaemon', 'securitygateway', 'victoriametrics', 'victorialogs', 'loki', 'graylog', 'nextcloud', 'immich', 'paperless', 'jellyfin', 'plex', 'redis', 'mongodb', 'rabbitmq', 'crowdsec', 'traefik', 'caddy', 'nginx', 'apache', 'npm', 'kubernetes', 'veeam', 'tailscale'];

/** Services in display order: the everyday checks first, the specialised ones after. */
const SERVICE_KINDS = ['http', 'ping', 'tcp', 'dns', 'tls', 'ntp', 'domain', 'push', 'smtp', 'postgres', 'mysql', 'mqtt', 'websocket'];

/** A domain's registration, read over RDAP: a service, polled rarely. */
const DOMAIN_KIND = 'domain';

/**
 * Words people type that the label does not contain. The search also reads
 * the label, the kind, the summary, the examples and the group title, so a
 * kind this build does not know is still found by its own words.
 */
const KIND_KEYWORDS: Record<string, string> = {
	snmp: 'switch router nas ups printer access point wifi firewall network hardware mikrotik unifi cisco',
	proxmox: 'pve hypervisor virtual machine vm lxc container cluster virtualization',
	pbs: 'backup datastore proxmox',
	pdm: 'proxmox console cluster federation',
	pmg: 'mail email spam antivirus proxmox gateway',
	synology: 'nas storage diskstation backup hyper backup raid',
	truenas: 'nas zfs storage pool backup replication snapshot',
	opnsense: 'firewall router gateway vpn wireguard dhcp',
	redfish: 'bmc idrac ilo ipmi xclarity hardware fan psu power supply server',
	mdaemon: 'mail email server smtp imap pop3 webmail worldclient windows mdaemon',
	securitygateway: 'mail email gateway spam antivirus quarantine smtp mdaemon windows',
	victoriametrics: 'metrics tsdb time series prometheus remote write vmagent observability monitoring',
	victorialogs: 'logs log server observability monitoring syslog',
	loki: 'logs log server grafana promtail alloy observability monitoring',
	graylog: 'logs log server syslog gelf opensearch elasticsearch siem observability monitoring',
	nextcloud: 'cloud files sync owncloud office calendar contacts php self-hosted app',
	immich: 'photos pictures videos gallery google photos backup self-hosted app',
	paperless: 'documents paperless-ngx scan ocr archive pdf self-hosted app',
	jellyfin: 'media server movies tv shows music streaming emby transcode self-hosted app',
	plex: 'media server movies tv shows music streaming transcode pms self-hosted app',
	pihole: 'dns ad blocker adblock blocklist gravity ftl dhcp sinkhole privacy',
	adguard: 'dns ad blocker adblock blocklist filter adguardhome doh dot dhcp sinkhole privacy',
	nut: 'ups battery power outage upsd upsc network ups tools apc eaton cyberpower synology',
	mikrotik: 'router routeros routerboard chr switch crs hap wireless firewall network',
	unifi: 'ubiquiti wifi access point ap switch gateway udm dream machine cloud key controller network',
	homeassistant: 'home assistant hass smart home zigbee z-wave iot automation battery sensors',
	vsphere: 'vmware esxi vcenter hypervisor virtual machine vm datastore',
	redis: 'redis valkey keydb cache key value in-memory database db replication',
	mongodb: 'mongo database db nosql document replica set wiredtiger',
	rabbitmq: 'rabbit amqp mqtt message broker queue messaging erlang',
	crowdsec: 'security ips ids bouncer ban firewall fail2ban intrusion attack blocklist',
	traefik: 'reverse proxy router load balancer ingress docker kubernetes certificate acme lets encrypt 502 503',
	caddy: 'reverse proxy web server upstream load balancer certificate acme lets encrypt 502 503',
	nginx: 'web server reverse proxy stub_status nginx plus upstream connections requests',
	apache: 'web server httpd apache2 mod_status workers scoreboard busy idle lamp',
	npm: 'nginx proxy manager reverse proxy hosts certificate lets encrypt ssl jc21',
	domain: 'domain name registration expiry expiration renew registrar whois rdap hold',
	kubernetes: 'k8s k3s k0s microk8s talos rke2 cluster container orchestration pod node deployment helm',
	pfsense: 'firewall router gateway vpn openvpn ipsec dhcp netgate pfsense plus multi wan failover',
	unraid: 'nas array parity disk cache pool docker vm storage lime technology tower',
	veeam: 'backup replication vbr job restore point repository vmware hyper-v agent windows',
	tailscale: 'vpn wireguard mesh tailnet headscale subnet router exit node zero trust remote access',
	fortigate: 'fortinet fortios firewall utm ngfw ipsec vpn sd-wan ha cluster fortiguard',
	sophos: 'sophos xg xgs sfos firewall utm ipsec vpn',
	agent: 'linux windows macos mac freebsd raspberry pi server pc desktop laptop vm cpu memory ram disk network install',
	http: 'website web site url api https page endpoint uptime',
	tcp: 'port ssh smb nfs share game server socket',
	dns: 'domain name resolve record',
	ping: 'icmp host latency reachable packet loss',
	tls: 'ssl certificate https expiry expiration',
	ntp: 'time clock sync chrony ntpd stratum offset leap second sntp',
	smtp: 'mail email relay postfix',
	postgres: 'database db sql postgresql',
	mysql: 'database db sql mariadb',
	mqtt: 'iot broker home assistant zigbee mosquitto',
	websocket: 'ws socket realtime',
	push: 'heartbeat cron job script backup scheduled task dead man switch'
};

/**
 * What the agent reports once installed, each as its own entry: nobody looks
 * for "Docker containers" under "Server with agent". Every one of them leads
 * to the agent's install; `next` says what happens then. Kept in line with
 * what `crates/agent/src/collect` really reads.
 */
export interface AgentFeature {
	id: string;
	label: string;
	summary: string;
	/** Shown above the install form: how this one arrives once the agent runs. */
	next: string;
	keywords: string;
	icon: typeof LucideIcon;
}

export const AGENT_FEATURES: AgentFeature[] = [
	{
		// Not something the agent reads on its own machine: relay mode
		// (`crates/agent/src/relay.rs`) runs this server's probes from the
		// agent's network. Listed here so "remote site" finds it.
		id: 'relay',
		label: 'Another network (remote site)',
		summary: 'An agent at a second site, a client’s office or behind a NAT probes the devices there for this server. Outbound only.',
		next: 'Install the agent on any machine at the remote site, then switch it to relay mode. Add the devices of that site as usual, with their local address, and pick the agent under More options → Reached through.',
		keywords: 'relay remote site branch office second site client customer nat firewall vpn vps network probe proxy satellite outpost',
		icon: RadioTower
	},
	{
		id: 'docker',
		label: 'Docker containers',
		summary: 'State, health, restarts and image updates of every container, with restart and update from here.',
		next: 'The agent reads the containers from the Docker socket. Install it on the machine that runs Docker: they appear on that machine’s page.',
		keywords: 'docker container compose image update restart portainer',
		icon: Container
	},
	{
		id: 'plakar',
		label: 'Plakar backups',
		summary: 'Age, count and state of the snapshots in each kloset, found without any setup.',
		next: 'The agent finds Plakar and its klosets on its own. Install it on the machine that runs the backups: the Backups panel appears on its page.',
		keywords: 'plakar backup snapshot kloset restore',
		icon: DatabaseBackup
	},
	{
		id: 'backups',
		label: 'restic / Borg backups',
		summary: 'Age of the last snapshot of each repository, and whether it can still be read.',
		next: 'Install the agent on a machine that can open the repositories, then list them under “restic_repos:” or “borg_repos:” in its agent.yaml, with a password file. The password never leaves the machine.',
		keywords: 'restic borg borgbackup borgmatic backup snapshot archive repository repo',
		icon: ArchiveRestore
	},
	{
		id: 'wireguard',
		label: 'WireGuard tunnels',
		summary: 'Every peer of every interface: time since its last handshake, traffic, and tunnels gone silent.',
		next: 'The agent reads the tunnels with wg show, on Linux: install it on the machine that runs WireGuard. Peers with a persistent keepalive alert when they go silent.',
		keywords: 'wireguard wg vpn tunnel peer handshake site-to-site wg-quick wg-easy',
		icon: Waypoints
	},
	{
		id: 'hyperv',
		label: 'Hyper-V hosts',
		summary: 'Virtual machines in critical health, the host’s real CPU load with its guests, memory of each VM and virtual disk errors.',
		next: 'Install the agent on the Hyper-V host (Windows Server or Windows with the Hyper-V role): it finds the vmms service and reads Hyper-V’s performance counters on its own. The Hyper-V panel appears on that machine’s page.',
		keywords: 'hyper-v hyperv microsoft windows server virtualization virtual machine vm vhdx vmms host hypervisor',
		icon: Boxes
	},
	{
		id: 'services',
		label: 'System services',
		summary: 'systemd units, Windows services, launchd and rc.d services, running or stopped.',
		next: 'Install the agent, then name the services to watch under “services:” in its agent.yaml. Failed systemd units are reported without any list.',
		keywords: 'service systemd systemctl unit daemon windows service launchd rc.d nginx sshd',
		icon: Cog
	},
	{
		id: 'disks',
		label: 'Disk health (SMART)',
		summary: 'Wear, reallocated and pending sectors, temperature: a drive warns weeks before it fails.',
		next: 'The agent reads SMART with smartctl: install smartmontools on the machine. The installed service has the rights it needs; sleeping disks are never woken.',
		keywords: 'disk drive smart hdd ssd nvme health smartctl smartmontools wear',
		icon: Disc3
	},
	{
		id: 'zfs',
		label: 'ZFS pools',
		summary: 'Pool state, fill, device errors and the age of the last scrub.',
		next: 'The agent reads the pools with zpool, on Linux, FreeBSD or TrueNAS. Install it on the machine that holds them.',
		keywords: 'zfs zpool pool scrub openzfs storage raid',
		icon: Layers
	},
	{
		id: 'sensors',
		label: 'Temperatures and fans',
		summary: 'The machine’s own temperature probes against their critical point, and fan speeds.',
		next: 'The agent reads the probes the system exposes, on Linux, macOS and FreeBSD; fan speeds on Linux only. Windows exposes none.',
		keywords: 'temperature fan sensor hwmon heat cooling rpm',
		icon: Thermometer
	},
	{
		id: 'updates',
		label: 'Updates and reboots',
		summary: 'Pending and security updates, a reboot waiting, failed units, SELinux mode.',
		next: 'The agent reads them with dnf or apt, on Fedora, RHEL, Debian and Ubuntu families.',
		keywords: 'update upgrade patch security reboot dnf apt selinux os',
		icon: PackageCheck
	}
];

export function agentFeature(id: string | null | undefined): AgentFeature | null {
	return AGENT_FEATURES.find((f) => f.id === id) ?? null;
}

/** One entry of the picker: a kind, or a shortcut into the agent. */
export interface KindChoice {
	/** Unique: the kind, or `agent:<feature>`. */
	id: string;
	/** The kind the choice selects. */
	kind: string;
	/** The agent feature it stands for, if any. */
	feature: string | null;
	label: string;
	summary: string;
	icon: typeof LucideIcon;
	/** Added by an integration pack (`pack.<id>`), not built into the server. */
	pack: boolean;
	/** Lower-case words the search matches against. */
	haystack: string;
}

export interface KindGroup {
	id: 'devices' | 'agent' | 'services' | 'packs' | 'other';
	title: string;
	/** One line under the title saying how this group is collected. */
	hint: string;
	choices: KindChoice[];
}

function fold(text: string): string {
	return text
		.normalize('NFD')
		.replace(/[\u0300-\u036f]/g, '')
		.toLowerCase();
}

function kindChoice(c: CollectorInfo, groupTitle: string): KindChoice {
	return {
		id: c.kind,
		kind: c.kind,
		feature: null,
		label: c.label,
		summary: c.summary,
		icon: kindIcon(c.kind),
		pack: isPackKind(c.kind),
		haystack: fold(
			[c.label, c.kind, c.summary, c.examples.join(' '), KIND_KEYWORDS[c.kind] ?? (isPackKind(c.kind) ? 'pack community' : ''), groupTitle].join(' ')
		)
	};
}

/**
 * Splits the server's kinds into "Devices", "On a machine", "Services",
 * "Community packs" and "Other".
 *
 * Known device kinds keep a fixed order so the picker reads the same on every
 * instance; the agent opens its own group, followed by what it reports;
 * services are recognised by `isUptimeKind`; kinds added by an integration
 * pack (`pack.<id>`) have their own group, by label; anything else (a demo collector,
 * a kind added by a newer server) lands in "Other" untouched.
 */
export function groupCollectors(collectors: CollectorInfo[]): KindGroup[] {
	const DEVICES = 'Devices on the network';
	const AGENT = 'On a machine, with the agent';
	const SERVICES = 'Services and checks';
	const PACKS = 'Community packs';
	const OTHER = 'Other';
	const devices = collectors
		.filter((c) => DEVICE_KINDS.includes(c.kind))
		.sort((a, b) => DEVICE_KINDS.indexOf(a.kind) - DEVICE_KINDS.indexOf(b.kind))
		.map((c) => kindChoice(c, DEVICES));
	const agent = collectors.find((c) => c.kind === AGENT_KIND);
	const agentChoices: KindChoice[] = agent
		? [
				kindChoice(agent, AGENT),
				...AGENT_FEATURES.map((f) => ({
					id: `${AGENT_KIND}:${f.id}`,
					kind: AGENT_KIND,
					feature: f.id,
					label: f.label,
					summary: f.summary,
					icon: f.icon,
					pack: false,
					haystack: fold([f.label, f.summary, f.keywords, 'agent', AGENT].join(' '))
				}))
			]
		: [];
	// Heartbeats and domain registrations sit with the services: they watch
	// a job or a name, not a machine.
	const isService = (kind: string) => isUptimeKind(kind) || kind === PUSH_KIND || kind === DOMAIN_KIND;
	// Fixed order, most asked first; a service kind this build does not know goes last.
	const rank = (kind: string) => {
		const i = SERVICE_KINDS.indexOf(kind);
		return i === -1 ? SERVICE_KINDS.length : i;
	};
	const services = collectors
		.filter((c) => isService(c.kind))
		.sort((a, b) => rank(a.kind) - rank(b.kind))
		.map((c) => kindChoice(c, SERVICES));
	const packs = collectors
		.filter((c) => isPackKind(c.kind))
		.sort((a, b) => a.label.localeCompare(b.label))
		.map((c) => kindChoice(c, PACKS));
	const other = collectors
		.filter((c) => !DEVICE_KINDS.includes(c.kind) && c.kind !== AGENT_KIND && !isService(c.kind) && !isPackKind(c.kind))
		.map((c) => kindChoice(c, OTHER));
	const groups: KindGroup[] = [
		{ id: 'devices', title: DEVICES, hint: 'Read over the network, nothing to install.', choices: devices },
		{ id: 'agent', title: AGENT, hint: 'One small agent on the machine reports all of this.', choices: agentChoices },
		{ id: 'services', title: SERVICES, hint: 'Probed from this server, as a client would; heartbeats call in instead.', choices: services },
		{ id: 'packs', title: PACKS, hint: 'Added by integration packs, managed under Settings.', choices: packs },
		{ id: 'other', title: OTHER, hint: '', choices: other }
	];
	return groups.filter((group) => group.choices.length > 0);
}

/**
 * Keeps the choices matching every word of `query`, dropping empty groups.
 * An empty query keeps everything.
 */
export function filterGroups(groups: KindGroup[], query: string): KindGroup[] {
	const words = fold(query).split(/\s+/).filter(Boolean);
	if (words.length === 0) return groups;
	return groups
		.map((group) => ({ ...group, choices: group.choices.filter((c) => words.every((w) => c.haystack.includes(w))) }))
		.filter((group) => group.choices.length > 0);
}

/** The kind that enrols itself: no address form, an install command instead. */
export const AGENT_KIND = 'agent';
/** The only kind a network scan can find. */
export const SNMP_KIND = 'snmp';

/** Check intervals offered by the form. The server default is 60 s. */
export const INTERVALS = [
	{ value: 30, label: '30 s' },
	{ value: 60, label: '1 min' },
	{ value: 120, label: '2 min' },
	{ value: 300, label: '5 min' },
	{ value: 900, label: '15 min' }
] as const;

export const DEFAULT_INTERVAL = 60;

/**
 * Suggests a display name from an address: the host without scheme, port or
 * path. `https://example.org/health` becomes `example.org`.
 */
export function suggestName(address: string): string {
	const trimmed = address.trim();
	if (!trimmed) return '';
	const withoutScheme = trimmed.replace(/^[a-z][a-z0-9+.-]*:\/\//i, '');
	const host = withoutScheme.split(/[/?#]/)[0] ?? '';
	// IPv6 literals keep their brackets; anything else drops a trailing :port.
	if (host.startsWith('[')) return host.slice(0, host.indexOf(']') + 1);
	return host.replace(/:\d+$/, '') || trimmed;
}
