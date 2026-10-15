import type { LocalSubnet, SubnetSource } from '#lib/api/types.js';

/**
 * Pure helpers for the device form's "Test connection" button and for the
 * network scan's default subnet. No i18n here: they return codes and the
 * components map them to messages.
 */

/** Why the test button cannot run yet, in the order a person would fix it. */
export type TestBlocker = 'address' | 'credential' | 'relay';

export interface TestState {
	address: string;
	/** Errors of the credential fields the form would send (empty: nothing missing). */
	credentialErrors: Record<string, string>;
	/** True when the device is reached through a relay agent. */
	viaRelay: boolean;
}

/** `null` when the test can run. */
export function testBlocker(state: TestState): TestBlocker | null {
	if (!state.address.trim()) return 'address';
	if (state.viaRelay) return 'relay';
	if (Object.keys(state.credentialErrors).length > 0) return 'credential';
	return null;
}

/** First non-empty message: the one reason shown next to a disabled submit. */
export function firstReason(reasons: ReadonlyArray<string | null | undefined>): string | null {
	for (const reason of reasons) if (reason) return reason;
	return null;
}

const CIDR = /^\d{1,3}(\.\d{1,3}){3}\/\d{1,2}$/;

const SOURCES: readonly SubnetSource[] = ['config', 'devices', 'server'];

/** Keeps well-formed IPv4 CIDRs, server order preserved (most reliable source first). */
export function usableSubnets(raw: unknown): LocalSubnet[] {
	const list = raw;
	if (!Array.isArray(list)) return [];
	const seen = new Set<string>();
	const out: LocalSubnet[] = [];
	for (const item of list) {
		const cidr = typeof item?.cidr === 'string' ? item.cidr.trim() : '';
		if (!CIDR.test(cidr) || seen.has(cidr)) continue;
		seen.add(cidr);
		const source: SubnetSource = SOURCES.includes(item.source) ? item.source : 'server';
		out.push({
			cidr,
			source,
			...(typeof item.interface === 'string' ? { interface: item.interface } : {})
		});
	}
	return out;
}

/** What to pre-fill: nothing once the person typed something themselves. */
export function defaultSubnet(current: string, subnets: LocalSubnet[]): string | null {
	if (current.trim()) return null;
	return subnets[0]?.cidr ?? null;
}

/** One-line summary of a successful SNMP test: `sysName — sysDescr`, trimmed. */
export function identitySummary(sysname: string | null, sysdescr: string | null, max = 120): string {
	const descr = (sysdescr ?? '').replace(/\s+/g, ' ').trim();
	const clipped = descr.length > max ? `${descr.slice(0, max - 1)}…` : descr;
	return [sysname?.trim(), clipped].filter(Boolean).join(' — ');
}
