import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createMessages, findMessage, toModuleId } from '../src/lib/i18n/lookup.ts';

test('a key is exported under the safe id Paraglide gives it', () => {
	assert.equal(toModuleId('alerts_ack_button'), 'alerts_ack_button');
	assert.equal(toModuleId('settings.section.agents'), 'settings_section_agents');
	assert.equal(toModuleId('Foo'), 'foo1');
	assert.equal(toModuleId('then'), '_then');
	assert.equal(toModuleId('9lives'), '_9lives');
});

const en = {
	greet: ({ name }) => `Hello ${name}!`,
	settings_section_agents: () => 'Agents',
	only_en: () => 'English only'
};
const fr = {
	greet: ({ name }) => `Bonjour ${name} !`,
	settings_section_agents: () => 'Agents (fr)'
};

test('m speaks the active language', () => {
	const m = createMessages('fr', new Map([['fr', fr]]));
	assert.equal(m.greet({ name: 'Noe' }), 'Bonjour Noe !');
	assert.equal(m['settings.section.agents'](), 'Agents (fr)');
});

test('m has nothing for a key that does not exist', () => {
	const m = createMessages('en', new Map([['en', en]]));
	assert.equal(m.kinds_label_nope, undefined);
	assert.equal(m.then, undefined);
	assert.equal('greet' in m, true);
	assert.equal('nope' in m, false);
	assert.equal(findMessage(undefined, 'greet'), undefined);
});

test('another language can be asked for once it is loaded', () => {
	const tables = new Map([['en', en]]);
	const m = createMessages('en', tables);
	assert.equal(m.greet({ name: 'Noe' }, { locale: 'fr' }), 'Hello Noe!');
	tables.set('fr', fr);
	assert.equal(m.greet({ name: 'Noe' }, { locale: 'fr' }), 'Bonjour Noe !');
	// A language that lacks the key falls back to the active one.
	assert.equal(m.only_en({}, { locale: 'fr' }), 'English only');
});

test('every language of the project has a loader', () => {
	const settings = JSON.parse(readFileSync(new URL('../project.inlang/settings.json', import.meta.url), 'utf8'));
	const source = readFileSync(new URL('../src/lib/i18n/messages.ts', import.meta.url), 'utf8');
	for (const locale of settings.locales) {
		assert.ok(
			source.includes(`import('#lib/paraglide/messages/${locale}.js')`),
			`no loader for ${locale} in src/lib/i18n/messages.ts`
		);
	}
});
