/**
 * MDaemon queues, sessions and message totals, as the Windows agent reads them
 * from MDaemon's performance counters (`dumbmonit_mdaemon_queue_messages`…).
 *
 * The series belong to the agent's target, not to the `mdaemon` device: the
 * device checks the mail ports from outside, the agent reads the counters on
 * the server itself. `pickAgent` finds which agent runs on the same machine.
 */
import type { MetricSeries, Target, TargetId } from '#lib/api/types.js';
import { explicitAgent, shortHost } from './agent-link.ts';

export interface QueueStat {
	queue: string;
	messages: number;
	frozen: boolean;
}

export interface MdaemonCounters {
	queues: QueueStat[];
	sessions: { protocol: string; value: number }[];
	/** Internal servers MDaemon reports as inactive (`server_active = 0`). */
	inactive: string[];
	running: boolean | null;
	uptimeSeconds: number | null;
	/** Messages over the last 24 hours, by protocol (`increase` of the totals). */
	messages24h: { protocol: string; value: number }[];
	/** Spam, virus and DNSBL verdicts over the last 24 hours. */
	filtered24h: { filter: string; verdict: string; value: number }[];
}

export const EMPTY_COUNTERS: MdaemonCounters = {
	queues: [],
	sessions: [],
	inactive: [],
	running: null,
	uptimeSeconds: null,
	messages24h: [],
	filtered24h: []
};

/** The order an administrator reads MDaemon's queues in. */
const QUEUE_ORDER = ['inbound', 'local', 'remote', 'retry', 'bad', 'holding', 'lan', 'quarantine', 'raw'];

export const QUEUE_LABELS: Record<string, string> = {
	inbound: 'Inbound',
	local: 'Local',
	remote: 'Remote',
	retry: 'Retry',
	bad: 'Bad',
	holding: 'Holding',
	lan: 'LAN',
	quarantine: 'Quarantine',
	raw: 'RAW'
};

export const PROTOCOL_LABELS: Record<string, string> = {
	smtp_in: 'SMTP in',
	smtp_out: 'SMTP out',
	pop3_in: 'POP3 in',
	pop3_out: 'POP3 out',
	pop3: 'POP3',
	imap: 'IMAP',
	webmail: 'Webmail',
	domainpop_in: 'DomainPOP in'
};

export const SERVER_LABELS: Record<string, string> = {
	smtp: 'SMTP',
	pop3: 'POP3',
	imap: 'IMAP',
	webmail: 'Webmail',
	webadmin: 'Remote Administration',
	activesync: 'ActiveSync',
	antispam: 'AntiSpam',
	antivirus: 'AntiVirus',
	minger: 'Minger',
	multipop: 'MultiPOP',
	domainpop: 'DomainPOP'
};

/** The instant query for the gauges of one agent. */
export function gaugesQuery(agent: TargetId): string {
	return `{__name__=~"dumbmonit_mdaemon_(queue_messages|queue_frozen|sessions_active|server_active|running|uptime_seconds)", target="${agent}"}`;
}

/** The instant query for the 24-hour totals of one agent. */
export function totalsQuery(agent: TargetId): string {
	return `increase({__name__=~"dumbmonit_mdaemon_(messages_total|filtered_messages_total)", target="${agent}"}[24h])`;
}

/** Which agents report a mail server's series at all (performance counters or the agent's own probe). */
export function reportingQuery(product: string): string {
	const own = `count by (target) (dumbmonit_agent_mail_info{product="${product}"})`;
	return product === 'mdaemon' ? `${own} or count by (target) (dumbmonit_mdaemon_queue_messages)` : own;
}

/** Windows services, queue folders and disk space the agent reads for one product. */
export function agentMailQuery(agent: TargetId, product: string): string {
	return `{__name__=~"dumbmonit_agent_mail_(info|service_up|queue_files|queue_oldest_seconds|data_free_bytes|data_total_bytes)", target="${agent}", product="${product}"}`;
}

export interface WindowsService {
	name: string;
	display: string;
	/** `auto`, `manual`, `disabled` or `other`: what Windows starts it with. */
	start: string;
	running: boolean;
}

export interface AgentMail {
	/** The agent found the product on the server (it sent `info`). */
	detected: boolean;
	version: string | null;
	services: WindowsService[];
	/** Messages (`.msg` files) waiting in each queue folder. */
	files: Record<string, number>;
	/** Age in seconds of the oldest message of each non-empty queue folder. */
	oldest: Record<string, number>;
	freeBytes: number | null;
	totalBytes: number | null;
}

export const EMPTY_AGENT_MAIL: AgentMail = {
	detected: false,
	version: null,
	services: [],
	files: {},
	oldest: {},
	freeBytes: null,
	totalBytes: null
};

/** Folds the agent's own series (`dumbmonit_agent_mail_*`) into one reading. */
export function foldAgentMail(series: MetricSeries[]): AgentMail {
	const out: AgentMail = { ...EMPTY_AGENT_MAIL, services: [], files: {}, oldest: {} };
	for (const serie of series) {
		const value = last(serie);
		if (!Number.isFinite(value)) continue;
		const m = serie.metric;
		switch ((m.__name__ ?? '').replace('dumbmonit_agent_mail_', '')) {
			case 'info':
				out.detected = true;
				out.version = m.version || null;
				break;
			case 'service_up':
				if (m.service) {
					out.services.push({ name: m.service, display: m.display || m.service, start: m.start || 'other', running: value >= 1 });
				}
				break;
			case 'queue_files':
				if (m.queue) out.files[m.queue] = value;
				break;
			case 'queue_oldest_seconds':
				if (m.queue) out.oldest[m.queue] = value;
				break;
			case 'data_free_bytes':
				out.freeBytes = value;
				break;
			case 'data_total_bytes':
				out.totalBytes = value;
				break;
		}
	}
	// Stopped services first: that is what the administrator looks for.
	out.services.sort((a, b) => Number(a.running) - Number(b.running) || a.display.localeCompare(b.display, 'en'));
	return out;
}

function last(serie: MetricSeries): number {
	return Number(serie.values.at(-1)?.[1]);
}

/**
 * Folds the two queries into one reading. `increase` drops the metric name,
 * so the totals are told apart by their labels: `filter` only exists on the
 * spam/virus/DNSBL series.
 */
export function foldCounters(gauges: MetricSeries[], totals: MetricSeries[]): MdaemonCounters {
	const out: MdaemonCounters = {
		...EMPTY_COUNTERS,
		queues: [],
		sessions: [],
		inactive: [],
		messages24h: [],
		filtered24h: []
	};
	const queues = new Map<string, QueueStat>();
	const queueOf = (name: string): QueueStat => {
		let q = queues.get(name);
		if (!q) {
			q = { queue: name, messages: Number.NaN, frozen: false };
			queues.set(name, q);
		}
		return q;
	};
	for (const serie of gauges) {
		const value = last(serie);
		if (!Number.isFinite(value)) continue;
		const m = serie.metric;
		switch ((m.__name__ ?? '').replace('dumbmonit_mdaemon_', '')) {
			case 'queue_messages':
				if (m.queue) queueOf(m.queue).messages = value;
				break;
			case 'queue_frozen':
				if (m.queue) queueOf(m.queue).frozen = value >= 1;
				break;
			case 'sessions_active':
				if (m.protocol) out.sessions.push({ protocol: m.protocol, value });
				break;
			case 'server_active':
				if (m.server && value < 1) out.inactive.push(m.server);
				break;
			case 'running':
				out.running = value >= 1;
				break;
			case 'uptime_seconds':
				out.uptimeSeconds = value;
				break;
		}
	}
	for (const serie of totals) {
		const value = Math.round(last(serie));
		if (!Number.isFinite(value)) continue;
		const m = serie.metric;
		if (m.filter && m.verdict) out.filtered24h.push({ filter: m.filter, verdict: m.verdict, value });
		else if (m.protocol) out.messages24h.push({ protocol: m.protocol, value });
	}
	const rank = (name: string) => {
		const i = QUEUE_ORDER.indexOf(name);
		return i < 0 ? QUEUE_ORDER.length : i;
	};
	out.queues = [...queues.values()]
		.filter((q) => Number.isFinite(q.messages))
		.sort((a, b) => rank(a.queue) - rank(b.queue) || a.queue.localeCompare(b.queue, 'en'));
	out.sessions.sort((a, b) => a.protocol.localeCompare(b.protocol, 'en'));
	out.inactive.sort((a, b) => a.localeCompare(b, 'en'));
	out.messages24h.sort((a, b) => a.protocol.localeCompare(b.protocol, 'en'));
	out.filtered24h.sort((a, b) => a.filter.localeCompare(b.filter, 'en') || a.verdict.localeCompare(b.verdict, 'en'));
	return out;
}

/**
 * The agent of a mail server monitor, and whether the user chose it.
 *
 * The `agent_id` setting wins and is returned even before the agent reports
 * anything (the page then says so). Without it, [`pickAgent`] looks for the
 * agent among those that already report this product's series.
 */
export function resolveAgent(
	device: Target,
	targets: Target[],
	reporting: Set<TargetId>
): { agent: Target | null; explicit: boolean } {
	const chosen = explicitAgent(device, targets);
	if (chosen) return { agent: chosen, explicit: true };
	return { agent: pickAgent(device, targets, reporting), explicit: false };
}

/**
 * The agent that runs on the same machine as a mail server monitor, among the
 * agents that report its series (`reporting`), when no agent was chosen. In order:
 *
 * 1. the device's parent is that agent (set by the user on the device page);
 * 2. the device is probed through that agent in relay mode;
 * 3. the device address and the agent's name are the same host name;
 * 4. there is exactly one MDaemon device and exactly one agent reporting
 *    MDaemon counters: they can only be the same server.
 *
 * `null` when none of these holds: showing another server's queues on this
 * device would be worse than showing none.
 */
export function pickAgent(device: Target, targets: Target[], reporting: Set<TargetId>): Target | null {
	const agents = targets.filter((t) => t.kind === 'agent' && reporting.has(t.id));
	if (agents.length === 0) return null;
	const byId = (id: TargetId | null) => (id === null ? undefined : agents.find((a) => a.id === id));
	const parent = byId(device.parent_id);
	if (parent) return parent;
	const relay = byId(device.via_agent);
	if (relay) return relay;
	const host = shortHost(device.address);
	const named = agents.filter((a) => shortHost(a.name) === host);
	if (named.length === 1) return named[0];
	const devices = targets.filter((t) => t.kind === device.kind);
	if (devices.length === 1 && agents.length === 1) return agents[0];
	return null;
}
