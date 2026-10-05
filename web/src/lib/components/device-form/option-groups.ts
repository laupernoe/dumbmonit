/**
 * Sub-headings for a long "More options" list (device edit form).
 *
 * A collector kind with many settings (Proxmox VE has ~24) reads as one
 * unbroken column of toggles. The server can name an explicit `group` per
 * option (`CollectorOption.group`, see `crates/server/src/api/collectors.rs`);
 * when it does, that is authoritative. Most kinds don't bother — for those,
 * a generic fallback buckets options by the first underscore-separated token
 * of their key (`scan_backup_storage` → `scan`), which is nameless clutter
 * most of the time but occasionally groups a few related toggles for free.
 * Grouping is skipped entirely below `MIN_OPTIONS_TO_GROUP`: a handful of
 * settings already reads fine as one list, headings would only add noise.
 */
import type { CollectorOption } from '$lib/api';

export interface OptionGroup {
	/** Empty: render with no sub-heading (grouping did not help, or wasn't needed). */
	label: string;
	options: CollectorOption[];
}

const FALLBACK_LABEL = 'More settings';
const MIN_OPTIONS_TO_GROUP = 6;

function titleCase(word: string): string {
	return word.length ? word[0].toUpperCase() + word.slice(1) : word;
}

function bucket(options: CollectorOption[], keyOf: (o: CollectorOption) => string): OptionGroup[] {
	const order: string[] = [];
	const buckets = new Map<string, CollectorOption[]>();
	for (const option of options) {
		const key = keyOf(option);
		if (!buckets.has(key)) {
			buckets.set(key, []);
			order.push(key);
		}
		buckets.get(key)!.push(option);
	}
	// The fallback bucket always reads last, however early its first member appeared.
	const ordered = order.filter((key) => key !== FALLBACK_LABEL);
	if (buckets.has(FALLBACK_LABEL)) ordered.push(FALLBACK_LABEL);
	return ordered.map((key) => ({ label: key, options: buckets.get(key)! }));
}

export function groupOptions(options: CollectorOption[]): OptionGroup[] {
	if (options.length === 0) return [];
	if (options.length < MIN_OPTIONS_TO_GROUP) return [{ label: '', options }];

	if (options.some((o) => o.group.trim())) {
		return bucket(options, (o) => o.group.trim() || FALLBACK_LABEL);
	}

	// Generic fallback: group by the key's first token, but only when at least
	// two options share it — a bucket of one is just a label for itself.
	const tokenOf = (o: CollectorOption) => o.key.split('_')[0];
	const counts = new Map<string, number>();
	for (const option of options) counts.set(tokenOf(option), (counts.get(tokenOf(option)) ?? 0) + 1);

	const groups = bucket(options, (o) => {
		const token = tokenOf(o);
		return (counts.get(token) ?? 0) >= 2 ? titleCase(token) : FALLBACK_LABEL;
	});
	// If every option ended up alone in "More settings", grouping didn't help: show one flat list.
	if (groups.length <= 1) return [{ label: '', options }];
	return groups;
}
