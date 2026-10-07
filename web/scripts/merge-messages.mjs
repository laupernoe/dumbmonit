#!/usr/bin/env node
// Fusionne messages/fragments/<zone>.<locale>.json dans messages/<locale>.json (trié, idempotent).
//   node scripts/merge-messages.mjs [--force] [--consume]   fusion
//   node scripts/merge-messages.mjs --check                 mêmes clés et mêmes {variables} que `en`
import { readFileSync, writeFileSync, readdirSync, existsSync, unlinkSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const dir = join(root, 'messages');
const fragDir = join(dir, 'fragments');
const SCHEMA = '$schema';

const settings = JSON.parse(readFileSync(join(root, 'project.inlang/settings.json'), 'utf8'));
const locales = settings.locales;
const base = settings.baseLocale;

const readJson = (p) => JSON.parse(readFileSync(p, 'utf8'));
const vars = (v) => [...new Set([...JSON.stringify(v).matchAll(/\{([^{}\s"]+)\}/g)].map((x) => x[1]))].sort();

export function check() {
	const problems = [];
	const ref = readJson(join(dir, `${base}.json`));
	delete ref[SCHEMA];
	for (const loc of locales) {
		if (loc === base) continue;
		const f = join(dir, `${loc}.json`);
		if (!existsSync(f)) {
			problems.push(`${loc}: file missing`);
			continue;
		}
		const data = readJson(f);
		delete data[SCHEMA];
		for (const k of Object.keys(ref)) {
			if (!(k in data)) problems.push(`${loc}: missing key ${k}`);
			else if (vars(ref[k]).join() !== vars(data[k]).join())
				problems.push(`${loc}: ${k}: variables {${vars(data[k])}} differ from ${base} {${vars(ref[k])}}`);
		}
		for (const k of Object.keys(data)) if (!(k in ref)) problems.push(`${loc}: extra key ${k} (not in ${base})`);
	}
	return problems;
}

function merge({ force, consume }) {
	const files = existsSync(fragDir) ? readdirSync(fragDir).filter((f) => f.endsWith('.json')).sort() : [];
	const frags = {}; // locale -> { key: { value, file } }
	const errors = [];
	for (const f of files) {
		const m = f.match(/^(.+)\.([^.]+)\.json$/);
		const loc = m && locales.includes(m[2]) ? m[2] : null;
		if (!loc) {
			errors.push(`fragments/${f}: name must be <zone>.<locale>.json with a known locale`);
			continue;
		}
		const data = readJson(join(fragDir, f));
		delete data[SCHEMA];
		const bucket = (frags[loc] ??= {});
		for (const [k, v] of Object.entries(data)) {
			const prev = bucket[k];
			if (prev && JSON.stringify(prev.value) !== JSON.stringify(v))
				errors.push(`duplicate key ${k} (${loc}) with different texts in fragments/${prev.file} and fragments/${f}`);
			else bucket[k] = { value: v, file: f };
		}
	}
	if (errors.length) return errors;
	const skipped = [];
	for (const loc of locales) {
		const p = join(dir, `${loc}.json`);
		const cur = existsSync(p) ? readJson(p) : {};
		const out = { ...cur };
		delete out[SCHEMA];
		for (const [k, { value }] of Object.entries(frags[loc] ?? {})) {
			if (k in out && !force && JSON.stringify(out[k]) !== JSON.stringify(value)) skipped.push(`${loc}: ${k}`);
			else out[k] = value;
		}
		const sorted = { [SCHEMA]: cur[SCHEMA] ?? 'https://inlang.com/schema/inlang-message-format' };
		for (const k of Object.keys(out).sort()) sorted[k] = out[k];
		writeFileSync(p, JSON.stringify(sorted, null, '\t') + '\n');
	}
	if (skipped.length) console.warn(`kept existing text (use --force to overwrite):\n  ${skipped.join('\n  ')}`);
	if (consume) for (const f of files) unlinkSync(join(fragDir, f));
	return [];
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
	const args = new Set(process.argv.slice(2));
	if (args.has('--check')) {
		const p = check();
		if (p.length) {
			console.error(`i18n check failed (${p.length}):\n  ${p.join('\n  ')}`);
			process.exit(1);
		}
		console.log('i18n check ok');
	} else {
		const errs = merge({ force: args.has('--force'), consume: args.has('--consume') });
		if (errs.length) {
			console.error(`merge refused:\n  ${errs.join('\n  ')}`);
			process.exit(1);
		}
		console.log('messages merged');
	}
}
