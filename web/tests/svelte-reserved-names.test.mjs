// Guard: a top-level name equal to a rune that the component also uses (`state` with `$state(...)`) makes Svelte read `$state(...)`
// as a store subscription ("e.subscribe is not a function" at run time). Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { parse } from 'svelte/compiler';

const RESERVED = new Set(['state', 'derived', 'effect', 'props', 'bindable', 'inspect', 'host']);
const root = new URL('../src', import.meta.url).pathname;

function files(dir) {
	return readdirSync(dir).flatMap((name) => {
		const path = join(dir, name);
		return statSync(path).isDirectory() ? files(path) : path.endsWith('.svelte') ? [path] : [];
	});
}

function names(pattern, out = []) {
	if (!pattern) return out;
	if (pattern.type === 'Identifier') out.push(pattern.name);
	else if (pattern.type === 'ObjectPattern')
		pattern.properties.forEach((p) => names(p.type === 'RestElement' ? p.argument : p.value, out));
	else if (pattern.type === 'ArrayPattern') pattern.elements.forEach((e) => names(e, out));
	else if (pattern.type === 'AssignmentPattern') names(pattern.left, out);
	else if (pattern.type === 'RestElement') names(pattern.argument, out);
	return out;
}

function topLevel(program) {
	const out = [];
	for (let node of program.body) {
		if (node.type === 'ExportNamedDeclaration' && node.declaration) node = node.declaration;
		if (node.type === 'FunctionDeclaration' || node.type === 'ClassDeclaration') out.push(node.id?.name);
		else if (node.type === 'VariableDeclaration') node.declarations.forEach((d) => names(d.id, out));
		else if (node.type === 'ImportDeclaration') node.specifiers.forEach((s) => out.push(s.local.name));
		else if (node.type === 'TSEnumDeclaration') out.push(node.id.name);
	}
	return out;
}

test('no component declares a top-level name that shadows a rune', () => {
	const bad = [];
	for (const file of files(root)) {
		const source = readFileSync(file, 'utf8');
		const ast = parse(source, { modern: true });
		for (const script of [ast.instance, ast.module]) {
			if (!script) continue;
			for (const name of topLevel(script.content)) {
				// Harmless unless the rune of that name is used in the component too.
				if (RESERVED.has(name) && new RegExp(`\\$${name}\\b`).test(source)) bad.push(`${file.slice(root.length + 1)}: ${name}`);
			}
		}
	}
	assert.deepEqual(bad, []);
});
