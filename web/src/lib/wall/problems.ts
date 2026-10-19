/**
 * The wall's problem list, made to fit: identical lines folded into one
 * ("lab-immich · Client device stale ×5"), the quieter ones (suppressed by
 * their parent, snoozed, acknowledged) gathered after what is down, and the
 * probe's long error cut to what a glance needs ("No data for 15 d").
 *
 * Pure (no imports), so `npm test` runs it as is; the component says the
 * codes it returns in the active language.
 */

/** What the list needs of a "Needs you" row (`SkyRow`), whatever its kind. */
export interface ProblemInput {
	key: string;
	/** 0 down/critical, 1 advisory, 2 info, 3 building up, 4 suppressed, 5 snoozed, 6 acked. */
	rank: number;
	tone: string;
	plate: string;
	name: string;
	what: string;
	since: string | null;
	kind: string | null;
	targetId: number | null;
}

export interface ProblemLine {
	key: string;
	rank: number;
	tone: string;
	plate: string;
	/** The devices' names, once each. */
	names: string[];
	what: string;
	/** How many rows this line stands for. */
	count: number;
	/** The oldest start among them. */
	since: string | null;
	kind: string | null;
	targetIds: number[];
}

/** From this rank on, a row is something someone already knows about or cannot fix here. */
export const QUIET_RANK = 4;

/**
 * Folds the rows into lines, in reading order. A serious row folds only with
 * its exact twin (same plate, device and cause); quiet rows with the same
 * plate and cause fold whatever the device, their names listed once.
 */
export function groupProblems(rows: ProblemInput[]): ProblemLine[] {
	const lines: ProblemLine[] = [];
	const byKey = new Map<string, ProblemLine>();
	const sorted = [...rows].sort((a, b) => a.rank - b.rank);
	for (const row of sorted) {
		const quiet = row.rank >= QUIET_RANK;
		const groupKey = quiet
			? `${row.rank}\u0000${row.plate}\u0000${row.what}`
			: `${row.rank}\u0000${row.plate}\u0000${row.name}\u0000${row.what}`;
		const known = byKey.get(groupKey);
		if (known) {
			known.count += 1;
			if (!known.names.includes(row.name)) known.names.push(row.name);
			if (row.targetId !== null && !known.targetIds.includes(row.targetId)) known.targetIds.push(row.targetId);
			if (row.since && (!known.since || row.since < known.since)) known.since = row.since;
			if (known.kind !== row.kind) known.kind = known.names.length > 1 ? null : known.kind;
			continue;
		}
		const line: ProblemLine = {
			key: row.key,
			rank: row.rank,
			tone: row.tone,
			plate: row.plate,
			names: [row.name],
			what: row.what,
			count: 1,
			since: row.since,
			kind: row.kind,
			targetIds: row.targetId !== null ? [row.targetId] : []
		};
		byKey.set(groupKey, line);
		lines.push(line);
	}
	return lines;
}

/** Quiet lines counted by rank (4 suppressed, 5 snoozed, 6 acked), for the one-line summary. */
export function quietCounts(lines: ProblemLine[]): { suppressed: number; snoozed: number; acked: number } {
	const counts = { suppressed: 0, snoozed: 0, acked: 0 };
	for (const line of lines) {
		if (line.rank === 4) counts.suppressed += line.count;
		else if (line.rank === 5) counts.snoozed += line.count;
		else if (line.rank >= 6) counts.acked += line.count;
	}
	return counts;
}

export type ShortCause =
	| { code: 'no_data'; seconds: number }
	| { code: 'agent_silent'; seconds: number }
	| { code: 'text'; text: string };

const UNIT_SECONDS: Record<string, number> = { d: 86_400, h: 3600, min: 60, s: 1 };

/**
 * "45 s", "12 min", "3 h 05", "10 d 21 h" (the server's `human_duration`) in
 * seconds; null when it is not one of those.
 */
export function parseServerDuration(text: string): number | null {
	const parts = text.trim().match(/^(\d+)\s*(d|h|min|s)(?:\s+(\d+)\s*(h|min)?)?$/);
	if (!parts) return null;
	let seconds = Number(parts[1]) * UNIT_SECONDS[parts[2]];
	if (parts[3] !== undefined) {
		// "3 h 05": the second number without a unit is minutes.
		const unit = parts[4] ?? (parts[2] === 'h' ? 'min' : 'h');
		seconds += Number(parts[3]) * UNIT_SECONDS[unit];
	}
	return seconds;
}

/**
 * The probe's error, said short: the plate already says "Unreachable", so the
 * "Monitor unreachable:" prefix goes, and the two long agent messages become
 * a duration; the "(limit …)" tail goes too. Anything else stays as written,
 * its first letter up.
 */
export function shortCause(text: string): ShortCause {
	let rest = text.trim().replace(/^monitor unreachable\s*:\s*/i, '');
	const noData = rest.match(/^no measurement for (.+?)\s*\(limit [^)]*\)\s*$/i);
	if (noData) {
		const seconds = parseServerDuration(noData[1]);
		if (seconds !== null) return { code: 'no_data', seconds };
	}
	const silent = rest.match(/^the agent stopped reporting this service (\d+) s ago/i);
	if (silent) return { code: 'agent_silent', seconds: Number(silent[1]) };
	rest = rest.replace(/\s*\(limit [^)]*\)\s*$/i, '');
	return { code: 'text', text: rest.charAt(0).toUpperCase() + rest.slice(1) };
}
