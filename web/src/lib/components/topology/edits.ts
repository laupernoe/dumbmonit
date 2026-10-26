/**
 * Corrections of the topology map: the undo/redo history and the small pure
 * helpers the editor needs. No DOM, no Svelte, no Paraglide: tested on its own.
 *
 * Every step goes through the server, so undoing is itself a request (delete
 * what was created, create again what was removed). A correction the server
 * re-creates gets a new id: the history renames it everywhere, so the steps
 * further back still point at the right one.
 */
import type {
	TopologyCorrection,
	TopologyCorrectionCreated,
	TopologyCorrectionInput,
	TopologyRule,
	TopologyRuleSpec,
	TopologyRuleStatus
} from '#lib/api/index.js';

/** What the history needs from the server. */
export interface EditApi {
	create(input: TopologyCorrectionInput): Promise<TopologyCorrectionCreated>;
	remove(id: number): Promise<void>;
	saveRule(spec: TopologyRuleSpec, status: TopologyRuleStatus, evidence: number): Promise<TopologyRule>;
	deleteRule(id: number): Promise<void>;
}

interface Saved {
	id: number;
	input: TopologyCorrectionInput;
}

export type Step =
	| { kind: 'create'; id: number; input: TopologyCorrectionInput; replaced: Saved[] }
	| { kind: 'remove'; id: number; input: TopologyCorrectionInput }
	| { kind: 'rule'; id: number; spec: TopologyRuleSpec; status: TopologyRuleStatus; evidence: number }
	| { kind: 'unrule'; id: number; spec: TopologyRuleSpec; status: TopologyRuleStatus; evidence: number };

/** The request that would create `correction` again. */
export function correctionInput(correction: TopologyCorrection): TopologyCorrectionInput {
	return {
		kind: correction.kind,
		from: correction.from,
		to: correction.to,
		source: correction.source,
		value: correction.value,
		use_for_alerts: correction.use_for_alerts
	};
}

/** The scope and effect of a rule or a suggestion, without the rest. */
export function ruleSpec(rule: TopologyRuleSpec): TopologyRuleSpec {
	return {
		kind: rule.kind,
		subnet: rule.subnet,
		address: rule.address,
		target_kind: rule.target_kind,
		source: rule.source,
		parent: rule.parent
	};
}

/** Already gone on the server: undoing it again is not an error. */
function missing(error: unknown): boolean {
	return typeof error === 'object' && error !== null && 'status' in error && (error as { status: unknown }).status === 404;
}

export class EditHistory {
	done: Step[] = [];
	undone: Step[] = [];
	#api: EditApi;

	constructor(api: EditApi) {
		this.#api = api;
	}

	get canUndo(): boolean {
		return this.done.length > 0;
	}

	get canRedo(): boolean {
		return this.undone.length > 0;
	}

	async create(input: TopologyCorrectionInput): Promise<TopologyCorrectionCreated> {
		const created = await this.#api.create(input);
		this.#push({ kind: 'create', id: created.correction.id, input, replaced: saved(created.replaced) });
		return created;
	}

	async remove(correction: TopologyCorrection): Promise<void> {
		await this.#removeId(correction.id);
		this.#push({ kind: 'remove', id: correction.id, input: correctionInput(correction) });
	}

	async rule(spec: TopologyRuleSpec, status: TopologyRuleStatus, evidence = 0): Promise<TopologyRule> {
		const rule = await this.#api.saveRule(spec, status, evidence);
		this.#push({ kind: 'rule', id: rule.id, spec: ruleSpec(spec), status, evidence });
		return rule;
	}

	async unrule(rule: TopologyRule): Promise<void> {
		await this.#api.deleteRule(rule.id);
		this.#push({ kind: 'unrule', id: rule.id, spec: ruleSpec(rule), status: rule.status, evidence: rule.evidence });
	}

	/** Takes the last step back. A failure leaves the history as it was. */
	async undo(): Promise<boolean> {
		const step = this.done.at(-1);
		if (!step) return false;
		await this.#revert(step);
		this.done.pop();
		this.undone.push(step);
		return true;
	}

	async redo(): Promise<boolean> {
		const step = this.undone.at(-1);
		if (!step) return false;
		await this.#apply(step);
		this.undone.pop();
		this.done.push(step);
		return true;
	}

	#push(step: Step) {
		this.done.push(step);
		this.undone = [];
	}

	async #removeId(id: number) {
		try {
			await this.#api.remove(id);
		} catch (error) {
			if (!missing(error)) throw error;
		}
	}

	async #recreate(old: number, input: TopologyCorrectionInput): Promise<TopologyCorrectionCreated> {
		const created = await this.#api.create(input);
		this.#rename(old, created.correction.id);
		return created;
	}

	async #revert(step: Step) {
		switch (step.kind) {
			case 'create':
				await this.#removeId(step.id);
				for (const old of step.replaced) await this.#recreate(old.id, old.input);
				return;
			case 'remove':
				await this.#recreate(step.id, step.input);
				return;
			case 'rule':
				await this.#api.deleteRule(step.id);
				return;
			case 'unrule':
				this.#renameRule(step.id, (await this.#api.saveRule(step.spec, step.status, step.evidence)).id);
				return;
		}
	}

	async #apply(step: Step) {
		switch (step.kind) {
			case 'create': {
				const created = await this.#recreate(step.id, step.input);
				step.replaced = saved(created.replaced);
				return;
			}
			case 'remove':
				await this.#removeId(step.id);
				return;
			case 'rule':
				this.#renameRule(step.id, (await this.#api.saveRule(step.spec, step.status, step.evidence)).id);
				return;
			case 'unrule':
				await this.#api.deleteRule(step.id);
				return;
		}
	}

	#rename(old: number, id: number) {
		for (const step of [...this.done, ...this.undone]) {
			if (step.kind === 'create' || step.kind === 'remove') {
				if (step.id === old) step.id = id;
			}
			if (step.kind === 'create') for (const r of step.replaced) if (r.id === old) r.id = id;
		}
	}

	#renameRule(old: number, id: number) {
		for (const step of [...this.done, ...this.undone]) {
			if ((step.kind === 'rule' || step.kind === 'unrule') && step.id === old) step.id = id;
		}
	}
}

function saved(list: TopologyCorrection[]): Saved[] {
	return list.map((c) => ({ id: c.id, input: correctionInput(c) }));
}

export interface Pickable {
	id: string;
	name: string;
	kind: string;
}

/** Nodes matching a search, by name then type, closest first; never `except`. */
export function searchNodes<T extends Pickable>(nodes: Iterable<T>, query: string, except: string, limit = 8): T[] {
	const q = query.trim().toLowerCase();
	const found: { node: T; rank: number }[] = [];
	for (const node of nodes) {
		if (node.id === except) continue;
		const name = node.name.toLowerCase();
		const rank = !q ? 2 : name.startsWith(q) ? 0 : name.includes(q) ? 1 : node.kind.toLowerCase().includes(q) ? 2 : -1;
		if (rank >= 0) found.push({ node, rank });
	}
	found.sort((a, b) => a.rank - b.rank || a.node.name.localeCompare(b.node.name, 'en', { sensitivity: 'base', numeric: true }));
	return found.slice(0, limit).map((f) => f.node);
}

/** Both ends are devices: a re-parenting can also be their declared parent (alerts). */
export function canUseForAlerts(child: string, parent: string): boolean {
	return /^t\d+$/.test(child) && /^t\d+$/.test(parent);
}

/** Link types a correction can draw, in the order the editor offers them. */
export const LINK_KINDS = ['manual', 'lldp', 'hosted', 'hypervisor', 'backup', 'samehost', 'relay', 'firewall'] as const;
