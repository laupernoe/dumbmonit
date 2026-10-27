/**
 * "Where is it?" — the first question of the add flow.
 *
 * The user picks a site (a location of the world map, or none) and the page
 * proposes what to add there: devices a network scan found and nobody
 * monitors yet, then the kinds that fit the site. Everything here is pure, so
 * it is tested without a browser (`tests/add-by-site.test.mjs`).
 *
 * The site's defaults travel to the form through the URL (`?location=`,
 * `?via_agent=`, `?folder=`), like the network scan's pre-fill.
 */
import type { CollectorInfo, MapLocation, NetscanDevice, NetscanSchedule, RelayAgent, Target, TargetPayload } from '#lib/api/types.js';

/** URL value and radio key of "no site". */
export const NO_SITE = 'none';

/** Kinds proposed when a site has nothing to learn from, most useful first. */
export const COMMON_KINDS = ['http', 'ping', 'tcp', 'snmp'];
/** Kinds that never make sense as a one-click suggestion (own enrolment, children of an agent). */
const NOT_SUGGESTED = new Set(['agent', 'agent_service']);
/** How many kind suggestions one site shows. */
export const MAX_KINDS = 5;

/** What a site pre-fills in a new device. */
export interface SiteDefaults {
	location_id: number | null;
	/** The relay agent that probes for the site, when there is one. */
	via_agent: number | null;
	/** Folder most of the site's devices are in; empty: none. */
	folder: string;
}

export const NO_DEFAULTS: SiteDefaults = { location_id: null, via_agent: null, folder: '' };

const same = (a: string | null | undefined, b: string | null | undefined) =>
	!!a && !!b && a.trim().toLowerCase() === b.trim().toLowerCase();

/** Ids of the devices placed at a location. */
export function memberIds(location: MapLocation): Set<number> {
	return new Set(location.members.map((member) => member.target_id));
}

/**
 * Locations to offer. In MSP mode (a client is in view) a location stays when
 * one of its devices belongs to that client, or when it holds none at all
 * (brand new); the other clients' sites are not shown.
 */
export function visibleLocations(locations: MapLocation[], targets: Target[], client: number | null): MapLocation[] {
	const sorted = [...locations].sort(
		(a, b) => Number(b.main_site) - Number(a.main_site) || a.name.localeCompare(b.name, 'en', { sensitivity: 'base' })
	);
	if (client === null) return sorted;
	const owner = new Map(targets.map((t) => [t.id, t.client_id]));
	return sorted.filter((location) => {
		if (location.members.length === 0) return true;
		return location.members.some((member) => {
			const id = owner.get(member.target_id);
			return id === undefined || id === client;
		});
	});
}

/** Relay agents that probe for a location: declared for its site, or already used by its devices. Busiest first. */
export function relaysOf(location: MapLocation | null, relays: RelayAgent[], targets: Target[]): RelayAgent[] {
	if (!location) return [];
	const members = memberIds(location);
	const used = new Set<number>();
	for (const t of targets) if (members.has(t.id) && t.via_agent !== null) used.add(t.via_agent);
	return relays
		.filter((r) => r.relay && (used.has(r.id) || (location.site !== '' && same(r.site, location.site))))
		.sort((a, b) => b.relayed - a.relayed || a.name.localeCompare(b.name, 'en', { sensitivity: 'base' }));
}

/** Location, relay and folder a new device of this site starts with. */
export function defaultsFor(location: MapLocation | null, relays: RelayAgent[], targets: Target[]): SiteDefaults {
	if (!location) return NO_DEFAULTS;
	const members = memberIds(location);
	const counts = new Map<string, number>();
	for (const t of targets) {
		if (members.has(t.id) && t.group_name) counts.set(t.group_name, (counts.get(t.group_name) ?? 0) + 1);
	}
	const folder = [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))[0]?.[0] ?? '';
	return { location_id: location.id, via_agent: relaysOf(location, relays, targets)[0]?.id ?? null, folder };
}

/**
 * Kinds worth proposing for a site: the ones its devices already use (most
 * used first), then the common ones; only kinds this server knows.
 */
export function suggestedKinds(location: MapLocation | null, targets: Target[], collectors: CollectorInfo[], max = MAX_KINDS): string[] {
	const known = new Set(collectors.map((c) => c.kind));
	const members = location ? memberIds(location) : null;
	const counts = new Map<string, number>();
	for (const t of targets) {
		if (members && !members.has(t.id)) continue;
		if (NOT_SUGGESTED.has(t.kind) || !known.has(t.kind)) continue;
		counts.set(t.kind, (counts.get(t.kind) ?? 0) + 1);
	}
	const used = [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).map(([kind]) => kind);
	const out: string[] = [];
	for (const kind of [...used, ...COMMON_KINDS]) {
		if (known.has(kind) && !out.includes(kind)) out.push(kind);
	}
	return out.slice(0, max);
}

/** Host part of a device address (`https://nas.lan:5001/x` gives `nas.lan`). */
export function hostOf(address: string): string {
	const bare = address.trim().replace(/^[a-z][a-z0-9+.-]*:\/\//i, '');
	const host = bare.split(/[/?#]/)[0] ?? '';
	if (host.startsWith('[')) return host.slice(1, host.indexOf(']')).toLowerCase();
	return host.replace(/:\d+$/, '').toLowerCase();
}

/** A device a scan found that nobody monitors, with what to create for it. */
export interface Discovered {
	device: NetscanDevice;
	label: string;
	/** Kind proposed for it. */
	kind: string;
	/** Address to monitor (an URL for `http`). */
	address: string;
	/** `snmp` needs a community: it opens the form instead of creating at once. */
	direct: boolean;
}

/** Kind and address proposed for a discovered device, from its open ports. */
export function proposalFor(device: NetscanDevice, kinds: Set<string>): { kind: string; address: string; direct: boolean } | null {
	const open = (port: number, proto: 'tcp' | 'udp' = 'tcp') => device.ports.some((p) => p.port === port && p.protocol === proto);
	if (kinds.has('snmp') && open(161, 'udp')) return { kind: 'snmp', address: device.ip, direct: false };
	if (kinds.has('http') && open(443)) return { kind: 'http', address: `https://${device.ip}`, direct: true };
	if (kinds.has('http') && open(80)) return { kind: 'http', address: `http://${device.ip}`, direct: true };
	if (kinds.has('ping')) return { kind: 'ping', address: device.ip, direct: true };
	return null;
}

/**
 * Devices the scans of a site found and no monitor covers yet. A scan belongs
 * to the site of the relay agent that ran it; the server's own scans belong
 * to the main site and to "no site".
 *
 * `siteRelays` are the relays of the chosen site, `anyRelays` every relay
 * placed at some site (so "no site" skips what a site already claims).
 */
export function discoveredFor(
	location: MapLocation | null,
	inventory: NetscanDevice[],
	schedules: NetscanSchedule[],
	siteRelays: RelayAgent[],
	anyRelays: Set<number>,
	targets: Target[],
	collectors: CollectorInfo[]
): Discovered[] {
	const monitored = new Set(targets.map((t) => hostOf(t.address)));
	const mine = new Set(siteRelays.map((r) => r.id));
	const kinds = new Set(collectors.map((c) => c.kind));
	const agentOf = new Map(schedules.map((s) => [s.id, s.agent_id]));
	const out: Discovered[] = [];
	for (const device of inventory) {
		if (!agentOf.has(device.schedule_id)) continue;
		const agent = agentOf.get(device.schedule_id) ?? null;
		const belongs =
			agent === null ? location === null || location.main_site : location !== null ? mine.has(agent) : !anyRelays.has(agent);
		if (!belongs || monitored.has(device.ip.toLowerCase())) continue;
		if (device.hostname && monitored.has(device.hostname.toLowerCase())) continue;
		const proposal = proposalFor(device, kinds);
		if (!proposal) continue;
		out.push({ device, label: device.hostname || device.ip, ...proposal });
	}
	// Unknown ones first (never acknowledged), then by address.
	return out.sort((a, b) => Number(a.device.known) - Number(b.device.known) || a.device.ip.localeCompare(b.device.ip, 'en', { numeric: true }));
}

/** Payload that creates a discovered device with the site's defaults. */
export function payloadFor(found: Discovered, defaults: SiteDefaults): TargetPayload {
	return {
		name: found.label,
		address: found.address,
		kind: found.kind,
		via_agent: defaults.via_agent,
		location_id: defaults.location_id,
		group_name: defaults.folder
	};
}

/** Query parameters that carry the site's defaults to the form. */
export function defaultsParams(defaults: SiteDefaults): Record<string, string> {
	const out: Record<string, string> = {};
	if (defaults.location_id !== null) out.location = String(defaults.location_id);
	if (defaults.via_agent !== null) out.via_agent = String(defaults.via_agent);
	if (defaults.folder) out.folder = defaults.folder;
	return out;
}
