/**
 * Words and tones for the blocklist panel. The server sends raw states and
 * counts; only their presentation is decided here. A tone always comes with
 * a word: status is never shown by colour alone.
 */
import type { BlacklistAddress, BlacklistCheck, BlacklistOverview, BlacklistProviderLink } from '#lib/api/index.js';
import type { Tone } from '#lib/ui/index.js';
import { m } from '#lib/paraglide/messages.js';

export interface Verdict {
	tone: Tone;
	word: string;
	detail?: string;
}

/** The headline of the last check. */
export function overallVerdict(view: BlacklistOverview): Verdict | null {
	if (view.sampled_at === null) return null;
	if (view.addresses_checked === null) return { tone: 'ghost', word: m.devicesb_blacklist_state_running() };
	if ((view.listed_major_lists ?? 0) > 0) return { tone: 'warning', word: m.devicesb_blacklist_state_listed() };
	if ((view.listed_lists ?? 0) > 0) return { tone: 'advisory', word: m.devicesb_blacklist_state_listed() };
	if ((view.unknown_checks ?? 0) > 0) return { tone: 'advisory', word: m.devicesb_blacklist_state_incomplete() };
	return { tone: 'signal', word: m.devicesb_blacklist_state_clean() };
}

export function checkTone(check: BlacklistCheck): Tone {
	switch (check.state) {
		case 'listed':
			return check.major ? 'warning' : 'advisory';
		case 'clean':
			return 'signal';
		case 'not_applicable':
			return 'muted';
		default:
			return 'ghost';
	}
}

export function checkWord(check: BlacklistCheck): string {
	switch (check.state) {
		case 'listed':
			return m.devicesb_blacklist_state_listed();
		case 'clean':
			return m.devicesb_blacklist_state_clean();
		case 'refused':
			return m.devicesb_blacklist_state_refused();
		case 'not_applicable':
			return m.devicesb_blacklist_state_not_applicable();
		default:
			return m.devicesb_blacklist_state_unknown();
	}
}

/** Reverse DNS of a sending address, and whether it resolves back (FCrDNS). */
export function reverseVerdict(address: BlacklistAddress): Verdict {
	if (address.fcrdns === true) return { tone: 'signal', word: m.devicesb_blacklist_reverse_ok() };
	if (address.fcrdns === false && address.ptr === null) return { tone: 'advisory', word: m.devicesb_blacklist_reverse_none() };
	if (address.fcrdns === false) return { tone: 'advisory', word: m.devicesb_blacklist_reverse_mismatch() };
	return { tone: 'ghost', word: m.devicesb_blacklist_state_unknown() };
}

/** Where a checked address comes from. */
export function sourceLabel(address: BlacklistAddress): string {
	switch (address.source) {
		case 'mx':
			return m.devicesb_blacklist_source_mx({ host: address.mx ?? '' });
		case 'domain':
			return m.devicesb_blacklist_source_domain();
		case 'extra':
			return m.devicesb_blacklist_source_extra();
		default:
			return m.devicesb_blacklist_source_input();
	}
}

/** SPF wants exactly one record, DMARC at least one; `policy` is DMARC's `p=`. */
export function authVerdict(records: number | null, policy: string | null): Verdict {
	if (records === null) return { tone: 'ghost', word: m.devicesb_blacklist_state_unknown() };
	if (records === 0) return { tone: 'advisory', word: m.devicesb_blacklist_auth_missing() };
	const detail = policy ? m.devicesb_blacklist_auth_policy({ policy }) : undefined;
	if (records > 1 && policy === null) return { tone: 'advisory', word: m.devicesb_blacklist_auth_several() };
	return { tone: 'signal', word: m.devicesb_blacklist_auth_present(), detail };
}

/** A provider link's label; an unknown kind shows its host. */
export function linkLabel(link: BlacklistProviderLink): string {
	switch (link.kind) {
		case 'postmaster':
			return m.devicesb_blacklist_link_postmaster();
		case 'guidelines':
			return m.devicesb_blacklist_link_guidelines();
		case 'contact':
			return m.devicesb_blacklist_link_contact();
		case 'snds':
			return m.devicesb_blacklist_link_snds();
		case 'delist_m365':
			return m.devicesb_blacklist_link_delist_m365();
		case 'hub':
			return m.devicesb_blacklist_link_hub();
		default:
			try {
				return new URL(link.url).host;
			} catch {
				return link.url;
			}
	}
}
