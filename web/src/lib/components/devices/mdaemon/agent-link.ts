/**
 * The link between a mail server monitor (MDaemon, SecurityGateway) and the
 * DumbMonit agent installed on the same Windows server.
 *
 * The queues, sessions, Windows services and disk space of those products can
 * only be read on the server itself, so the monitor is complete only with the
 * agent. The link is explicit: the monitor stores the agent's monitor id in its
 * `agent_id` setting (a collector option of `input: agent`). The helpers here
 * read it, and guess it from the address when a monitor is being added.
 */
import type { Target, TargetId } from '#lib/api/types.js';

/** The setting that holds the agent's monitor id. */
export const AGENT_TAG = 'agent_id';

/** The monitor kinds that rely on an agent on the server. */
export const MAIL_KINDS = ['mdaemon', 'securitygateway'] as const;

export function isMailKind(kind: string): boolean {
	return (MAIL_KINDS as readonly string[]).includes(kind);
}

/** `mail.example.com:444` → `mail`; an IP address stays whole. */
export function shortHost(address: string): string {
	let host = address.trim().toLowerCase();
	host = host.replace(/^[a-z]+:\/\//, '').replace(/\/.*$/, '');
	if (host.startsWith('[')) return host.slice(1, host.indexOf(']'));
	host = host.replace(/:\d+$/, '');
	if (/^\d{1,3}(\.\d{1,3}){3}$/.test(host)) return host;
	return host.split('.')[0];
}

/**
 * The agent whose host name (or address) is the one typed for the monitor, or
 * `null` when none or several match: pre-selecting the wrong server would show
 * another machine's queues, which is worse than choosing by hand.
 */
export function guessAgent(address: string, targets: Target[]): Target | null {
	const host = shortHost(address);
	if (!host) return null;
	const found = targets.filter(
		(t) => t.kind === 'agent' && (shortHost(t.name) === host || (t.address.trim() !== '' && shortHost(t.address) === host))
	);
	return found.length === 1 ? found[0] : null;
}

/** The agent the monitor names in its `agent_id` setting, when that monitor still exists. */
export function explicitAgent(device: Pick<Target, 'tags'>, targets: Target[]): Target | null {
	const id = Number(device.tags?.[AGENT_TAG]);
	if (!Number.isInteger(id) || id <= 0) return null;
	return targets.find((t) => t.kind === 'agent' && t.id === (id as TargetId)) ?? null;
}

/** An agent that has not reported for this long (seconds) is considered silent. */
export const SILENT_AFTER_SECONDS = 300;

/** True when the last report is older than [`SILENT_AFTER_SECONDS`], or there never was one. */
export function isSilent(lastSeen: Date | null, now: number = Date.now()): boolean {
	if (!lastSeen) return true;
	return (now - lastSeen.getTime()) / 1000 > SILENT_AFTER_SECONDS;
}
