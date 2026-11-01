/**
 * The message lookup behind `m`, without any bundler or generated code.
 *
 * Paraglide's own `m` carries the nine languages of every message in the same
 * module, so the browser downloaded all of them (about 4.5 MB of JavaScript)
 * to show one. Here each language is a module of its own (`locale-modules`),
 * only the active one is loaded, and `m.<key>` is looked up in it.
 */

/** What a message function of a language module takes. */
export type Inputs = Record<string, unknown>;

/** The functions of one language module, by the safe id Paraglide gives them. */
export type LocaleTable = Readonly<Record<string, ((inputs: Inputs) => string) | undefined>>;

/** What a caller of `m.<key>(…)` sees, like Paraglide's own signature. */
export type Message = (inputs?: Inputs, options?: { locale?: string }) => string;

const RESERVED = new Set([
	'break', 'case', 'catch', 'class', 'const', 'continue', 'debugger', 'default', 'delete', 'do',
	'else', 'export', 'extends', 'false', 'finally', 'for', 'function', 'if', 'import', 'in',
	'instanceof', 'new', 'null', 'return', 'super', 'switch', 'this', 'throw', 'true', 'try',
	'typeof', 'var', 'void', 'while', 'with', 'let', 'static', 'yield', 'await', 'enum',
	'implements', 'interface', 'package', 'private', 'protected', 'public', 'then'
]);

/**
 * The name Paraglide exports a message under in a language module (the same
 * rule as its `toSafeModuleId`): `settings.section.agents` becomes
 * `settings_section_agents`. Ordinary keys are their own id.
 */
export function toModuleId(id: string): string {
	const result = id.toLowerCase().replace(/[^a-z0-9_]/g, '_');
	if (/^[0-9]/.test(result) || RESERVED.has(result)) return `_${result}`;
	let uppercase = 0;
	for (const char of id) if (/[A-Z]/.test(char)) uppercase += 1;
	return result + (uppercase > 0 ? String(uppercase) : '');
}

/** The function of `key` in a language module, if it has one. */
export function findMessage(table: LocaleTable | undefined, key: string): ((inputs: Inputs) => string) | undefined {
	if (!table) return undefined;
	return table[key] ?? table[toModuleId(key)];
}

/**
 * `m`: reading a key gives its function, or `undefined` for a key that does not
 * exist (some callers build a key and fall back when there is none).
 *
 * Calls speak the active language. `options.locale` asks for another one, which
 * must have been loaded before (`loadMessages`); otherwise the active language
 * answers.
 */
export function createMessages(active: string, tables: ReadonlyMap<string, LocaleTable>): Record<string, Message | undefined> {
	const wrappers = new Map<string, Message>();
	const wrapperFor = (key: string): Message | undefined => {
		const known = wrappers.get(key);
		if (known) return known;
		const own = findMessage(tables.get(active), key);
		if (!own) return undefined;
		const message: Message = (inputs = {}, options = {}) => {
			const wanted = options.locale;
			if (wanted !== undefined && wanted !== active) {
				const other = findMessage(tables.get(wanted), key);
				if (other) return other(inputs);
			}
			return own(inputs);
		};
		wrappers.set(key, message);
		return message;
	};
	return new Proxy<Record<string, Message | undefined>>(Object.create(null), {
		get: (_target, key) => (typeof key === 'string' ? wrapperFor(key) : undefined),
		has: (_target, key) => typeof key === 'string' && wrapperFor(key) !== undefined
	});
}
