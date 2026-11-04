import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const src = readFileSync(new URL('../static/sw.js', import.meta.url), 'utf8');
const body = src.match(/function shouldRenotify[\s\S]*?\n}\n/)[0];
const shouldRenotify = new Function(`${body}; return shouldRenotify;`)();

test('first display rings', () => {
	assert.equal(shouldRenotify({ tag: 'a' }, 'Critical · db', []), true);
});

test('a reminder with the same title replaces silently', () => {
	assert.equal(shouldRenotify({ tag: 'a' }, 'Critical · db', [{ title: 'Critical · db' }]), false);
});

test('an escalation (new title) rings again', () => {
	assert.equal(shouldRenotify({ tag: 'a' }, 'Critical · db', [{ title: 'Warning · db' }]), true);
});

test('a resolution or an untagged push never renotifies', () => {
	assert.equal(shouldRenotify({ tag: 'a', resolved: true }, 'Resolved · db', []), false);
	assert.equal(shouldRenotify({}, 'x', []), false);
});
